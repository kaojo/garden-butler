extern crate chrono;
extern crate config;
extern crate core;
extern crate cron;
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

use std::ops::Deref;
use std::str::{FromStr, Utf8Error};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{Future, Stream};
use rumqtt::{Notification, QoS, Publish};
use serde::export::PhantomData;
use tokio::runtime::{Builder, Runtime};
use tokio_timer::clock::Clock;

use embedded::{PinLayout, ToggleValve, ValvePinNumber};
use embedded::configuration::LayoutConfig;
#[cfg(not(feature = "gpio"))]
use embedded::fake::FakePinLayout;
#[cfg(feature = "gpio")]
use embedded::gpio::GpioPinLayout;
use mqtt::configuration::MqttConfig;
use mqtt::{MqttSession};
use schedule::{WateringScheduleConfigs, WateringScheduler};
use std::num::ParseIntError;

mod embedded;
mod schedule;
mod mqtt;


#[cfg(feature = "gpio")]
pub const LAYOUT_TYPE: PhantomData<GpioPinLayout> = PhantomData;
#[cfg(not(feature = "gpio"))]
pub const LAYOUT_TYPE: PhantomData<FakePinLayout> = PhantomData;

fn main() {
    println!("Garden buttler starting ...");

    let mut rt = Builder::new().clock(Clock::system()).build().unwrap();

    let layout_config = LayoutConfig::default();
    println!("{:?}", layout_config);
    let layout = create_pin_layout(&layout_config, LAYOUT_TYPE);

    #[cfg(feature = "gpio")]
        {
            let button_streams = layout.lock().unwrap().get_button_streams();
            rt.spawn(button_streams);
        }

    let scheduler = create_and_start_schedules(&mut rt, &layout);

    let mqtt_config = MqttConfig::default();
    let mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(mqtt_config.clone());
    // TODO replace global subscription with only relevant topics for reception of commands
    mqtt_session.lock().unwrap().client.subscribe(format!("{}/garden-butler/#", &mqtt_config.client_id), QoS::AtLeastOnce).unwrap();
    rt.spawn(
        mqtt_message_listener(Arc::clone(&mqtt_session), &layout)
    );

    wait_for_termination(&mut rt);
}

fn create_pin_layout<T, U>(config: &LayoutConfig, _: PhantomData<T>) -> Arc<Mutex<T>>
    where T: PinLayout<U> + 'static, U: ToggleValve + Send + 'static {
    Arc::new(Mutex::new(T::new(config)))
}

fn mqtt_message_listener<T, U>(mqtt_session: Arc<Mutex<MqttSession>>, pin_layout: &Arc<Mutex<T>>) -> impl Future<Item=(), Error=()> + Send
    where T: PinLayout<U> + Send + 'static, U: ToggleValve + Send + 'static {
    let layout = Arc::clone(pin_layout);

    tokio_timer::Interval::new_interval(Duration::from_secs(1))
        .map_err(|_| ())
        .map(move |_| {
            mqtt_session
                .lock()
                .unwrap()
                .receiver
                .try_recv()
                .map_err(|_| ())
        })
        .inspect(|n| {
            match n {
                Ok(r) => { println!("{:?}", r); }
                Err(_) => {}
            }
        })
        .for_each(move |n| {
            match n {
                Ok(notification) => {
                    match notification {
                        Notification::Publish(publish) => {
                            if publish.topic_name.ends_with("/garden-butler/command/layout/open") {
                                let s = get_valve_pin_num_from_message(publish);
                                if let Ok(Ok(pin_num)) = s {
                                    if let Ok(valve) = layout.lock().unwrap().find_pin(ValvePinNumber(pin_num)) {
                                        valve.lock().unwrap().turn_on().map_err(|_| ())?;
                                    }
                                }
                            } else if publish.topic_name.ends_with("/garden-butler/command/layout/close") {
                                let s = get_valve_pin_num_from_message(publish);
                                if let Ok(Ok(pin_num)) = s {
                                    if let Ok(valve) = layout.lock().unwrap().find_pin(ValvePinNumber(pin_num)) {
                                        valve.lock().unwrap().turn_off().map_err(|_| ())?;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(_) => {}
            }
            Ok(())
        })
        .map_err(|_| ())
}

fn get_valve_pin_num_from_message(publish: Publish) -> Result<Result<u8, ParseIntError>, Utf8Error> {
    std::str::from_utf8(publish.payload.deref()).map(|s| u8::from_str(s))
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
/*
struct LayoutFacade {
    inner: dyn PinLayout<dyn ToggleValve>
}

impl<U> PinLayout<U> for LayoutFacade where U: ToggleValve {
    fn find_pin(&self, valve_pin_num: ValvePinNumber) -> Result<&Arc<Mutex<U>>, ()> {
        self.inner.find_pin(valve_pin_num)
    }
}
*/
