#[derive(Serialize, Deserialize, Debug)]
pub struct WateringScheduleConfigs {
    pub enabled: Option<bool>,
    schedules: Vec<WateringScheduleConfig>,
}

impl WateringScheduleConfigs {
    pub fn get_schedules(&self) -> &[WateringScheduleConfig] {
        &self.schedules
    }
}

impl Default for WateringScheduleConfigs {
    fn default() -> Self {
        let mut settings = config::Config::default();
        settings
            .merge(config::File::new(
                "watering-schedules",
                config::FileFormat::Json,
            ))
            .unwrap()
            .merge(config::Environment::with_prefix("WATERING"))
            .unwrap();
        let watering_configs = settings
            .try_into::<WateringScheduleConfigs>()
            .expect("Watering schedules config contains errors");
        println!("{:?}", watering_configs);
        watering_configs
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WateringScheduleConfig {
    schedule: ScheduleConfig,
    valve: u8,
}

impl WateringScheduleConfig {
    pub fn get_schedule(&self) -> &ScheduleConfig {
        &self.schedule
    }
    pub fn get_valve(&self) -> u8 {
        self.valve
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleConfig {
    cron_expression: String,
    duration_seconds: u64, // TODO add end_date_time
}

impl ScheduleConfig {
    pub fn get_cron_expression(&self) -> &String {
        &self.cron_expression
    }
    pub fn get_duration_seconds(&self) -> &u64 {
        &self.duration_seconds
    }
}
