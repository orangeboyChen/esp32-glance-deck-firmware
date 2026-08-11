use std::{
    collections::VecDeque,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::display::{DisplayCache, DisplayPage, DisplayRelease, DISPLAY_IMAGE_BYTES};

pub const MAX_CACHED_PAGE_COUNT: usize = 8;

#[derive(Debug)]
pub enum FlashCacheError {
    Io(io::Error),
    Codec(serde_json::Error),
    InvalidHash,
    InvalidFrame,
}

impl std::fmt::Display for FlashCacheError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "flash cache I/O error: {error}"),
            Self::Codec(error) => write!(formatter, "flash cache metadata error: {error}"),
            Self::InvalidHash => formatter.write_str("flash cache hash is invalid"),
            Self::InvalidFrame => formatter.write_str("flash cache frame is invalid"),
        }
    }
}

impl std::error::Error for FlashCacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Codec(error) => Some(error),
            Self::InvalidHash | Self::InvalidFrame => None,
        }
    }
}

impl From<io::Error> for FlashCacheError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for FlashCacheError {
    fn from(error: serde_json::Error) -> Self {
        Self::Codec(error)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CacheIndex {
    current_release: Option<DisplayRelease>,
    previous_release: Option<DisplayRelease>,
    page_hashes: VecDeque<String>,
}

/// A persistent display cache with a strict Flash capacity and a one-frame RAM
/// read limit. The caller never receives more than a 15,000-byte ST7305 frame.
pub struct FlashDisplayCache {
    root: PathBuf,
    index: CacheIndex,
}

impl FlashDisplayCache {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, FlashCacheError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let index_path = root.join("index.json");
        let index = match fs::read(&index_path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => CacheIndex::default(),
            Err(error) => return Err(error.into()),
        };
        Ok(Self { root, index })
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn page_path(&self, hash: &str) -> Result<PathBuf, FlashCacheError> {
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(FlashCacheError::InvalidHash);
        }
        Ok(self.root.join(hash))
    }

    fn save_index(&self) -> Result<(), FlashCacheError> {
        let temporary = self.root.join("index.next");
        fs::write(&temporary, serde_json::to_vec(&self.index)?)?;
        fs::rename(temporary, self.index_path())?;
        Ok(())
    }

    fn touch(&mut self, hash: &str) {
        self.index.page_hashes.retain(|candidate| candidate != hash);
        self.index.page_hashes.push_back(hash.to_owned());
    }

    fn evict_unreferenced(&mut self) -> Result<(), FlashCacheError> {
        while self.index.page_hashes.len() > MAX_CACHED_PAGE_COUNT {
            let Some(hash) = self.index.page_hashes.pop_front() else {
                break;
            };
            let _ = fs::remove_file(self.page_path(&hash)?);
        }
        Ok(())
    }
}

impl DisplayCache for FlashDisplayCache {
    type Error = FlashCacheError;

    fn current_release(&self) -> Result<Option<DisplayRelease>, Self::Error> {
        Ok(self.index.current_release.clone())
    }

    fn previous_release(&self) -> Result<Option<DisplayRelease>, Self::Error> {
        Ok(self.index.previous_release.clone())
    }

    fn contains_page(&self, image_sha256: &str) -> Result<bool, Self::Error> {
        Ok(self.page_path(image_sha256)?.is_file())
    }

    fn read_page(&self, image_sha256: &str) -> Result<Option<Vec<u8>>, Self::Error> {
        let path = self.page_path(image_sha256)?;
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if bytes.len() != DISPLAY_IMAGE_BYTES {
            return Err(FlashCacheError::InvalidFrame);
        }
        Ok(Some(bytes))
    }

    fn commit_release(
        &mut self,
        release: &DisplayRelease,
        pages: &[(DisplayPage, Vec<u8>)],
    ) -> Result<(), Self::Error> {
        for (page, frame) in pages {
            if frame.len() != DISPLAY_IMAGE_BYTES {
                return Err(FlashCacheError::InvalidFrame);
            }
            let destination = self.page_path(&page.image_sha256)?;
            let temporary = self.root.join(format!("{}.next", page.image_sha256));
            fs::write(&temporary, frame)?;
            fs::rename(temporary, destination)?;
            self.touch(&page.image_sha256);
        }
        self.evict_unreferenced()?;
        self.index.previous_release = self.index.current_release.replace(release.clone());
        self.save_index()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_FORMAT, DISPLAY_WIDTH};

    fn page(frame: &[u8]) -> DisplayPage {
        DisplayPage {
            page_id: "usage".to_owned(),
            image_format: DISPLAY_IMAGE_FORMAT.to_owned(),
            image_width: DISPLAY_WIDTH,
            image_height: DISPLAY_HEIGHT,
            image_url: "https://console.example/image".to_owned(),
            image_sha256: hex::encode(Sha256::digest(frame)),
            image_bytes: DISPLAY_IMAGE_BYTES,
        }
    }

    fn test_root() -> PathBuf {
        static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "glance-deck-cache-{}-{}-{}",
            std::process::id(),
            NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn persists_a_verified_frame_and_release_metadata() {
        let root = test_root();
        let frame = vec![0x55; DISPLAY_IMAGE_BYTES];
        let page = page(&frame);
        let release = DisplayRelease {
            release_id: "release-1".to_owned(),
            document_version: 1,
            active_page_id: "usage".to_owned(),
            pages: vec![page.clone()],
        };
        let mut cache = FlashDisplayCache::open(&root).unwrap();
        cache
            .commit_release(&release, &[(page.clone(), frame.clone())])
            .unwrap();
        drop(cache);
        let cache = FlashDisplayCache::open(&root).unwrap();
        assert_eq!(cache.current_release().unwrap(), Some(release));
        assert_eq!(cache.read_page(&page.image_sha256).unwrap(), Some(frame));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_non_frame_files_without_loading_them() {
        let root = test_root();
        let cache = FlashDisplayCache::open(&root).unwrap();
        let hash = "a".repeat(64);
        fs::write(cache.page_path(&hash).unwrap(), [0_u8; 1]).unwrap();
        assert!(matches!(
            cache.read_page(&hash),
            Err(FlashCacheError::InvalidFrame)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn evicts_the_least_recently_added_frame_above_the_flash_limit() {
        let root = test_root();
        let mut cache = FlashDisplayCache::open(&root).unwrap();
        let mut first_hash = String::new();
        for index in 0..=MAX_CACHED_PAGE_COUNT {
            let frame = vec![index as u8; DISPLAY_IMAGE_BYTES];
            let mut page = page(&frame);
            page.page_id = format!("page-{index}");
            let release = DisplayRelease {
                release_id: format!("release-{index}"),
                document_version: 1,
                active_page_id: page.page_id.clone(),
                pages: vec![page.clone()],
            };
            if index == 0 {
                first_hash = page.image_sha256.clone();
            }
            cache.commit_release(&release, &[(page, frame)]).unwrap();
        }
        assert!(!cache.contains_page(&first_hash).unwrap());
        assert_eq!(cache.index.page_hashes.len(), MAX_CACHED_PAGE_COUNT);
        fs::remove_dir_all(root).unwrap();
    }
}
