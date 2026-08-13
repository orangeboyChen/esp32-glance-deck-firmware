mod canvas;
mod font;
mod maintenance;
mod ota;
mod provisioning;

pub use maintenance::{error_frame, maintenance_frame, MaintenanceScreen};
pub use ota::ota_frame;
pub use provisioning::{pairing_code_frame, wifi_setup_frame};

use crate::display::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

pub fn page_indicator_frame(
    frame: &[u8],
    active_index: usize,
    page_count: usize,
) -> Option<Vec<u8>> {
    if frame.len() != DISPLAY_WIDTH * DISPLAY_HEIGHT / 8
        || page_count == 0
        || page_count > 10
        || active_index >= page_count
    {
        return None;
    }
    let mut overlay = frame.to_vec();
    let spacing = 14_i32;
    let group_width = (page_count as i32 - 1) * spacing;
    let center_x = DISPLAY_WIDTH as i32 / 2;
    let top = DISPLAY_HEIGHT as i32 - 22;
    for index in 0..page_count {
        let center = center_x - group_width / 2 + index as i32 * spacing;
        draw_circle(&mut overlay, center, top, 4, index == active_index);
    }
    Some(overlay)
}

fn draw_circle(frame: &mut [u8], center_x: i32, center_y: i32, radius: i32, filled: bool) {
    for y in center_y - radius..=center_y + radius {
        for x in center_x - radius..=center_x + radius {
            if x < 0 || y < 0 || x >= DISPLAY_WIDTH as i32 || y >= DISPLAY_HEIGHT as i32 {
                continue;
            }
            let distance = (x - center_x) * (x - center_x) + (y - center_y) * (y - center_y);
            if distance <= radius * radius && (filled || distance >= (radius - 1) * (radius - 1)) {
                let offset = y as usize * DISPLAY_WIDTH + x as usize;
                frame[offset / 8] |= 0x80 >> (offset % 8);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::DISPLAY_IMAGE_BYTES;

    #[test]
    fn overlays_bounded_circular_page_indicators_without_mutating_source() {
        let frame = vec![0; DISPLAY_IMAGE_BYTES];
        let overlay = page_indicator_frame(&frame, 1, 3).unwrap();
        assert_ne!(overlay, frame);
        assert!(page_indicator_frame(&frame, 3, 3).is_none());
        assert!(page_indicator_frame(&frame, 0, 11).is_none());
    }
}
