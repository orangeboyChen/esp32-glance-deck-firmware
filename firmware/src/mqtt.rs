use serde::{Deserialize, Serialize};

use crate::config::validate_device_id;

pub const TOPIC_PREFIX: &str = "glance_deck";
pub const MAX_MQTT_PAYLOAD_BYTES: usize = 8192;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceTopics {
    root: String,
}

impl DeviceTopics {
    pub fn new(device_id: &str) -> Self {
        debug_assert!(validate_device_id(device_id).is_ok());
        Self {
            root: format!("{TOPIC_PREFIX}/{device_id}"),
        }
    }

    pub fn release(&self) -> String {
        format!("{}/release", self.root)
    }
    pub fn command(&self) -> String {
        format!("{}/command", self.root)
    }
    pub fn ota(&self) -> String {
        format!("{}/ota", self.root)
    }
    pub fn state(&self) -> String {
        format!("{}/state", self.root)
    }
    pub fn availability(&self) -> String {
        format!("{}/availability", self.root)
    }
    pub fn ota_state(&self) -> String {
        format!("{}/ota/state", self.root)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Device_command_action {
    Show_page,
    Next_page,
    Previous_page,
    Set_rotation,
    Refresh_release,
    Enter_maintenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Device_command {
    pub command_id: String,
    pub action: Device_command_action,
    #[serde(default)]
    pub payload: Command_payload,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Command_payload {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub rotation_seconds: Option<u16>,
}

impl Device_command {
    pub fn from_payload(payload: &[u8]) -> Result<Self, serde_json::Error> {
        if payload.len() > MAX_MQTT_PAYLOAD_BYTES {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "MQTT payload exceeds limit",
            )));
        }
        serde_json::from_slice(payload)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceState {
    pub version: u16,
    pub page_id: String,
    pub wifi_rssi: i16,
    pub display_release_id: Option<String>,
    pub display_updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_status: Option<Command_status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<Device_power_state>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Device_power_state {
    pub source: Power_source,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_mv: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Power_source {
    Usb,
    Battery,
    Usb_and_battery,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command_status {
    Confirmed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ota_phase {
    Downloading,
    Verifying,
    Rebooting,
    Healthy,
    Rolled_back,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ota_state {
    pub job_id: String,
    pub phase: Ota_phase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ota_command {
    pub job_id: String,
    pub nonce: String,
    pub version: String,
    pub manifest_url: String,
    pub image_sha256: String,
}

impl Ota_command {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.job_id.is_empty()
            || self.job_id.len() > 96
            || self.nonce.is_empty()
            || self.nonce.len() > 128
        {
            return Err("ota_identifiers_invalid");
        }
        if !self.manifest_url.starts_with("https://")
            || self.version.is_empty()
            || self.version.len() > 64
        {
            return Err("ota_manifest_invalid");
        }
        if self.image_sha256.len() != 64
            || !self
                .image_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("ota_hash_invalid");
        }
        Ok(())
    }
}

pub trait Mqtt_client {
    type Error;

    fn publish(&mut self, topic: &str, payload: &[u8], retained: bool) -> Result<(), Self::Error>;
    fn subscribe(&mut self, topic: &str) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_device_namespace() {
        let topics = DeviceTopics::new("office-deck");
        assert_eq!(topics.release(), "glance_deck/office-deck/release");
        assert_eq!(topics.ota_state(), "glance_deck/office-deck/ota/state");
    }

    #[test]
    fn parses_supported_command() {
        let command: Device_command = serde_json::from_str(
            r#"{"command_id":"abc","action":"show_page","payload":{"page_id":"usage"}}"#,
        )
        .unwrap();
        assert_eq!(command.action, Device_command_action::Show_page);
        assert_eq!(command.payload.page_id.as_deref(), Some("usage"));
    }

    #[test]
    fn rejects_unknown_command_fields() {
        assert!(Device_command::from_payload(
            br#"{"command_id":"abc","action":"next_page","payload":{"unexpected":true}}"#
        )
        .is_err());
    }

    #[test]
    fn validates_remote_ota_command() {
        let command = Ota_command {
            job_id: "job-123".to_owned(),
            nonce: "nonce".to_owned(),
            version: "1.0.0".to_owned(),
            manifest_url: "https://releases.example/manifest.json".to_owned(),
            image_sha256: "a".repeat(64),
        };
        assert_eq!(command.validate(), Ok(()));
    }

    #[test]
    fn rejects_invalid_ota_and_oversized_mqtt_payloads() {
        let mut command = Ota_command {
            job_id: "job".to_owned(),
            nonce: "nonce".to_owned(),
            version: "1".to_owned(),
            manifest_url: "https://example.test/manifest".to_owned(),
            image_sha256: "a".repeat(64),
        };
        command.manifest_url = "http://example.test".to_owned();
        assert_eq!(command.validate(), Err("ota_manifest_invalid"));
        command.manifest_url = "https://example.test".to_owned();
        command.image_sha256 = "a".repeat(63);
        assert_eq!(command.validate(), Err("ota_hash_invalid"));
        assert!(Device_command::from_payload(&vec![b'x'; MAX_MQTT_PAYLOAD_BYTES + 1]).is_err());
    }
}
