use std::ops::Deref;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::FutureExt;
use rumqtt::{Notification, Publish, QoS};
use tokio::sync::mpsc::Sender;

use crate::embedded::command::LayoutCommand;
use crate::embedded::ValvePinNumber;
use crate::mqtt::configuration::MqttConfig;
use crate::mqtt::MqttSession;
use crate::schedule::WateringConfigCommand;
use crate::schedule::WateringScheduleConfig;

pub struct MqttCommandListener {
    inner: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl MqttCommandListener {
    pub fn new(
        mqtt_session: Arc<Mutex<MqttSession>>,
        mqtt_config: Arc<Mutex<MqttConfig>>,
        layout_command_sender: &Option<Sender<LayoutCommand>>,
        watering_config_command_sender: &Option<Sender<WateringConfigCommand>>,
    ) -> MqttCommandListener {
        let layout_command_tx = layout_command_sender.as_ref().cloned();
        let watering_config_command_tx = watering_config_command_sender.as_ref().cloned();

        subscribe_to_commands(&mqtt_session, &mqtt_config);

        let mqtt_session_2 = Arc::clone(&mqtt_session);

        let listener = tokio::time::interval(Duration::from_secs(1))
            .map(move |_| mqtt_session.lock().unwrap().receiver.try_recv())
            .inspect(|n| log_commands(n))
            .for_each(move |n| {
                match n {
                    Ok(Notification::Publish(publish)) => {
                        if is_valve_open_topic(&publish) {
                            MqttCommandListener::send_valve_open_command(
                                &layout_command_tx,
                                &publish,
                            )
                        } else if is_valve_close_topic(&publish) {
                            MqttCommandListener::send_valve_close_command(
                                &layout_command_tx,
                                &publish,
                            )
                        } else if is_schedule_enable_topic(&publish) {
                            MqttCommandListener::send_schedule_enable_command(
                                &watering_config_command_tx,
                                &publish,
                            )
                        } else if is_schedule_disable_topic(&publish) {
                            MqttCommandListener::send_schedule_disable_command(
                                &watering_config_command_tx,
                                &publish,
                            )
                        } else if is_schedule_delete_topic(&publish) {
                            MqttCommandListener::send_schedule_delete_command(
                                &watering_config_command_tx,
                                &publish,
                            )
                        } else if is_schedule_create_topic(&publish) {
                            MqttCommandListener::send_schedule_create_command(
                                &watering_config_command_tx,
                                &publish,
                            )
                        }
                    }
                    Ok(Notification::Reconnection) => {
                        let mut guard = mqtt_session_2.lock().unwrap();
                        let topic = guard.config.client_id.clone() + "/garden-butler/status/health";
                        guard
                            .publish(topic, QoS::ExactlyOnce, true, "ONLINE")
                            .map_err(|e| println!("error = {}", e))
                            .unwrap_or(());
                    }
                    Err(_) => {}
                    _ => println!("other mqtt message"),
                }
                future::ready(())
            })
            .map(|_| ());
        let inner = listener.boxed();
        MqttCommandListener { inner }
    }

    fn send_valve_close_command(
        layout_command_tx: &Option<Sender<LayoutCommand>>,
        publish: &Publish,
    ) {
        let s = get_valve_pin_num_from_message(&publish);
        if let Ok(pin_num) = s {
            if let Some(tx) = layout_command_tx {
                match tx.clone().try_send(LayoutCommand::Close(pin_num)) {
                    Ok(_) => {}
                    Err(e) => println!("error sending close command = {}", e),
                }
            }
        }
    }

    fn send_valve_open_command(
        layout_command_tx: &Option<Sender<LayoutCommand>>,
        publish: &Publish,
    ) {
        let s = get_valve_pin_num_from_message(&publish);
        if let Ok(pin_num) = s {
            if let Some(tx) = layout_command_tx {
                match tx.clone().try_send(LayoutCommand::Open(pin_num)) {
                    Ok(_) => {}
                    Err(e) => println!("error sending open command = {}", e),
                }
            }
        }
    }

    fn send_schedule_enable_command(
        watering_command_tx: &Option<Sender<WateringConfigCommand>>,
        publish: &Publish,
    ) {
        let schedule_config_result: Result<WateringScheduleConfig, ()> =
            get_schedule_config_from_message(publish);
        if let Ok(schedule_config) = schedule_config_result {
            if let Some(tx) = watering_command_tx {
                match tx
                    .clone()
                    .try_send(WateringConfigCommand::Enable(schedule_config))
                {
                    Ok(_) => println!("watering enable command send"),
                    Err(e) => println!("error sending schedule enable command = {}", e),
                }
            }
        }
    }

    fn send_schedule_disable_command(
        watering_command_tx: &Option<Sender<WateringConfigCommand>>,
        publish: &Publish,
    ) {
        let schedule_config_result: Result<WateringScheduleConfig, ()> =
            get_schedule_config_from_message(publish);
        if let Ok(schedule_config) = schedule_config_result {
            if let Some(tx) = watering_command_tx {
                match tx
                    .clone()
                    .try_send(WateringConfigCommand::Disable(schedule_config))
                {
                    Ok(_) => println!("watering disable command send"),
                    Err(e) => println!("error sending schedule enable command = {}", e),
                }
            }
        }
    }

    fn send_schedule_delete_command(
        watering_command_tx: &Option<Sender<WateringConfigCommand>>,
        publish: &Publish,
    ) {
        let schedule_config_result: Result<WateringScheduleConfig, ()> =
            get_schedule_config_from_message(publish);
        if let Ok(schedule_config) = schedule_config_result {
            if let Some(tx) = watering_command_tx {
                match tx
                    .clone()
                    .try_send(WateringConfigCommand::Delete(schedule_config))
                {
                    Ok(_) => println!("watering delete command send"),
                    Err(e) => println!("error sending schedule enable command = {}", e),
                }
            }
        }
    }

    fn send_schedule_create_command(
        watering_command_tx: &Option<Sender<WateringConfigCommand>>,
        publish: &Publish,
    ) {
        let schedule_config_result: Result<WateringScheduleConfig, ()> =
            get_schedule_config_from_message(publish);
        if let Ok(schedule_config) = schedule_config_result {
            if let Some(tx) = watering_command_tx {
                match tx
                    .clone()
                    .try_send(WateringConfigCommand::Create(schedule_config))
                {
                    Ok(_) => println!("watering create command send"),
                    Err(e) => println!("error sending schedule enable command = {}", e),
                }
            }
        }
    }
}

fn is_valve_open_topic(publish: &Publish) -> bool {
    publish
        .topic_name
        .ends_with("/garden-butler/command/layout/open")
}

fn is_valve_close_topic(publish: &Publish) -> bool {
    publish
        .topic_name
        .ends_with("/garden-butler/command/layout/close")
}

fn is_schedule_enable_topic(publish: &Publish) -> bool {
    publish
        .topic_name
        .ends_with("/garden-butler/command/watering-schedule/enable")
}

fn is_schedule_disable_topic(publish: &Publish) -> bool {
    publish
        .topic_name
        .ends_with("/garden-butler/command/watering-schedule/disable")
}

fn is_schedule_delete_topic(publish: &Publish) -> bool {
    publish
        .topic_name
        .ends_with("/garden-butler/command/watering-schedule/delete")
}

fn is_schedule_create_topic(publish: &Publish) -> bool {
    publish
        .topic_name
        .ends_with("/garden-butler/command/watering-schedule/create")
}

fn subscribe_to_commands(
    mqtt_session: &Arc<Mutex<MqttSession>>,
    mqtt_config: &Arc<Mutex<MqttConfig>>,
) {
    let topic = format!(
        "{}/garden-butler/command/#",
        &mqtt_config.lock().unwrap().client_id
    );
    mqtt_session
        .lock()
        .unwrap()
        .subscribe(topic, QoS::AtLeastOnce)
        .unwrap();
}

fn log_commands(n: &Result<Notification, crossbeam::TryRecvError>) {
    match n {
        Ok(r) => {
            println!("{:?}", r);
        }
        Err(_) => {}
    }
}

impl Future for MqttCommandListener {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}

fn get_valve_pin_num_from_message(publish: &Publish) -> Result<ValvePinNumber, ()> {
    std::str::from_utf8(publish.payload.deref())
        .map_err(|_| ())
        .and_then(|s| u8::from_str(s).map_err(|_| ()))
        .map(ValvePinNumber)
}

fn get_schedule_config_from_message(publish: &Publish) -> Result<WateringScheduleConfig, ()> {
    let payload_string = std::str::from_utf8(publish.payload.deref());
    payload_string
        .map_err(|e| println!("{}", e))
        .and_then(|json_str| serde_json::from_str(json_str).map_err(|e| println!("{}", e)))
}
