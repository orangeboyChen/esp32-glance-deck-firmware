use super::canvas::Canvas;

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
            canvas.centered_text(142, "PLEASE WAIT", 2);
        }
        MaintenanceScreen::Overview => {
            canvas.centered_text(72, "MAINTENANCE", 3);
            canvas.centered_text(116, "SHORT: CHECK UPDATE", 1);
            canvas.centered_text(154, "LONG: WIFI SETUP", 2);
        }
        MaintenanceScreen::ConfirmWifiSetup => {
            canvas.header("WIFI SETUP", None);
            canvas.centered_text(128, "LONG AGAIN TO START", 2);
            canvas.centered_text(172, "SHORT TO CANCEL", 1);
        }
        MaintenanceScreen::StartingWifiSetup => {
            canvas.header("WIFI SETUP", Some("STARTING ACCESS POINT"));
            canvas.centered_text(144, "RESTARTING", 2);
        }
        MaintenanceScreen::CheckingUpdate => {
            canvas.centered_text(62, "SYSTEM UPDATE", 3);
            canvas.centered_text(110, "CHECKING FOR RELEASE", 1);
            canvas.centered_text(146, "CHECKING", 3);
        }
        MaintenanceScreen::UpdateReady { version } => {
            canvas.centered_text(54, "SYSTEM UPDATE", 3);
            canvas.centered_text(102, "UPDATE READY", 1);
            canvas.centered_text(130, &format!("VERSION {version}"), 1);
            canvas.centered_text(178, "LONG TO APPLY", 2);
            canvas.centered_text(218, "SHORT TO CANCEL", 1);
        }
        MaintenanceScreen::UpToDate => {
            canvas.centered_text(142, "UP TO DATE", 3);
        }
        MaintenanceScreen::UpdateCheckFailed { reason } => {
            canvas.centered_text(54, "SYSTEM UPDATE", 3);
            canvas.centered_text(102, "CHECK FAILED", 1);
            canvas.centered_text(170, &bounded_text(reason), 1);
        }
    }
    Ok(canvas.finish())
}

pub fn error_frame(failure: &str, action: &str, reason: Option<&str>) -> Vec<u8> {
    let mut canvas = Canvas::new();
    canvas.centered_text(90, &bounded_text(failure), 3);
    canvas.centered_text(142, &bounded_text(action), 2);
    if let Some(reason) = reason {
        canvas.centered_text(184, &bounded_text(reason), 2);
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
