use crate::display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_WIDTH};

use super::font::{draw_text, text_width};

pub const CONTENT_LEFT: usize = 28;
pub const CONTENT_RIGHT: usize = DISPLAY_WIDTH - CONTENT_LEFT;

pub struct Canvas {
    frame: Vec<u8>,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            frame: vec![0; DISPLAY_IMAGE_BYTES],
        }
    }

    pub fn finish(self) -> Vec<u8> {
        self.frame
    }

    pub fn header(&mut self, title: &str, subtitle: Option<&str>) {
        self.text(CONTENT_LEFT, 26, title, 3);
        if let Some(subtitle) = subtitle {
            self.text(CONTENT_LEFT, 70, subtitle, 1);
        }
    }

    pub fn centered_text(&mut self, top: usize, text: &str, scale: usize) {
        self.text(
            (DISPLAY_WIDTH.saturating_sub(text_width(text, scale))) / 2,
            top,
            text,
            scale,
        );
    }

    pub fn text(&mut self, left: usize, top: usize, text: &str, scale: usize) {
        draw_text(&mut self.frame, left, top, text, scale);
    }

    pub fn ellipsized_text(
        &mut self,
        left: usize,
        top: usize,
        text: &str,
        scale: usize,
        max_width: usize,
    ) {
        let shown = ellipsize_to_width(text, scale, max_width);
        self.text(left, top, &shown, scale);
    }

    pub fn row(&mut self, top: usize, label: &str, value: &str) {
        self.text(CONTENT_LEFT, top, label, 1);
        self.text(
            CONTENT_RIGHT.saturating_sub(text_width(value, 1)),
            top,
            value,
            1,
        );
    }

    pub fn progress(&mut self, top: usize, percent: u8) {
        let width = CONTENT_RIGHT - CONTENT_LEFT;
        let fill_width = width.saturating_mul(percent.min(100) as usize) / 100;
        self.stroke_rect(CONTENT_LEFT, top, width, 8, 2);
        if fill_width > 4 {
            self.fill_rect(CONTENT_LEFT + 2, top + 2, fill_width - 4, 4);
        }
    }

    pub fn stroke_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        stroke: usize,
    ) {
        self.fill_rect(left, top, width, stroke);
        self.fill_rect(left, top + height.saturating_sub(stroke), width, stroke);
        self.fill_rect(left, top, stroke, height);
        self.fill_rect(left + width.saturating_sub(stroke), top, stroke, height);
    }

    pub fn fill_rect(&mut self, left: usize, top: usize, width: usize, height: usize) {
        for y in top..(top + height).min(DISPLAY_HEIGHT) {
            for x in left..(left + width).min(DISPLAY_WIDTH) {
                let offset = y * DISPLAY_WIDTH + x;
                self.frame[offset / 8] |= 0x80 >> (offset % 8);
            }
        }
    }
}

fn ellipsize_to_width(text: &str, scale: usize, max_width: usize) -> String {
    if text_width(text, scale) <= max_width {
        return text.to_owned();
    }
    let ellipsis = "...";
    let available_width = max_width.saturating_sub(text_width(ellipsis, scale));
    let mut shown = String::new();
    for character in text.chars() {
        let mut candidate = shown.clone();
        candidate.push(character);
        if text_width(&candidate, scale) > available_width {
            break;
        }
        shown = candidate;
    }
    format!("{shown}{ellipsis}")
}

#[cfg(test)]
mod tests {
    use super::ellipsize_to_width;
    use crate::local_screen::font::text_width;

    #[test]
    fn ellipsizes_to_the_actual_proportional_font_width() {
        let shown = ellipsize_to_width("WWWWWWWWWWWWWWWWWWWWWWWWWWWW", 1, 280);
        assert!(shown.ends_with("..."));
        assert!(text_width(&shown, 1) <= 280);
    }

    #[test]
    fn uses_a_two_pixel_progress_track_stroke() {
        let mut canvas = super::Canvas::new();
        canvas.progress(142, 42);
        let frame = canvas.finish();
        let first_row = 142 * crate::display::DISPLAY_WIDTH + super::CONTENT_LEFT;
        let second_row = 143 * crate::display::DISPLAY_WIDTH + super::CONTENT_LEFT;
        assert_ne!(frame[first_row / 8] & (0x80 >> (first_row % 8)), 0);
        assert_ne!(frame[second_row / 8] & (0x80 >> (second_row % 8)), 0);
    }
}
