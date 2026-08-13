use super::{canvas::Canvas, draw_icon, Icon};
use crate::mqtt::OtaPhase;

pub fn ota_frame(phase: &OtaPhase, percent: Option<u8>) -> Vec<u8> {
    let mut canvas = Canvas::new();
    let subtitle = match phase {
        OtaPhase::Downloading => "DOWNLOADING",
        OtaPhase::Verifying => "VERIFYING IMAGE",
        OtaPhase::Rebooting => "RESTARTING",
        OtaPhase::Healthy => "COMPLETE",
        OtaPhase::RolledBack => "ROLLED BACK",
        OtaPhase::Failed => "UPDATE FAILED",
    };
    canvas.header("SYSTEM UPDATE", Some(subtitle));
    draw_icon(&mut canvas, Icon::Download, 340, 26);
    canvas.horizontal_line(28, 96, 344);
    if matches!(phase, OtaPhase::Downloading) {
        let percent = percent.unwrap_or(0).min(100);
        canvas.row(118, "PROGRESS", &format!("{percent}%"));
        canvas.progress(144, percent);
        canvas.centered_text(159, "KEEP POWER CONNECTED", 1);
        canvas.draw_stage_flow(200, 230, 0);
    } else {
        canvas.draw_stage_flow(
            200,
            144,
            if matches!(phase, OtaPhase::Rebooting) {
                2
            } else {
                1
            },
        );
        canvas.centered_text(182, subtitle, 2);
    }
    canvas.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::DISPLAY_IMAGE_BYTES;

    #[test]
    fn renders_a_separate_progress_line_and_bar() {
        let frame = ota_frame(&OtaPhase::Downloading, Some(42));
        assert_eq!(frame.len(), DISPLAY_IMAGE_BYTES);
        assert_ne!(frame, ota_frame(&OtaPhase::Downloading, Some(0)));
    }

    #[test]
    fn balances_the_progress_bar_with_equal_visible_vertical_gaps() {
        let frame = ota_frame(&OtaPhase::Downloading, Some(42));
        let row_is_occupied = |y: usize| {
            (0..crate::display::DISPLAY_WIDTH).any(|x| {
                let offset = y * crate::display::DISPLAY_WIDTH + x;
                frame[offset / 8] & (0x80 >> (offset % 8)) != 0
            })
        };
        let progress_text_bottom = (118..144).rev().find(|&y| row_is_occupied(y)).unwrap();
        let reminder_top = (152..230).find(|&y| row_is_occupied(y)).unwrap();
        let gap_above = 144 - progress_text_bottom - 1;
        let gap_below = reminder_top - (144 + 8);

        assert_eq!(gap_above, gap_below);
    }
}
