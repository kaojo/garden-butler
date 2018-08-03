mod gpio;

extern crate ctrlc;
extern crate sysfs_gpio;

use gpio::PinLayout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

fn run_start_sequence(pin_layout: &PinLayout) -> Result<(), sysfs_gpio::Error> {
    println!("Garden buttler starting ...");
    for millis in [200, 200, 400, 200, 200].iter() {
        let running_led = pin_layout.get_power_pin();
        let error_led = pin_layout.get_error_pin();
        let valves = pin_layout.get_valve_pins();
        running_led.set_value(1)?;
        error_led.set_value(1)?;
        for v in &valves {
            v.set_value(1)?;
        }

        sleep(Duration::from_millis(*millis));
        running_led.set_value(0)?;
        error_led.set_value(0)?;
        for v in &valves {
            v.set_value(0)?;
        }

        sleep(Duration::from_millis(200));
    }
    println!("Garden buttler started ...");

    Ok(())
}

fn power_on(layout: &PinLayout) -> Result<(), sysfs_gpio::Error> {
    layout.get_power_pin().set_value(1)?;
    Ok(())
}

fn main() {
    let layout = PinLayout::new(23, 17, vec![27]);
    
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
    power_on(&layout).expect("Power Pin set to 1;");

    while running.load(Ordering::SeqCst) {

    }

    println!("Exiting garden buttler ...");
}
