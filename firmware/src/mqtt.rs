use serde::{Deserialize, Serialize};

use crate::config::validate_device_id;

pub const TOPIC_PREFIX: &str = "glance_deck";
pub const MAX_MQTT_PAYLOAD_BYTES: usize = 4096;

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
    pub page_id: Option<String>,
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
        let command: Device_command =
            serde_json::from_str(r#"{"command_id":"abc","action":"show_page","page_id":"usage"}"#)
                .unwrap();
        assert_eq!(command.action, Device_command_action::Show_page);
    }

    #[test]
    fn rejects_unknown_command_fields() {
        assert!(Device_command::from_payload(
            br#"{"command_id":"abc","action":"next_page","unexpected":true}"#
        )
        .is_err());
    }
}
