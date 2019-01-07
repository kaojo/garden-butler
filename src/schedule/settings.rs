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
    valve: u64,
}

impl WateringScheduleConfig {
    pub fn new(schedule: ScheduleConfig, valve: u64) -> Self {
        WateringScheduleConfig { schedule, valve }
    }
    pub fn get_schedule(&self) -> &ScheduleConfig {
        &self.schedule
    }
    pub fn get_valve(&self) -> u64 {
        self.valve
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleConfig {
    cron_expression: String,
    duration_seconds: u64
    // TODO add end_date_time
}

impl ScheduleConfig {
    pub fn new(cron_expression: String, duration_seconds: u64) -> Self {
        ScheduleConfig { cron_expression, duration_seconds }
    }
    pub fn get_cron_expression(&self) -> &String {
        &self.cron_expression
    }
    pub fn get_duration_seconds(&self) -> &u64 {
        &self.duration_seconds
    }
}
