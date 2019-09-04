use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use futures::{Future, lazy, Stream};
use sysfs_gpio::{Direction, Edge, Error, Pin};

pub use self::configuration::{LayoutConfig, ValveConfig};

mod configuration;

#[derive(PartialEq, Eq, Hash)]
pub struct ValvePinNumber(pub u64);

pub fn set_pin_value(pin: &Option<Pin>, value: u8) {
    match pin {
        Some(p) => p
            .set_value(value)
            .expect("GPIO Pin is not working. Could not set value."),
        _ => (),
    }
}

pub struct PinLayout {
    power_pin: Option<Pin>,
    error_pin: Option<Pin>,
    toggle_valves: Vec<Arc<Mutex<ToggleValve>>>,
}

impl Drop for PinLayout {
    fn drop(&mut self) {
        println!("Drop Pinlayout");
        self.unexport_all();
    }
}

impl PinLayout {
    pub fn from_config(layout: &LayoutConfig) -> PinLayout {
        let result = PinLayout {
            power_pin: layout
                .get_power_pin_num()
                .map(|num| create_pin(num, Direction::Out)),
            error_pin: layout
                .get_error_pin_num()
                .map(|num| create_pin(num, Direction::Out)),
            toggle_valves: layout
                .get_valves()
                .iter()
                .map(|valve_conf| Arc::new(Mutex::new(ToggleValve::from_config(valve_conf))))
                .collect(),
        };

        result
            .run_start_sequence()
            .expect("StartSequence could not run.");
        result
            .power_on()
            .expect("Power Pin could not be turned on.");

        result
    }

    fn run_start_sequence(&self) -> Result<(), Error> {
        for millis in [200, 200, 400, 200, 200].iter() {
            let running_led = self.power_pin;
            let error_led = self.error_pin;
            let valves = self.get_valve_pins();
            set_pin_value(&running_led, 1);
            set_pin_value(&error_led, 1);
            for v in valves.iter() {
                v.lock().unwrap().get_valve_pin().set_value(1)?;
                set_pin_value(v.lock().unwrap().get_status_led_pin(), 1);
            }

            sleep(Duration::from_millis(*millis));
            set_pin_value(&running_led, 0);
            set_pin_value(&error_led, 0);
            for v in valves.iter() {
                v.lock().unwrap().get_valve_pin().set_value(0)?;
                set_pin_value(v.lock().unwrap().get_status_led_pin(), 0);
            }

            sleep(Duration::from_millis(200));
        }

        Ok(())
    }

    fn power_on(&self) -> Result<(), Error> {
        set_pin_value(&self.power_pin, 1);
        Ok(())
    }

    pub fn find_pin(&self, valve_pin_num: u64) -> Result<&Arc<Mutex<ToggleValve>>, ()> {
        match self
            .get_valve_pins()
            .iter()
            .find(|ref valve_pin| valve_pin_num == valve_pin.lock().unwrap().get_valve_pin().get_pin())
            {
                None => Err(()),
                Some(pin) => Ok(pin),
            }
    }

    pub fn get_button_streams(&self) -> impl Future<Item=(), Error=()> {
        let valve_pins = self.get_valve_pins();
        let mut valves: Vec<Arc<Mutex<ToggleValve>>> = Vec::new();
        for pin in valve_pins {
            valves.push(Arc::clone(&pin))
        }
        return lazy(move || {
            for toggle_valve in valves {
                let toggle_valve_raw = toggle_valve.lock().unwrap();
                if let Some(button_pin) = toggle_valve_raw.get_button_pin() {
                    let clone = Arc::clone(&toggle_valve);
                    tokio::spawn(
                        button_pin
                            .get_value_stream()
                            .expect("Expect a valid value stream.")
                            .for_each(move |_val| {
                                let clone_raw = clone.lock().unwrap();
                                let valve_pin = clone_raw.get_valve_pin();
                                let status_pin = clone_raw.get_status_led_pin();
                                let new_val = 1 - valve_pin.get_value()?;
                                valve_pin.set_value(new_val)?;
                                match status_pin {
                                    Some(status) => status.set_value(new_val)?,
                                    None => (),
                                }
                                Ok(())
                            })
                            .map_err(|err| println!("error = {:?}", err)),
                    );
                }
            }
            Ok(())
        });
    }

    pub fn get_valve_pins(&self) -> &Vec<Arc<Mutex<ToggleValve>>> {
        &self.toggle_valves
    }

    fn unexport_all(&self) -> Result<(), Error> {
        if let Some(pin) = self.power_pin {
            pin.set_value(0)?;
            pin.unexport()?;
        }
        if let Some(pin) = self.error_pin {
            pin.set_value(0)?;
            pin.unexport()?;
        }

        for toggle_valve in &self.toggle_valves {
            let tv = toggle_valve.lock().unwrap();
            let v = tv.get_valve_pin();
            v.set_value(0)?;
            v.unexport()?;
            if let Some(pin) = tv.get_button_pin() {
                pin.unexport()?;
            }
            if let Some(pin) = tv.get_status_led_pin() {
                pin.set_value(0)?;
                pin.unexport()?;
            }
        }
        Ok(())
    }
}

pub struct ToggleValve {
    valve_pin: Pin,
    status_led_pin: Option<Pin>,
    button_pin: Option<Pin>,
}

impl ToggleValve {
    pub fn from_config(valve: &ValveConfig) -> ToggleValve {
        ToggleValve {
            valve_pin: create_pin(valve.get_valve_pin_num(), Direction::Out),
            status_led_pin: valve
                .get_status_led_pin_num()
                .map(|p| create_pin(p, Direction::Out)),
            button_pin: valve
                .get_button_pin_num()
                .map(|p| create_pin(p, Direction::In)),
        }
    }

    pub fn turn_on(&self) -> Result<(), Error> {
        self.valve_pin.set_value(1)?;
        set_pin_value(&self.status_led_pin, 1);
        Ok(())
    }

    pub fn turn_off(&self) -> Result<(), Error> {
        self.valve_pin.set_value(0)?;
        set_pin_value(&self.status_led_pin, 0);
        Ok(())
    }

    pub fn get_valve_pin(&self) -> &Pin {
        &self.valve_pin
    }

    pub fn get_button_pin(&self) -> &Option<Pin> {
        &self.button_pin
    }

    pub fn get_status_led_pin(&self) -> &Option<Pin> {
        &self.status_led_pin
    }
}

fn create_pin(pin_num: u8, direction: Direction) -> Pin {
    let pin = Pin::new(pin_num as u64);
    pin.export().expect("GPIO error.");
    pin.set_direction(direction)
        .expect("Could not set gpio pin direction.");
    match direction {
        Direction::In => pin
            .set_edge(Edge::RisingEdge)
            .expect("Could not set gpio pin edge"),
        _ => {}
    }
    pin
}
