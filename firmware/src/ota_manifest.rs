use sha2::{Digest, Sha256};

use crate::display::hex_lower;

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ota_manifest {
    pub board_model: String,
    pub version: String,
    pub image_url: String,
    pub image_sha256: String,
    pub signature: String,
}

impl Ota_manifest {
    pub fn validate(&self, expected_board: &str, expected_hash: &str) -> Result<(), &'static str> {
        if self.board_model != expected_board || self.version.is_empty() || self.version.len() > 64
        {
            return Err("ota_manifest_identity_invalid");
        }
        if !self.image_url.starts_with("https://") {
            return Err("ota_manifest_url_invalid");
        }
        if self.image_sha256.len() != 64
            || !self
                .image_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self.image_sha256 != expected_hash
        {
            return Err("ota_manifest_hash_invalid");
        }
        if self.signature.is_empty() {
            return Err("ota_manifest_signature_missing");
        }
        Ok(())
    }

    pub fn verify_image(&self, image: &[u8]) -> Result<(), &'static str> {
        (hex_lower(&Sha256::digest(image)) == self.image_sha256.to_ascii_lowercase())
            .then_some(())
            .ok_or("ota_image_hash_invalid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_manifest_identity_and_image_hash() {
        let image = b"firmware";
        let manifest = Ota_manifest {
            board_model: "ESP32-S3-RLCD-4.2".into(),
            version: "1.2.3".into(),
            image_url: "https://example.test/firmware.bin".into(),
            image_sha256: hex_lower(&Sha256::digest(image)),
            signature: "signed".into(),
        };
        assert_eq!(
            manifest.validate("ESP32-S3-RLCD-4.2", &manifest.image_sha256),
            Ok(())
        );
        assert_eq!(manifest.verify_image(image), Ok(()));
        assert_eq!(
            manifest.verify_image(b"tampered"),
            Err("ota_image_hash_invalid")
        );
    }
}
