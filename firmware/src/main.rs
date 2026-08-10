use anyhow::{Context, Result};
use esp_idf_svc::log::EspLogger;
use log::{info, warn};

use glance_deck_firmware::{
    display::{DisplayCache, DisplayRelease},
    esp_config::load_device_config,
    esp_mqtt::EspDeviceMqtt,
    esp_storage::{DisplayStorage, HttpsPageDownloader},
    flash_cache::FlashDisplayCache,
    mqtt::{DeviceState, Device_command},
    provisioning_esp::{restart_requested, start_network, NetworkRuntime},
    release_sync::{synchronize_page, synchronize_release},
    rlcd::RlcdRenderer,
};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("ESP32 Glance Deck firmware starting");
    let network_runtime = start_network()?;
    let (_wifi, partition) = match network_runtime {
        NetworkRuntime::Connected { wifi, partition } => (wifi, partition),
        NetworkRuntime::Provisioning { wifi, _portal } => {
            info!("Wi-Fi provisioning portal is active");
            return wait_for_restart_with_portal(wifi, _portal);
        }
    };
    let config = load_device_config(&partition)?.context("device has not completed enrollment")?;
    let storage = DisplayStorage::mount()?;
    let mut cache = FlashDisplayCache::open(storage.cache_path())?;
    let mut renderer = RlcdRenderer::new()?;
    let mut downloader = HttpsPageDownloader::new();
    let mut mqtt = EspDeviceMqtt::connect(&config.mqtt, &config.device_id)?;
    info!("device runtime connected as {}", config.device_id);

    loop {
        if restart_requested() {
            info!("Wi-Fi settings saved; restarting after HTTP response completed");
            std::thread::sleep(std::time::Duration::from_millis(250));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
        if let Some(message) = mqtt.next_message() {
            if message.topic == mqtt.topics().release() {
                match serde_json::from_slice::<DisplayRelease>(&message.payload)
                    .map_err(anyhow::Error::from)
                    .and_then(|release| {
                        synchronize_release(&mut cache, &mut downloader, &release)
                            .map_err(|error| anyhow::anyhow!("release sync failed: {error:?}"))?;
                        render_and_report(
                            &mut cache,
                            &mut renderer,
                            &mut mqtt,
                            &release,
                            &release.active_page_id,
                            None,
                            true,
                        )
                    }) {
                    Ok(()) => info!("display release applied"),
                    Err(error) => warn!("display release retained prior frame: {error:#}"),
                }
            } else if message.topic == mqtt.topics().command() {
                match Device_command::from_payload(&message.payload) {
                    Ok(command)
                        if matches!(
                            command.action,
                            glance_deck_firmware::mqtt::Device_command_action::Show_page
                        ) =>
                    {
                        let Some(release) = cache.current_release()? else {
                            continue;
                        };
                        let Some(page_id) = command.payload.page_id.as_deref() else {
                            continue;
                        };
                        let result =
                            synchronize_page(&mut cache, &mut downloader, &release, page_id)
                                .map_err(|error| anyhow::anyhow!("page sync failed: {error:?}"))
                                .and_then(|_| {
                                    render_and_report(
                                        &mut cache,
                                        &mut renderer,
                                        &mut mqtt,
                                        &release,
                                        page_id,
                                        Some(&command.command_id),
                                        true,
                                    )
                                });
                        if let Err(error) = result {
                            warn!("page command failed: {error:#}");
                            publish_command_result(
                                &mut mqtt,
                                &release,
                                page_id,
                                &command.command_id,
                                false,
                                Some("page_not_available"),
                            )?;
                        }
                    }
                    Ok(_) => {}
                    Err(error) => warn!("rejected command: {error}"),
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn render_and_report(
    cache: &mut FlashDisplayCache,
    renderer: &mut RlcdRenderer,
    mqtt: &mut EspDeviceMqtt,
    release: &DisplayRelease,
    page_id: &str,
    command_id: Option<&str>,
    confirmed: bool,
) -> Result<()> {
    let page = release.page(page_id).context("requested page missing")?;
    let frame = cache
        .read_page(&page.image_sha256)?
        .context("requested frame missing")?;
    page.validate_image(&frame)?;
    renderer.flush_frame(&frame)?;
    let state = DeviceState {
        version: 1,
        page_id: page_id.to_owned(),
        wifi_rssi: 0,
        display_release_id: Some(release.release_id.clone()),
        display_updated_at: None,
        command_id: command_id.map(str::to_owned),
        command_status: command_id.map(|_| {
            if confirmed {
                glance_deck_firmware::mqtt::Command_status::Confirmed
            } else {
                glance_deck_firmware::mqtt::Command_status::Failed
            }
        }),
        error_message: None,
        firmware_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        power: None,
    };
    let payload = serde_json::to_vec(&state)?;
    glance_deck_firmware::mqtt::Mqtt_client::publish(mqtt, &mqtt.topics().state(), &payload, true)?;
    Ok(())
}

fn publish_command_result(
    mqtt: &mut EspDeviceMqtt,
    release: &DisplayRelease,
    page_id: &str,
    command_id: &str,
    confirmed: bool,
    error_message: Option<&str>,
) -> Result<()> {
    let state = DeviceState {
        version: 1,
        page_id: page_id.to_owned(),
        wifi_rssi: 0,
        display_release_id: Some(release.release_id.clone()),
        display_updated_at: None,
        command_id: Some(command_id.to_owned()),
        command_status: Some(if confirmed {
            glance_deck_firmware::mqtt::Command_status::Confirmed
        } else {
            glance_deck_firmware::mqtt::Command_status::Failed
        }),
        error_message: error_message.map(str::to_owned),
        firmware_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        power: None,
    };
    let payload = serde_json::to_vec(&state)?;
    glance_deck_firmware::mqtt::Mqtt_client::publish(mqtt, &mqtt.topics().state(), &payload, true)?;
    Ok(())
}

fn wait_for_restart_with_portal(
    _wifi: esp_idf_svc::wifi::BlockingWifi<esp_idf_svc::wifi::EspWifi<'static>>,
    _portal: esp_idf_svc::http::server::EspHttpServer<'static>,
) -> Result<()> {
    loop {
        if restart_requested() {
            std::thread::sleep(std::time::Duration::from_millis(250));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
