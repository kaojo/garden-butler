#[derive(Serialize, Deserialize, Debug)]
pub struct LayoutConfig {
    power: Option<u8>,
    error: Option<u8>,
    valves: Vec<ValveConfig>,
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
}

#[derive(Serialize, Deserialize, Debug)]
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
