extern crate chrono;
extern crate config;
extern crate core;
extern crate cron;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(feature = "gpio")]
extern crate sysfs_gpio;
extern crate tokio;
extern crate tokio_channel;
extern crate tokio_chrono;
extern crate tokio_signal;
extern crate tokio_timer;

use std::sync::{Arc, Mutex};

use futures::{Future, Stream};
use tokio::runtime::{Builder, Runtime};
use tokio_timer::clock::Clock;

use embedded::{PinLayout, ToggleValve};
use embedded::configuration::LayoutConfig;
#[cfg(not(feature = "gpio"))]
use embedded::fake::{FakePinLayout, FakeToggleValve};
#[cfg(feature = "gpio")]
use embedded::gpio::{GpioPinLayout, GpioToggleValve};
use schedule::{WateringScheduleConfigs, WateringScheduler};

mod embedded;
mod schedule;

fn main() {
    println!("Garden buttler starting ...");

    let mut rt = Builder::new().clock(Clock::system()).build().unwrap();

    let layout_config = get_layout_config();
    println!("{:?}", layout_config);

    #[cfg(feature = "gpio")]
        {
            println!("Starting garden butler in GPIO mode.");
            let layout = GpioPinLayout::from_config(&layout_config);
            let shared_layout: Arc<Mutex<GpioPinLayout>> = Arc::new(Mutex::new(layout));

            let button_streams = shared_layout.lock().unwrap().get_button_streams();
            rt.spawn(button_streams);


            let mut scheduler: WateringScheduler<GpioPinLayout, GpioToggleValve> =
                WateringScheduler::new(get_watering_configs(), Arc::clone(&shared_layout));
            if scheduler.enabled {
                scheduler
                    .start(&mut rt)
                    .expect("Error starting watering schedules");
            }
            wait_for_termination(&mut rt);
        }

    #[cfg(not(feature = "gpio"))]
        {
            println!("Starting garden butler in fake mode.");
            let layout = FakePinLayout::from_config(&layout_config);
            let shared_layout: Arc<Mutex<FakePinLayout>> = Arc::new(Mutex::new(layout));

            let mut scheduler: WateringScheduler<FakePinLayout, FakeToggleValve> =
                WateringScheduler::new(get_watering_configs(), Arc::clone(&shared_layout));
            if scheduler.enabled {
                scheduler
                    .start(&mut rt)
                    .expect("Error starting watering schedules");
            }
            wait_for_termination(&mut rt);
        }
}

fn get_layout_config() -> LayoutConfig {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::new("layout", config::FileFormat::Json))
        .unwrap()
        // Add in settings from the environment (with a prefix of LAYOUT)
        // Eg.. `LAYOUT_POWER=11 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("LAYOUT"))
        .unwrap();
    let layout_config = settings
        .try_into::<LayoutConfig>()
        .expect("Layout config contains errors");
    layout_config
}

fn get_watering_configs() -> WateringScheduleConfigs {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::new(
            "watering-schedules",
            config::FileFormat::Json,
        ))
        .unwrap()
        // Add in settings from the environment (with a prefix of LAYOUT)
        // Eg.. `LAYOUT_POWER=11 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("WATERING"))
        .unwrap();
    let watering_configs = settings
        .try_into::<WateringScheduleConfigs>()
        .expect("Watering schedules config contains errors");
    println!("{:?}", watering_configs);
    watering_configs
}

fn wait_for_termination(rt: &mut Runtime) {
    let ctrl_c = tokio_signal::ctrl_c().flatten_stream().take(1);
    let prog = ctrl_c.for_each(move |()| {
        println!("ctrl-c received!");
        // TODO maybe remove in the future since nothing is done here anymore
        // right now we only wait till the program is terminated
        Ok(())
    });
    println!("Garden buttler started ...");

    rt.block_on(prog).expect("Error waiting until app is terminated with ctrl+c");
    println!("Exiting garden buttler ...");
}
