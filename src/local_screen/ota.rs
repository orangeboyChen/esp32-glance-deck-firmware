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

    /// Every non-downloading phase renders the stage flow instead of the progress bar, and the
    /// frame must stay a complete display buffer in each case.
    #[test]
    fn renders_every_phase_as_a_complete_frame() {
        for (phase, percent) in [
            (OtaPhase::Verifying, None),
            (OtaPhase::Rebooting, None),
            (OtaPhase::Healthy, None),
            (OtaPhase::RolledBack, None),
            (OtaPhase::Failed, None),
            (OtaPhase::Downloading, Some(0)),
            (OtaPhase::Downloading, Some(100)),
        ] {
            let frame = ota_frame(&phase, percent);
            assert_eq!(frame.len(), DISPLAY_IMAGE_BYTES, "{phase:?}");
        }
    }

    /// A missing percentage is treated as zero rather than rendering a blank bar.
    #[test]
    fn defaults_a_missing_percentage_to_zero() {
        assert_eq!(
            ota_frame(&OtaPhase::Downloading, None),
            ota_frame(&OtaPhase::Downloading, Some(0))
        );
    }

    /// The display is 8-bit; clamping keeps a >100 value from wrapping the bar or the label.
    #[test]
    fn clamps_a_percentage_above_one_hundred() {
        assert_eq!(
            ota_frame(&OtaPhase::Downloading, Some(255)),
            ota_frame(&OtaPhase::Downloading, Some(100))
        );
    }

    /// Rebooting advances the stage flow, so it must not render the same frame as the other phases.
    #[test]
    fn distinguishes_rebooting_from_the_other_stages() {
        let rebooting = ota_frame(&OtaPhase::Rebooting, None);
        for phase in [OtaPhase::Verifying, OtaPhase::Healthy, OtaPhase::Failed] {
            assert_ne!(rebooting, ota_frame(&phase, None), "{phase:?}");
        }
        // Rolled back and rebooting land on different stages despite both being terminal-ish.
        assert_ne!(rebooting, ota_frame(&OtaPhase::RolledBack, None));
    }
}
