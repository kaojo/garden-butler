use core::{convert, fmt};
use std::sync::{Arc, Mutex};

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
    CLOSED,
}

#[derive(Debug)]
pub enum Error {
    GPIO(sysfs_gpio::Error),
    Unexpected(String),
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::GPIO(ref e) => e.description(),
            Error::Unexpected(_) => "An Unexpected Error Occurred",
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::GPIO(ref e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::GPIO(ref e) => e.fmt(f),
            Error::Unexpected(ref s) => write!(f, "Unexpected: {}", s),
        }
    }
}

impl convert::From<sysfs_gpio::Error> for Error {
    fn from(e: sysfs_gpio::Error) -> Error {
        Error::GPIO(e)
    }
}
