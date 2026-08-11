use std::fs;

use anyhow::{bail, Context, Result};
use embedded_svc::{
    http::{client::Client as HttpClient, Method},
    io::Read,
};
use esp_idf_svc::{
    fs::spiffs::Spiffs,
    http::client::{Configuration as HttpConfiguration, EspHttpConnection},
    io::vfs::MountedSpiffs,
};

use crate::{
    display::{DisplayPage, DISPLAY_IMAGE_BYTES},
    release_sync::ReleaseDownloader,
};

const DISPLAY_STORAGE_PATH: &str = "/spiffs";
const HTTP_READ_BUFFER_BYTES: usize = 1024;

pub struct DisplayStorage {
    _mount: MountedSpiffs<Spiffs>,
}

impl DisplayStorage {
    pub fn mount() -> Result<Self> {
        let spiffs = unsafe { Spiffs::new("storage") }.context("open display SPIFFS partition")?;
        let mount = MountedSpiffs::mount(spiffs, DISPLAY_STORAGE_PATH, 4)
            .context("mount display SPIFFS partition")?;
        Ok(Self { _mount: mount })
    }

    pub fn cache_path(&self) -> &'static str {
        DISPLAY_STORAGE_PATH
    }
}

pub struct HttpsPageDownloader;

impl HttpsPageDownloader {
    pub fn new() -> Self {
        Self
    }
}

impl ReleaseDownloader for HttpsPageDownloader {
    type Error = anyhow::Error;

    fn download(&mut self, page: &DisplayPage) -> Result<Vec<u8>> {
        if !page.image_url.starts_with("https://") || page.image_bytes != DISPLAY_IMAGE_BYTES {
            bail!("display_page_metadata_invalid")
        }
        let connection = EspHttpConnection::new(&HttpConfiguration {
            buffer_size: Some(HTTP_READ_BUFFER_BYTES),
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        })?;
        let mut client = HttpClient::wrap(connection);
        let request = client.request(Method::Get, &page.image_url, &[])?;
        let mut response = request.submit()?;
        if response.status() != 200 {
            bail!("display_download_http_status_{}", response.status())
        }
        let content_length = response
            .header("content-length")
            .and_then(|length| length.parse::<usize>().ok());
        if content_length != Some(DISPLAY_IMAGE_BYTES) {
            bail!("display_download_length_rejected")
        }

        let mut frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
        let mut offset = 0;
        while offset < frame.len() {
            let count = response.read(&mut frame[offset..])?;
            if count == 0 {
                break;
            }
            offset += count;
        }
        if offset != DISPLAY_IMAGE_BYTES || response.read(&mut [0_u8; 1])? != 0 {
            bail!("display_download_truncated_or_oversized")
        }
        Ok(frame)
    }
}
