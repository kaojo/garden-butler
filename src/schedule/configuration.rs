use std::io::Write;

#[derive(Serialize, Deserialize, Debug)]
pub struct WateringScheduleConfigs {
    pub schedules: Vec<WateringScheduleConfig>,
}

impl WateringScheduleConfigs {
    pub fn get_schedules(&self) -> &[WateringScheduleConfig] {
        &self.schedules
    }

    pub fn enable_schedule(
        &mut self,
        schedule: &WateringScheduleConfig,
    ) -> Result<WateringScheduleConfig, ()> {
        let existing_schedule: Option<&mut WateringScheduleConfig> = self.find_schedule(schedule);
        match existing_schedule {
            None => Err(()),
            Some(s) => {
                s.enabled = true;
                self.save()?;
                Ok(*schedule)
            }
        }
    }

    pub fn disable_schedule(
        &mut self,
        schedule: &WateringScheduleConfig,
    ) -> Result<WateringScheduleConfig, ()> {
        let existing_schedule: Option<&mut WateringScheduleConfig> = self.find_schedule(schedule);
        match existing_schedule {
            None => Err(()),
            Some(s) => {
                s.enabled = false;
                self.save()?;
                Ok(*schedule)
            }
        }
    }

    pub fn delete_schedule(
        &mut self,
        schedule: &WateringScheduleConfig,
    ) -> Result<WateringScheduleConfig, ()> {
        let index = self.find_schedule_index(schedule);
        match index {
            None => Err(()),
            Some(i) => {
                self.schedules.remove(i);
                self.save()?;
                Ok(*schedule)
            }
        }
    }
    pub fn create_schedule(
        &mut self,
        schedule: WateringScheduleConfig,
    ) -> Result<WateringScheduleConfig, ()> {
        let existing_schedule: Option<&mut WateringScheduleConfig> = self.find_schedule(&schedule);
        match existing_schedule {
            None => {
                self.schedules.push(schedule.clone());
                self.save()?;
                Ok(schedule)
            }
            Some(_) => Err(()),
        }
    }

    fn find_schedule(
        &mut self,
        schedule: &WateringScheduleConfig,
    ) -> Option<&mut WateringScheduleConfig> {
        let index = self.find_schedule_index(schedule);
        index.and_then(move |i| self.schedules.get_mut(i))
    }

    fn find_schedule_index(&self, schedule: &WateringScheduleConfig) -> Option<usize> {
        self.schedules
            .iter()
            .position(|item| item.valve == schedule.valve && item.schedule == schedule.schedule)
    }

    fn save(&self) -> Result<(), ()> {
        let json_string = serde_json::to_string(self).map_err(|_| ())?;
        let mut file = std::fs::File::create("watering-schedules.json").map_err(|_| ())?;
        file.write(json_string.as_bytes())
            .map(|_| ())
            .map_err(|_| ())
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WateringScheduleConfig {
    schedule: ScheduleConfig,
    valve: u8,
    pub enabled: bool,
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
