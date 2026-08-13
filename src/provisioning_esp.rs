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
        portal_ssid: String,
        portal_password: String,
    },
}

pub fn start_network() -> Result<NetworkRuntime> {
    RESTART_REQUESTED.store(false, Ordering::Release);
    let peripherals = Peripherals::take().context("take ESP peripherals")?;
    let partition = EspDefaultNvsPartition::take().context("open default NVS partition")?;
    let event_loop = EspSystemEventLoop::take().context("take system event loop")?;
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
    let portal_ssid = portal_ssid()?;
    let portal_password = format!("GD{:08X}", unsafe { esp_idf_svc::sys::esp_random() });
    wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: portal_ssid
            .as_str()
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
        "provisioning AP started; SSID={portal_ssid}, password={portal_password}, URL=http://{PORTAL_ADDRESS}"
    );

    Ok(NetworkRuntime::Provisioning {
        wifi,
        _portal: portal,
        portal_ssid,
        portal_password,
    })
}

fn portal_ssid() -> Result<String> {
    let mut mac = [0_u8; 6];
    let result = unsafe { esp_idf_svc::sys::esp_efuse_mac_get_default(mac.as_mut_ptr()) };
    if result != esp_idf_svc::sys::ESP_OK {
        bail!("read device MAC for provisioning SSID failed: {result}");
    }
    Ok(format!("GlanceDeck-{:02X}{:02X}", mac[4], mac[5]))
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
        let config: PortalConfig = match serde_json::from_slice::<PortalConfig>(&payload) {
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
struct PortalConfig {
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

const PORTAL_HTML: &str = r###"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
  <meta name="theme-color" content="#f4f7f2">
  <title>Set up Glance Deck</title>
  <style>
    :root { color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    * { box-sizing: border-box; }
    body { align-items: center; background: #f4f7f2; color: #17322a; display: flex; margin: 0; min-height: 100dvh; padding: max(24px, env(safe-area-inset-top)) max(16px, env(safe-area-inset-right)) max(24px, env(safe-area-inset-bottom)) max(16px, env(safe-area-inset-left)); }
    main { margin: auto; max-width: 460px; width: 100%; }
    .masthead { align-items: center; display: flex; gap: 12px; margin: 0 8px 20px; }
    .mark { align-items: center; background: #1b6b55; border-radius: 12px; color: #fff; display: flex; font-size: 22px; font-weight: 800; height: 44px; justify-content: center; letter-spacing: -2px; width: 44px; }
    .eyebrow { color: #4c625b; font-size: 12px; font-weight: 700; letter-spacing: .12em; margin: 0 0 3px; text-transform: uppercase; }
    .name { font-size: 18px; font-weight: 750; letter-spacing: -.02em; margin: 0; }
    section { background: #fff; border: 1px solid #d6dfd9; border-radius: 20px; box-shadow: 0 16px 36px rgba(20, 56, 43, .09); overflow: hidden; }
    .intro { border-bottom: 1px solid #e4ebe6; padding: 28px 24px 22px; }
    h1 { font-size: clamp(25px, 7vw, 32px); letter-spacing: -.04em; line-height: 1.08; margin: 0 0 10px; }
    .intro p { color: #52655e; font-size: 16px; line-height: 1.5; margin: 0; }
    form { padding: 24px; }
    .field { margin-bottom: 18px; }
    label { display: block; font-size: 14px; font-weight: 700; margin: 0 0 8px; }
    .hint { color: #60736b; font-size: 13px; line-height: 1.4; margin: -2px 0 9px; }
    .input-wrap { position: relative; }
    input { appearance: none; background: #f8faf8; border: 1px solid #bdcac2; border-radius: 11px; color: inherit; font: inherit; font-size: 16px; min-height: 48px; outline: none; padding: 0 13px; width: 100%; }
    input:focus { background: #fff; border-color: #1b6b55; box-shadow: 0 0 0 3px rgba(27, 107, 85, .18); }
    input::placeholder { color: #83928c; }
    .password input { padding-right: 72px; }
    .toggle { background: transparent; border: 0; border-radius: 8px; color: #1b6b55; cursor: pointer; font: inherit; font-size: 13px; font-weight: 750; min-height: 36px; padding: 0 10px; position: absolute; right: 6px; top: 6px; }
    .toggle:focus-visible, button:focus-visible { outline: 3px solid #8dc9b0; outline-offset: 2px; }
    .submit { background: #1b6b55; border: 0; border-radius: 12px; color: #fff; cursor: pointer; font: inherit; font-size: 16px; font-weight: 750; min-height: 52px; transition: background .15s ease, transform .15s ease; width: 100%; }
    .submit:hover { background: #145440; }
    .submit:active { transform: translateY(1px); }
    .submit:disabled { background: #78988c; cursor: wait; }
    .status { border-radius: 10px; display: none; font-size: 14px; line-height: 1.4; margin: 18px 0 0; padding: 12px 13px; }
    .status.show { display: block; }
    .status.success { background: #e5f5ec; color: #0e543b; }
    .status.error { background: #fff0ed; color: #93321d; }
    .footnote { color: #6c7d76; font-size: 12px; line-height: 1.45; margin: 16px 8px 0; text-align: center; }
    @media (prefers-color-scheme: dark) { body { background: #102019; color: #edf4ef; } .masthead .eyebrow, .intro p, .hint, .footnote { color: #b6c7bd; } section { background: #172b22; border-color: #385449; box-shadow: none; } .intro { border-color: #385449; } input { background: #112119; border-color: #526b5f; color: #edf4ef; } input:focus { background: #172b22; } input::placeholder { color: #91a59a; } .status.success { background: #153d2b; color: #d7f5e2; } .status.error { background: #4c241d; color: #ffd9d0; } }
    @media (max-width: 360px) { .intro, form { padding-left: 18px; padding-right: 18px; } }
    @media (prefers-reduced-motion: reduce) { *, *::before, *::after { scroll-behavior: auto !important; transition-duration: .01ms !important; } }
  </style>
</head>
<body>
  <main>
    <header class="masthead">
      <div class="mark" aria-hidden="true">GD</div>
      <div><p class="eyebrow">Device setup</p><p class="name">Glance Deck</p></div>
    </header>
    <section aria-labelledby="setup-title">
      <div class="intro"><h1 id="setup-title">Connect your Deck</h1><p>Choose the Wi-Fi network your Deck should use, then enter the address of its Console.</p></div>
      <form id="setup-form">
        <div class="field"><label for="ssid">Wi-Fi network</label><input id="ssid" name="ssid" autocomplete="off" maxlength="32" required></div>
        <div class="field"><label for="password">Wi-Fi password <span aria-hidden="true">(optional)</span></label><div class="input-wrap password"><input id="password" name="password" autocomplete="current-password" maxlength="64" type="password"><button class="toggle" id="password-toggle" type="button" aria-controls="password" aria-pressed="false">Show</button></div></div>
        <div class="field"><label for="control-plane-url">Deck Console address</label><p class="hint">The HTTPS address where you manage this Deck.</p><input id="control-plane-url" name="control_plane_url" type="url" inputmode="url" autocapitalize="none" autocomplete="url" placeholder="https://deck.example.com" maxlength="256" required></div>
        <button class="submit" id="submit" type="submit">Save and connect</button>
        <p class="status" id="status" role="status" aria-live="polite"></p>
      </form>
    </section>
    <p class="footnote">After saving, your Deck restarts and joins the selected Wi-Fi network.</p>
  </main>
  <script>
    const form = document.getElementById('setup-form');
    const password = document.getElementById('password');
    const toggle = document.getElementById('password-toggle');
    const submit = document.getElementById('submit');
    const status = document.getElementById('status');
    toggle.addEventListener('click', () => {
      const showing = password.type === 'text';
      password.type = showing ? 'password' : 'text';
      toggle.textContent = showing ? 'Show' : 'Hide';
      toggle.setAttribute('aria-pressed', String(!showing));
    });
    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      if (!form.reportValidity()) return;
      submit.disabled = true;
      submit.textContent = 'Saving…';
      status.className = 'status show';
      status.textContent = 'Saving your connection settings…';
      try {
        const response = await fetch('/api/wifi', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(Object.fromEntries(new FormData(form))) });
        const message = await response.text();
        if (!response.ok) throw new Error(message || 'Unable to save these settings. Check each field and try again.');
        status.className = 'status show success';
        status.textContent = 'Saved. Your Deck is restarting and connecting to Wi-Fi.';
      } catch (error) {
        status.className = 'status show error';
        status.textContent = error instanceof Error ? error.message : 'Unable to save these settings. Try again.';
        submit.disabled = false;
        submit.textContent = 'Save and connect';
      }
    });
  </script>
</body>
</html>"###;

#[cfg(test)]
mod tests {
    use super::PORTAL_HTML;

    #[test]
    fn provisioning_portal_has_accessible_responsive_form_controls() {
        for required_fragment in [
            "viewport-fit=cover",
            "name=\"ssid\"",
            "name=\"password\"",
            "name=\"control_plane_url\"",
            "min-height: 48px",
            "prefers-reduced-motion",
            "aria-live=\"polite\"",
            "fetch('/api/wifi'",
        ] {
            assert!(
                PORTAL_HTML.contains(required_fragment),
                "missing {required_fragment}"
            );
        }
    }
}
