use std::sync::{Arc, Mutex};

use sysfs_gpio::{Error};

pub mod configuration;
pub mod gpio;

#[derive(PartialEq, Eq, Hash)]
pub struct ValvePinNumber(pub u64);

pub trait PinLayout<T> {
    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<T>>, ()>;
}

pub trait ToggleValve {
    fn turn_on(&self) -> Result<(), Error>;
    fn turn_off(&self) -> Result<(), Error>;
    fn toggle(&self) -> Result<(), Error>;
    fn get_valve_pin_num(&self) -> ValvePinNumber;
}
