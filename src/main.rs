use anyhow::{Context, Result};
use esp_idf_svc::log::EspLogger;
use log::{info, warn};
use std::{
    sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
    thread,
    time::{Duration, Instant},
};

use glance_deck_firmware::{
    buttons::{KeyButton, KeyEvent},
    display::{DisplayCache, DisplayRelease},
    enrollment::EnrollmentSession,
    esp_config::{
        load_control_plane_url, load_device_config, request_wifi_provisioning, save_device_config,
    },
    esp_enrollment::announce_and_claim,
    esp_mqtt::EspDeviceMqtt,
    esp_ota::{mark_running_image_healthy, EspHttpsOtaTransport, InactiveOtaWriter},
    esp_storage::{DisplayStorage, HttpsPageDownloader},
    flash_cache::FlashDisplayCache,
    local_screen::{
        error_frame, maintenance_frame, ota_frame, page_indicator_frame, wifi_setup_frame,
        MaintenanceScreen,
    },
    mqtt::{
        DeviceCommand, DeviceCommandAction, DeviceState, MqttClient, OtaCheckState, OtaCheckStatus,
        OtaCommand, OtaPhase, OtaState,
    },
    ota::OtaPolicy,
    ota_runtime::{OtaProcessor, OtaReporter},
    power::{PowerProvider, UnavailablePowerProvider},
    provisioning_esp::{load_active_wifi_config, restart_requested, start_network, NetworkRuntime},
    release_sync::{synchronize_page, synchronize_release},
    rlcd::RlcdRenderer,
};

fn main() {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    if let Err(error) = run() {
        log::error!("firmware stopped: {error:#}");
        loop {
            std::thread::sleep(Duration::from_secs(10));
        }
    }
}

fn run() -> Result<()> {

    info!("ESP32 Glance Deck firmware starting");
    let mut renderer = RlcdRenderer::new()?;
    renderer.flush_frame(
        &maintenance_frame(MaintenanceScreen::Connecting).map_err(anyhow::Error::msg)?,
    )?;
    let network_runtime = start_network()?;
    let (_wifi, partition) = match network_runtime {
        NetworkRuntime::Connected { wifi, partition } => (wifi, partition),
        NetworkRuntime::Provisioning {
            wifi,
            _portal,
            portal_ssid,
            portal_password,
        } => {
            info!("Wi-Fi provisioning portal is active");
            let frame = wifi_setup_frame(&portal_ssid, &portal_password)
                .map_err(|error| anyhow::anyhow!(error))?;
            renderer.flush_frame(&frame)?;
            return wait_for_restart_with_portal(wifi, _portal);
        }
    };
    let storage = DisplayStorage::mount()?;
    let mut cache = FlashDisplayCache::open(storage.cache_path())?;
    let config = match load_device_config(&partition)? {
        Some(config) => config,
        None => enroll_device(&partition, &mut renderer)?,
    };
    let mut downloader = HttpsPageDownloader::new();
    let mut mqtt = EspDeviceMqtt::connect(&config.mqtt, &config.device_id)?;
    let mut key = KeyButton::new()?;
    let mut power = UnavailablePowerProvider;
    let (ota_jobs, ota_states) = start_ota_worker()?;
    let mut maintenance_long_presses = 0_u8;
    let mut last_ota_nonce: Option<String> = None;
    let mut local_ota_candidate: Option<OtaCommand> = None;
    let mut pending_page_indicator: Option<PendingPageIndicator> = None;
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
                        } else if let Ok(frame) =
                            maintenance_frame(MaintenanceScreen::CheckingUpdate)
                        {
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
                        &mut pending_page_indicator,
                    ) {
                        warn!("local page change retained prior frame: {error:#}");
                    } else {
                        current_page_id = Some(page_id);
                    }
                }
            }
            Some(KeyEvent::LongPress) => {
                if let Some(candidate) = local_ota_candidate.take() {
                    if let Err(error) = queue_ota(
                        &serde_json::to_vec(&candidate)?,
                        &mut power,
                        &mut last_ota_nonce,
                        &ota_jobs,
                    ) {
                        warn!("local OTA failed: {error:#}");
                    }
                    continue;
                }
                maintenance_long_presses = maintenance_long_presses.saturating_add(1);
                info!("maintenance long press {maintenance_long_presses}/3");
                let screen = match maintenance_long_presses {
                    1 => MaintenanceScreen::Overview,
                    2 => MaintenanceScreen::ConfirmWifiSetup,
                    _ => MaintenanceScreen::StartingWifiSetup,
                };
                if let Ok(frame) = maintenance_frame(screen) {
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
                    queue_ota(&message.payload, &mut power, &mut last_ota_nonce, &ota_jobs)
                {
                    warn!("OTA job failed: {error:#}");
                }
            } else if message.topic == mqtt.topics().ota_check_state() {
                match serde_json::from_slice::<OtaCheckState>(&message.payload) {
                    Ok(state) => match state.status {
                        OtaCheckStatus::Available => {
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
                                let candidate = OtaCommand {
                                    job_id,
                                    nonce,
                                    version,
                                    manifest_url,
                                    image_sha256,
                                };
                                if candidate.validate().is_ok() {
                                    if let Ok(frame) =
                                        maintenance_frame(MaintenanceScreen::UpdateReady {
                                            version: &candidate.version,
                                        })
                                    {
                                        let _ = renderer.flush_frame(&frame);
                                    }
                                    local_ota_candidate = Some(candidate);
                                }
                            }
                        }
                        OtaCheckStatus::UpToDate => {
                            if let Ok(frame) = maintenance_frame(MaintenanceScreen::UpToDate) {
                                let _ = renderer.flush_frame(&frame);
                            }
                        }
                        OtaCheckStatus::Failed => {
                            let reason = state.error_message.as_deref().unwrap_or("UNKNOWN ERROR");
                            if let Ok(frame) =
                                maintenance_frame(MaintenanceScreen::UpdateCheckFailed { reason })
                            {
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
                                &mut pending_page_indicator,
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
                match DeviceCommand::from_payload(&message.payload) {
                    Ok(command) => {
                        let Some(release) = cache.current_release()? else {
                            continue;
                        };
                        let page_id = match command.action {
                            DeviceCommandAction::ShowPage => command.payload.page_id.clone(),
                            DeviceCommandAction::NextPage => Some(adjacent_page_id(
                                &release,
                                current_page_id.as_deref(),
                                false,
                            )?),
                            DeviceCommandAction::PreviousPage => Some(adjacent_page_id(
                                &release,
                                current_page_id.as_deref(),
                                true,
                            )?),
                            DeviceCommandAction::RefreshRelease => Some(
                                current_page_id
                                    .clone()
                                    .unwrap_or_else(|| release.active_page_id.clone()),
                            ),
                            DeviceCommandAction::SetRotation
                            | DeviceCommandAction::EnterMaintenance => None,
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
                                        &mut pending_page_indicator,
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
        while let Ok(state) = ota_states.try_recv() {
            pending_page_indicator = None;
            report_ota_state(&mut mqtt, &mut renderer, state.clone());
            if state.phase == OtaPhase::Rebooting {
                std::thread::sleep(Duration::from_millis(250));
                unsafe { esp_idf_svc::sys::esp_restart() };
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
        restore_expired_page_indicator(&mut renderer, &mut pending_page_indicator)?;
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

struct OtaJob {
    command: OtaCommand,
    external_power: bool,
    battery_percent: Option<u8>,
    public_key: [u8; 32],
}

struct OtaWorkerReporter {
    states: SyncSender<OtaState>,
}

impl OtaReporter for OtaWorkerReporter {
    fn report(&mut self, state: OtaState) {
        let _ = self.states.send(state);
    }
}

fn start_ota_worker() -> Result<(SyncSender<OtaJob>, Receiver<OtaState>)> {
    let (job_sender, job_receiver) = sync_channel(1);
    let (state_sender, state_receiver) = sync_channel(16);
    thread::Builder::new()
        .name("ota-worker".to_owned())
        .stack_size(12 * 1024)
        .spawn(move || ota_worker(job_receiver, state_sender))
        .context("ota_worker_start_failed")?;
    Ok((job_sender, state_receiver))
}

fn ota_worker(jobs: Receiver<OtaJob>, states: SyncSender<OtaState>) {
    while let Ok(job) = jobs.recv() {
        let mut reporter = OtaWorkerReporter {
            states: states.clone(),
        };
        let writer = match InactiveOtaWriter::begin() {
            Ok(writer) => writer,
            Err(error) => {
                reporter.report(OtaState {
                    job_id: job.command.job_id.clone(),
                    phase: OtaPhase::Failed,
                    error_message: Some("ota_partition_unavailable".to_owned()),
                    progress_percent: None,
                });
                warn!("OTA partition unavailable: {error:#}");
                continue;
            }
        };
        let mut policy = OtaPolicy::new(job.external_power, job.battery_percent);
        let processor = OtaProcessor {
            board_model: "ESP32-S3-RLCD-4.2".to_owned(),
            public_key: job.public_key,
        };
        let mut transport = EspHttpsOtaTransport::new();
        if let Err(error) = processor.run(
            &job.command,
            &mut policy,
            &mut transport,
            writer,
            &mut reporter,
        ) {
            reporter.report(OtaState {
                job_id: job.command.job_id,
                phase: OtaPhase::Failed,
                error_message: Some(format!("{error:?}")),
                progress_percent: None,
            });
        }
    }
}

fn queue_ota(
    payload: &[u8],
    power: &mut impl glance_deck_firmware::power::PowerProvider,
    last_ota_nonce: &mut Option<String>,
    jobs: &SyncSender<OtaJob>,
) -> Result<()> {
    let command: OtaCommand =
        serde_json::from_slice(payload).context("ota_command_json_invalid")?;
    command.validate().map_err(anyhow::Error::msg)?;
    if last_ota_nonce.as_deref() == Some(command.nonce.as_str()) {
        anyhow::bail!("ota_nonce_replayed");
    }
    let measurement = power.sample();
    let external_power = matches!(
        measurement.source,
        glance_deck_firmware::mqtt::PowerSource::Usb
            | glance_deck_firmware::mqtt::PowerSource::UsbAndBattery
    );
    let public_key = firmware_public_key().context("firmware_public_key_missing")?;
    let job = OtaJob {
        command: command.clone(),
        external_power,
        battery_percent: measurement.battery_percent,
        public_key,
    };
    match jobs.try_send(job) {
        Ok(()) => {
            *last_ota_nonce = Some(command.nonce);
            Ok(())
        }
        Err(TrySendError::Full(_)) => anyhow::bail!("ota_already_running"),
        Err(TrySendError::Disconnected(_)) => anyhow::bail!("ota_worker_unavailable"),
    }
}

fn report_ota_state(mqtt: &mut EspDeviceMqtt, renderer: &mut RlcdRenderer, state: OtaState) {
    let frame = if state.phase == OtaPhase::Failed {
        error_frame(
            "UPDATE FAILED",
            "RETRY UPDATE",
            state.error_message.as_deref(),
        )
    } else {
        ota_frame(&state.phase, state.progress_percent)
    };
    if let Err(error) = renderer.flush_frame(&frame) {
        warn!("failed to render OTA status: {error:#}");
    }
    let Ok(payload) = serde_json::to_vec(&state) else {
        return;
    };
    let topic = mqtt.topics().ota_state();
    if let Err(error) = MqttClient::publish(mqtt, &topic, &payload, true) {
        warn!("failed to publish OTA state: {error}");
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
    let session = EnrollmentSession::from_random(random);
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
    power: &mut impl PowerProvider,
    pending_page_indicator: &mut Option<PendingPageIndicator>,
) -> Result<()> {
    let page = release.page(page_id).context("requested page missing")?;
    let frame = cache
        .read_page(&page.image_sha256)?
        .context("requested frame is not cached")?;
    page.validate_image(&frame)?;
    renderer.flush_frame(&frame)?;
    schedule_page_indicator(
        renderer,
        &frame,
        release,
        page_id,
        power,
        pending_page_indicator,
    )?;
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
    power: &mut impl PowerProvider,
    pending_page_indicator: &mut Option<PendingPageIndicator>,
) -> Result<()> {
    let page = release.page(page_id).context("requested page missing")?;
    let frame = cache
        .read_page(&page.image_sha256)?
        .context("requested frame missing")?;
    page.validate_image(&frame)?;
    renderer.flush_frame(&frame)?;
    schedule_page_indicator(
        renderer,
        &frame,
        release,
        page_id,
        power,
        pending_page_indicator,
    )?;
    publish_state(mqtt, release, page_id, command_id, confirmed, None, power)
}

struct PendingPageIndicator {
    restore_at: Instant,
    frame: Vec<u8>,
}

fn schedule_page_indicator(
    renderer: &mut RlcdRenderer,
    frame: &[u8],
    release: &DisplayRelease,
    page_id: &str,
    power: &mut impl PowerProvider,
    pending: &mut Option<PendingPageIndicator>,
) -> Result<()> {
    *pending = None;
    if power
        .sample()
        .battery_percent
        .is_some_and(|percent| percent < 10)
    {
        return Ok(());
    }
    let Some(active_index) = release
        .pages
        .iter()
        .position(|page| page.page_id == page_id)
    else {
        return Ok(());
    };
    let Some(overlay) = page_indicator_frame(frame, active_index, release.pages.len()) else {
        return Ok(());
    };
    renderer.flush_frame(&overlay)?;
    *pending = Some(PendingPageIndicator {
        restore_at: Instant::now() + Duration::from_secs(2),
        frame: frame.to_vec(),
    });
    Ok(())
}

fn restore_expired_page_indicator(
    renderer: &mut RlcdRenderer,
    pending: &mut Option<PendingPageIndicator>,
) -> Result<()> {
    if pending
        .as_ref()
        .is_some_and(|indicator| Instant::now() >= indicator.restore_at)
    {
        if let Some(indicator) = pending.take() {
            renderer.flush_frame(&indicator.frame)?;
        }
    }
    Ok(())
}

fn publish_state(
    mqtt: &mut EspDeviceMqtt,
    release: &DisplayRelease,
    page_id: &str,
    command_id: Option<&str>,
    confirmed: bool,
    error_message: Option<&str>,
    power: &mut impl PowerProvider,
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
                glance_deck_firmware::mqtt::CommandStatus::Confirmed
            } else {
                glance_deck_firmware::mqtt::CommandStatus::Failed
            }
        }),
        error_message: error_message.map(str::to_owned),
        firmware_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        power: Some(power.sample()),
    };
    let payload = serde_json::to_vec(&state)?;
    glance_deck_firmware::mqtt::MqttClient::publish(mqtt, &mqtt.topics().state(), &payload, true)?;
    Ok(())
}

fn publish_command_result(
    mqtt: &mut EspDeviceMqtt,
    release: &DisplayRelease,
    page_id: &str,
    command_id: &str,
    confirmed: bool,
    error_message: Option<&str>,
    power: &mut impl PowerProvider,
) -> Result<()> {
    let state = DeviceState {
        version: 1,
        page_id: page_id.to_owned(),
        wifi_rssi: 0,
        display_release_id: Some(release.release_id.clone()),
        display_updated_at: None,
        command_id: Some(command_id.to_owned()),
        command_status: Some(if confirmed {
            glance_deck_firmware::mqtt::CommandStatus::Confirmed
        } else {
            glance_deck_firmware::mqtt::CommandStatus::Failed
        }),
        error_message: error_message.map(str::to_owned),
        firmware_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        power: Some(power.sample()),
    };
    let payload = serde_json::to_vec(&state)?;
    glance_deck_firmware::mqtt::MqttClient::publish(mqtt, &mqtt.topics().state(), &payload, true)?;
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
