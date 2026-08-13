use super::{canvas::Canvas, draw_icon, Icon};

const CREDENTIAL_VALUE_SCALE: usize = 3;
const HEADER_DIVIDER_TOP: usize = 96;
const SSID_LABEL_TOP: usize = 114;
const SSID_VALUE_TOP: usize = 132;
const CREDENTIAL_DIVIDER_TOP: usize = 178;
const PASSWORD_LABEL_TOP: usize = 194;
const PASSWORD_VALUE_TOP: usize = 212;

pub fn pairing_code_frame(pairing_code: &str) -> Result<Vec<u8>, &'static str> {
    if pairing_code.len() != 6 || !pairing_code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("pairing_code_invalid");
    }
    let mut canvas = Canvas::new();
    canvas.header("PAIR DEVICE", Some("ENTER CODE IN CONSOLE"));
    draw_icon(&mut canvas, Icon::Pairing, 340, 26);
    canvas.horizontal_line(28, HEADER_DIVIDER_TOP, 344);
    canvas.stroke_rect(56, 112, 288, 78, 2);
    canvas.centered_text(124, pairing_code, 5);
    canvas.centered_text(214, "ENTER THIS CODE IN CONSOLE", 1);
    Ok(canvas.finish())
}

pub fn wifi_setup_frame(ssid: &str, password: &str) -> Result<Vec<u8>, &'static str> {
    if ssid.is_empty()
        || password.len() != 10
        || !password
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    {
        return Err("wifi_setup_invalid");
    }
    let mut canvas = Canvas::new();
    canvas.header("WIFI SETUP", Some("JOIN THIS NETWORK"));
    draw_icon(&mut canvas, Icon::Wifi, 340, 26);
    canvas.horizontal_line(28, HEADER_DIVIDER_TOP, 344);
    canvas.text(28, SSID_LABEL_TOP, "SSID", 1);
    canvas.ellipsized_text(28, SSID_VALUE_TOP, ssid, CREDENTIAL_VALUE_SCALE, 344);
    canvas.horizontal_line(28, CREDENTIAL_DIVIDER_TOP, 344);
    canvas.text(28, PASSWORD_LABEL_TOP, "PASSWORD", 1);
    canvas.text(28, PASSWORD_VALUE_TOP, password, CREDENTIAL_VALUE_SCALE);
    Ok(canvas.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{display::DISPLAY_IMAGE_BYTES, local_screen::font::text_width};

    #[test]
    fn renders_pairing_code_in_the_shared_local_layout() {
        assert_eq!(
            pairing_code_frame("123456").unwrap().len(),
            DISPLAY_IMAGE_BYTES
        );
        assert_ne!(
            pairing_code_frame("123456").unwrap(),
            pairing_code_frame("654321").unwrap()
        );
    }

    #[test]
    fn bounds_and_ellipsizes_wifi_credentials() {
        assert_eq!(
            wifi_setup_frame("GlanceDeck-AB12", "GD12AB34EF")
                .unwrap()
                .len(),
            DISPLAY_IMAGE_BYTES
        );
        assert_eq!(
            wifi_setup_frame("", "GD12AB34EF"),
            Err("wifi_setup_invalid")
        );
        assert_eq!(
            wifi_setup_frame("GlanceDeck", "short"),
            Err("wifi_setup_invalid")
        );
    }

    #[test]
    fn uses_the_same_large_credential_font_for_ssid_and_password() {
        assert_eq!(super::CREDENTIAL_VALUE_SCALE, 3);
        assert!(text_width("GlanceDeck-AB12", super::CREDENTIAL_VALUE_SCALE) <= 344);
        assert!(text_width("GD12AB34EF", super::CREDENTIAL_VALUE_SCALE) <= 344);
    }
}
