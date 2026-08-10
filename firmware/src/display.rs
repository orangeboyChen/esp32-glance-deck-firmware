use core::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{MAX_DISPLAY_RELEASE_BYTES, SUPPORTED_DISPLAY_DOCUMENT_VERSION};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayRelease {
    pub release_id: String,
    pub document_version: u16,
    pub image_url: String,
    pub image_sha256: String,
    pub image_bytes: usize,
    pub active_page_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayReleaseError {
    Unsupported_document_version,
    Invalid_release_id,
    Insecure_image_url,
    Invalid_hash,
    Image_too_large,
    Empty_page_id,
    Content_hash_mismatch,
}

impl fmt::Display for DisplayReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DisplayReleaseError {}

impl DisplayRelease {
    pub fn validate_metadata(&self) -> Result<(), DisplayReleaseError> {
        if self.document_version != SUPPORTED_DISPLAY_DOCUMENT_VERSION {
            return Err(DisplayReleaseError::Unsupported_document_version);
        }
        if !is_safe_id(&self.release_id) {
            return Err(DisplayReleaseError::Invalid_release_id);
        }
        if !self.image_url.starts_with("https://") {
            return Err(DisplayReleaseError::Insecure_image_url);
        }
        if self.image_sha256.len() != 64
            || !self
                .image_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(DisplayReleaseError::Invalid_hash);
        }
        if self.image_bytes == 0 || self.image_bytes > MAX_DISPLAY_RELEASE_BYTES {
            return Err(DisplayReleaseError::Image_too_large);
        }
        if self.active_page_id.is_empty() {
            return Err(DisplayReleaseError::Empty_page_id);
        }
        Ok(())
    }

    pub fn validate_image(&self, image: &[u8]) -> Result<(), DisplayReleaseError> {
        self.validate_metadata()?;
        if image.len() != self.image_bytes {
            return Err(DisplayReleaseError::Content_hash_mismatch);
        }
        let digest = Sha256::digest(image);
        let actual_hash = hex_lower(&digest);
        if actual_hash != self.image_sha256.to_ascii_lowercase() {
            return Err(DisplayReleaseError::Content_hash_mismatch);
        }
        Ok(())
    }
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

pub trait Display_cache {
    type Error;

    fn current_release(&self) -> Result<Option<DisplayRelease>, Self::Error>;
    fn replace(&mut self, release: &DisplayRelease, image: &[u8]) -> Result<(), Self::Error>;
    fn read_image(&self, release_id: &str) -> Result<Option<Vec<u8>>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(image: &[u8]) -> DisplayRelease {
        DisplayRelease {
            release_id: "release_20260811".to_owned(),
            document_version: SUPPORTED_DISPLAY_DOCUMENT_VERSION,
            image_url: "https://console.example/releases/1.png".to_owned(),
            image_sha256: hex_lower(&Sha256::digest(image)),
            image_bytes: image.len(),
            active_page_id: "usage".to_owned(),
        }
    }

    #[test]
    fn accepts_matching_image() {
        let image = b"valid display resource";
        assert_eq!(release(image).validate_image(image), Ok(()));
    }

    #[test]
    fn preserves_cached_image_when_download_is_wrong() {
        let image = b"valid display resource";
        assert_eq!(
            release(image).validate_image(b"invalid"),
            Err(DisplayReleaseError::Content_hash_mismatch)
        );
    }

    #[test]
    fn rejects_invalid_release_metadata() {
        let image = b"valid";
        let mut candidate = release(image);
        candidate.document_version = 2;
        assert_eq!(
            candidate.validate_metadata(),
            Err(DisplayReleaseError::Unsupported_document_version)
        );
        candidate.document_version = SUPPORTED_DISPLAY_DOCUMENT_VERSION;
        candidate.release_id = "bad id".to_owned();
        assert_eq!(
            candidate.validate_metadata(),
            Err(DisplayReleaseError::Invalid_release_id)
        );
        candidate.release_id = "valid".to_owned();
        candidate.image_url = "http://example.test/image".to_owned();
        assert_eq!(
            candidate.validate_metadata(),
            Err(DisplayReleaseError::Insecure_image_url)
        );
        candidate.image_url = "https://example.test/image".to_owned();
        candidate.image_sha256 = "wrong".to_owned();
        assert_eq!(
            candidate.validate_metadata(),
            Err(DisplayReleaseError::Invalid_hash)
        );
        candidate.image_sha256 = hex_lower(&Sha256::digest(image));
        candidate.image_bytes = 0;
        assert_eq!(
            candidate.validate_metadata(),
            Err(DisplayReleaseError::Image_too_large)
        );
        candidate.image_bytes = image.len();
        candidate.active_page_id.clear();
        assert_eq!(
            candidate.validate_metadata(),
            Err(DisplayReleaseError::Empty_page_id)
        );
    }
}
