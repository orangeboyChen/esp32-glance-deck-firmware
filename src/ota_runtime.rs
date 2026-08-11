use sha2::{Digest, Sha256};

use crate::{
    mqtt::{Ota_command, Ota_phase, Ota_state},
    ota::Ota_policy,
    ota_manifest::Ota_manifest,
};

pub const MAX_OTA_MANIFEST_BYTES: usize = 8 * 1024;
pub const MAX_OTA_IMAGE_BYTES: usize = 4 * 1024 * 1024;

pub trait OtaTransport {
    type Error;

    fn fetch_manifest(&mut self, url: &str, max_bytes: usize) -> Result<Vec<u8>, Self::Error>;
    fn stream_image(
        &mut self,
        url: &str,
        max_bytes: usize,
        on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), Self::Error>,
    ) -> Result<usize, Self::Error>;
}

pub trait OtaImageWriter {
    type Error;

    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), Self::Error>;
    fn finish(self) -> Result<(), Self::Error>;
}

pub trait OtaReporter {
    fn report(&mut self, state: Ota_state);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OtaRunError<E> {
    InvalidCommand(&'static str),
    PowerUnsafe,
    Transport(E),
    ManifestJson,
    Manifest(&'static str),
    Writer,
    ImageTooLarge,
    ImageEmpty,
    ImageHash,
}

/// Runs one authenticated OTA job. The processor never reboots itself: the
/// caller publishes `rebooting`, commits the inactive partition, then runs
/// the platform-specific reboot/health handshake.
pub struct OtaProcessor {
    pub board_model: String,
    pub public_key: [u8; 32],
}

impl OtaProcessor {
    pub fn run<T, W, R>(
        &self,
        command: &Ota_command,
        policy: &mut Ota_policy,
        transport: &mut T,
        writer: W,
        reporter: &mut R,
    ) -> Result<(), OtaRunError<T::Error>>
    where
        T: OtaTransport,
        W: OtaImageWriter,
        R: OtaReporter,
    {
        command.validate().map_err(OtaRunError::InvalidCommand)?;
        if !policy.can_start() {
            return Err(OtaRunError::PowerUnsafe);
        }
        reporter.report(Ota_state {
            job_id: command.job_id.clone(),
            phase: Ota_phase::Downloading,
            error_message: None,
        });
        let manifest_bytes = transport
            .fetch_manifest(&command.manifest_url, MAX_OTA_MANIFEST_BYTES)
            .map_err(OtaRunError::Transport)?;
        let manifest: Ota_manifest =
            serde_json::from_slice(&manifest_bytes).map_err(|_| OtaRunError::ManifestJson)?;
        manifest
            .validate(&self.board_model, &command.image_sha256)
            .map_err(OtaRunError::Manifest)?;
        if manifest.version != command.version {
            return Err(OtaRunError::Manifest("ota_manifest_version_mismatch"));
        }
        policy
            .transition(Ota_phase::Verifying)
            .map_err(OtaRunError::Manifest)?;
        reporter.report(Ota_state {
            job_id: command.job_id.clone(),
            phase: Ota_phase::Verifying,
            error_message: None,
        });
        manifest
            .verify_signature(&self.public_key)
            .map_err(OtaRunError::Manifest)?;

        let mut writer = writer;
        let mut hasher = Sha256::new();
        let mut total = 0usize;
        let mut image_too_large = false;
        let mut writer_failed = false;
        let image_url = manifest.image_url.clone();
        transport
            .stream_image(&image_url, MAX_OTA_IMAGE_BYTES, &mut |chunk| {
                if chunk.is_empty() {
                    return Ok(());
                }
                let Some(next_total) = total.checked_add(chunk.len()) else {
                    image_too_large = true;
                    return Ok(());
                };
                total = next_total;
                if total > MAX_OTA_IMAGE_BYTES {
                    image_too_large = true;
                    return Ok(());
                }
                hasher.update(chunk);
                if writer.write_chunk(chunk).is_err() {
                    writer_failed = true;
                }
                Ok(())
            })
            .map_err(OtaRunError::Transport)?;
        if image_too_large {
            return Err(OtaRunError::ImageTooLarge);
        }
        if writer_failed {
            return Err(OtaRunError::Writer);
        }
        if total == 0 {
            return Err(OtaRunError::ImageEmpty);
        }
        if hex::encode(hasher.finalize()) != manifest.image_sha256.to_ascii_lowercase() {
            return Err(OtaRunError::ImageHash);
        }
        writer.finish().map_err(|_| OtaRunError::Writer)?;
        policy
            .transition(Ota_phase::Rebooting)
            .map_err(OtaRunError::Manifest)?;
        reporter.report(Ota_state {
            job_id: command.job_id.clone(),
            phase: Ota_phase::Rebooting,
            error_message: None,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use sha2::Digest;

    struct Transport {
        manifest: Vec<u8>,
        image: Vec<u8>,
    }
    impl OtaTransport for Transport {
        type Error = &'static str;
        fn fetch_manifest(&mut self, _: &str, max: usize) -> Result<Vec<u8>, Self::Error> {
            if self.manifest.len() > max {
                Err("manifest_large")
            } else {
                Ok(self.manifest.clone())
            }
        }
        fn stream_image(
            &mut self,
            _: &str,
            max: usize,
            callback: &mut dyn FnMut(&[u8]) -> Result<(), Self::Error>,
        ) -> Result<usize, Self::Error> {
            if self.image.len() > max {
                return Err("image_large");
            }
            for chunk in self.image.chunks(3) {
                callback(chunk)?;
            }
            Ok(self.image.len())
        }
    }
    #[derive(Default)]
    struct Writer {
        bytes: Vec<u8>,
        finished: bool,
    }
    impl OtaImageWriter for Writer {
        type Error = ();
        fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ()> {
            self.bytes.extend_from_slice(chunk);
            Ok(())
        }
        fn finish(mut self) -> Result<(), ()> {
            self.finished = true;
            Ok(())
        }
    }
    #[derive(Default)]
    struct Reporter {
        phases: Vec<Ota_phase>,
    }
    impl OtaReporter for Reporter {
        fn report(&mut self, state: Ota_state) {
            self.phases.push(state.phase);
        }
    }

    fn fixture() -> (Ota_command, Transport, [u8; 32]) {
        let image = b"firmware-image".to_vec();
        let key = SigningKey::from_bytes(&[7; 32]);
        let hash = hex::encode(Sha256::digest(&image));
        let mut manifest = Ota_manifest {
            board_model: "ESP32-S3-RLCD-4.2".into(),
            version: "1.2.3".into(),
            image_url: "https://example.test/image.bin".into(),
            image_sha256: hash.clone(),
            signature: String::new(),
        };
        manifest.signature =
            hex::encode(key.sign(manifest.canonical_payload().as_bytes()).to_bytes());
        let bytes = serde_json::to_vec(&manifest).unwrap();
        (
            Ota_command {
                job_id: "job".into(),
                nonce: "nonce".into(),
                version: "1.2.3".into(),
                manifest_url: "https://example.test/manifest.json".into(),
                image_sha256: hash,
            },
            Transport {
                manifest: bytes,
                image,
            },
            key.verifying_key().to_bytes(),
        )
    }

    #[test]
    fn runs_authenticated_stream_and_reports_phases() {
        let (command, mut transport, key) = fixture();
        let mut policy = Ota_policy::new(true, None);
        let mut reporter = Reporter::default();
        let result = OtaProcessor {
            board_model: "ESP32-S3-RLCD-4.2".into(),
            public_key: key,
        }
        .run(
            &command,
            &mut policy,
            &mut transport,
            Writer::default(),
            &mut reporter,
        );
        assert_eq!(result, Ok(()));
        assert_eq!(
            reporter.phases,
            vec![
                Ota_phase::Downloading,
                Ota_phase::Verifying,
                Ota_phase::Rebooting
            ]
        );
    }

    #[test]
    fn rejects_unsafe_power_before_network() {
        let (command, mut transport, key) = fixture();
        let mut policy = Ota_policy::new(false, Some(29));
        let mut reporter = Reporter::default();
        assert_eq!(
            OtaProcessor {
                board_model: "ESP32-S3-RLCD-4.2".into(),
                public_key: key
            }
            .run(
                &command,
                &mut policy,
                &mut transport,
                Writer::default(),
                &mut reporter
            ),
            Err(OtaRunError::PowerUnsafe)
        );
        assert!(reporter.phases.is_empty());
    }
}
