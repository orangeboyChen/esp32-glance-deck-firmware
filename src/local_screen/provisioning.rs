use super::canvas::Canvas;

pub fn pairing_code_frame(pairing_code: &str) -> Result<Vec<u8>, &'static str> {
    if pairing_code.len() != 6 || !pairing_code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("pairing_code_invalid");
    }
    let mut canvas = Canvas::new();
    canvas.header("PAIR DEVICE", Some("ENTER CODE IN CONSOLE"));
    canvas.centered_text(120, pairing_code, 5);
    canvas.centered_text(184, "OPEN CONSOLE TO CONTINUE", 1);
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
    canvas.text(28, 112, "SSID", 1);
    canvas.ellipsized_text(92, 112, ssid, 1, 280);
    canvas.text(28, 164, "PASSWORD", 1);
    canvas.text(28, 188, password, 3);
    Ok(canvas.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::DISPLAY_IMAGE_BYTES;

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
}
