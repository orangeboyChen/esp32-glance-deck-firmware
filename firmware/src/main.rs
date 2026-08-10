use anyhow::Result;
use esp_idf_svc::log::EspLogger;
use log::info;

use glance_deck_firmware::provisioning::ProvisioningState;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32 Glance Deck firmware starting");
    let provisioning_state = ProvisioningState::Unprovisioned;
    info!("firmware state: {provisioning_state:?}");
    info!("awaiting Wi-Fi provisioning, MQTT enrollment, and RLCD adapter");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
