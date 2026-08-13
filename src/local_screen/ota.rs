use super::canvas::Canvas;
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
    if matches!(phase, OtaPhase::Downloading) {
        let percent = percent.unwrap_or(0).min(100);
        canvas.row(116, "PROGRESS", &format!("{percent}%"));
        canvas.progress(142, percent);
        canvas.centered_text(194, "KEEP POWER CONNECTED", 1);
    } else {
        canvas.centered_text(138, subtitle, 2);
        canvas.centered_text(194, "KEEP POWER CONNECTED", 1);
    }
    canvas.centered_text(240, "DOWNLOAD VERIFY RESTART", 1);
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
}
