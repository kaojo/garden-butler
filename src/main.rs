extern crate futures;
extern crate sysfs_gpio;
extern crate tokio;
extern crate tokio_signal;

use std::thread::sleep;
use std::time::Duration;

use futures::{Future, lazy, Stream};
use sysfs_gpio::Error;
use tokio::reactor::Handle;
use tokio::runtime::Runtime;

use gpio::{PinLayout, ToggleValve};

mod gpio;

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

fn main() {
    let layout = PinLayout::new(23, 17, vec![ToggleValve::new(27, 22)]);
    let layout_clone = layout.clone();

    run_start_sequence(&layout).expect("StartSequence run.");
    power_on(&layout).expect("Power Pin turned on.");

    let mut rt = Runtime::new().unwrap();

    let button_streams = lazy(move || {
        for toggle_valve in layout_clone.get_valve_pins() {
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
    rt.spawn(button_streams);

    // wait until program is terminated
    let ctrl_c = tokio_signal::ctrl_c().flatten_stream().take(1).map_err(|err| println!("error = {:?}", err));

    // Process each ctrl-c as it comes in
    let prog = ctrl_c.for_each(move |()| {
        println!("ctrl-c received!");
        layout.unexport_all().expect("Should unexport all exported gpio pins.");
        Ok(())
    });
    rt.block_on(prog);

    println!("Exiting garden buttler ...");
}
