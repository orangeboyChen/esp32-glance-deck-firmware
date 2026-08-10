use crate::{
    display::DISPLAY_IMAGE_BYTES,
    local_screen::pairing_code_frame,
    pages::{CachedPage, PageRenderer},
};
use anyhow::{bail, Result};

unsafe extern "C" {
    fn glance_deck_rlcd_init() -> i32;
    fn glance_deck_rlcd_flush(frame: *const u8, length: usize) -> i32;
}

impl PageRenderer for RlcdRenderer {
    type Error = anyhow::Error;

    fn render_cached_page(&mut self, _page: &CachedPage, frame: &[u8]) -> Result<(), Self::Error> {
        self.flush_frame(frame)
    }
}

pub struct RlcdRenderer;

impl RlcdRenderer {
    pub fn new() -> Result<Self> {
        let result = unsafe { glance_deck_rlcd_init() };
        if result != 0 {
            bail!("initialize Waveshare ST7305 RLCD failed: {result}");
        }
        Ok(Self)
    }

    pub fn flush_frame(&mut self, frame: &[u8]) -> Result<()> {
        if frame.len() != DISPLAY_IMAGE_BYTES {
            bail!("invalid ST7305 frame length");
        }
        let result = unsafe { glance_deck_rlcd_flush(frame.as_ptr(), frame.len()) };
        if result != 0 {
            bail!("flush Waveshare ST7305 RLCD failed: {result}");
        }
        Ok(())
    }

    pub fn show_pairing_code(&mut self, pairing_code: &str) -> Result<()> {
        let frame = pairing_code_frame(pairing_code).map_err(|error| anyhow::anyhow!(error))?;
        self.flush_frame(&frame)
    }
}
