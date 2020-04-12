#[derive(Serialize, Deserialize, Debug)]
pub struct WateringScheduleConfigs {
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
    enabled: bool,
}

impl WateringScheduleConfig {
    pub fn get_schedule(&self) -> &ScheduleConfig {
        &self.schedule
    }
    pub fn get_valve(&self) -> u8 {
        self.valve
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleConfig {
    start_hour: u8,
    start_minute: u8,
    end_hour: u8,
    end_minute: u8,
}

impl ScheduleConfig {
    pub fn get_start_hour(&self) -> &u8 {
        &self.start_hour
    }
    pub fn get_start_minute(&self) -> &u8 {
        &self.start_minute
    }
    pub fn get_end_hour(&self) -> &u8 {
        &self.end_hour
    }
    pub fn get_end_minute(&self) -> &u8 {
        &self.end_minute
    }
}
