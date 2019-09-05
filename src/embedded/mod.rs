use std::sync::{Arc, Mutex};

use sysfs_gpio::{Error};

pub mod configuration;
pub mod gpio;
pub mod fake;

#[derive(PartialEq, Eq, Hash)]
pub struct ValvePinNumber(pub u8);

pub trait PinLayout<T> {
    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<T>>, ()>;
}

pub trait ToggleValve {
    fn turn_on(&mut self) -> Result<(), Error>;
    fn turn_off(&mut self) -> Result<(), Error>;
    fn toggle(&mut self) -> Result<(), Error>;
    fn get_valve_pin_num(&self) -> &ValvePinNumber;
}

pub enum ValveStatus {
    OPEN,
    CLOSED
}
