mod gpio;

extern crate ctrlc;
extern crate futures;
extern crate sysfs_gpio;
extern crate tokio_core;

use futures::{Future, Stream};
use gpio::{PinLayout, ToggleValve};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use sysfs_gpio::Error;
use tokio_core::reactor::{Core, Handle};

fn run_start_sequence(pin_layout: &PinLayout) -> Result<(), Error> {
    println!("Garden buttler starting ...");
    for millis in [200, 200, 400, 200, 200].iter() {
        let running_led = pin_layout.get_power_pin();
        let error_led = pin_layout.get_error_pin();
        let valves = pin_layout.get_valve_pins();
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
    println!("Garden buttler started ...");

    Ok(())
}

fn power_on(layout: &PinLayout) -> Result<(), Error> {
    layout.get_power_pin().set_value(1)?;
    Ok(())
}

fn stream_button_presses(handle: &Handle, layout: &PinLayout) -> Result<(), Error> {
    for toggle_valve in &layout.get_valve_pins() {
        let button_pin = toggle_valve.get_button_pin();
        let valve_pin = toggle_valve.get_valve_pin().clone();
        handle.spawn(
            button_pin
                .get_value_stream(&handle)?
                .for_each(move |_val| {
                    let new_val = 1 - valve_pin.get_value()?;
                    valve_pin.set_value(new_val)?;
                    Ok(())
                })
                .map_err(|_| ()),
        );
    }
    Ok(())
}

fn main() {
    let layout = PinLayout::new(23, 17, vec![ToggleValve::new(27, 22)]);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let layout_clone = layout.clone();
    ctrlc::set_handler(move || {
        println!("Shutting down garden buttler ...");
        layout_clone
            .unexport_all()
            .expect("Unexport all gpio pins.");
        r.store(false, Ordering::SeqCst);
    }).expect("Ctrl-C handler");

    run_start_sequence(&layout).expect("StartSequence run.");
    power_on(&layout).expect("Power Pin turned on.");

    let mut l = Core::new().expect("Tokio Core created.");
    let handle = l.handle();
    stream_button_presses(&handle, &layout).expect("Stream button presses.");
    while running.load(Ordering::SeqCst) {
        l.turn(None);
    }

    println!("Exiting garden buttler ...");
}
