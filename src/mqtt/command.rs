use std::marker::PhantomData;
use std::num::ParseIntError;
use std::ops::Deref;
use std::str::{FromStr, Utf8Error};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rumqtt::{Notification, Publish};
use tokio::prelude::{Future, Stream};

use embedded::{PinLayout, ToggleValve, ValvePinNumber};
use mqtt::MqttSession;

pub fn command_listener<T, U>(session: &Arc<Mutex<MqttSession>>, pin_layout: &Arc<Mutex<T>>) -> impl Future<Item=(), Error=()> + Send
    where T: PinLayout<U> + Send + 'static, U: ToggleValve + Send + 'static {
    let layout = Arc::clone(pin_layout);
    let mqtt_session = Arc::clone(session);

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
