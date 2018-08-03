use sysfs_gpio;

#[derive(Clone)]
pub struct PinLayout {
    power_pin: sysfs_gpio::Pin,
    error_pin: sysfs_gpio::Pin,
    valve_pins: Vec<sysfs_gpio::Pin>,
}

impl PinLayout {
    pub fn new(power_pin_num: u64, error_pin_num: u64, valve_pin_nums: Vec<u64>) -> PinLayout {
        let power_pin = sysfs_gpio::Pin::new(power_pin_num);
        power_pin.export().expect("GPIO error.");
        power_pin
            .set_direction(sysfs_gpio::Direction::Out)
            .expect("GPIO error.");

        let error_pin = sysfs_gpio::Pin::new(error_pin_num);
        error_pin.export().expect("GPIO error.");
        error_pin
            .set_direction(sysfs_gpio::Direction::Out)
            .expect("GPIO error.");

        let mut valve_pins = Vec::with_capacity(valve_pin_nums.len());
        for valve_num in valve_pin_nums {
            let valve = sysfs_gpio::Pin::new(valve_num);
            valve.export().expect("GPIO error.");
            valve
                .set_direction(sysfs_gpio::Direction::Out)
                .expect("GPIO error.");

            valve_pins.push(valve);
        }

        PinLayout {
            power_pin: power_pin,
            error_pin: error_pin,
            valve_pins: valve_pins,
        }
    }

    pub fn get_power_pin(&self) -> &sysfs_gpio::Pin {
        &self.power_pin
    }
    pub fn get_error_pin(&self) -> &sysfs_gpio::Pin {
        &self.error_pin
    }
    pub fn get_valve_pins(&self) -> Vec<&sysfs_gpio::Pin> {
        let mut refs = Vec::with_capacity(self.valve_pins.len());
        for i in &self.valve_pins {
            refs.push(i);
        }
        refs
    }

    pub fn unexport_all(&self) -> Result<(), sysfs_gpio::Error> {
        self.power_pin.set_value(0)?;
        self.power_pin.unexport()?;
        self.error_pin.set_value(0)?;
        self.error_pin.unexport()?;
        for v in &self.valve_pins {
            v.set_value(0)?;
            v.unexport()?;
        }
        Ok(())
    }
}
