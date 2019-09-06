
#[derive(Serialize, Deserialize, Debug)]
pub struct MqttConfig {
    pub client_id: String,
    pub broker_hostname: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub cert_path: Option<String>,
}
