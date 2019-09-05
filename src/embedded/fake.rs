use std::sync::{Arc, Mutex};

use embedded::{Error, PinLayout, ToggleValve, ValvePinNumber, ValveStatus};
use embedded::ValveStatus::{CLOSED, OPEN};

struct FakePinLayout {
    toggle_valves: Vec<Arc<Mutex<FakeToggleValve>>>,
}

impl PinLayout<FakeToggleValve> for FakePinLayout {
    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<FakeToggleValve>>, ()> {
        let result_option = self.toggle_valves
            .iter()
            .find(|ref valve_pin|
                valve_pin_num == *valve_pin.lock().unwrap().get_valve_pin_num()
            );
        match result_option
            {
                None => Err(()),
                Some(valve) => Ok(valve),
            }
    }
}

struct FakeToggleValve {
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

    fn toggle(&mut self) -> Result<(), Error> {
        match self.status {
            OPEN => {
                println!("Turning off valve {}", self.valve_pin_number.0);
                self.status = CLOSED
            }
            CLOSED => {
                println!("Turning on valve {}", self.valve_pin_number.0);
                self.status = OPEN
            }
        }
        Ok(())
    }

    fn get_valve_pin_num(&self) -> &ValvePinNumber {
        &self.valve_pin_number
    }
}