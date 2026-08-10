use anyhow::{Context, Result};
use esp_idf_svc::log::EspLogger;
use log::{info, warn};
use std::time::{Duration, Instant};

use glance_deck_firmware::{
    buttons::{KeyButton, KeyEvent},
    display::{DisplayCache, DisplayRelease},
    enrollment::Enrollment_session,
    esp_config::{
        load_control_plane_url, load_device_config, request_wifi_provisioning, save_device_config,
    },
    esp_enrollment::announce_and_claim,
    esp_mqtt::EspDeviceMqtt,
    esp_ota::{mark_running_image_healthy, EspHttpsOtaTransport, InactiveOtaWriter},
    esp_storage::{DisplayStorage, HttpsPageDownloader},
    flash_cache::FlashDisplayCache,
    local_screen::{maintenance_frame, wifi_setup_frame},
    mqtt::{
        DeviceState, Device_command, Device_command_action, Mqtt_client, Ota_check_state,
        Ota_check_status, Ota_command, Ota_phase, Ota_state,
    },
    ota::Ota_policy,
    ota_runtime::{OtaProcessor, OtaReporter},
    power::{Power_provider, Unavailable_power_provider},
    provisioning_esp::{load_active_wifi_config, restart_requested, start_network, NetworkRuntime},
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
        NetworkRuntime::Provisioning {
            wifi,
            _portal,
            portal_password,
        } => {
            info!("Wi-Fi provisioning portal is active");
            let mut renderer = RlcdRenderer::new()?;
            let frame =
                wifi_setup_frame(&portal_password).map_err(|error| anyhow::anyhow!(error))?;
            renderer.flush_frame(&frame)?;
            return wait_for_restart_with_portal(wifi, _portal);
        }
    };
    let storage = DisplayStorage::mount()?;
    let mut cache = FlashDisplayCache::open(storage.cache_path())?;
    let mut renderer = RlcdRenderer::new()?;
    let config = match load_device_config(&partition)? {
        Some(config) => config,
        None => enroll_device(&partition, &mut renderer)?,
    };
    let mut downloader = HttpsPageDownloader::new();
    let mut mqtt = EspDeviceMqtt::connect(&config.mqtt, &config.device_id)?;
    let mut key = KeyButton::new()?;
    let mut power = Unavailable_power_provider;
    let mut maintenance_long_presses = 0_u8;
    let mut last_ota_nonce: Option<String> = None;
    let mut local_ota_candidate: Option<Ota_command> = None;
    let mut last_periodic_state = Instant::now();
    let mut current_page_id = cache
        .current_release()?
        .map(|release| release.active_page_id);
    if let Some(release) = cache.current_release()? {
        let page_id = current_page_id
            .as_deref()
            .unwrap_or(&release.active_page_id)
            .to_owned();
        let page = release
            .page(&page_id)
            .context("cached release active page missing")?;
        let frame = cache
            .read_page(&page.image_sha256)?
            .context("cached release active frame missing")?;
        page.validate_image(&frame)?;
        renderer.flush_frame(&frame)?;
        if let Err(error) = mark_running_image_healthy() {
            warn!("OTA boot health confirmation unavailable: {error:#}");
        }
    } else {
        // A newly enrolled device may not yet have a release. Its local pairing
        // frame has already been rendered, and Wi-Fi plus MQTT are connected,
        // which is sufficient for candidate-image health confirmation.
        if let Err(error) = mark_running_image_healthy() {
            warn!("OTA boot health confirmation unavailable: {error:#}");
        }
    }
    info!("device runtime connected as {}", config.device_id);

    loop {
        if restart_requested() {
            info!("Wi-Fi settings saved; restarting after HTTP response completed");
            std::thread::sleep(std::time::Duration::from_millis(250));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
        match key.poll() {
            Some(KeyEvent::ShortPress) => {
                if maintenance_long_presses > 0 {
                    if maintenance_long_presses == 1 {
                        if let Err(error) = mqtt.request_ota_check() {
                            warn!("local OTA check request failed: {error}");
                        } else if let Ok(frame) = maintenance_frame("CHECKING UPDATE") {
                            let _ = renderer.flush_frame(&frame);
                        }
                    }
                    maintenance_long_presses = 0;
                    continue;
                }
                if let Some(release) = cache.current_release()? {
                    let page_id = adjacent_page_id(&release, current_page_id.as_deref(), false)?;
                    if let Err(error) = render_cached_page(
                        &mut cache,
                        &mut renderer,
                        &mut mqtt,
                        &release,
                        &page_id,
                        None,
                        &mut power,
                    ) {
                        warn!("local page change retained prior frame: {error:#}");
                    } else {
                        current_page_id = Some(page_id);
                    }
                }
            }
            Some(KeyEvent::LongPress) => {
                if let Some(candidate) = local_ota_candidate.take() {
                    if let Err(error) = handle_ota(
                        &serde_json::to_vec(&candidate)?,
                        &mut mqtt,
                        &mut power,
                        &mut last_ota_nonce,
                    ) {
                        warn!("local OTA failed: {error:#}");
                    }
                    continue;
                }
                maintenance_long_presses = maintenance_long_presses.saturating_add(1);
                info!("maintenance long press {maintenance_long_presses}/3");
                let message = match maintenance_long_presses {
                    1 => "MAINTENANCE",
                    2 => "HOLD 3X",
                    _ => "STARTING",
                };
                if let Ok(frame) = maintenance_frame(message) {
                    if let Err(error) = renderer.flush_frame(&frame) {
                        warn!("maintenance frame could not be rendered: {error:#}");
                    }
                }
                if maintenance_long_presses >= 3 {
                    request_wifi_provisioning(&partition)?;
                    info!("Wi-Fi reprovisioning requested; restarting");
                    unsafe { esp_idf_svc::sys::esp_restart() };
                }
            }
            None => {}
        }
        if let Some(message) = mqtt.next_message() {
            if message.topic == mqtt.topics().ota() {
                if let Err(error) =
                    handle_ota(&message.payload, &mut mqtt, &mut power, &mut last_ota_nonce)
                {
                    warn!("OTA job failed: {error:#}");
                }
            } else if message.topic == mqtt.topics().ota_check_state() {
                match serde_json::from_slice::<Ota_check_state>(&message.payload) {
                    Ok(state) => match state.status {
                        Ota_check_status::Available => {
                            if let (
                                Some(job_id),
                                Some(nonce),
                                Some(version),
                                Some(manifest_url),
                                Some(image_sha256),
                            ) = (
                                state.job_id,
                                state.nonce,
                                state.version,
                                state.manifest_url,
                                state.image_sha256,
                            ) {
                                let candidate = Ota_command {
                                    job_id,
                                    nonce,
                                    version,
                                    manifest_url,
                                    image_sha256,
                                };
                                if candidate.validate().is_ok() {
                                    local_ota_candidate = Some(candidate);
                                    if let Ok(frame) = maintenance_frame("UPDATE READY") {
                                        let _ = renderer.flush_frame(&frame);
                                    }
                                }
                            }
                        }
                        Ota_check_status::Up_to_date => {
                            if let Ok(frame) = maintenance_frame("UP TO DATE") {
                                let _ = renderer.flush_frame(&frame);
                            }
                        }
                        Ota_check_status::Failed => {
                            if let Ok(frame) = maintenance_frame("UPDATE CHECK FAILED") {
                                let _ = renderer.flush_frame(&frame);
                            }
                        }
                    },
                    Err(error) => warn!("rejected OTA check state: {error}"),
                }
            } else if message.topic == mqtt.topics().release() {
                match serde_json::from_slice::<DisplayRelease>(&message.payload) {
                    Ok(release) => {
                        let active_page_id = release.active_page_id.clone();
                        match (|| -> Result<()> {
                            synchronize_release(&mut cache, &mut downloader, &release).map_err(
                                |error| anyhow::anyhow!("release sync failed: {error:?}"),
                            )?;
                            render_and_report(
                                &mut cache,
                                &mut renderer,
                                &mut mqtt,
                                &release,
                                &release.active_page_id,
                                None,
                                true,
                                &mut power,
                            )
                        })() {
                            Ok(()) => {
                                current_page_id = Some(active_page_id);
                                info!("display release applied")
                            }
                            Err(error) => warn!("display release retained prior frame: {error:#}"),
                        }
                    }
                    Err(error) => warn!("rejected release metadata: {error}"),
                }
            } else if message.topic == mqtt.topics().command() {
                match Device_command::from_payload(&message.payload) {
                    Ok(command) => {
                        let Some(release) = cache.current_release()? else {
                            continue;
                        };
                        let page_id = match command.action {
                            Device_command_action::Show_page => command.payload.page_id.clone(),
                            Device_command_action::Next_page => Some(adjacent_page_id(
                                &release,
                                current_page_id.as_deref(),
                                false,
                            )?),
                            Device_command_action::Previous_page => Some(adjacent_page_id(
                                &release,
                                current_page_id.as_deref(),
                                true,
                            )?),
                            Device_command_action::Refresh_release => Some(
                                current_page_id
                                    .clone()
                                    .unwrap_or_else(|| release.active_page_id.clone()),
                            ),
                            Device_command_action::Set_rotation
                            | Device_command_action::Enter_maintenance => None,
                        };
                        let Some(page_id) = page_id else {
                            publish_command_result(
                                &mut mqtt,
                                &release,
                                current_page_id
                                    .as_deref()
                                    .unwrap_or(&release.active_page_id),
                                &command.command_id,
                                false,
                                Some("command_not_available"),
                                &mut power,
                            )?;
                            continue;
                        };
                        let result =
                            synchronize_page(&mut cache, &mut downloader, &release, &page_id)
                                .map_err(|error| anyhow::anyhow!("page sync failed: {error:?}"))
                                .and_then(|_| {
                                    render_and_report(
                                        &mut cache,
                                        &mut renderer,
                                        &mut mqtt,
                                        &release,
                                        &page_id,
                                        Some(&command.command_id),
                                        true,
                                        &mut power,
                                    )
                                });
                        if let Err(error) = result {
                            warn!("page command failed: {error:#}");
                            publish_command_result(
                                &mut mqtt,
                                &release,
                                &page_id,
                                &command.command_id,
                                false,
                                Some("page_not_available"),
                                &mut power,
                            )?;
                        } else {
                            current_page_id = Some(page_id);
                        }
                    }
                    Err(error) => warn!("rejected command: {error}"),
                }
            }
        }
        if last_periodic_state.elapsed() >= Duration::from_secs(15 * 60) {
            if let Some(release) = cache.current_release()? {
                let page_id = current_page_id
                    .as_deref()
                    .unwrap_or(&release.active_page_id);
                if let Err(error) =
                    publish_state(&mut mqtt, &release, page_id, None, true, None, &mut power)
                {
                    warn!("periodic device state publish failed: {error:#}");
                }
            }
            last_periodic_state = Instant::now();
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

struct MqttOtaReporter<'a> {
    mqtt: &'a mut EspDeviceMqtt,
}

impl OtaReporter for MqttOtaReporter<'_> {
    fn report(&mut self, state: Ota_state) {
        let Ok(payload) = serde_json::to_vec(&state) else {
            return;
        };
        let topic = self.mqtt.topics().ota_state();
        if let Err(error) = Mqtt_client::publish(self.mqtt, &topic, &payload, true) {
            warn!("failed to publish OTA state: {error}");
        }
    }
}

fn handle_ota(
    payload: &[u8],
    mqtt: &mut EspDeviceMqtt,
    power: &mut impl glance_deck_firmware::power::Power_provider,
    last_ota_nonce: &mut Option<String>,
) -> Result<()> {
    let command: Ota_command =
        serde_json::from_slice(payload).context("ota_command_json_invalid")?;
    command.validate().map_err(anyhow::Error::msg)?;
    if last_ota_nonce.as_deref() == Some(command.nonce.as_str()) {
        anyhow::bail!("ota_nonce_replayed");
    }
    let mut reporter = MqttOtaReporter { mqtt };
    let measurement = power.sample();
    let external_power = matches!(
        measurement.source,
        glance_deck_firmware::mqtt::Power_source::Usb
            | glance_deck_firmware::mqtt::Power_source::Usb_and_battery
    );
    let mut policy = Ota_policy::new(external_power, measurement.battery_percent);
    let public_key = match firmware_public_key() {
        Some(key) => key,
        None => {
            let error = "firmware_public_key_missing";
            reporter.report(Ota_state {
                job_id: command.job_id,
                phase: Ota_phase::Failed,
                error_message: Some(error.to_owned()),
            });
            return Err(anyhow::anyhow!(error));
        }
    };
    let processor = OtaProcessor {
        board_model: "ESP32-S3-RLCD-4.2".to_owned(),
        public_key,
    };
    let mut transport = EspHttpsOtaTransport::new();
    let writer = match InactiveOtaWriter::begin() {
        Ok(writer) => writer,
        Err(error) => {
            reporter.report(Ota_state {
                job_id: command.job_id,
                phase: Ota_phase::Failed,
                error_message: Some("ota_partition_unavailable".to_owned()),
            });
            return Err(error);
        }
    };
    match processor.run(&command, &mut policy, &mut transport, writer, &mut reporter) {
        Ok(()) => {
            *last_ota_nonce = Some(command.nonce.clone());
            std::thread::sleep(std::time::Duration::from_millis(250));
            unsafe { esp_idf_svc::sys::esp_restart() };
            Ok(())
        }
        Err(error) => {
            reporter.report(Ota_state {
                job_id: command.job_id,
                phase: Ota_phase::Failed,
                error_message: Some(format!("{error:?}")),
            });
            Err(anyhow::anyhow!("ota_failed_{error:?}"))
        }
    }
}

fn firmware_public_key() -> Option<[u8; 32]> {
    let encoded = option_env!("FIRMWARE_MANIFEST_PUBLIC_KEY_HEX")?;
    let bytes = hex::decode(encoded).ok()?;
    bytes.try_into().ok()
}

fn enroll_device(
    partition: &esp_idf_svc::nvs::EspDefaultNvsPartition,
    renderer: &mut RlcdRenderer,
) -> Result<glance_deck_firmware::config::DeviceConfig> {
    let control_plane_url = load_control_plane_url(partition)?
        .context("control plane URL missing; reopen Wi-Fi setup")?;
    let wifi = load_active_wifi_config(partition)?;
    let mut random = [0_u8; 32];
    for word in random.chunks_exact_mut(4) {
        word.copy_from_slice(&unsafe { esp_idf_svc::sys::esp_random() }.to_le_bytes());
    }
    let session = Enrollment_session::from_random(random);
    renderer.show_pairing_code(&session.pairing_code)?;
    loop {
        match announce_and_claim(&control_plane_url, &session, wifi.clone()) {
            Ok(Some(config)) => {
                save_device_config(partition, &config)?;
                return Ok(config);
            }
            Ok(None) => {}
            Err(error) => warn!("enrollment request failed: {error:#}"),
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

fn adjacent_page_id(
    release: &DisplayRelease,
    current_page_id: Option<&str>,
    previous: bool,
) -> Result<String> {
    let current_index = current_page_id
        .and_then(|page_id| {
            release
                .pages
                .iter()
                .position(|page| page.page_id == page_id)
        })
        .or_else(|| {
            release
                .pages
                .iter()
                .position(|page| page.page_id == release.active_page_id)
        })
        .context("release has no active page")?;
    let page_count = release.pages.len();
    let index = if previous {
        (current_index + page_count - 1) % page_count
    } else {
        (current_index + 1) % page_count
    };
    Ok(release.pages[index].page_id.clone())
}

fn render_cached_page(
    cache: &mut FlashDisplayCache,
    renderer: &mut RlcdRenderer,
    mqtt: &mut EspDeviceMqtt,
    release: &DisplayRelease,
    page_id: &str,
    command_id: Option<&str>,
    power: &mut impl Power_provider,
) -> Result<()> {
    let page = release.page(page_id).context("requested page missing")?;
    let frame = cache
        .read_page(&page.image_sha256)?
        .context("requested frame is not cached")?;
    page.validate_image(&frame)?;
    renderer.flush_frame(&frame)?;
    publish_state(mqtt, release, page_id, command_id, true, None, power)
}

fn render_and_report(
    cache: &mut FlashDisplayCache,
    renderer: &mut RlcdRenderer,
    mqtt: &mut EspDeviceMqtt,
    release: &DisplayRelease,
    page_id: &str,
    command_id: Option<&str>,
    confirmed: bool,
    power: &mut impl Power_provider,
) -> Result<()> {
    let page = release.page(page_id).context("requested page missing")?;
    let frame = cache
        .read_page(&page.image_sha256)?
        .context("requested frame missing")?;
    page.validate_image(&frame)?;
    renderer.flush_frame(&frame)?;
    publish_state(mqtt, release, page_id, command_id, confirmed, None, power)
}

fn publish_state(
    mqtt: &mut EspDeviceMqtt,
    release: &DisplayRelease,
    page_id: &str,
    command_id: Option<&str>,
    confirmed: bool,
    error_message: Option<&str>,
    power: &mut impl Power_provider,
) -> Result<()> {
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
        error_message: error_message.map(str::to_owned),
        firmware_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        power: Some(power.sample()),
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
    power: &mut impl Power_provider,
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
        power: Some(power.sample()),
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
