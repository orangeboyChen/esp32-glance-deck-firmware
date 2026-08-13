use serde::{Deserialize, Serialize};

use crate::mqtt::DeviceTopics;

pub const MAX_DEVICE_ID_LEN: usize = 64;
pub const MAX_WIFI_SSID_LEN: usize = 32;
pub const MAX_MQTT_HOST_LEN: usize = 253;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MqttConfig {
    pub broker_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeviceConfig {
    pub device_id: String,
    pub wifi: WifiConfig,
    pub mqtt: MqttConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    EmptyDeviceId,
    DeviceIdTooLong,
    InvalidDeviceId,
    EmptyWifiSsid,
    WifiSsidTooLong,
    EmptyMqttHost,
    MqttHostTooLong,
    InsecureMqttUrl,
    EmptyMqttCredentials,
}

impl DeviceConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_device_id(&self.device_id)?;

        if self.wifi.ssid.is_empty() {
            return Err(ConfigError::EmptyWifiSsid);
        }
        if self.wifi.ssid.len() > MAX_WIFI_SSID_LEN {
            return Err(ConfigError::WifiSsidTooLong);
        }
        if self.mqtt.broker_url.is_empty() {
            return Err(ConfigError::EmptyMqttHost);
        }
        if self.mqtt.broker_url.len() > MAX_MQTT_HOST_LEN {
            return Err(ConfigError::MqttHostTooLong);
        }
        if !(self.mqtt.broker_url.starts_with("mqtts://")
            || self.mqtt.broker_url.starts_with("wss://"))
        {
            return Err(ConfigError::InsecureMqttUrl);
        }
        if self.mqtt.username.is_empty() || self.mqtt.password.is_empty() {
            return Err(ConfigError::EmptyMqttCredentials);
        }
        Ok(())
    }

    pub fn topics(&self) -> Result<DeviceTopics, ConfigError> {
        self.validate()?;
        Ok(DeviceTopics::new(&self.device_id))
    }
}

pub fn validate_device_id(device_id: &str) -> Result<(), ConfigError> {
    if device_id.is_empty() {
        return Err(ConfigError::EmptyDeviceId);
    }
    if device_id.len() > MAX_DEVICE_ID_LEN {
        return Err(ConfigError::DeviceIdTooLong);
    }
    if !device_id.bytes().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == b'-'
    }) {
        return Err(ConfigError::InvalidDeviceId);
    }
    Ok(())
}

pub trait ConfigStore {
    type Error;

    fn load(&self) -> Result<Option<DeviceConfig>, Self::Error>;
    fn save(&mut self, config: &DeviceConfig) -> Result<(), Self::Error>;
    fn clear(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device_config() -> DeviceConfig {
        DeviceConfig {
            device_id: "office-deck".to_owned(),
            wifi: WifiConfig {
                ssid: "lan".to_owned(),
                password: "not-empty".to_owned(),
            },
            mqtt: MqttConfig {
                broker_url: "mqtts://broker.example".to_owned(),
                username: "office-deck".to_owned(),
                password: "not-empty".to_owned(),
            },
        }
    }

    #[test]
    fn accepts_enrolled_device_config() {
        assert_eq!(device_config().validate(), Ok(()));
    }

    #[test]
    fn rejects_unsafe_broker_transport() {
        let mut config = device_config();
        config.mqtt.broker_url = "mqtt://broker.example".to_owned();
        assert_eq!(config.validate(), Err(ConfigError::InsecureMqttUrl));
    }

    #[test]
    fn validates_device_identifier_and_connection_limits() {
        assert_eq!(validate_device_id(""), Err(ConfigError::EmptyDeviceId));
        assert_eq!(
            validate_device_id("UPPER"),
            Err(ConfigError::InvalidDeviceId)
        );
        assert_eq!(
            validate_device_id(&"a".repeat(MAX_DEVICE_ID_LEN + 1)),
            Err(ConfigError::DeviceIdTooLong)
        );
        let mut config = device_config();
        config.wifi.ssid.clear();
        assert_eq!(config.validate(), Err(ConfigError::EmptyWifiSsid));
        config.wifi.ssid = "a".repeat(MAX_WIFI_SSID_LEN + 1);
        assert_eq!(config.validate(), Err(ConfigError::WifiSsidTooLong));
        config.wifi.ssid = "lan".to_owned();
        config.mqtt.broker_url.clear();
        assert_eq!(config.validate(), Err(ConfigError::EmptyMqttHost));
        config.mqtt.broker_url = "s".repeat(MAX_MQTT_HOST_LEN + 1);
        assert_eq!(config.validate(), Err(ConfigError::MqttHostTooLong));
        config.mqtt.broker_url = "mqtts://broker.example".to_owned();
        config.mqtt.username.clear();
        assert_eq!(config.validate(), Err(ConfigError::EmptyMqttCredentials));
        assert_eq!(
            device_config().topics().unwrap().state(),
            "glance_deck/office-deck/state"
        );
    }
}
