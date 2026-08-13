use super::{canvas::Canvas, draw_icon, Icon};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceScreen<'a> {
    Connecting,
    Overview,
    ConfirmWifiSetup,
    StartingWifiSetup,
    CheckingUpdate,
    UpdateReady { version: &'a str },
    UpToDate,
    UpdateCheckFailed { reason: &'a str },
}

pub fn maintenance_frame(screen: MaintenanceScreen<'_>) -> Result<Vec<u8>, &'static str> {
    let mut canvas = Canvas::new();
    match screen {
        MaintenanceScreen::Connecting => {
            canvas.header("CONNECTING", Some("WIFI AND CONTROL PLANE"));
            draw_icon(&mut canvas, Icon::Wifi, 340, 26);
            canvas.horizontal_line(28, 96, 344);
            canvas.draw_stage_flow(200, 144, 0);
            canvas.centered_text(180, "JOINING NETWORK", 1);
        }
        MaintenanceScreen::Overview => {
            draw_icon(&mut canvas, Icon::Maintenance, 184, 28);
            canvas.centered_text(78, "MAINTENANCE", 3);
            canvas.horizontal_line(28, 118, 344);
            canvas.vertical_line(200, 136, 72);
            draw_icon(&mut canvas, Icon::ShortPress, 112, 140);
            draw_icon(&mut canvas, Icon::LongPress, 256, 140);
            canvas.centered_text_at(128, 178, "CHECK UPDATE", 1);
            canvas.centered_text_at(272, 178, "WIFI SETUP", 1);
            canvas.centered_text_at(128, 208, "SHORT", 1);
            canvas.centered_text_at(272, 208, "HOLD", 1);
        }
        MaintenanceScreen::ConfirmWifiSetup => {
            canvas.header("WIFI SETUP", Some("CONFIRM ACCESS POINT"));
            draw_icon(&mut canvas, Icon::Wifi, 340, 26);
            canvas.horizontal_line(28, 96, 344);
            draw_icon(&mut canvas, Icon::LongPress, 184, 122);
            canvas.centered_text(166, "HOLD TO START", 2);
            canvas.centered_text(214, "SHORT PRESS CANCELS", 1);
        }
        MaintenanceScreen::StartingWifiSetup => {
            canvas.header("WIFI SETUP", Some("STARTING ACCESS POINT"));
            draw_icon(&mut canvas, Icon::Wifi, 340, 26);
            canvas.horizontal_line(28, 96, 344);
            canvas.draw_stage_flow(200, 144, 1);
            canvas.centered_text(180, "RESTARTING", 2);
        }
        MaintenanceScreen::CheckingUpdate => {
            canvas.header("SYSTEM UPDATE", Some("CHECKING FOR RELEASE"));
            draw_icon(&mut canvas, Icon::Update, 340, 26);
            canvas.horizontal_line(28, 96, 344);
            draw_icon(&mut canvas, Icon::Checking, 184, 124);
            canvas.centered_text(180, "CHECKING", 2);
        }
        MaintenanceScreen::UpdateReady { version } => {
            canvas.header("SYSTEM UPDATE", Some("UPDATE READY"));
            draw_icon(&mut canvas, Icon::Update, 340, 26);
            canvas.horizontal_line(28, 96, 344);
            draw_icon(&mut canvas, Icon::Download, 184, 116);
            canvas.centered_text(164, &format!("VERSION {version}"), 1);
            draw_icon(&mut canvas, Icon::LongPress, 184, 190);
            canvas.centered_text(220, "HOLD TO APPLY", 1);
        }
        MaintenanceScreen::UpToDate => {
            canvas.header("SYSTEM UPDATE", Some("CHECK COMPLETE"));
            draw_icon(&mut canvas, Icon::Update, 340, 26);
            canvas.horizontal_line(28, 96, 344);
            draw_icon(&mut canvas, Icon::CheckMark, 184, 124);
            canvas.centered_text(180, "UP TO DATE", 2);
        }
        MaintenanceScreen::UpdateCheckFailed { reason } => {
            canvas.header("SYSTEM UPDATE", Some("CHECK FAILED"));
            draw_icon(&mut canvas, Icon::Update, 340, 26);
            canvas.horizontal_line(28, 96, 344);
            draw_icon(&mut canvas, Icon::Failed, 184, 124);
            canvas.centered_text(182, &bounded_text(reason), 1);
        }
    }
    Ok(canvas.finish())
}

pub fn error_frame(failure: &str, action: &str, reason: Option<&str>) -> Vec<u8> {
    let mut canvas = Canvas::new();
    draw_icon(&mut canvas, Icon::Error, 184, 32);
    canvas.centered_text(86, &bounded_text(failure), 3);
    canvas.horizontal_line(28, 124, 344);
    canvas.centered_text(150, &bounded_text(action), 2);
    if let Some(reason) = reason {
        canvas.centered_text(194, &bounded_text(reason), 1);
    }
    canvas.finish()
}

fn bounded_text(value: &str) -> String {
    let normalized: String = value
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' => byte.to_ascii_uppercase(),
            b'A'..=b'Z' | b'0'..=b'9' | b' ' | b'-' | b'.' => byte,
            _ => b' ',
        })
        .map(char::from)
        .collect();
    if normalized.len() <= 16 {
        normalized
    } else {
        format!("{}...", &normalized[..13])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::DISPLAY_IMAGE_BYTES;

    #[test]
    fn renders_every_bounded_maintenance_state() {
        for screen in [
            MaintenanceScreen::Overview,
            MaintenanceScreen::Connecting,
            MaintenanceScreen::ConfirmWifiSetup,
            MaintenanceScreen::StartingWifiSetup,
            MaintenanceScreen::CheckingUpdate,
            MaintenanceScreen::UpdateReady { version: "0.2.0" },
            MaintenanceScreen::UpToDate,
            MaintenanceScreen::UpdateCheckFailed { reason: "NETWORK" },
        ] {
            assert_eq!(
                maintenance_frame(screen).unwrap().len(),
                DISPLAY_IMAGE_BYTES
            );
        }
    }

    #[test]
    fn bounds_error_text_to_the_local_glyph_contract() {
        let frame = error_frame(
            "wifi_connection_failed",
            "reopen_setup",
            Some("auth failed"),
        );
        assert_eq!(frame.len(), DISPLAY_IMAGE_BYTES);
        assert_eq!(
            bounded_text("abcdefghijklmnopqrstuvwxyz"),
            "ABCDEFGHIJKLM..."
        );
    }
}
