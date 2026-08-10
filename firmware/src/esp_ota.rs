use anyhow::{bail, Result};
use esp_idf_svc::sys::{self, esp_err_t, esp_ota_handle_t, esp_partition_t, ESP_OK};

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

fn check(result: esp_err_t, operation: &str) -> Result<()> {
    if result == ESP_OK {
        Ok(())
    } else {
        bail!("{operation}_failed_{result}")
    }
}
