use std::thread::sleep;
use std::time::Duration;

use futures::{Future, lazy, Stream};
use futures::Lazy;
use sysfs_gpio::{Direction, Edge, Error, Pin};

#[derive(Clone)]
pub struct PinLayout {
    power_pin: Pin,
    error_pin: Pin,
    toggle_valves: Vec<ToggleValve>,
}

impl PinLayout {
    pub fn new(
        power_pin_num: u64,
        error_pin_num: u64,
        toggle_valves: Vec<ToggleValve>,
    ) -> PinLayout {
        let power_pin = Pin::new(power_pin_num);
        power_pin.export().expect("GPIO error.");
        power_pin
            .set_direction(Direction::Out)
            .expect("GPIO error.");

        let error_pin = Pin::new(error_pin_num);
        error_pin.export().expect("GPIO error.");
        error_pin
            .set_direction(Direction::Out)
            .expect("GPIO error.");

        PinLayout {
            power_pin,
            error_pin,
            toggle_valves,
        }
    }

    pub fn run_start_sequence(&self) -> Result<(), Error> {
        for millis in [200, 200, 400, 200, 200].iter() {
            let running_led = self.power_pin;
            let error_led = self.error_pin;
            let valves = self.get_valve_pins();
            running_led.set_value(1)?;
            error_led.set_value(1)?;
            for v in &valves {
                v.get_valve_pin().set_value(1)?;
            }

            sleep(Duration::from_millis(*millis));
            running_led.set_value(0)?;
            error_led.set_value(0)?;
            for v in &valves {
                v.get_valve_pin().set_value(0)?;
            }

            sleep(Duration::from_millis(200));
        }

        Ok(())
    }

    pub fn power_on(&self) -> Result<(), Error> {
        self.power_pin.set_value(1)?;
        Ok(())
    }

    pub fn get_button_streams(&self) -> impl Future<Item = (), Error = ()> {
        let clone = self.clone();
        return lazy(move || {
            for toggle_valve in clone.get_valve_pins() {
                let button_pin = toggle_valve.get_button_pin();
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
            Ok(())
        });
    }

    fn get_valve_pins(&self) -> Vec<&ToggleValve> {
        let mut refs = Vec::with_capacity(self.toggle_valves.len());
        for i in &self.toggle_valves {
            refs.push(i);
        }
        refs
    }

    pub fn unexport_all(&self) -> Result<(), Error> {
        self.power_pin.set_value(0)?;
        self.power_pin.unexport()?;
        self.error_pin.set_value(0)?;
        self.error_pin.unexport()?;
        for toggle_valve in &self.toggle_valves {
            let v = toggle_valve.get_valve_pin();
            v.set_value(0)?;
            v.unexport()?;
            let b = toggle_valve.get_button_pin();
            b.unexport()?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ToggleValve {
    valve_pin: Pin,
    button_pin: Pin,
}

impl ToggleValve {
    pub fn new(valve_pin_num: u64, button_pin_num: u64) -> ToggleValve {
        let valve_pin = Pin::new(valve_pin_num);
        valve_pin.export().expect("GPIO error.");
        valve_pin
            .set_direction(Direction::Out)
            .expect("GPIO error.");

        let button_pin = Pin::new(button_pin_num);
        button_pin.export().expect("GPIO error.");
        button_pin.set_edge(Edge::RisingEdge).expect("Edge set.");
        button_pin
            .set_direction(Direction::In)
            .expect("GPIO error.");

        ToggleValve {
            valve_pin,
            button_pin,
        }
    }

    pub fn get_valve_pin(&self) -> &Pin {
        &self.valve_pin
    }

    pub fn get_button_pin(&self) -> &Pin {
        &self.button_pin
    }

}
