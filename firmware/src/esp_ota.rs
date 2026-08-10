use anyhow::{bail, Context, Result};
use embedded_svc::{
    http::{client::Client as HttpClient, Method},
    io::Read,
};
use esp_idf_svc::sys::{self, esp_err_t, esp_ota_handle_t, esp_partition_t, ESP_OK};

use crate::ota_runtime::{OtaImageWriter, OtaTransport};

use esp_idf_svc::http::client::EspHttpConnection;

const OTA_HTTP_BUFFER_BYTES: usize = 4096;

pub struct InactiveOtaWriter {
    handle: esp_ota_handle_t,
    partition: *const esp_partition_t,
    finished: bool,
}

impl InactiveOtaWriter {
    pub fn begin() -> Result<Self> {
        let partition = unsafe { sys::esp_ota_get_next_update_partition(core::ptr::null()) };
        if partition.is_null() {
            bail!("ota_partition_unavailable")
        }
        let mut handle = 0;
        check(
            unsafe { sys::esp_ota_begin(partition, u32::MAX as usize, &mut handle) },
            "ota_begin",
        )?;
        Ok(Self {
            handle,
            partition,
            finished: false,
        })
    }

    pub fn write(&mut self, image: &[u8]) -> Result<()> {
        if self.finished || image.is_empty() {
            bail!("ota_writer_closed")
        }
        check(
            unsafe { sys::esp_ota_write(self.handle, image.as_ptr() as *const _, image.len()) },
            "ota_write",
        )
    }

    pub fn finish(mut self) -> Result<()> {
        check(unsafe { sys::esp_ota_end(self.handle) }, "ota_end")?;
        check(
            unsafe { sys::esp_ota_set_boot_partition(self.partition) },
            "ota_set_boot_partition",
        )?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for InactiveOtaWriter {
    fn drop(&mut self) {
        if !self.finished {
            let _ = unsafe { sys::esp_ota_abort(self.handle) };
        }
    }
}

impl OtaImageWriter for InactiveOtaWriter {
    type Error = anyhow::Error;

    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), Self::Error> {
        self.write(chunk)
    }

    fn finish(self) -> Result<(), Self::Error> {
        InactiveOtaWriter::finish(self)
    }
}

/// Bounded HTTPS transport used only for signed OTA manifests and images.
/// The device never accepts an HTTP OTA URL and never buffers the image.
pub struct EspHttpsOtaTransport;

impl EspHttpsOtaTransport {
    pub fn new() -> Self {
        Self
    }

    fn connection(&self, url: &str) -> Result<EspHttpConnection> {
        if !url.starts_with("https://") {
            bail!("ota_url_not_https");
        }
        let connection = esp_idf_svc::http::client::EspHttpConnection::new(
            &esp_idf_svc::http::client::Configuration {
                buffer_size: Some(OTA_HTTP_BUFFER_BYTES),
                crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
                ..Default::default()
            },
        )?;
        Ok(connection)
    }
}

impl OtaTransport for EspHttpsOtaTransport {
    type Error = anyhow::Error;

    fn fetch_manifest(&mut self, url: &str, max_bytes: usize) -> Result<Vec<u8>, Self::Error> {
        let connection = self.connection(url)?;
        let mut client = HttpClient::wrap(connection);
        let request = client.request(Method::Get, url, &[])?;
        let mut response = request.submit()?;
        if response.status() != 200 {
            bail!("ota_http_status_{}", response.status());
        }
        let length = response
            .header("content-length")
            .and_then(|value| value.parse::<usize>().ok())
            .context("ota_manifest_length_missing")?;
        if length == 0 || length > max_bytes {
            bail!("ota_manifest_length_rejected");
        }
        let mut body = vec![0_u8; length];
        let mut offset = 0;
        while offset < length {
            let count = response.read(&mut body[offset..])?;
            if count == 0 {
                bail!("ota_manifest_truncated");
            }
            offset += count;
        }
        if response.read(&mut [0_u8; 1])? != 0 {
            bail!("ota_manifest_oversized");
        }
        Ok(body)
    }

    fn stream_image(
        &mut self,
        url: &str,
        max_bytes: usize,
        on_chunk: &mut dyn FnMut(&[u8]) -> Result<(), Self::Error>,
    ) -> Result<usize, Self::Error> {
        let connection = self.connection(url)?;
        let mut client = HttpClient::wrap(connection);
        let request = client.request(Method::Get, url, &[])?;
        let mut response = request.submit()?;
        if response.status() != 200 {
            bail!("ota_http_status_{}", response.status());
        }
        let length = response
            .header("content-length")
            .and_then(|value| value.parse::<usize>().ok())
            .context("ota_image_length_missing")?;
        if length == 0 || length > max_bytes {
            bail!("ota_image_length_rejected");
        }
        let mut buffer = [0_u8; OTA_HTTP_BUFFER_BYTES];
        let mut total = 0usize;
        while total < length {
            let chunk_length = (length - total).min(OTA_HTTP_BUFFER_BYTES);
            let count = response.read(&mut buffer[..chunk_length])?;
            if count == 0 {
                bail!("ota_image_truncated");
            }
            on_chunk(&buffer[..count])?;
            total += count;
        }
        if response.read(&mut [0_u8; 1])? != 0 {
            bail!("ota_image_oversized");
        }
        Ok(total)
    }
}

pub fn mark_running_image_healthy() -> Result<()> {
    check(
        unsafe { sys::esp_ota_mark_app_valid_cancel_rollback() },
        "ota_mark_valid",
    )
}

pub fn rollback_running_image() -> Result<()> {
    check(
        unsafe { sys::esp_ota_mark_app_invalid_rollback_and_reboot() },
        "ota_mark_rollback",
    )
}

fn check(result: esp_err_t, operation: &str) -> Result<()> {
    if result == ESP_OK {
        Ok(())
    } else {
        bail!("{operation}_failed_{result}")
    }
}
