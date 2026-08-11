use crate::{
    display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_WIDTH},
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
        let native_frame = st7305_landscape_frame(frame);
        let result = unsafe { glance_deck_rlcd_flush(native_frame.as_ptr(), native_frame.len()) };
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

fn st7305_landscape_frame(frame: &[u8]) -> Vec<u8> {
    let mut native_frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
    let rows_per_byte = 4;
    let columns_per_byte = 2;
    let bytes_per_column = DISPLAY_HEIGHT / rows_per_byte;

    for y in 0..DISPLAY_HEIGHT {
        let inverted_y = DISPLAY_HEIGHT - 1 - y;
        let byte_y = inverted_y / rows_per_byte;
        let local_y = inverted_y % rows_per_byte;

        for x in 0..DISPLAY_WIDTH {
            let source_offset = y * DISPLAY_WIDTH + x;
            let source_bit = 0x80 >> (source_offset % 8);
            if frame[source_offset / 8] & source_bit == 0 {
                continue;
            }

            let byte_x = x / columns_per_byte;
            let local_x = x % columns_per_byte;
            let destination_offset = byte_x * bytes_per_column + byte_y;
            let destination_bit = 0x80 >> (local_y * columns_per_byte + local_x);
            native_frame[destination_offset] |= destination_bit;
        }
    }

    native_frame
}

#[cfg(test)]
mod tests {
    use super::st7305_landscape_frame;
    use crate::display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_WIDTH};

    #[test]
    fn converts_row_major_pixels_to_st7305_landscape_layout() {
        let mut frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
        let pixels = [(0, 0), (1, 0), (0, 1), (399, 299)];
        for (x, y) in pixels {
            let offset = y * DISPLAY_WIDTH + x;
            frame[offset / 8] |= 0x80 >> (offset % 8);
        }

        let native_frame = st7305_landscape_frame(&frame);
        let bytes_per_column = DISPLAY_HEIGHT / 4;

        assert_eq!(native_frame[74], 0x0B);
        assert_eq!(native_frame[199 * bytes_per_column], 0x80);
        assert_eq!(native_frame.iter().map(|byte| byte.count_ones()).sum::<u32>(), 4);
    }
}
