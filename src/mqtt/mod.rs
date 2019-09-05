use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rumqtt::{MqttClient, MqttOptions, Notification, QoS, Receiver, SecurityOptions};

use mqtt::configuration::MqttConfig;
use futures::{Stream, Async};

pub mod configuration;

pub struct MqttSession {
    pub client: MqttClient,
    pub receiver: Receiver<Notification>,
}

impl MqttSession {
    pub fn from_config(config: MqttConfig) -> Arc<Mutex<MqttSession>> {
        let mqtt_options = MqttOptions::new(
            config.client_id,
            config.broker_hostname,
            config.port.unwrap_or(1883))
            .set_security_opts(
                SecurityOptions::UsernamePassword(
                    String::from(config.username.unwrap_or("".to_string())),
                    String::from(config.password.unwrap_or("".to_string())),
                )
            );
        let (client, receiver) = MqttClient::start(mqtt_options).unwrap();
        Arc::new(Mutex::new(MqttSession {
            client,
            receiver,
        }))
    }

    pub fn from_config_s(config: MqttConfig) -> MqttSession {
        let mqtt_options = MqttOptions::new(
            config.client_id,
            config.broker_hostname,
            config.port.unwrap_or(1883))
            .set_security_opts(
                SecurityOptions::UsernamePassword(
                    String::from(config.username.unwrap_or("".to_string())),
                    String::from(config.password.unwrap_or("".to_string())),
                )
            );
        let (client, receiver) = MqttClient::start(mqtt_options).unwrap();
        MqttSession {
            client,
            receiver,
        }
    }
}
