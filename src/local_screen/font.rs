use crate::display::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

const FIRST_CHARACTER: u8 = b' ';
const LAST_CHARACTER: u8 = b'~';

struct Font {
    width: usize,
    height: usize,
    glyph_bytes: usize,
    bitmap: &'static [u8],
    advances: &'static [u8],
}

const SMALL: Font = Font {
    width: 12,
    height: 20,
    glyph_bytes: 30,
    bitmap: include_bytes!("../../assets/local-font/noto-sans-sc-1.bin"),
    advances: include_bytes!("../../assets/local-font/noto-sans-sc-1.widths"),
};
const MEDIUM: Font = Font {
    width: 16,
    height: 26,
    glyph_bytes: 52,
    bitmap: include_bytes!("../../assets/local-font/noto-sans-sc-2.bin"),
    advances: include_bytes!("../../assets/local-font/noto-sans-sc-2.widths"),
};
const LARGE: Font = Font {
    width: 22,
    height: 36,
    glyph_bytes: 99,
    bitmap: include_bytes!("../../assets/local-font/noto-sans-sc-3.bin"),
    advances: include_bytes!("../../assets/local-font/noto-sans-sc-3.widths"),
};
const PAIRING: Font = Font {
    width: 36,
    height: 56,
    glyph_bytes: 252,
    bitmap: include_bytes!("../../assets/local-font/noto-sans-sc-5.bin"),
    advances: include_bytes!("../../assets/local-font/noto-sans-sc-5.widths"),
};

fn select_font(scale: usize) -> &'static Font {
    match scale {
        1 => &SMALL,
        2 => &MEDIUM,
        3 => &LARGE,
        5 => &PAIRING,
        _ => &SMALL,
    }
}

pub fn text_width(text: &str, scale: usize) -> usize {
    let font = select_font(scale);
    text.bytes()
        .map(|character| glyph_advance(font, character))
        .sum()
}

pub fn draw_text(frame: &mut [u8], left: usize, top: usize, text: &str, scale: usize) {
    let font = select_font(scale);
    let mut cursor = left;
    for character in text.bytes() {
        if !(FIRST_CHARACTER..=LAST_CHARACTER).contains(&character) {
            continue;
        }
        let glyph_start = (character - FIRST_CHARACTER) as usize * font.glyph_bytes;
        let glyph = &font.bitmap[glyph_start..glyph_start + font.glyph_bytes];
        for y in 0..font.height {
            for x in 0..font.width {
                let pixel = y * font.width + x;
                if glyph[pixel / 8] & (0x80 >> (pixel % 8)) == 0 {
                    continue;
                }
                let destination_x = cursor + x;
                let destination_y = top + y;
                if destination_x >= DISPLAY_WIDTH || destination_y >= DISPLAY_HEIGHT {
                    continue;
                }
                let destination = destination_y * DISPLAY_WIDTH + destination_x;
                frame[destination / 8] |= 0x80 >> (destination % 8);
            }
        }
        cursor += glyph_advance(font, character);
    }
}

fn glyph_advance(font: &Font, character: u8) -> usize {
    if !(FIRST_CHARACTER..=LAST_CHARACTER).contains(&character) {
        return 0;
    }
    font.advances[(character - FIRST_CHARACTER) as usize] as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::DISPLAY_IMAGE_BYTES;

    #[test]
    fn renders_every_character_used_by_local_credentials() {
        let mut frame = vec![0; DISPLAY_IMAGE_BYTES];
        draw_text(&mut frame, 0, 0, "GD12AB34EFXYZ", 3);
        assert!(frame.iter().any(|byte| *byte != 0));
    }
}
