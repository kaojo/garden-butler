use std::num::ParseIntError;
use std::ops::Deref;
use std::str::{FromStr, Utf8Error};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam::Sender;
use rumqtt::{Notification, Publish, QoS};
use tokio::prelude::{Future, Stream};

use embedded::command::LayoutCommand;
use embedded::ValvePinNumber;
use mqtt::MqttSession;

pub fn command_listener(
    session: &Arc<Mutex<MqttSession>>,
    sender: Sender<LayoutCommand>,
) -> impl Future<Item=(), Error=()> + Send {

    // listen to mqtt messages that send commands
    let mut mqtt_session = session.lock().unwrap();
    let mqtt_config = &mqtt_session.config;
    let topic = format!("{}/garden-butler/command/#", &mqtt_config.client_id);

    mqtt_session
        .subscribe(topic, QoS::AtLeastOnce)
        .unwrap();

    let mqtt_session = Arc::clone(session);
    let mqtt_session_2 = Arc::clone(session);

    tokio_timer::Interval::new_interval(Duration::from_secs(1))
        .map_err(|_| ())
        .map(move |_| {
            mqtt_session
                .lock()
                .unwrap()
                .receiver
                .try_recv()
        })
        .inspect(|n| match n {
            Ok(r) => {
                println!("{:?}", r);
            }
            Err(_) => {}
        })
        .for_each(move |n| {
            match n {
                Ok(Notification::Publish(publish)) => {
                    if publish
                        .topic_name
                        .ends_with("/garden-butler/command/layout/open")
                    {
                        let s = get_valve_pin_num_from_message(publish);
                        if let Ok(Ok(pin_num)) = s {
                            match sender.send(LayoutCommand::Open(pin_num)) {
                                Ok(_) => {}
                                Err(e) => { println!("error sending open command = {}", e) }
                            }
                        }
                    } else if publish
                        .topic_name
                        .ends_with("/garden-butler/command/layout/close")
                    {
                        let s = get_valve_pin_num_from_message(publish);
                        if let Ok(Ok(pin_num)) = s {
                            match sender.send(LayoutCommand::Close(pin_num)) {
                                Ok(_) => {}
                                Err(e) => { println!("error sending close command = {}", e) }
                            }
                        }
                    }
                }
                Ok(Notification::Reconnection) => {
                    let mut guard = mqtt_session_2.lock().unwrap();
                    let topic = guard.config.client_id.clone() + "/garden-butler/status/health";
                    guard.publish(topic, QoS::ExactlyOnce, true, "ONLINE").map_err(|e| println!("error = {}", e))?;
                }
                _ => {}
            }
            Ok(())
        })
        .map_err(|_| ())
}

fn get_valve_pin_num_from_message(
    publish: Publish,
) -> Result<Result<ValvePinNumber, ParseIntError>, Utf8Error> {
    std::str::from_utf8(publish.payload.deref())
        .map(|s| u8::from_str(s).map(|n| ValvePinNumber(n)))
}
