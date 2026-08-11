use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use embedded_svc::mqtt::client::{EventPayload, QoS};
use esp_idf_svc::mqtt::client::{Details, EspMqttClient, MqttClientConfiguration};

use crate::{
    config::MqttConfig,
    mqtt::{DeviceTopics, Mqtt_client, MAX_MQTT_PAYLOAD_BYTES},
};

const MAX_PENDING_MESSAGES: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncomingMqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
}

/// ESP-IDF MQTT wrapper. The callback performs no display or network work: it
/// accepts only complete bounded messages and leaves the application loop to
/// parse commands or download display pages.
pub struct EspDeviceMqtt {
    client: EspMqttClient<'static>,
    topics: DeviceTopics,
    inbound: Arc<Mutex<VecDeque<IncomingMqttMessage>>>,
}

impl EspDeviceMqtt {
    pub fn connect(config: &MqttConfig, device_id: &str) -> Result<Self> {
        let topics = DeviceTopics::new(device_id);
        let inbound = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_PENDING_MESSAGES)));
        let callback_queue = inbound.clone();
        let release_topic = topics.release();
        let command_topic = topics.command();
        let ota_topic = topics.ota();
        let ota_check_state_topic = topics.ota_check_state();
        let client = EspMqttClient::new_cb(
            &config.broker_url,
            &MqttClientConfiguration {
                client_id: Some(device_id),
                username: Some(&config.username),
                password: Some(&config.password),
                crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
                buffer_size: MAX_MQTT_PAYLOAD_BYTES,
                out_buffer_size: MAX_MQTT_PAYLOAD_BYTES,
                ..Default::default()
            },
            move |event| {
                let EventPayload::Received {
                    topic: Some(topic),
                    data,
                    details: Details::Complete,
                    ..
                } = event.payload()
                else {
                    return;
                };
                if data.len() > MAX_MQTT_PAYLOAD_BYTES
                    || (topic != release_topic
                        && topic != command_topic
                        && topic != ota_topic
                        && topic != ota_check_state_topic)
                {
                    return;
                }
                if let Ok(mut queue) = callback_queue.lock() {
                    if queue.len() == MAX_PENDING_MESSAGES {
                        queue.pop_front();
                    }
                    queue.push_back(IncomingMqttMessage {
                        topic: topic.to_owned(),
                        payload: data.to_vec(),
                    });
                }
            },
        )?;
        let mut mqtt = Self {
            client,
            topics,
            inbound,
        };
        mqtt.client
            .subscribe(&mqtt.topics.release(), QoS::AtLeastOnce)?;
        mqtt.client
            .subscribe(&mqtt.topics.command(), QoS::AtLeastOnce)?;
        mqtt.client
            .subscribe(&mqtt.topics.ota(), QoS::AtLeastOnce)?;
        mqtt.client
            .subscribe(&mqtt.topics.ota_check_state(), QoS::AtLeastOnce)?;
        mqtt.client.publish(
            &mqtt.topics.availability(),
            QoS::AtLeastOnce,
            true,
            b"online",
        )?;
        Ok(mqtt)
    }

    pub fn next_message(&self) -> Option<IncomingMqttMessage> {
        self.inbound.lock().ok()?.pop_front()
    }

    pub fn topics(&self) -> &DeviceTopics {
        &self.topics
    }

    pub fn request_ota_check(&mut self) -> Result<(), esp_idf_svc::sys::EspError> {
        self.client
            .publish(
                &self.topics.ota_check(),
                QoS::AtLeastOnce,
                false,
                br#"{"version":1}"#,
            )
            .map(|_| ())
    }
}

impl Mqtt_client for EspDeviceMqtt {
    type Error = esp_idf_svc::sys::EspError;

    fn publish(&mut self, topic: &str, payload: &[u8], retained: bool) -> Result<(), Self::Error> {
        self.client
            .publish(topic, QoS::AtLeastOnce, retained, payload)
            .map(|_| ())
    }

    fn subscribe(&mut self, topic: &str) -> Result<(), Self::Error> {
        self.client.subscribe(topic, QoS::AtLeastOnce).map(|_| ())
    }
}
