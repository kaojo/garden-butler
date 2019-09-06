extern crate chrono;
extern crate config;
extern crate core;
extern crate cron;
#[macro_use]
extern crate futures;
extern crate rumqtt;
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
use std::time::Duration;

use futures::{Future, Stream, stream};
use rumqtt::QoS;
use tokio::runtime::{Builder, Runtime};
use tokio_timer::clock::Clock;

use embedded::{PinLayout, ToggleValve};
use embedded::configuration::LayoutConfig;
#[cfg(not(feature = "gpio"))]
use embedded::fake::{FakePinLayout, FakeToggleValve};
#[cfg(feature = "gpio")]
use embedded::gpio::{GpioPinLayout, GpioToggleValve};
use mqtt::configuration::MqttConfig;
use mqtt::MqttSession;
use schedule::{WateringScheduleConfigs, WateringScheduler};

mod embedded;
mod schedule;
mod mqtt;

fn main() {
    println!("Garden buttler starting ...");

    let mut rt = Builder::new().clock(Clock::system()).build().unwrap();

    let layout_config = LayoutConfig::default();
    println!("{:?}", layout_config);

    let mqtt_config = MqttConfig::default();
    let mut mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(mqtt_config.clone());
    // TODO replace global subscription with only relevant topics for reception of commands
    mqtt_session.lock().unwrap().client.subscribe(format!("{}/garden-butler/#", &mqtt_config.client_id), QoS::AtLeastOnce).unwrap();
    rt.spawn(
        mqtt_message_logger(Arc::clone(&mqtt_session))
    );

    #[cfg(feature = "gpio")]
        {
            println!("Starting garden butler in GPIO mode.");
            let layout = GpioPinLayout::from_config(&layout_config);

            let button_streams = layout.lock().unwrap().get_button_streams();
            rt.spawn(button_streams);

            let scheduler = create_and_start_schedules(&mut rt, &layout);

            wait_for_termination(&mut rt);
        }

    #[cfg(not(feature = "gpio"))]
        {
            println!("Starting garden butler in fake mode.");
            let layout = FakePinLayout::from_config(&layout_config);

            let scheduler = create_and_start_schedules(&mut rt, &layout);

            wait_for_termination(&mut rt);
        }
}

fn mqtt_message_logger(mqtt_session: Arc<Mutex<MqttSession>>) -> impl Future<Item=(), Error=()> + Send {
    let clone = Arc::clone(&mqtt_session);
    tokio_timer::Interval::new_interval(Duration::from_secs(1))
        .for_each(move |_| {
            let result = mqtt_session.lock().unwrap().receiver.try_recv();
            match result {
                Ok(n) => {
                    println!("{:?}", n);
                    Ok(())
                }
                Err(_) => { Ok(()) }
            }
        })
        .map_err(|_| ())
}

fn create_and_start_schedules<T, U>(mut rt: &mut Runtime, shared_layout: &Arc<Mutex<T>>) -> WateringScheduler<T, U>
    where T: PinLayout<U> + 'static, U: ToggleValve + Send + 'static
{
    let mut scheduler =
        WateringScheduler::new(WateringScheduleConfigs::default(), Arc::clone(&shared_layout));
    if scheduler.enabled {
        scheduler
            .start(&mut rt)
            .expect("Error starting watering schedules");
    }
    scheduler
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
