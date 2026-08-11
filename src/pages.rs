use crate::display::{DisplayPage, DisplayRelease};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedPage {
    pub page_id: String,
    pub image_sha256: String,
    pub image_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PageError {
    NoPages,
    ActivePageMissing,
}

pub struct PageNavigator {
    pages: Vec<CachedPage>,
    active_index: usize,
}

impl PageNavigator {
    pub fn from_release(release: &DisplayRelease) -> Result<Self, PageError> {
        if release.pages.is_empty() {
            return Err(PageError::NoPages);
        }
        let active_index = release
            .pages
            .iter()
            .position(|page| page.page_id == release.active_page_id)
            .ok_or(PageError::ActivePageMissing)?;
        Ok(Self {
            pages: release.pages.iter().map(CachedPage::from).collect(),
            active_index,
        })
    }

    pub fn active_page(&self) -> &CachedPage {
        &self.pages[self.active_index]
    }

    pub fn next_page(&mut self) -> &CachedPage {
        self.active_index = (self.active_index + 1) % self.pages.len();
        self.active_page()
    }

    pub fn previous_page(&mut self) -> &CachedPage {
        self.active_index = (self.active_index + self.pages.len() - 1) % self.pages.len();
        self.active_page()
    }
}

impl From<&DisplayPage> for CachedPage {
    fn from(page: &DisplayPage) -> Self {
        Self {
            page_id: page.page_id.clone(),
            image_sha256: page.image_sha256.clone(),
            image_bytes: page.image_bytes,
        }
    }
}

pub trait PageRenderer {
    type Error;

    /// Flush a previously verified, locally cached 1-bit RLCD page resource.
    fn render_cached_page(&mut self, page: &CachedPage, frame: &[u8]) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_local_navigation_without_network() {
        let release = DisplayRelease {
            release_id: "release-1".to_owned(),
            document_version: 1,
            active_page_id: "usage".to_owned(),
            pages: vec![
                DisplayPage {
                    page_id: "usage".to_owned(),
                    image_format: "mono1-msb".to_owned(),
                    image_width: 400,
                    image_height: 300,
                    image_url: "https://example.test/usage".to_owned(),
                    image_sha256: "a".repeat(64),
                    image_bytes: 15_000,
                },
                DisplayPage {
                    page_id: "alerts".to_owned(),
                    image_format: "mono1-msb".to_owned(),
                    image_width: 400,
                    image_height: 300,
                    image_url: "https://example.test/alerts".to_owned(),
                    image_sha256: "a".repeat(64),
                    image_bytes: 15_000,
                },
            ],
        };
        let mut navigator = PageNavigator::from_release(&release).unwrap();
        assert_eq!(navigator.next_page().page_id, "alerts");
        assert_eq!(navigator.next_page().page_id, "usage");
    }
}
