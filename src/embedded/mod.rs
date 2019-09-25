#[cfg(feature = "gpio")]
use core::convert;
use core::fmt;
use embedded::configuration::LayoutConfig;
use std::sync::{Arc, Mutex};

pub mod command;
pub mod configuration;
#[cfg(not(feature = "gpio"))]
pub mod fake;
#[cfg(feature = "gpio")]
pub mod gpio;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct ValvePinNumber(pub u8);

pub trait PinLayout<T> {
    fn new(config: &LayoutConfig) -> Self;
    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<T>>, ()>;
    fn get_layout_status(&self) -> LayoutStatus;
}

pub trait ToggleValve {
    fn turn_on(&mut self) -> Result<(), Error>;
    fn turn_off(&mut self) -> Result<(), Error>;
    fn toggle(&mut self) -> Result<(), Error>;
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
#[cfg(feature = "gpio")]
pub enum Error {
    GPIO(sysfs_gpio::Error),
}

#[derive(Debug)]
#[cfg(not(feature = "gpio"))]
pub enum Error {
    Unexpected(String),
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        #[cfg(feature = "gpio")]
        {
            match *self {
                Error::GPIO(ref e) => e.description(),
            }
        }
        #[cfg(not(feature = "gpio"))]
        {
            match *self {
                Error::Unexpected(_) => "An Unexpected Error Occurred",
            }
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        #[cfg(feature = "gpio")]
        {
            match *self {
                Error::GPIO(ref e) => Some(e),
            }
        }
        #[cfg(not(feature = "gpio"))]
        {
            match *self {
                _ => None,
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(feature = "gpio")]
        {
            match *self {
                Error::GPIO(ref e) => e.fmt(f),
            }
        }
        #[cfg(not(feature = "gpio"))]
        {
            match *self {
                Error::Unexpected(ref s) => write!(f, "Unexpected: {}", s),
            }
        }
    }
}

#[cfg(feature = "gpio")]
impl convert::From<sysfs_gpio::Error> for Error {
    fn from(e: sysfs_gpio::Error) -> Error {
        Error::GPIO(e)
    }
}
