use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};

use rumqtt::{ConnectionMethod, MqttClient, MqttOptions, Notification, Receiver, SecurityOptions};

use mqtt::configuration::MqttConfig;

pub mod configuration;

pub struct MqttSession {
    pub client: MqttClient,
    pub receiver: Receiver<Notification>,
}

impl MqttSession {
    pub fn from_config(config: MqttConfig) -> Arc<Mutex<MqttSession>> {
        let mut cert_file = File::open(config.cert_path.unwrap_or("/app/root-ca.crt".to_string())).expect("Could not open root ca for mqtt connection.");
        let mut cert = Vec::new();
        // read the whole file
        cert_file.read_to_end(&mut cert).expect("Could not read cert file");

        let mqtt_options = MqttOptions::new(
            config.client_id,
            config.broker_hostname,
            config.port.unwrap_or(8883))
            .set_security_opts(
                SecurityOptions::UsernamePassword(
                    config.username.unwrap_or("".to_string()),
                    config.password.unwrap_or("".to_string()),
                )
            )
            .set_connection_method(ConnectionMethod::Tls(cert, None));
        let (client, receiver) = MqttClient::start(mqtt_options).unwrap();
        Arc::new(Mutex::new(MqttSession {
            client,
            receiver,
        }))
    }
}
