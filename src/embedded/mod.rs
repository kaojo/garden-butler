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

#[derive(PartialEq, Eq, Hash)]
pub struct ValvePinNumber(pub u8);

pub trait PinLayout<T> {
    fn new(config: &LayoutConfig) -> Self;
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
    CLOSED,
}

#[derive(Debug)]
#[cfg(feature = "gpio")]
pub enum Error {
    GPIO(sysfs_gpio::Error),
    Unexpected(String),
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
                Error::Unexpected(_) => "An Unexpected Error Occurred",
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
                _ => None,
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
                Error::Unexpected(ref s) => write!(f, "Unexpected: {}", s),
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
