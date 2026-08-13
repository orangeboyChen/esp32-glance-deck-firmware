use crate::display::{DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_WIDTH};

use super::font::{draw_text, text_width};

pub const CONTENT_LEFT: usize = 28;
pub const CONTENT_RIGHT: usize = DISPLAY_WIDTH - CONTENT_LEFT;
const PROGRESS_TRACK_HEIGHT: usize = 8;
const PROGRESS_STROKE: usize = 2;
const PROGRESS_FILL_HEIGHT: usize = 4;
const PROGRESS_VERTICAL_INSET: usize = (PROGRESS_TRACK_HEIGHT - PROGRESS_FILL_HEIGHT) / 2;

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
        self.centered_text_at(DISPLAY_WIDTH / 2, top, text, scale);
    }

    pub fn centered_text_at(&mut self, center_x: usize, top: usize, text: &str, scale: usize) {
        self.text(
            center_x.saturating_sub(text_width(text, scale) / 2),
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
        self.stroke_rect(
            CONTENT_LEFT,
            top,
            width,
            PROGRESS_TRACK_HEIGHT,
            PROGRESS_STROKE,
        );
        if fill_width > 4 {
            self.fill_rect(
                CONTENT_LEFT + PROGRESS_STROKE,
                top + PROGRESS_VERTICAL_INSET,
                fill_width - 4,
                PROGRESS_FILL_HEIGHT,
            );
        }
    }

    pub fn horizontal_line(&mut self, left: usize, top: usize, width: usize) {
        self.fill_rect(left, top, width, 1);
    }

    pub fn vertical_line(&mut self, left: usize, top: usize, height: usize) {
        self.fill_rect(left, top, 1, height);
    }

    pub fn line(&mut self, from_x: i32, from_y: i32, to_x: i32, to_y: i32) {
        let mut x = from_x;
        let mut y = from_y;
        let delta_x = (to_x - from_x).abs();
        let step_x = if from_x < to_x { 1 } else { -1 };
        let delta_y = -(to_y - from_y).abs();
        let step_y = if from_y < to_y { 1 } else { -1 };
        let mut error = delta_x + delta_y;
        loop {
            self.pixel(x, y);
            if x == to_x && y == to_y {
                break;
            }
            let double_error = 2 * error;
            if double_error >= delta_y {
                error += delta_y;
                x += step_x;
            }
            if double_error <= delta_x {
                error += delta_x;
                y += step_y;
            }
        }
    }

    pub fn fill_circle(&mut self, center_x: i32, center_y: i32, radius: i32) {
        for y in center_y - radius..=center_y + radius {
            for x in center_x - radius..=center_x + radius {
                if (x - center_x) * (x - center_x) + (y - center_y) * (y - center_y)
                    <= radius * radius
                {
                    self.pixel(x, y);
                }
            }
        }
    }

    pub fn blit_mono1(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        bitmap: &[u8],
    ) {
        if bitmap.len() != width.saturating_mul(height) / 8 {
            return;
        }
        for source_y in 0..height {
            for source_x in 0..width {
                let source = source_y * width + source_x;
                if bitmap[source / 8] & (0x80 >> (source % 8)) != 0 {
                    self.pixel((left + source_x) as i32, (top + source_y) as i32);
                }
            }
        }
    }

    pub fn draw_key_gesture(&mut self, left: usize, top: usize, held: bool) {
        self.stroke_circle(left as i32 + 10, top as i32 + 10, 9);
        if held {
            self.fill_circle(left as i32 + 10, top as i32 + 10, 3);
            self.vertical_line(left + 22, top + 3, 14);
            self.horizontal_line(left + 19, top + 3, 7);
            self.horizontal_line(left + 19, top + 16, 7);
        } else {
            self.fill_circle(left as i32 + 10, top as i32 + 10, 2);
            self.line(
                left as i32 + 22,
                top as i32 + 10,
                left as i32 + 30,
                top as i32 + 10,
            );
            self.line(
                left as i32 + 27,
                top as i32 + 7,
                left as i32 + 30,
                top as i32 + 10,
            );
            self.line(
                left as i32 + 27,
                top as i32 + 13,
                left as i32 + 30,
                top as i32 + 10,
            );
        }
    }

    pub fn draw_centered_key_gesture(&mut self, center_x: usize, top: usize, held: bool) {
        self.draw_key_gesture(center_x.saturating_sub(13), top, held);
    }

    pub fn draw_stage_flow(&mut self, center_x: usize, top: usize, active_stage: usize) {
        let centers = [center_x - 42, center_x, center_x + 42];
        for index in 0..2 {
            let from = centers[index] + 8;
            let to = centers[index + 1] - 8;
            self.horizontal_line(from, top + 7, to - from);
            self.line(to as i32 - 3, (top + 4) as i32, to as i32, (top + 7) as i32);
            self.line(
                to as i32 - 3,
                (top + 10) as i32,
                to as i32,
                (top + 7) as i32,
            );
        }
        for (index, center) in centers.into_iter().enumerate() {
            if index <= active_stage {
                self.fill_circle(center as i32, top as i32 + 7, 7);
            } else {
                self.stroke_circle(center as i32, top as i32 + 7, 7);
            }
        }
    }

    pub fn stroke_circle(&mut self, center_x: i32, center_y: i32, radius: i32) {
        let inner_radius = radius.saturating_sub(2);
        for y in center_y - radius..=center_y + radius {
            for x in center_x - radius..=center_x + radius {
                let distance = (x - center_x) * (x - center_x) + (y - center_y) * (y - center_y);
                if distance <= radius * radius && distance >= inner_radius * inner_radius {
                    self.pixel(x, y);
                }
            }
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

    fn pixel(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= DISPLAY_WIDTH as i32 || y >= DISPLAY_HEIGHT as i32 {
            return;
        }
        let offset = y as usize * DISPLAY_WIDTH + x as usize;
        self.frame[offset / 8] |= 0x80 >> (offset % 8);
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

    #[test]
    fn centers_progress_fill_with_equal_vertical_insets() {
        assert_eq!(super::PROGRESS_VERTICAL_INSET, 2);
        assert_eq!(
            super::PROGRESS_TRACK_HEIGHT
                - super::PROGRESS_VERTICAL_INSET
                - super::PROGRESS_FILL_HEIGHT,
            super::PROGRESS_VERTICAL_INSET,
        );
    }
}
