use std::thread::sleep;
use std::time::Duration;

use futures::{Future, lazy, Stream};
use sysfs_gpio::{Direction, Edge, Error, Pin};

pub use self::settings::{LayoutConfig, ValveConfig};

mod settings;

pub fn set_pin_value(pin: &Option<Pin>, value: u8) {
    match pin {
        Some(p) => p.set_value(value).expect("GPIO Pin is not working. Could not set value."),
        _ => ()
    }
}

#[derive(Clone)]
pub struct PinLayout {
    power_pin: Option<Pin>,
    error_pin: Option<Pin>,
    toggle_valves: Vec<ToggleValve>,
}

impl PinLayout {
    pub fn from_config(layout: &LayoutConfig) -> PinLayout {
        let result = PinLayout {
            power_pin: layout.get_power_pin_num().map(|num| create_pin(num, Direction::Out)),
            error_pin: layout.get_error_pin_num().map(|num| create_pin(num, Direction::Out)),
            toggle_valves: layout.get_valves().iter().map(|valve_conf| ToggleValve::from_config(valve_conf)).collect(),
        };
        result
    }

    pub fn run_start_sequence(&self) -> Result<(), Error> {
        for millis in [200, 200, 400, 200, 200].iter() {
            let running_led = self.power_pin;
            let error_led = self.error_pin;
            let valves = self.get_valve_pins();
            set_pin_value(&running_led, 1);
            set_pin_value(&error_led, 1);
            for v in valves.iter() {
                v.get_valve_pin().set_value(1)?;
                set_pin_value(v.get_status_led_pin(), 1);
            }

            sleep(Duration::from_millis(*millis));
            set_pin_value(&running_led, 0);
            set_pin_value(&error_led, 0);
            for v in valves.iter() {
                v.get_valve_pin().set_value(0)?;
                set_pin_value(v.get_status_led_pin(), 0);
            }

            sleep(Duration::from_millis(200));
        }

        Ok(())
    }

    pub fn power_on(&self) -> Result<(), Error> {
        set_pin_value(&self.power_pin, 1);
        Ok(())
    }

    pub fn get_button_streams(&self) -> impl Future<Item=(), Error=()> {
        let clone = self.clone();
        return lazy(move || {
            for toggle_valve in clone.get_valve_pins() {
                if let Some(button_pin) = toggle_valve.get_button_pin() {
                    let valve_pin = toggle_valve.get_valve_pin().clone();
                    tokio::spawn(
                        button_pin
                            .get_value_stream().expect("Expect a valid value stream.")
                            .for_each(move |_val| {
                                let new_val = 1 - valve_pin.get_value()?;
                                valve_pin.set_value(new_val)?;
                                Ok(())
                            }).map_err(|err| {
                            println!("error = {:?}", err)
                        }),
                    );
                }
            }
            Ok(())
        });
    }

    pub fn get_valve_pins(&self) -> &[ToggleValve] {
        &self.toggle_valves
    }

    pub fn unexport_all(&self) -> Result<(), Error> {
        if let Some(pin) = self.power_pin {
            pin.set_value(0)?;
            pin.unexport()?;
        }
        if let Some(pin) = self.error_pin {
            pin.set_value(0)?;
            pin.unexport()?;
        }

        for toggle_valve in &self.toggle_valves {
            let v = toggle_valve.get_valve_pin();
            v.set_value(0)?;
            v.unexport()?;
            if let Some(pin) = toggle_valve.get_button_pin() {
                pin.unexport()?;
            }
            if let Some(pin) = toggle_valve.get_status_led_pin() {
                pin.set_value(0)?;
                pin.unexport()?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ToggleValve {
    valve_pin: Pin,
    status_led_pin: Option<Pin>,
    button_pin: Option<Pin>,
}

impl ToggleValve {
    pub fn from_config(valve: &ValveConfig) -> ToggleValve {
        ToggleValve {
            valve_pin: create_pin(valve.get_valve_pin_num(), Direction::Out),
            status_led_pin: valve.get_status_led_pin_num().map(|p| create_pin(p, Direction::Out)),
            button_pin: valve.get_button_pin_num().map(|p| create_pin(p, Direction::In)),
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
    pin.set_direction(direction).expect("Could not set gpio pin direction.");
    match direction {
        Direction::In => pin.set_edge(Edge::RisingEdge).expect("Could not set gpio pin edge"),
        _ => {}
    }
    pin
}
