use std::{env, fs, path::Path};

use glance_deck_firmware::{
    local_screen::{
        error_frame, maintenance_frame, ota_frame, pairing_code_frame, wifi_setup_frame,
        MaintenanceScreen,
    },
    mqtt::OtaPhase,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_directory = env::args()
        .nth(1)
        .ok_or("usage: render-local-screens <output-directory>")?;
    let output_directory = Path::new(&output_directory);
    fs::create_dir_all(output_directory)?;

    let screens = [
        ("boot", maintenance_frame(MaintenanceScreen::Connecting)?),
        ("pairing", pairing_code_frame("123456")?),
        ("wifi", wifi_setup_frame("GlanceDeck-AB12", "GD12AB34EF")?),
        (
            "maintenance",
            maintenance_frame(MaintenanceScreen::Overview)?,
        ),
        (
            "maintenance-hold",
            maintenance_frame(MaintenanceScreen::ConfirmWifiSetup)?,
        ),
        (
            "maintenance-starting",
            maintenance_frame(MaintenanceScreen::StartingWifiSetup)?,
        ),
        (
            "ota-checking",
            maintenance_frame(MaintenanceScreen::CheckingUpdate)?,
        ),
        (
            "ota-ready",
            maintenance_frame(MaintenanceScreen::UpdateReady { version: "0.2.0" })?,
        ),
        (
            "ota-uptodate",
            maintenance_frame(MaintenanceScreen::UpToDate)?,
        ),
        (
            "ota-failed",
            maintenance_frame(MaintenanceScreen::UpdateCheckFailed {
                reason: "NETWORK TIMEOUT",
            })?,
        ),
        ("ota-status", ota_frame(&OtaPhase::Downloading, Some(42))),
        (
            "error",
            error_frame("WIFI CONN FAILED", "REOPEN SETUP", Some("AUTH FAILED")),
        ),
    ];

    for (name, frame) in screens {
        fs::write(output_directory.join(format!("{name}.mono1")), frame)?;
    }
    Ok(())
}
