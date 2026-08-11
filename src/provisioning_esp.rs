use core::convert::TryInto;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use embedded_svc::{
    http::{Headers, Method},
    io::{Read, Write},
    wifi::{AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration},
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    http::server::{Configuration as HttpConfiguration, EspHttpServer},
    nvs::{EspDefaultNvs, EspDefaultNvsPartition},
    wifi::{BlockingWifi, EspWifi},
};
use log::{info, warn};

use crate::{
    config::WifiConfig,
    esp_config::{save_control_plane_url, take_wifi_provisioning_request},
};

const NVS_NAMESPACE: &str = "glance_deck";
const ACTIVE_WIFI_KEY: &str = "wifi_active";
const CANDIDATE_WIFI_KEY: &str = "wifi_candidate";
const MAX_WIFI_CONFIG_BYTES: usize = 256;
const MAX_PORTAL_REQUEST_BYTES: usize = 256;
const PORTAL_SSID: &str = "GlanceDeck-Setup";
const PORTAL_ADDRESS: &str = "192.168.4.1";
static RESTART_REQUESTED: AtomicBool = AtomicBool::new(false);

pub enum NetworkRuntime {
    Connected {
        wifi: BlockingWifi<EspWifi<'static>>,
        partition: EspDefaultNvsPartition,
    },
    Provisioning {
        wifi: BlockingWifi<EspWifi<'static>>,
        _portal: EspHttpServer<'static>,
        portal_password: String,
    },
}

pub fn start_network() -> Result<NetworkRuntime> {
    RESTART_REQUESTED.store(false, Ordering::Release);
    let peripherals = Peripherals::take().context("take ESP peripherals")?;
    let event_loop = EspSystemEventLoop::take().context("take system event loop")?;
    let partition = EspDefaultNvsPartition::take().context("open default NVS partition")?;
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(
            peripherals.modem,
            event_loop.clone(),
            Some(partition.clone()),
        )?,
        event_loop,
    )?;

    if take_wifi_provisioning_request(&partition)? {
        info!("starting requested Wi-Fi reprovisioning portal");
        return start_portal(wifi, partition);
    }

    // A portal submission never replaces the known-good configuration. The candidate
    // becomes active only after this boot obtains a DHCP address with it.
    if let Some(candidate) = load_wifi_config(&partition, CANDIDATE_WIFI_KEY)? {
        match connect_station(&mut wifi, &candidate) {
            Ok(()) => {
                promote_candidate(&partition, &candidate)?;
                info!("candidate Wi-Fi obtained an IP and is now active");
                return Ok(NetworkRuntime::Connected { wifi, partition });
            }
            Err(error) => {
                warn!("candidate Wi-Fi failed; preserved previous configuration: {error:#}")
            }
        }
    }

    match load_wifi_config(&partition, ACTIVE_WIFI_KEY)? {
        Some(config) => match connect_station(&mut wifi, &config) {
            Ok(()) => {
                info!("Wi-Fi connected using NVS configuration");
                Ok(NetworkRuntime::Connected { wifi, partition })
            }
            Err(error) => {
                warn!("saved Wi-Fi connection failed; starting provisioning portal: {error:#}");
                start_portal(wifi, partition)
            }
        },
        None => start_portal(wifi, partition),
    }
}

pub fn restart_requested() -> bool {
    RESTART_REQUESTED.load(Ordering::Acquire)
}

pub fn load_active_wifi_config(partition: &EspDefaultNvsPartition) -> Result<WifiConfig> {
    load_wifi_config(partition, ACTIVE_WIFI_KEY)?.context("active Wi-Fi configuration missing")
}

fn connect_station(wifi: &mut BlockingWifi<EspWifi<'static>>, config: &WifiConfig) -> Result<()> {
    if config.ssid.is_empty() || config.ssid.len() > 32 || config.password.len() > 64 {
        bail!("saved Wi-Fi configuration has invalid lengths");
    }
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: config
            .ssid
            .as_str()
            .try_into()
            .map_err(|_| anyhow::anyhow!("wifi_ssid_invalid"))?,
        password: config
            .password
            .as_str()
            .try_into()
            .map_err(|_| anyhow::anyhow!("wifi_password_invalid"))?,
        auth_method: if config.password.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        },
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    Ok(())
}

fn start_portal(
    mut wifi: BlockingWifi<EspWifi<'static>>,
    partition: EspDefaultNvsPartition,
) -> Result<NetworkRuntime> {
    let portal_password = format!("GD{:08X}", unsafe { esp_idf_svc::sys::esp_random() });
    wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: PORTAL_SSID
            .try_into()
            .map_err(|_| anyhow::anyhow!("portal_ssid_invalid"))?,
        password: portal_password
            .as_str()
            .try_into()
            .map_err(|_| anyhow::anyhow!("portal_password_invalid"))?,
        auth_method: AuthMethod::WPA2Personal,
        channel: 6,
        max_connections: 4,
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    let store = Arc::new(Mutex::new(EspDefaultNvs::new(
        partition,
        NVS_NAMESPACE,
        true,
    )?));
    let portal = start_portal_server(store)?;
    info!(
        "provisioning AP started; SSID={PORTAL_SSID}, password={portal_password}, URL=http://{PORTAL_ADDRESS}"
    );

    Ok(NetworkRuntime::Provisioning {
        wifi,
        _portal: portal,
        portal_password,
    })
}

fn start_portal_server(store: Arc<Mutex<EspDefaultNvs>>) -> Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(&HttpConfiguration {
        stack_size: 8192,
        ..Default::default()
    })?;
    server.fn_handler("/", Method::Get, |request| {
        request
            .into_ok_response()?
            .write_all(PORTAL_HTML.as_bytes())
            .map(|_| ())
    })?;

    // Captive portal probes used by Android, iOS, Windows, and macOS all land on setup.
    for path in [
        "/generate_204",
        "/hotspot-detect.html",
        "/ncsi.txt",
        "/connecttest.txt",
    ] {
        server.fn_handler(path, Method::Get, |request| {
            request
                .into_status_response(302)?
                .write_all(b"Open / for Glance Deck setup")
                .map(|_| ())
        })?;
    }

    server.fn_handler::<anyhow::Error, _>("/api/wifi", Method::Post, move |mut request| {
        let length = request.content_len().unwrap_or(0) as usize;
        if length == 0 || length > MAX_PORTAL_REQUEST_BYTES {
            request
                .into_status_response(413)?
                .write_all(b"invalid request length")?;
            return Ok(());
        }
        let mut payload = vec![0; length];
        request.read_exact(&mut payload)?;
        let config: Portal_config = match serde_json::from_slice::<Portal_config>(&payload) {
            Ok(config)
                if !config.ssid.is_empty()
                    && config.ssid.len() <= 32
                    && config.password.len() <= 64
                    && config.control_plane_url.starts_with("https://")
                    && config.control_plane_url.len() <= 256 =>
            {
                config
            }
            _ => {
                request
                    .into_status_response(400)?
                    .write_all(b"invalid Wi-Fi configuration")?;
                return Ok(());
            }
        };
        save_wifi_config(
            &store,
            CANDIDATE_WIFI_KEY,
            &WifiConfig {
                ssid: config.ssid,
                password: config.password,
            },
        )?;
        let mut nvs = store
            .lock()
            .map_err(|_| anyhow::anyhow!("NVS lock poisoned"))?;
        save_control_plane_url(&mut nvs, &config.control_plane_url)?;
        request
            .into_ok_response()?
            .write_all(b"saved; connecting to network")?;
        RESTART_REQUESTED.store(true, Ordering::Release);
        Ok(())
    })?;
    Ok(server)
}

#[derive(serde::Deserialize)]
struct Portal_config {
    ssid: String,
    password: String,
    control_plane_url: String,
}

fn load_wifi_config(partition: &EspDefaultNvsPartition, key: &str) -> Result<Option<WifiConfig>> {
    let nvs = EspDefaultNvs::new(partition.clone(), NVS_NAMESPACE, true)?;
    let mut buffer = [0_u8; MAX_WIFI_CONFIG_BYTES];
    let Some(value) = nvs.get_raw(key, &mut buffer)? else {
        return Ok(None);
    };
    Ok(Some(
        serde_json::from_slice(value).context("decode Wi-Fi configuration from NVS")?,
    ))
}

fn save_wifi_config(
    store: &Arc<Mutex<EspDefaultNvs>>,
    key: &str,
    config: &WifiConfig,
) -> Result<()> {
    let encoded = serde_json::to_vec(config)?;
    if encoded.len() > MAX_WIFI_CONFIG_BYTES {
        bail!("Wi-Fi configuration exceeds NVS limit");
    }
    store
        .lock()
        .map_err(|_| anyhow::anyhow!("NVS lock poisoned"))?
        .set_raw(key, &encoded)?;
    Ok(())
}

fn promote_candidate(partition: &EspDefaultNvsPartition, candidate: &WifiConfig) -> Result<()> {
    let store = Arc::new(Mutex::new(EspDefaultNvs::new(
        partition.clone(),
        NVS_NAMESPACE,
        true,
    )?));
    save_wifi_config(&store, ACTIVE_WIFI_KEY, candidate)?;
    store
        .lock()
        .map_err(|_| anyhow::anyhow!("NVS lock poisoned"))?
        .remove(CANDIDATE_WIFI_KEY)?;
    Ok(())
}

const PORTAL_HTML: &str = r#"<!doctype html><html><head><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>Glance Deck setup</title></head><body><h1>Glance Deck setup</h1><form id=f><label>Network <input name=ssid maxlength=32 required></label><br><label>Password <input name=password type=password maxlength=64></label><br><label>Console URL <input name=control_plane_url type=url placeholder=https://deck.example required></label><br><button>Connect</button></form><p id=s></p><script>f.onsubmit=async e=>{e.preventDefault();s.textContent='Saving…';let r=await fetch('/api/wifi',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(Object.fromEntries(new FormData(f))) });s.textContent=await r.text()}</script></body></html>"#;
