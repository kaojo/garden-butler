#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayoutConfig {
    power: Option<u8>,
    error: Option<u8>,
    pump: Option<PumpConfig>,
    valves: Vec<ValveConfig>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        let mut settings = config::Config::default();
        settings
            .merge(config::File::new("layout", config::FileFormat::Json))
            .unwrap()
            // Add in settings from the environment (with a prefix of LAYOUT)
            // Eg.. `LAYOUT_POWER=11 ./target/app` would set the `debug` key
            .merge(config::Environment::with_prefix("LAYOUT"))
            .unwrap();
        let layout_config = settings
            .try_into::<LayoutConfig>()
            .expect("Layout config contains errors");
        println!("{:?}", layout_config);
        layout_config
    }
}

impl LayoutConfig {
    pub fn get_power_pin_num(&self) -> Option<u8> {
        self.power
    }
    pub fn get_error_pin_num(&self) -> Option<u8> {
        self.error
    }
    pub fn get_valves(&self) -> &Vec<ValveConfig> {
        &self.valves
    }
    pub fn get_pump(&self) -> &Option<PumpConfig> {
        &self.pump
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValveConfig {
    valve: u8,
    button: Option<u8>,
    status_led: Option<u8>,
}

impl ValveConfig {
    pub fn get_valve_pin_num(&self) -> u8 {
        self.valve
    }
    pub fn get_status_led_pin_num(&self) -> Option<u8> {
        self.status_led
    }
    pub fn get_button_pin_num(&self) -> Option<u8> {
        self.button
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PumpConfig {
    power_pin: u8,
    status_led: Option<u8>,
}

impl PumpConfig {
    pub fn get_power_pin_num(&self) -> u8 {
        self.power_pin
    }
    pub fn get_status_led_pin_num(&self) -> Option<u8> {
        self.status_led
    }
}
