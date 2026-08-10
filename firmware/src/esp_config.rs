use anyhow::{bail, Context, Result};
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};

use crate::config::DeviceConfig;

const NVS_NAMESPACE: &str = "glance_deck";
const DEVICE_CONFIG_KEY: &str = "device_config";
const MAX_DEVICE_CONFIG_BYTES: usize = 768;

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
