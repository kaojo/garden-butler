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
use embedded::gpio::GpioPinLayout;
use mqtt::configuration::MqttConfig;
use mqtt::MqttSession;
use schedule::{WateringScheduleConfigs, WateringScheduler};

mod embedded;
mod schedule;
mod mqtt;

fn main() {
    println!("Garden buttler starting ...");

    let mut rt = Builder::new().clock(Clock::system()).build().unwrap();

    let layout_config = get_layout_config();
    println!("{:?}", layout_config);

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

            let mqtt_config = get_mqtt_config();
            let mut mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(mqtt_config);

            mqtt_session.lock().unwrap().client.subscribe("raspi/#", QoS::AtLeastOnce).unwrap();

            rt.spawn(
                notification_logger(mqtt_session)
            );

            wait_for_termination(&mut rt);
        }
}

fn notification_logger(mqtt_session: Arc<Mutex<MqttSession>>) -> impl Future<Item=(), Error=()> + Send {
    let clone = Arc::clone(&mqtt_session);
    tokio_timer::Interval::new_interval(Duration::from_secs(1))
        .for_each(move |_| {
            let result = clone.lock().unwrap().receiver.try_recv();
            match result {
                Ok(n) => {
                    println!("{:?}", n);
                    Ok(())
                }
                Err(_) => {Ok(())}
            }
        })
        .map_err(|_| ())
}

fn create_and_start_schedules<T, U>(mut rt: &mut Runtime, shared_layout: &Arc<Mutex<T>>) -> WateringScheduler<T, U>
    where T: PinLayout<U> + 'static, U: ToggleValve + Send + 'static
{
    let mut scheduler =
        WateringScheduler::new(get_watering_configs(), Arc::clone(&shared_layout));
    if scheduler.enabled {
        scheduler
            .start(&mut rt)
            .expect("Error starting watering schedules");
    }
    scheduler
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

fn get_mqtt_config() -> MqttConfig {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::new(
            "mqtt",
            config::FileFormat::Json,
        ))
        .unwrap()
        // Add in settings from the environment (with a prefix of LAYOUT)
        // Eg.. `LAYOUT_POWER=11 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("MQTT"))
        .unwrap();
    let config = settings
        .try_into::<MqttConfig>()
        .expect("Mqtt config contains errors");
    println!("{:?}", config);
    config
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
