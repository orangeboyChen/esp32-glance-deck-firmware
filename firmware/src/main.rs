use anyhow::Result;
use esp_idf_svc::log::EspLogger;
use log::info;

use glance_deck_firmware::provisioning_esp::{restart_requested, start_network};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32 Glance Deck firmware starting");
    let _network_runtime = start_network()?;
    info!("network runtime started; awaiting MQTT enrollment and RLCD adapter");

    loop {
        if restart_requested() {
            info!("Wi-Fi settings saved; restarting after HTTP response completed");
            std::thread::sleep(std::time::Duration::from_millis(250));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
