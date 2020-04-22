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

        let mqtt_options = MqttSession::create_mqtt_options(config_clone);

        let (client, receiver) = MqttClient::start(mqtt_options).unwrap();

        Arc::new(Mutex::new(MqttSession {
            client,
            receiver,
            config,
        }))
    }

    fn create_mqtt_options(config_clone: MqttConfig) -> MqttOptions {
        let cert_path = config_clone
            .cert_path
            .unwrap_or("/app/root-ca.crt".to_string());
        let cert = MqttSession::read_cert(cert_path);
        MqttOptions::new(
            config_clone.client_id.clone(),
            config_clone.broker_hostname,
            config_clone.port.unwrap_or(8883),
        )
        .set_security_opts(SecurityOptions::UsernamePassword(
            config_clone.username.unwrap_or("".to_string()),
            config_clone.password.unwrap_or("".to_string()),
        ))
        .set_ca(cert)
        .set_last_will(device_offline_last_will(config_clone.client_id))
        .set_clean_session(true)
    }

    fn read_cert(cert_path: String) -> Vec<u8> {
        let mut cert_file =
            File::open(cert_path).expect("Could not open root ca for mqtt connection.");
        let mut cert = Vec::new();
        // read the whole file
        cert_file
            .read_to_end(&mut cert)
            .expect("Could not read cert file");
        cert
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
