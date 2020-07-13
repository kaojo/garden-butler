use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use futures::prelude::*;
use sysfs_gpio::{Direction, Edge, Pin};

use crate::communication::create_abortable_task;
use crate::embedded::configuration::{LayoutConfig, PumpConfig, ValveConfig};
use crate::embedded::ValveStatus::{CLOSED, OPEN};
use crate::embedded::{
    Error, LayoutStatus, PinLayout, ToggleValve, ToggleValveStatus, ValvePinNumber,
};

pub struct GpioPinLayout {
    power_pin: Option<Pin>,
    error_pin: Option<Pin>,
    pump: Option<Arc<Mutex<GpioPumpPin>>>,
    toggle_valves: Vec<Arc<Mutex<GpioToggleValve>>>,
}

impl PinLayout<GpioToggleValve> for GpioPinLayout {
    fn new(config: &LayoutConfig) -> Self {
        let layout = GpioPinLayout {
            power_pin: config
                .get_power_pin_num()
                .map(|num| create_pin(num, Direction::Out)),
            error_pin: config
                .get_error_pin_num()
                .map(|num| create_pin(num, Direction::Out)),
            pump: config
                .get_pump()
                .as_ref()
                .map(|pump_config| Arc::new(Mutex::new(create_pump_pin(pump_config)))),
            toggle_valves: config
                .get_valves()
                .iter()
                .map(|valve_conf| Arc::new(Mutex::new(GpioToggleValve::from_config(valve_conf))))
                .collect(),
        };

        layout
            .run_start_sequence()
            .expect("StartSequence could not run.");
        layout
            .power_on()
            .expect("Power Pin could not be turned on.");

        layout
    }

    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<GpioToggleValve>>, ()> {
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
                    let status = match valve.get_valve_pin().get_value() {
                        Ok(0) => CLOSED,
                        Ok(1) => OPEN,
                        _ => {
                            print!(
                                "Could not get value for valve pin {}",
                                valve.get_valve_pin_num().0
                            );
                            CLOSED
                        }
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
        if let Some(pump) = &self.pump {
            pump.lock().unwrap().turn_on()?;
        }

        self.find_pin(valve_pin_num)
            .map_err(|_| Error::Unexpected(String::from("Valve not found.")))
            .and_then(|valve| valve.lock().unwrap().turn_on())
    }

    fn turn_off(&mut self, valve_pin_num: ValvePinNumber) -> Result<(), Error> {
        if let Some(pump) = &self.pump {
            // only turn off if no other valve is turned on
            let other_valve_open = self.toggle_valves.iter().any(|v| {
                let valve = v.lock().unwrap();
                valve.valve_pin_number != valve_pin_num && valve.valve_pin.get_value().unwrap() == 1
            });
            if !other_valve_open {
                pump.lock().unwrap().turn_off()?;
            }
        }

        self.find_pin(valve_pin_num)
            .map_err(|_| Error::Unexpected(String::from("Valve not found.")))
            .and_then(|valve| valve.lock().unwrap().turn_off())
    }
}

impl Drop for GpioPinLayout {
    fn drop(&mut self) {
        println!("Drop Pinlayout");
        self.unexport_all()
            .expect("Unexport should always work but didn't for some reason.");
    }
}

impl GpioPinLayout {
    fn run_start_sequence(&self) -> Result<(), Error> {
        for millis in [200, 200, 400, 200, 200].iter() {
            let running_led = self.power_pin;
            let error_led = self.error_pin;
            let valves = self.get_valve_pins();
            set_pin_value(&running_led, 1);
            set_pin_value(&error_led, 1);
            for v in valves.iter() {
                v.lock().unwrap().status_on()?;
            }

            sleep(Duration::from_millis(*millis));
            set_pin_value(&running_led, 0);
            set_pin_value(&error_led, 0);
            for v in valves.iter() {
                v.lock().unwrap().status_off()?;
            }

            sleep(Duration::from_millis(200));
        }

        Ok(())
    }

    fn power_on(&self) -> Result<(), Error> {
        set_pin_value(&self.power_pin, 1);
        Ok(())
    }

    pub fn spawn_button_streams(&self, ctrl_c_receiver: tokio::sync::watch::Receiver<String>) {
        let valve_pins = self.get_valve_pins();
        let mut valves: Vec<Arc<Mutex<GpioToggleValve>>> = Vec::new();
        for pin in valve_pins {
            valves.push(Arc::clone(&pin))
        }
        for toggle_valve in valves {
            let toggle_valve_raw = toggle_valve.lock().unwrap();
            if let Some(button_pin) = toggle_valve_raw.get_button_pin() {
                let clone = Arc::clone(&toggle_valve);
                let button_stream = button_pin
                    .get_value_stream()
                    .expect("Expect a valid value stream.")
                    .for_each(move |_val| {
                        let mut clone_raw = clone.lock().unwrap();
                        clone_raw.toggle().expect("button stream error");
                        future::ready(())
                    });

                let task = create_abortable_task(
                    button_stream,
                    "button_stream".to_string(),
                    ctrl_c_receiver.clone(),
                );
                tokio::spawn(task);
            }
        }
    }

    pub fn get_valve_pins(&self) -> &Vec<Arc<Mutex<GpioToggleValve>>> {
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

pub struct GpioToggleValve {
    valve_pin_number: ValvePinNumber,
    valve_pin: Pin,
    status_led_pin: Option<Pin>,
    button_pin: Option<Pin>,
}

impl ToggleValve for GpioToggleValve {
    fn turn_on(&mut self) -> Result<(), Error> {
        self.valve_pin.set_value(1)?;
        set_pin_value(&self.status_led_pin, 1);
        Ok(())
    }

    fn turn_off(&mut self) -> Result<(), Error> {
        self.valve_pin.set_value(0)?;
        set_pin_value(&self.status_led_pin, 0);
        Ok(())
    }

    fn toggle(&mut self) -> Result<(), Error> {
        let new_val = 1 - self.valve_pin.get_value()?;
        self.valve_pin.set_value(new_val)?;
        if let Some(status) = self.status_led_pin {
            status.set_value(new_val)?
        }
        Ok(())
    }

    fn get_valve_pin_num(&self) -> &ValvePinNumber {
        &self.valve_pin_number
    }
}

impl GpioToggleValve {
    pub fn from_config(valve: &ValveConfig) -> GpioToggleValve {
        GpioToggleValve {
            valve_pin_number: ValvePinNumber(valve.get_valve_pin_num()),
            valve_pin: create_pin(valve.get_valve_pin_num(), Direction::Out),
            status_led_pin: valve
                .get_status_led_pin_num()
                .map(|p| create_pin(p, Direction::Out)),
            button_pin: valve
                .get_button_pin_num()
                .map(|p| create_pin(p, Direction::In)),
        }
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

    fn status_on(&mut self) -> Result<(), Error> {
        set_pin_value(&self.status_led_pin, 1);
        Ok(())
    }

    fn status_off(&mut self) -> Result<(), Error> {
        set_pin_value(&self.status_led_pin, 0);
        Ok(())
    }
}

pub struct GpioPumpPin {
    pump_pin: Pin,
    status_led_pin: Option<Pin>,
}

impl GpioPumpPin {
    pub fn turn_on(&mut self) -> Result<(), Error> {
        self.pump_pin.set_value(1)?;
        set_pin_value(&self.status_led_pin, 1);
        Ok(())
    }
    pub fn turn_off(&mut self) -> Result<(), Error> {
        self.pump_pin.set_value(0)?;
        set_pin_value(&self.status_led_pin, 0);
        Ok(())
    }
}

fn create_pin(pin_num: u8, direction: Direction) -> Pin {
    let pin = Pin::new(pin_num as u64);
    pin.export().expect("GPIO error.");
    pin.set_direction(direction)
        .expect("Could not set gpio pin direction.");
    if let Direction::In = direction {
        pin.set_edge(Edge::RisingEdge)
            .expect("Could not set gpio pin edge")
    }
    pin
}

fn create_pump_pin(pump_config: &PumpConfig) -> GpioPumpPin {
    let pump_pin = create_pin(pump_config.get_power_pin_num(), Direction::Out);
    let status_led_pin = pump_config
        .get_status_led_pin_num()
        .map(|num| create_pin(num, Direction::Out));
    GpioPumpPin {
        pump_pin,
        status_led_pin,
    }
}

fn set_pin_value(pin: &Option<Pin>, value: u8) {
    if let Some(p) = pin {
        p.set_value(value)
            .expect("GPIO Pin is not working. Could not set value.")
    }
}
