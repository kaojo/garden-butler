#[derive(Serialize, Deserialize, Debug)]
pub struct WateringScheduleConfigs {
    schedules: Vec<WateringScheduleConfig>
}

impl WateringScheduleConfigs {
    pub fn get_schedules(&self) -> &[WateringScheduleConfig] {
        &self.schedules
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WateringScheduleConfig {
    schedule: ScheduleConfig,
    valve: u8,
}

impl WateringScheduleConfig {
    pub fn new(schedule: ScheduleConfig, valve: u8) -> Self {
        WateringScheduleConfig { schedule, valve }
    }
    pub fn get_schedule(&self) -> &ScheduleConfig {
        &self.schedule
    }
    pub fn get_valve(&self) -> u8 {
        self.valve
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleConfig {
    chronExpression: String,
    durationSeconds: u64
    // TODO add end_date_time
}

impl ScheduleConfig {
    pub fn new(chronExpression: String, durationSeconds: u64) -> Self {
        ScheduleConfig { chronExpression, durationSeconds }
    }
    pub fn get_chron_expression(&self) -> &String {
        &self.chronExpression
    }
    pub fn get_duration(&self) -> &u64 {
        &self.durationSeconds
    }
}
