use crate::display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_WIDTH};

const DIGIT_WIDTH: usize = 38;
const DIGIT_HEIGHT: usize = 72;
const SEGMENT: usize = 6;
const DIGIT_GAP: usize = 16;

pub fn pairing_code_frame(pairing_code: &str) -> Result<Vec<u8>, &'static str> {
    if pairing_code.len() != 6 || !pairing_code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("pairing_code_invalid");
    }
    let mut frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
    rectangle(&mut frame, 12, 12, DISPLAY_WIDTH - 24, DISPLAY_HEIGHT - 24);
    for (index, digit) in pairing_code.bytes().enumerate() {
        let left = 33 + index * (DIGIT_WIDTH + DIGIT_GAP);
        draw_digit(&mut frame, left, 112, digit - b'0');
    }
    Ok(frame)
}

fn draw_digit(frame: &mut [u8], left: usize, top: usize, digit: u8) {
    const SEGMENTS: [u8; 10] = [
        0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
        0b1111111, 0b1101111,
    ];
    let mask = SEGMENTS[digit as usize];
    let horizontal = [
        (left + SEGMENT, top),
        (left + SEGMENT, top + DIGIT_HEIGHT / 2 - SEGMENT / 2),
        (left + SEGMENT, top + DIGIT_HEIGHT - SEGMENT),
    ];
    let vertical = [
        (left, top + SEGMENT),
        (left + DIGIT_WIDTH - SEGMENT, top + SEGMENT),
        (left, top + DIGIT_HEIGHT / 2 + SEGMENT / 2),
        (
            left + DIGIT_WIDTH - SEGMENT,
            top + DIGIT_HEIGHT / 2 + SEGMENT / 2,
        ),
    ];
    for (index, (x, y)) in horizontal.into_iter().enumerate() {
        if mask & (1 << index) != 0 {
            fill(frame, x, y, DIGIT_WIDTH - 2 * SEGMENT, SEGMENT);
        }
    }
    for (index, (x, y)) in vertical.into_iter().enumerate() {
        if mask & (1 << (index + 3)) != 0 {
            fill(frame, x, y, SEGMENT, DIGIT_HEIGHT / 2 - SEGMENT);
        }
    }
}

fn rectangle(frame: &mut [u8], left: usize, top: usize, width: usize, height: usize) {
    fill(frame, left, top, width, 2);
    fill(frame, left, top + height - 2, width, 2);
    fill(frame, left, top, 2, height);
    fill(frame, left + width - 2, top, 2, height);
}

fn fill(frame: &mut [u8], left: usize, top: usize, width: usize, height: usize) {
    for y in top..(top + height).min(DISPLAY_HEIGHT) {
        for x in left..(left + width).min(DISPLAY_WIDTH) {
            let offset = y * DISPLAY_WIDTH + x;
            frame[offset / 8] |= 0x80 >> (offset % 8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_bounded_pairing_code_frame() {
        let frame = pairing_code_frame("123456").unwrap();
        assert_eq!(frame.len(), DISPLAY_IMAGE_BYTES);
        assert!(frame.iter().any(|byte| *byte != 0));
        assert_ne!(frame, pairing_code_frame("654321").unwrap());
    }

    #[test]
    fn rejects_non_six_digit_codes() {
        assert_eq!(pairing_code_frame("12345"), Err("pairing_code_invalid"));
        assert_eq!(pairing_code_frame("12A456"), Err("pairing_code_invalid"));
    }
}
