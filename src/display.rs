use core::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{MAX_DISPLAY_RELEASE_BYTES, SUPPORTED_DISPLAY_DOCUMENT_VERSION};

pub const DISPLAY_IMAGE_FORMAT: &str = "mono1-msb";
// Row-major 400 x 300 source frame. The renderer converts this to ST7305 RAM layout.
pub const DISPLAY_WIDTH: usize = 400;
pub const DISPLAY_HEIGHT: usize = 300;
pub const DISPLAY_IMAGE_BYTES: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT / 8;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayRelease {
    pub release_id: String,
    pub document_version: u16,
    pub active_page_id: String,
    pub pages: Vec<DisplayPage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayPage {
    pub page_id: String,
    pub image_format: String,
    pub image_width: usize,
    pub image_height: usize,
    pub image_url: String,
    pub image_sha256: String,
    pub image_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayReleaseError {
    UnsupportedDocumentVersion,
    UnsupportedImageFormat,
    InvalidImageDimensions,
    InvalidReleaseId,
    InsecureImageUrl,
    InvalidHash,
    ImageTooLarge,
    EmptyPageId,
    MissingActivePage,
    SystemPageNotLast,
    ContentHashMismatch,
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
            return Err(DisplayReleaseError::UnsupportedDocumentVersion);
        }
        if !is_safe_id(&self.release_id) {
            return Err(DisplayReleaseError::InvalidReleaseId);
        }
        if self.active_page_id.is_empty() {
            return Err(DisplayReleaseError::EmptyPageId);
        }
        if self.pages.is_empty()
            || !self
                .pages
                .iter()
                .any(|page| page.page_id == self.active_page_id)
        {
            return Err(DisplayReleaseError::MissingActivePage);
        }
        for page in &self.pages {
            page.validate_metadata()?;
        }
        if self
            .pages
            .iter()
            .position(|page| page.page_id == "system")
            .is_some_and(|index| index + 1 != self.pages.len())
        {
            return Err(DisplayReleaseError::SystemPageNotLast);
        }
        Ok(())
    }

    pub fn page(&self, page_id: &str) -> Option<&DisplayPage> {
        self.pages.iter().find(|page| page.page_id == page_id)
    }
}

impl DisplayPage {
    pub fn validate_metadata(&self) -> Result<(), DisplayReleaseError> {
        if !is_safe_id(&self.page_id) {
            return Err(DisplayReleaseError::EmptyPageId);
        }
        if self.image_format != DISPLAY_IMAGE_FORMAT {
            return Err(DisplayReleaseError::UnsupportedImageFormat);
        }
        if self.image_width != DISPLAY_WIDTH || self.image_height != DISPLAY_HEIGHT {
            return Err(DisplayReleaseError::InvalidImageDimensions);
        }
        if !self.image_url.starts_with("https://") {
            return Err(DisplayReleaseError::InsecureImageUrl);
        }
        if self.image_sha256.len() != 64
            || !self
                .image_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(DisplayReleaseError::InvalidHash);
        }
        if self.image_bytes != DISPLAY_IMAGE_BYTES || self.image_bytes > MAX_DISPLAY_RELEASE_BYTES {
            return Err(DisplayReleaseError::ImageTooLarge);
        }
        Ok(())
    }

    pub fn validate_image(&self, image: &[u8]) -> Result<(), DisplayReleaseError> {
        self.validate_metadata()?;
        if image.len() != self.image_bytes {
            return Err(DisplayReleaseError::ContentHashMismatch);
        }
        let actual_hash = hex_lower(&Sha256::digest(image));
        if actual_hash != self.image_sha256.to_ascii_lowercase() {
            return Err(DisplayReleaseError::ContentHashMismatch);
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

pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

pub trait DisplayCache {
    type Error;

    fn current_release(&self) -> Result<Option<DisplayRelease>, Self::Error>;
    fn previous_release(&self) -> Result<Option<DisplayRelease>, Self::Error>;
    fn contains_page(&self, image_sha256: &str) -> Result<bool, Self::Error>;
    fn read_page(&self, image_sha256: &str) -> Result<Option<Vec<u8>>, Self::Error>;
    /// Persist each new resource to a temporary key, verify its hash, then atomically
    /// update the active manifest. The prior complete release remains recoverable.
    fn commit_release(
        &mut self,
        release: &DisplayRelease,
        pages: &[(DisplayPage, Vec<u8>)],
    ) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(page_id: &str, image: &[u8]) -> DisplayPage {
        DisplayPage {
            page_id: page_id.to_owned(),
            image_format: DISPLAY_IMAGE_FORMAT.to_owned(),
            image_width: DISPLAY_WIDTH,
            image_height: DISPLAY_HEIGHT,
            image_url: format!("https://console.example/releases/{page_id}.bin"),
            image_sha256: hex_lower(&Sha256::digest(image)),
            image_bytes: image.len(),
        }
    }

    #[test]
    fn accepts_all_cached_pages() {
        let image = &[0x55; DISPLAY_IMAGE_BYTES];
        let release = DisplayRelease {
            release_id: "release_20260811".to_owned(),
            document_version: 1,
            active_page_id: "usage".to_owned(),
            pages: vec![page("usage", image), page("alert", image)],
        };
        assert_eq!(release.validate_metadata(), Ok(()));
        assert_eq!(release.page("usage").unwrap().validate_image(image), Ok(()));
    }

    #[test]
    fn rejects_invalid_page_before_cache_commit() {
        let image = &[0x55; DISPLAY_IMAGE_BYTES];
        let mut invalid = page("usage", image);
        invalid.image_width = 300;
        assert_eq!(
            invalid.validate_metadata(),
            Err(DisplayReleaseError::InvalidImageDimensions)
        );
    }

    #[test]
    fn rejects_a_system_page_before_the_final_indicator_position() {
        let image = &[0x55; DISPLAY_IMAGE_BYTES];
        let release = DisplayRelease {
            release_id: "release_20260811".to_owned(),
            document_version: 1,
            active_page_id: "usage".to_owned(),
            pages: vec![page("system", image), page("usage", image)],
        };
        assert_eq!(
            release.validate_metadata(),
            Err(DisplayReleaseError::SystemPageNotLast)
        );
    }
}
