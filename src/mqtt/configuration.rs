
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MqttConfig {
    pub client_id: String,
    pub broker_hostname: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub cert_path: Option<String>,
}

impl Default for MqttConfig {
    fn default() -> Self {
        let mut settings = config::Config::default();
        settings
            .merge(config::File::new(
                "mqtt",
                config::FileFormat::Json,
            ))
            .unwrap()
            .merge(config::Environment::with_prefix("MQTT"))
            .unwrap();
        let config = settings
            .try_into::<MqttConfig>()
            .expect("Mqtt config contains errors");
        println!("{:?}", config);
        config
    }
}
