use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};

use rumqtt::{
    ClientError, LastWill, MqttClient, MqttOptions, Notification, QoS, Receiver, SecurityOptions,
};

use crate::mqtt::configuration::MqttConfig;

pub mod command;
pub mod configuration;
pub mod status;

pub struct MqttSession {
    pub client: MqttClient,
    pub receiver: Receiver<Notification>,
    pub config: MqttConfig,
}

impl MqttSession {
    pub fn from_config(config: MqttConfig) -> Arc<Mutex<MqttSession>> {
        let config_clone = config.clone();
        let mut cert_file = File::open(config.cert_path.unwrap_or("/app/root-ca.crt".to_string()))
            .expect("Could not open root ca for mqtt connection.");
        let mut cert = Vec::new();
        // read the whole file
        cert_file
            .read_to_end(&mut cert)
            .expect("Could not read cert file");

        let mqtt_options = MqttOptions::new(
            config.client_id.clone(),
            config.broker_hostname,
            config.port.unwrap_or(8883),
        )
        .set_security_opts(SecurityOptions::UsernamePassword(
            config.username.unwrap_or("".to_string()),
            config.password.unwrap_or("".to_string()),
        ))
        .set_ca(cert)
        .set_last_will(device_offline_last_will(config.client_id))
        .set_clean_session(false);
        let (client, receiver) = MqttClient::start(mqtt_options).unwrap();
        Arc::new(Mutex::new(MqttSession {
            client,
            receiver,
            config: config_clone,
        }))
    }

    pub fn get_client_id(&self) -> &str {
        &self.config.client_id
    }

    pub fn publish<S, V, B>(
        &mut self,
        topic: S,
        qos: QoS,
        retained: B,
        payload: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
        B: Into<bool>,
    {
        self.client.publish(topic, qos, retained, payload)
    }

    pub fn subscribe<S>(&mut self, topic: S, qos: QoS) -> Result<(), ClientError>
    where
        S: Into<String>,
    {
        self.client.subscribe(topic, qos)
    }
}

fn device_offline_last_will(client_id: String) -> LastWill {
    LastWill {
        topic: format!("{}/garden-butler/status/health", client_id),
        message: String::from("OFFLINE"),
        qos: QoS::AtLeastOnce,
        retain: true,
    }
}
