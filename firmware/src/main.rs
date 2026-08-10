use std::thread;
use std::time::Duration;

use anyhow::Result;
use esp_idf_svc::log::EspLogger;
use log::info;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32 Glance Deck firmware starting");
    info!("awaiting Wi-Fi provisioning, MQTT enrollment, and RLCD adapter");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
