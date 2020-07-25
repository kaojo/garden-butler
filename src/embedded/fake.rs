use std::sync::{Arc, Mutex};

use crate::embedded::configuration::{LayoutConfig, ValveConfig};
use crate::embedded::ValveStatus::{CLOSED, OPEN};
use crate::embedded::{
    Error, LayoutStatus, PinLayout, ToggleValve, ToggleValveStatus, ValvePinNumber, ValveStatus,
};

pub struct FakePinLayout {
    toggle_valves: Vec<Arc<Mutex<FakeToggleValve>>>,
}

impl Drop for FakePinLayout {
    fn drop(&mut self) {
        println!("Drop Pinlayout.")
    }
}

impl PinLayout<FakeToggleValve> for FakePinLayout {
    fn new(config: &LayoutConfig) -> Self {
        FakePinLayout {
            toggle_valves: config
                .get_valves()
                .iter()
                .map(|valve_conf| Arc::new(Mutex::new(FakeToggleValve::from_config(valve_conf))))
                .collect(),
        }
    }

    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<FakeToggleValve>>, ()> {
        let result_option = self
            .toggle_valves
            .iter()
            .find(|ref valve_pin| valve_pin_num == *valve_pin.lock().unwrap().get_valve_pin_num());
        match result_option {
            None => Err(()),
            Some(valve) => Ok(valve),
        }
    }

    fn get_layout_status(&self) -> LayoutStatus {
        LayoutStatus {
            valves: self
                .toggle_valves
                .iter()
                .map(|tv| {
                    let valve = tv.lock().unwrap();
                    let valve_pin_number = ValvePinNumber(valve.valve_pin_number.0);
                    let status = match valve.status {
                        OPEN => OPEN,
                        CLOSED => CLOSED,
                    };
                    ToggleValveStatus {
                        valve_pin_number,
                        status,
                    }
                })
                .collect(),
        }
    }

    fn turn_on(&mut self, valve_pin_num: ValvePinNumber) -> Result<(), Error> {
        if let Ok(valve) = self.find_pin(valve_pin_num) {
            valve.lock().unwrap().turn_on()?;
        }
        Ok(())
    }

    fn turn_off(&mut self, valve_pin_num: ValvePinNumber) -> Result<(), Error> {
        if let Ok(valve) = self.find_pin(valve_pin_num) {
            valve.lock().unwrap().turn_off()?;
        }
        Ok(())
    }
}

pub struct FakeToggleValve {
    valve_pin_number: ValvePinNumber,
    status: ValveStatus,
}

impl ToggleValve for FakeToggleValve {
    fn turn_on(&mut self) -> Result<(), Error> {
        println!("Turning on valve {}", self.valve_pin_number.0);
        self.status = OPEN;
        Ok(())
    }

    fn turn_off(&mut self) -> Result<(), Error> {
        println!("Turning off valve {}", self.valve_pin_number.0);
        self.status = CLOSED;
        Ok(())
    }

    fn is_on(&self) -> Result<bool, Error> {
        Ok(self.status == OPEN)
    }

    fn get_valve_pin_num(&self) -> &ValvePinNumber {
        &self.valve_pin_number
    }
}

impl FakeToggleValve {
    pub fn from_config(valve: &ValveConfig) -> FakeToggleValve {
        FakeToggleValve {
            status: CLOSED,
            valve_pin_number: ValvePinNumber(valve.get_valve_pin_num()),
        }
    }
}
