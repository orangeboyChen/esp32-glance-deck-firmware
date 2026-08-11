use anyhow::{bail, Context, Result};
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};

use crate::config::DeviceConfig;

const NVS_NAMESPACE: &str = "glance_deck";
const DEVICE_CONFIG_KEY: &str = "device_config";
const CONTROL_PLANE_URL_KEY: &str = "control_plane_url";
const WIFI_PROVISIONING_REQUEST_KEY: &str = "wifi_reprovision";
const MAX_DEVICE_CONFIG_BYTES: usize = 768;
const MAX_CONTROL_PLANE_URL_BYTES: usize = 256;

pub fn load_device_config(partition: &EspDefaultNvsPartition) -> Result<Option<DeviceConfig>> {
    let nvs = EspDefaultNvs::new(partition.clone(), NVS_NAMESPACE, true)?;
    let mut buffer = [0_u8; MAX_DEVICE_CONFIG_BYTES];
    let Some(value) = nvs.get_raw(DEVICE_CONFIG_KEY, &mut buffer)? else {
        return Ok(None);
    };
    let config: DeviceConfig =
        serde_json::from_slice(value).context("decode device configuration")?;
    config
        .validate()
        .map_err(|error| anyhow::anyhow!("device configuration invalid: {error:?}"))?;
    Ok(Some(config))
}

pub fn save_device_config(partition: &EspDefaultNvsPartition, config: &DeviceConfig) -> Result<()> {
    config
        .validate()
        .map_err(|error| anyhow::anyhow!("device configuration invalid: {error:?}"))?;
    let encoded = serde_json::to_vec(config)?;
    if encoded.len() > MAX_DEVICE_CONFIG_BYTES {
        bail!("device configuration exceeds NVS capacity")
    }
    let mut nvs = EspDefaultNvs::new(partition.clone(), NVS_NAMESPACE, true)?;
    nvs.set_raw(DEVICE_CONFIG_KEY, &encoded)?;
    Ok(())
}

pub fn load_control_plane_url(partition: &EspDefaultNvsPartition) -> Result<Option<String>> {
    let nvs = EspDefaultNvs::new(partition.clone(), NVS_NAMESPACE, true)?;
    let mut buffer = [0_u8; MAX_CONTROL_PLANE_URL_BYTES];
    let Some(value) = nvs.get_raw(CONTROL_PLANE_URL_KEY, &mut buffer)? else {
        return Ok(None);
    };
    let url = std::str::from_utf8(value)
        .context("decode control plane URL")?
        .to_owned();
    if !url.starts_with("https://") || url.len() > MAX_CONTROL_PLANE_URL_BYTES {
        bail!("control plane URL is invalid")
    }
    Ok(Some(url))
}

pub fn save_control_plane_url(store: &mut EspDefaultNvs, url: &str) -> Result<()> {
    if !url.starts_with("https://") || url.len() > MAX_CONTROL_PLANE_URL_BYTES {
        bail!("control plane URL is invalid")
    }
    store.set_raw(CONTROL_PLANE_URL_KEY, url.as_bytes())?;
    Ok(())
}

pub fn request_wifi_provisioning(partition: &EspDefaultNvsPartition) -> Result<()> {
    let mut nvs = EspDefaultNvs::new(partition.clone(), NVS_NAMESPACE, true)?;
    nvs.set_raw(WIFI_PROVISIONING_REQUEST_KEY, &[1])?;
    Ok(())
}

pub fn take_wifi_provisioning_request(partition: &EspDefaultNvsPartition) -> Result<bool> {
    let mut nvs = EspDefaultNvs::new(partition.clone(), NVS_NAMESPACE, true)?;
    let mut value = [0_u8; 1];
    let requested = nvs
        .get_raw(WIFI_PROVISIONING_REQUEST_KEY, &mut value)?
        .is_some_and(|raw| raw == [1]);
    if requested {
        nvs.set_raw(WIFI_PROVISIONING_REQUEST_KEY, &[0])?;
    }
    Ok(requested)
}
