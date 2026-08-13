use crate::display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_WIDTH};

pub fn landscape_frame(frame: &[u8]) -> Vec<u8> {
    let mut native_frame = vec![0xFF_u8; DISPLAY_IMAGE_BYTES];
    let bytes_per_column = DISPLAY_HEIGHT / 4;

    for y in 0..DISPLAY_HEIGHT {
        let controller_row = DISPLAY_HEIGHT - 1 - y;
        for x in 0..DISPLAY_WIDTH {
            let source = y * DISPLAY_WIDTH + x;
            if frame[source / 8] & (0x80 >> (source % 8)) == 0 {
                continue;
            }
            let controller = (x / 2) * bytes_per_column + controller_row / 4;
            let bit = 0x80 >> ((controller_row % 4) * 2 + x % 2);
            native_frame[controller] &= !bit;
        }
    }

    native_frame
}

#[cfg(test)]
mod tests {
    use super::landscape_frame;
    use crate::display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_WIDTH};

    #[test]
    fn converts_row_major_pixels_to_st7305_landscape_layout() {
        let mut frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
        for (x, y) in [(0, 0), (1, 0), (0, 1), (399, 299)] {
            let offset = y * DISPLAY_WIDTH + x;
            frame[offset / 8] |= 0x80 >> (offset % 8);
        }

        let native_frame = landscape_frame(&frame);
        assert_eq!(native_frame[74], 0xF4);
        assert_eq!(native_frame[199 * (DISPLAY_HEIGHT / 4)], 0xBF);
        assert_eq!(
            native_frame
                .iter()
                .map(|byte| (!byte).count_ones())
                .sum::<u32>(),
            4
        );
    }

    #[test]
    fn preserves_every_pixel_at_the_official_landscape_controller_address() {
        let mut frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                if (x / 3 + y / 5) % 2 == 0 {
                    let source = y * DISPLAY_WIDTH + x;
                    frame[source / 8] |= 0x80 >> (source % 8);
                }
            }
        }

        let native_frame = landscape_frame(&frame);
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                let source = y * DISPLAY_WIDTH + x;
                let controller_row = DISPLAY_HEIGHT - 1 - y;
                let controller = (x / 2) * (DISPLAY_HEIGHT / 4) + controller_row / 4;
                let bit = 0x80 >> ((controller_row % 4) * 2 + x % 2);
                let expected_black = frame[source / 8] & (0x80 >> (source % 8)) != 0;
                assert_eq!(
                    native_frame[controller] & bit == 0,
                    expected_black,
                    "pixel ({x}, {y})"
                );
            }
        }
    }
}
