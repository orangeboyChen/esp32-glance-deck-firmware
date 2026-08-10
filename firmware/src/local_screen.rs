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

pub fn maintenance_frame(message: &str) -> Result<Vec<u8>, &'static str> {
    let bytes = message.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 16
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b' ')
    {
        return Err("maintenance_message_invalid");
    }
    let scale = 4;
    let glyph_width = 5 * scale;
    let spacing = scale;
    let total_width = bytes.len() * (glyph_width + spacing) - spacing;
    let left = (DISPLAY_WIDTH - total_width.min(DISPLAY_WIDTH)) / 2;
    let top = (DISPLAY_HEIGHT - 7 * scale) / 2;
    let mut frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
    rectangle(&mut frame, 12, 12, DISPLAY_WIDTH - 24, DISPLAY_HEIGHT - 24);
    for (index, byte) in bytes.iter().enumerate() {
        let x = left + index * (glyph_width + spacing);
        draw_glyph(&mut frame, x, top, *byte, scale);
    }
    Ok(frame)
}

pub fn wifi_setup_frame(password: &str) -> Result<Vec<u8>, &'static str> {
    if password.len() != 10
        || !password
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    {
        return Err("wifi_password_invalid");
    }
    let mut frame = vec![0_u8; DISPLAY_IMAGE_BYTES];
    rectangle(&mut frame, 12, 12, DISPLAY_WIDTH - 24, DISPLAY_HEIGHT - 24);
    draw_centered_text(&mut frame, "WIFI SETUP", 54, 3);
    draw_centered_text(&mut frame, password, 174, 3);
    Ok(frame)
}

fn draw_centered_text(frame: &mut [u8], text: &str, top: usize, scale: usize) {
    let glyph_width = 5 * scale;
    let spacing = scale;
    let total_width = text.len() * (glyph_width + spacing) - spacing;
    let left = (DISPLAY_WIDTH - total_width.min(DISPLAY_WIDTH)) / 2;
    for (index, byte) in text.bytes().enumerate() {
        draw_glyph(
            frame,
            left + index * (glyph_width + spacing),
            top,
            byte,
            scale,
        );
    }
}

fn draw_glyph(frame: &mut [u8], left: usize, top: usize, character: u8, scale: usize) {
    let rows = match character {
        b'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'C' => [0x0f, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0f],
        b'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        b'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        b'F' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        b'G' => [0x0f, 0x10, 0x10, 0x17, 0x11, 0x11, 0x0f],
        b'H' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'I' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1f],
        b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        b'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        b'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        b'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1b, 0x11],
        b'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        b'X' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        b'3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
        b' ' => [0; 7],
        _ => return,
    };
    for (row, mask) in rows.into_iter().enumerate() {
        for column in 0..5 {
            if mask & (1 << (4 - column)) != 0 {
                fill(
                    frame,
                    left + column * scale,
                    top + row * scale,
                    scale,
                    scale,
                );
            }
        }
    }
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

    #[test]
    fn renders_bounded_maintenance_message() {
        let frame = maintenance_frame("MAINTENANCE").unwrap();
        assert_eq!(frame.len(), DISPLAY_IMAGE_BYTES);
        assert!(frame.iter().any(|byte| *byte != 0));
        assert_eq!(
            maintenance_frame("maintenance"),
            Err("maintenance_message_invalid")
        );
        assert_eq!(maintenance_frame(""), Err("maintenance_message_invalid"));
    }

    #[test]
    fn renders_wifi_setup_credentials_without_accepting_unsafe_passwords() {
        let frame = wifi_setup_frame("GD12AB34EF").unwrap();
        assert_eq!(frame.len(), DISPLAY_IMAGE_BYTES);
        assert!(frame.iter().any(|byte| *byte != 0));
        assert_eq!(wifi_setup_frame("short"), Err("wifi_password_invalid"));
        assert_eq!(
            wifi_setup_frame("GD12-ab34EF"),
            Err("wifi_password_invalid")
        );
    }
}
