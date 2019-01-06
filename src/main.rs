extern crate chrono;
extern crate config;
extern crate core;
extern crate cron;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate sysfs_gpio;
extern crate tokio;
extern crate tokio_channel;
extern crate tokio_chrono;
extern crate tokio_signal;
extern crate tokio_timer;

use futures::{Future, Stream};
use tokio::runtime::{Builder, Runtime};
use tokio_timer::clock::Clock;

use embedded::{LayoutConfig, PinLayout};
use schedule::{ScheduleConfig, WateringScheduleConfig, WateringScheduler};

mod embedded;
mod schedule;

fn main() {
    println!("Garden buttler starting ...");

    let layout_config = get_layout_config();
    println!("{:?}", layout_config);
    let layout = PinLayout::from_config(&layout_config);

    layout.run_start_sequence().expect("StartSequence run.");
    layout.power_on().expect("Power Pin turned on.");

    let mut rt = Builder::new().clock(Clock::system()).build().unwrap();

    let button_streams = layout.get_button_streams();
    rt.spawn(button_streams);

    let schedule = ScheduleConfig::new(String::from("5 * * * * *"), 10);
    println!("{:?}", schedule);
    let watering_schedule_config = WateringScheduleConfig::new(schedule, 27);

    let mut scheduler: WateringScheduler = WateringScheduler::new(layout.clone());
    let schedule_task = scheduler.create_schedule(watering_schedule_config);
    match schedule_task {
        Ok(st) => {
            rt.spawn(st);
        }
        Err(_) => println!("Error creating scheduler task"),
    }

    // wait until program is terminated
    wait_for_termination(layout, &mut rt);
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

fn wait_for_termination(layout: PinLayout, rt: &mut Runtime) {
    let ctrl_c = tokio_signal::ctrl_c().flatten_stream().take(1);
    let prog = ctrl_c.for_each(move |()| {
        println!("ctrl-c received!");
        layout
            .unexport_all()
            .expect("Unexport of GPIO pins failed.");
        Ok(())
    });
    println!("Garden buttler started ...");
    rt.block_on(prog)
        .expect("Error waiting until app is terminated with ctrl+c");
    println!("Exiting garden buttler ...");
}
