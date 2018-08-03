use sysfs_gpio::{Direction, Edge, Error, Pin};

#[derive(Clone)]
pub struct PinLayout {
    power_pin: Pin,
    error_pin: Pin,
    valve_pins: Vec<ToggleValve>,
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
            power_pin: power_pin,
            error_pin: error_pin,
            valve_pins: toggle_valves,
        }
    }

    pub fn get_power_pin(&self) -> &Pin {
        &self.power_pin
    }
    pub fn get_error_pin(&self) -> &Pin {
        &self.error_pin
    }
    pub fn get_valve_pins(&self) -> Vec<&ToggleValve> {
        let mut refs = Vec::with_capacity(self.valve_pins.len());
        for i in &self.valve_pins {
            refs.push(i);
        }
        refs
    }

    pub fn unexport_all(&self) -> Result<(), Error> {
        self.power_pin.set_value(0)?;
        self.power_pin.unexport()?;
        self.error_pin.set_value(0)?;
        self.error_pin.unexport()?;
        for toggle_valve in &self.valve_pins {
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
            valve_pin: valve_pin,
            button_pin: button_pin,
        }
    }

    pub fn get_valve_pin(&self) -> &Pin {
        &self.valve_pin
    }

    pub fn get_button_pin(&self) -> &Pin {
        &self.button_pin
    }
}
