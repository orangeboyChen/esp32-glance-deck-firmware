use serde::{Deserialize, Serialize};

use crate::config::DeviceConfig;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentClaim {
    pub pairing_code: String,
    pub device_id: String,
    pub mqtt: crate::config::MqttConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProvisioningState {
    Unprovisioned,
    WifiConfigured,
    AwaitingEnrollment,
    Enrolled,
}

pub trait ProvisioningService {
    type Error;

    fn start_captive_portal(&mut self) -> Result<(), Self::Error>;
    fn stop_captive_portal(&mut self) -> Result<(), Self::Error>;
    fn connect_wifi(&mut self, config: &DeviceConfig) -> Result<(), Self::Error>;
    fn show_pairing_code(&mut self, pairing_code: &str) -> Result<(), Self::Error>;
}

impl EnrollmentClaim {
    pub fn into_device_config(self, wifi: crate::config::WifiConfig) -> DeviceConfig {
        DeviceConfig {
            device_id: self.device_id,
            wifi,
            mqtt: self.mqtt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MqttConfig;

    #[test]
    fn enrollment_claim_keeps_only_device_connection_data() {
        let config = EnrollmentClaim {
            pairing_code: "123456".to_owned(),
            device_id: "office-deck".to_owned(),
            mqtt: MqttConfig {
                broker_url: "mqtts://broker.example".to_owned(),
                username: "office-deck".to_owned(),
                password: "credential".to_owned(),
            },
        }
        .into_device_config(crate::config::WifiConfig {
            ssid: "lan".to_owned(),
            password: "wifi-password".to_owned(),
        });

        assert_eq!(config.device_id, "office-deck");
        assert_eq!(config.validate(), Ok(()));
    }
}
