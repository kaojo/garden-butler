#[cfg(feature = "gpio")]
use core::convert;
use core::fmt;
use std::sync::{Arc, Mutex};

use crate::embedded::configuration::LayoutConfig;

pub mod command;
pub mod configuration;
#[cfg(not(feature = "gpio"))]
pub mod fake;
#[cfg(feature = "gpio")]
pub mod gpio;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct ValvePinNumber(pub u8);

pub trait PinLayout<T> {
    fn new(config: &LayoutConfig) -> Self;
    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<T>>, ()>;
    fn get_layout_status(&self) -> LayoutStatus;
    fn turn_on(&mut self, valve_pin_num: ValvePinNumber) -> Result<(), Error>;
    fn turn_off(&mut self, valve_pin_num: ValvePinNumber) -> Result<(), Error>;
}

pub trait ToggleValve {
    fn turn_on(&mut self) -> Result<(), Error>;
    fn turn_off(&mut self) -> Result<(), Error>;
    fn is_on(&self) -> Result<bool, Error>;
    fn get_valve_pin_num(&self) -> &ValvePinNumber;
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ValveStatus {
    OPEN,
    CLOSED,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LayoutStatus {
    valves: Vec<ToggleValveStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToggleValveStatus {
    valve_pin_number: ValvePinNumber,
    status: ValveStatus,
}

#[derive(Debug)]
pub enum Error {
    Unexpected(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Unexpected(ref s) => write!(f, "Unexpected: {}", s),
        }
    }
}

#[cfg(feature = "gpio")]
impl convert::From<sysfs_gpio::Error> for Error {
    fn from(e: sysfs_gpio::Error) -> Error {
        Error::Unexpected(e.to_string())
    }
}
