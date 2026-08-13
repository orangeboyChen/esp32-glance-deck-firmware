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

/// Synchronizes retained metadata and just the currently active page. Page metadata
/// is intentionally cheap to retain; image frames download only when the device is
/// about to display them. A failed active-page download leaves the previous complete
/// release and visible frame untouched.
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
    synchronize_page(cache, downloader, release, &release.active_page_id)
}

/// Ensures a particular page is locally usable. It is called for a requested local
/// or remote page change, so cached pages switch immediately and uncached pages use
/// exactly one HTTPS download. Offline callers can detect `Download` and keep the
/// current verified frame rather than clearing the reflective panel.
pub fn synchronize_page<C, D>(
    cache: &mut C,
    downloader: &mut D,
    release: &DisplayRelease,
    page_id: &str,
) -> Result<usize, ReleaseSyncError<C::Error, D::Error>>
where
    C: DisplayCache,
    D: ReleaseDownloader,
{
    release
        .validate_metadata()
        .map_err(ReleaseSyncError::InvalidMetadata)?;
    let page = release
        .page(page_id)
        .ok_or(ReleaseSyncError::InvalidMetadata(
            DisplayReleaseError::MissingActivePage,
        ))?;
    let (image, downloaded) = match cache
        .read_page(&page.image_sha256)
        .map_err(ReleaseSyncError::Cache)?
    {
        Some(image) => (image, 0),
        None => (
            downloader
                .download(page)
                .map_err(ReleaseSyncError::Download)?,
            1,
        ),
    };
    page.validate_image(&image)
        .map_err(ReleaseSyncError::InvalidImage)?;
    cache
        .commit_release(release, &[(page.clone(), image)])
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

    fn release(image: &[u8], alerts_image: &[u8]) -> DisplayRelease {
        DisplayRelease {
            release_id: "release-1".to_owned(),
            document_version: 1,
            active_page_id: "usage".to_owned(),
            pages: vec![
                DisplayPage {
                    page_id: "usage".to_owned(),
                    image_format: DISPLAY_IMAGE_FORMAT.to_owned(),
                    image_width: DISPLAY_WIDTH,
                    image_height: DISPLAY_HEIGHT,
                    image_url: "https://console.example/usage".to_owned(),
                    image_sha256: hex::encode(Sha256::digest(image)),
                    image_bytes: DISPLAY_IMAGE_BYTES,
                },
                DisplayPage {
                    page_id: "alerts".to_owned(),
                    image_format: DISPLAY_IMAGE_FORMAT.to_owned(),
                    image_width: DISPLAY_WIDTH,
                    image_height: DISPLAY_HEIGHT,
                    image_url: "https://console.example/alerts".to_owned(),
                    image_sha256: hex::encode(Sha256::digest(alerts_image)),
                    image_bytes: DISPLAY_IMAGE_BYTES,
                },
                DisplayPage {
                    page_id: "system".to_owned(),
                    image_format: DISPLAY_IMAGE_FORMAT.to_owned(),
                    image_width: DISPLAY_WIDTH,
                    image_height: DISPLAY_HEIGHT,
                    image_url: "https://console.example/system".to_owned(),
                    image_sha256: hex::encode(Sha256::digest(image)),
                    image_bytes: DISPLAY_IMAGE_BYTES,
                },
            ],
        }
    }

    #[test]
    fn downloads_only_the_active_page_and_commits_the_manifest() {
        let image = vec![0x55; DISPLAY_IMAGE_BYTES];
        let alerts_image = vec![0xaa; DISPLAY_IMAGE_BYTES];
        let mut cache = Cache::default();
        let mut downloader = Downloader {
            image: image.clone(),
            downloads: 0,
            fail: false,
        };
        assert_eq!(
            synchronize_release(&mut cache, &mut downloader, &release(&image, &alerts_image))
                .unwrap(),
            1
        );
        assert_eq!(downloader.downloads, 1);
        assert!(cache.current_release().unwrap().is_some());
        assert!(!cache
            .frames
            .contains_key(&hex::encode(Sha256::digest(alerts_image))));
    }

    #[test]
    fn reuses_verified_frames_and_preserves_cache_on_download_failure() {
        let image = vec![0x55; DISPLAY_IMAGE_BYTES];
        let alerts_image = vec![0xaa; DISPLAY_IMAGE_BYTES];
        let cached_release = release(&image, &alerts_image);
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

        let changed = release(&vec![0x11; DISPLAY_IMAGE_BYTES], &alerts_image);
        assert!(matches!(
            synchronize_release(&mut cache, &mut downloader, &changed),
            Err(ReleaseSyncError::Download("network"))
        ));
        assert_eq!(
            cache.current_release().unwrap().unwrap().release_id,
            "release-1"
        );
    }

    #[test]
    fn downloads_an_uncached_requested_page_only_when_it_is_shown() {
        let usage = vec![0x55; DISPLAY_IMAGE_BYTES];
        let alerts = vec![0xaa; DISPLAY_IMAGE_BYTES];
        let release = release(&usage, &alerts);
        let mut cache = Cache::default();
        cache
            .frames
            .insert(release.pages[0].image_sha256.clone(), usage);
        let mut downloader = Downloader {
            image: alerts.clone(),
            downloads: 0,
            fail: false,
        };

        assert_eq!(
            synchronize_page(&mut cache, &mut downloader, &release, "alerts").unwrap(),
            1
        );
        assert_eq!(downloader.downloads, 1);
        assert_eq!(
            cache.frames.get(&release.pages[1].image_sha256),
            Some(&alerts)
        );
    }
}
