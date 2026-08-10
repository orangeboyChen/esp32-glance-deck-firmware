use crate::display::{DisplayCache, DisplayPage, DisplayRelease, DisplayReleaseError};

pub trait ReleaseDownloader {
    type Error;

    fn download(&mut self, page: &DisplayPage) -> Result<Vec<u8>, Self::Error>;
}

#[derive(Debug)]
pub enum ReleaseSyncError<CacheError, DownloadError> {
    InvalidMetadata(DisplayReleaseError),
    Cache(CacheError),
    Download(DownloadError),
    InvalidImage(DisplayReleaseError),
}

/// Synchronizes a retained release safely: existing verified frames are reused;
/// all missing frames must verify before the cache atomically changes its active
/// manifest. A failed download therefore leaves the previous release visible.
pub fn synchronize_release<C, D>(
    cache: &mut C,
    downloader: &mut D,
    release: &DisplayRelease,
) -> Result<usize, ReleaseSyncError<C::Error, D::Error>>
where
    C: DisplayCache,
    D: ReleaseDownloader,
{
    release
        .validate_metadata()
        .map_err(ReleaseSyncError::InvalidMetadata)?;
    let mut downloaded = 0;
    let mut pages = Vec::with_capacity(release.pages.len());
    for page in &release.pages {
        let image = match cache
            .read_page(&page.image_sha256)
            .map_err(ReleaseSyncError::Cache)?
        {
            Some(image) => image,
            None => {
                downloaded += 1;
                downloader
                    .download(page)
                    .map_err(ReleaseSyncError::Download)?
            }
        };
        page.validate_image(&image)
            .map_err(ReleaseSyncError::InvalidImage)?;
        pages.push((page.clone(), image));
    }
    cache
        .commit_release(release, &pages)
        .map_err(ReleaseSyncError::Cache)?;
    Ok(downloaded)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::display::{
        DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_IMAGE_FORMAT, DISPLAY_WIDTH,
    };

    #[derive(Default)]
    struct Cache {
        frames: BTreeMap<String, Vec<u8>>,
        committed: Option<DisplayRelease>,
    }

    impl DisplayCache for Cache {
        type Error = &'static str;

        fn current_release(&self) -> Result<Option<DisplayRelease>, Self::Error> {
            Ok(self.committed.clone())
        }
        fn previous_release(&self) -> Result<Option<DisplayRelease>, Self::Error> {
            Ok(None)
        }
        fn contains_page(&self, hash: &str) -> Result<bool, Self::Error> {
            Ok(self.frames.contains_key(hash))
        }
        fn read_page(&self, hash: &str) -> Result<Option<Vec<u8>>, Self::Error> {
            Ok(self.frames.get(hash).cloned())
        }
        fn commit_release(
            &mut self,
            release: &DisplayRelease,
            pages: &[(DisplayPage, Vec<u8>)],
        ) -> Result<(), Self::Error> {
            for (page, image) in pages {
                self.frames.insert(page.image_sha256.clone(), image.clone());
            }
            self.committed = Some(release.clone());
            Ok(())
        }
    }

    struct Downloader {
        image: Vec<u8>,
        downloads: usize,
        fail: bool,
    }
    impl ReleaseDownloader for Downloader {
        type Error = &'static str;
        fn download(&mut self, _page: &DisplayPage) -> Result<Vec<u8>, Self::Error> {
            self.downloads += 1;
            if self.fail {
                Err("network")
            } else {
                Ok(self.image.clone())
            }
        }
    }

    fn release(image: &[u8]) -> DisplayRelease {
        DisplayRelease {
            release_id: "release-1".to_owned(),
            document_version: 1,
            active_page_id: "usage".to_owned(),
            pages: vec![DisplayPage {
                page_id: "usage".to_owned(),
                image_format: DISPLAY_IMAGE_FORMAT.to_owned(),
                image_width: DISPLAY_WIDTH,
                image_height: DISPLAY_HEIGHT,
                image_url: "https://console.example/image".to_owned(),
                image_sha256: hex::encode(Sha256::digest(image)),
                image_bytes: DISPLAY_IMAGE_BYTES,
            }],
        }
    }

    #[test]
    fn downloads_verifies_and_commits_missing_frames() {
        let image = vec![0x55; DISPLAY_IMAGE_BYTES];
        let mut cache = Cache::default();
        let mut downloader = Downloader {
            image: image.clone(),
            downloads: 0,
            fail: false,
        };
        assert_eq!(
            synchronize_release(&mut cache, &mut downloader, &release(&image)).unwrap(),
            1
        );
        assert_eq!(downloader.downloads, 1);
        assert!(cache.current_release().unwrap().is_some());
    }

    #[test]
    fn reuses_verified_frames_and_preserves_cache_on_download_failure() {
        let image = vec![0x55; DISPLAY_IMAGE_BYTES];
        let cached_release = release(&image);
        let mut cache = Cache::default();
        cache
            .frames
            .insert(cached_release.pages[0].image_sha256.clone(), image.clone());
        let mut downloader = Downloader {
            image,
            downloads: 0,
            fail: true,
        };
        assert_eq!(
            synchronize_release(&mut cache, &mut downloader, &cached_release).unwrap(),
            0
        );
        assert_eq!(downloader.downloads, 0);

        let changed = release(&vec![0xaa; DISPLAY_IMAGE_BYTES]);
        assert!(matches!(
            synchronize_release(&mut cache, &mut downloader, &changed),
            Err(ReleaseSyncError::Download("network"))
        ));
        assert_eq!(
            cache.current_release().unwrap().unwrap().release_id,
            "release-1"
        );
    }
}
