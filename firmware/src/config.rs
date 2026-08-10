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
    Empty_device_id,
    Device_id_too_long,
    Invalid_device_id,
    Empty_wifi_ssid,
    Wifi_ssid_too_long,
    Empty_mqtt_host,
    Mqtt_host_too_long,
    Insecure_mqtt_url,
    Empty_mqtt_credentials,
}

impl DeviceConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_device_id(&self.device_id)?;

        if self.wifi.ssid.is_empty() {
            return Err(ConfigError::Empty_wifi_ssid);
        }
        if self.wifi.ssid.len() > MAX_WIFI_SSID_LEN {
            return Err(ConfigError::Wifi_ssid_too_long);
        }
        if self.mqtt.broker_url.is_empty() {
            return Err(ConfigError::Empty_mqtt_host);
        }
        if self.mqtt.broker_url.len() > MAX_MQTT_HOST_LEN {
            return Err(ConfigError::Mqtt_host_too_long);
        }
        if !(self.mqtt.broker_url.starts_with("mqtts://")
            || self.mqtt.broker_url.starts_with("ssl://"))
        {
            return Err(ConfigError::Insecure_mqtt_url);
        }
        if self.mqtt.username.is_empty() || self.mqtt.password.is_empty() {
            return Err(ConfigError::Empty_mqtt_credentials);
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
        return Err(ConfigError::Empty_device_id);
    }
    if device_id.len() > MAX_DEVICE_ID_LEN {
        return Err(ConfigError::Device_id_too_long);
    }
    if !device_id.bytes().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == b'-'
    }) {
        return Err(ConfigError::Invalid_device_id);
    }
    Ok(())
}

pub trait Config_store {
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
        assert_eq!(config.validate(), Err(ConfigError::Insecure_mqtt_url));
    }
}
