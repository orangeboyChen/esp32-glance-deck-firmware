use anyhow::{bail, Result};
use embedded_svc::{
    http::{client::Client as HttpClient, Method},
    io::{Read, Write},
};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use serde::Deserialize;

use crate::{
    config::{DeviceConfig, WifiConfig},
    enrollment::Enrollment_session,
};

const MAX_RESPONSE_BYTES: usize = 1024;

#[derive(Deserialize)]
struct Claim_response {
    status: String,
    device_id: Option<String>,
    mqtt: Option<crate::config::MqttConfig>,
}

pub fn announce_and_claim(
    control_plane_url: &str,
    session: &Enrollment_session,
    wifi: WifiConfig,
) -> Result<Option<DeviceConfig>> {
    if !control_plane_url.starts_with("https://") {
        bail!("control_plane_url_insecure")
    }
    session.validate().map_err(|error| anyhow::anyhow!(error))?;
    let payload = serde_json::json!({ "pairing_code": session.pairing_code, "claim_secret": session.claim_secret, "board_model": "ESP32-S3-RLCD-4.2" });
    post_json(
        &format!(
            "{}/api/v1/enrollment/announce",
            control_plane_url.trim_end_matches('/')
        ),
        &payload,
    )?;
    let response = post_json(
        &format!(
            "{}/api/v1/enrollment/claim",
            control_plane_url.trim_end_matches('/')
        ),
        &serde_json::json!({ "pairing_code": session.pairing_code, "claim_secret": session.claim_secret }),
    )?;
    let claim: Claim_response = serde_json::from_slice(&response)?;
    if claim.status == "pending" {
        return Ok(None);
    }
    let (Some(device_id), Some(mqtt)) = (claim.device_id, claim.mqtt) else {
        bail!("enrollment_claim_invalid")
    };
    let config = DeviceConfig {
        device_id,
        wifi,
        mqtt,
    };
    config
        .validate()
        .map_err(|error| anyhow::anyhow!("enrollment_config_invalid: {error:?}"))?;
    Ok(Some(config))
}

fn post_json(url: &str, value: &serde_json::Value) -> Result<Vec<u8>> {
    let body = serde_json::to_vec(value)?;
    let connection = EspHttpConnection::new(&Configuration {
        buffer_size: Some(1024),
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = HttpClient::wrap(connection);
    let mut request = client.request(
        Method::Post,
        url,
        &[
            ("content-type", "application/json"),
            ("content-length", &body.len().to_string()),
        ],
    )?;
    request.write_all(&body)?;
    let mut response = request.submit()?;
    if response.status() != 200 && response.status() != 201 {
        bail!("enrollment_http_status_{}", response.status())
    }
    let mut bytes = Vec::with_capacity(MAX_RESPONSE_BYTES);
    response
        .take(MAX_RESPONSE_BYTES as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() == MAX_RESPONSE_BYTES {
        bail!("enrollment_response_too_large")
    }
    Ok(bytes)
}
