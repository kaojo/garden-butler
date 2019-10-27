extern crate chrono;
extern crate config;
extern crate core;
extern crate crossbeam;
#[macro_use]
extern crate futures;
extern crate rumqtt;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[cfg(feature = "gpio")]
extern crate sysfs_gpio;
extern crate tokio;
extern crate tokio_channel;
extern crate tokio_signal;
extern crate tokio_timer;

use std::sync::{Arc, Mutex};

use crossbeam::{Receiver, Sender};
use futures::{Future, Stream};
use futures::future::lazy;
use rumqtt::QoS;
use serde::export::PhantomData;

use communication::ReceiverFuture;
use embedded::{PinLayout, ToggleValve};
use embedded::command::{LayoutCommand, LayoutCommandListener};
use embedded::configuration::LayoutConfig;
#[cfg(not(feature = "gpio"))]
use embedded::fake::FakePinLayout;
#[cfg(feature = "gpio")]
use embedded::gpio::GpioPinLayout;
use mqtt::command::command_listener;
use mqtt::configuration::MqttConfig;
use mqtt::MqttSession;
use mqtt::status::{LayoutConfigStatus, PinLayoutStatus, WateringScheduleConfigStatus};
use schedule::{WateringScheduleConfigs, WateringScheduler};

mod communication;
mod embedded;
mod mqtt;
mod schedule;

#[cfg(feature = "gpio")]
pub const LAYOUT_TYPE: PhantomData<GpioPinLayout> = PhantomData;
#[cfg(not(feature = "gpio"))]
pub const LAYOUT_TYPE: PhantomData<FakePinLayout> = PhantomData;

fn main() {
    println!("Garden buttler starting ...");

    tokio::run(lazy(|| {
        let layout_config = LayoutConfig::default();
        let layout = create_pin_layout(layout_config.clone(), LAYOUT_TYPE);

        // this vec stores all channels, this is needed so the sender and receiver don't become
        // disconnected if any data is dropped
        let mut ctrl_c_channels = Vec::new();

        // command listening
        let (layout_command_sender, layout_command_receiver): (Sender<LayoutCommand>, Receiver<LayoutCommand>) = crossbeam::unbounded();
        let (layout_status_send_sender, layout_status_send_receiver): (Sender<Result<(), ()>>, Receiver<Result<(), ()>>) = crossbeam::unbounded();
        tokio::spawn(
            {
                let (s, r) = crossbeam::unbounded();
                ctrl_c_channels.push((s.clone(), r.clone()));

                LayoutCommandListener::new(
                    Arc::clone(&layout),
                    layout_command_receiver.clone(),
                    layout_status_send_sender.clone())
                    .select2(ReceiverFuture::new(r.clone()))
                    .map(|_| ())
                    .map_err(|_| ())
            }
        );

        // listen for physical button presses
        #[cfg(feature = "gpio")]
            {
                tokio::spawn(
                    {
                        let (s, r) = crossbeam::unbounded();
                        ctrl_c_channels.push((s.clone(), r.clone()));
                        let button_streams = layout.lock().unwrap().get_button_streams();
                        button_streams
                            .select2(ReceiverFuture::new(r.clone()))
                            .map(|_| ())
                            .map_err(|_| ())
                    }
                );
            }

        let mqtt_config = MqttConfig::default();
        let mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(mqtt_config.clone());

        // listen to mqtt messages that span commands
        mqtt_session.lock().unwrap().client
            .subscribe(format!("{}/garden-butler/command/#", &mqtt_config.client_id), QoS::AtLeastOnce)
            .unwrap();
        tokio::spawn({
            let command_sender_clone = layout_command_sender.clone();
            let (s, r) = crossbeam::unbounded();
            ctrl_c_channels.push((s.clone(), r.clone()));
            command_listener(&mqtt_session, command_sender_clone)
                .select2(ReceiverFuture::new(r.clone()))
                .map(|_| ())
                .map_err(|_| ())
        });

        // spawn preconfigured automatic watering tasks
        let mut scheduler =
            WateringScheduler::new(WateringScheduleConfigs::default(), layout_command_sender.clone());
        scheduler.start().into_iter().for_each(|schedule_future| {
            let (s, r) = crossbeam::unbounded();
            ctrl_c_channels.push((s.clone(), r.clone()));
            tokio::spawn(
                schedule_future
                    .select2(ReceiverFuture::new(r.clone()))
                    .map(|_| ())
                    .map_err(|_| ()),
            );
        });
        let watering_scheduler = Arc::new(Mutex::new(scheduler));

        // report layout status
        tokio::spawn({
            let (s, r) = crossbeam::unbounded();
            ctrl_c_channels.push((s.clone(), r.clone()));

            PinLayoutStatus::new(
                Arc::clone(&layout),
                Arc::clone(&mqtt_session),
                mqtt_config.clone(),
                layout_status_send_receiver,
            )
                .select2(ReceiverFuture::new(r.clone()))
                .map(|_| ())
                .map_err(|_| ())
        });

        // report automatic watering configuration
        tokio::spawn({
            let (s, r) = crossbeam::unbounded();
            ctrl_c_channels.push((s.clone(), r.clone()));

            WateringScheduleConfigStatus::new(Arc::clone(&watering_scheduler), Arc::clone(&mqtt_session))
                .select2(ReceiverFuture::new(r.clone()))
                .map(|_| ())
                .map_err(|_| ())
        });

        // report layout configuration
        tokio::spawn({
            let (s, r) = crossbeam::unbounded();
            ctrl_c_channels.push((s.clone(), r.clone()));

            LayoutConfigStatus::new(&layout_config, Arc::clone(&mqtt_session))
                .select2(ReceiverFuture::new(r.clone()))
                .map(|_| ())
                .map_err(|_| ())
        });

        report_online(Arc::clone(&mqtt_session));

        // listen for program termination
        let ctrl_c = tokio_signal::ctrl_c()
            .flatten_stream()
            .take(1)
            .map_err(|e| println!("ctrlc-error = {:?}", e));
        let prog = ctrl_c.for_each(move |_| {
            println!(
                "ctrl-c received! Sending message to {} futures.",
                ctrl_c_channels.len()
            );
            ctrl_c_channels.iter().for_each(|sender| {
                sender
                    .0
                    .send("ctrl-c received!".to_string())
                    .map_err(|e| println!("send error = {}", e.0))
                    .unwrap_or_default();
            });
            Ok(())
        });
        prog
    }));
    println!("Exiting garden buttler ...");
}

fn report_online(mqtt_session: Arc<Mutex<MqttSession>>) {
    let mut session = mqtt_session.lock().unwrap();
    let topic = format!("{}/garden-butler/status/health", session.get_client_id());
    let message = "ONLINE";
    match session.publish(topic, QoS::ExactlyOnce, true, message) {
        Ok(_) => {
            println!("Garden buttler started ...");
        }
        Err(e) => {
            println!("error starting garden butler = {:?}", e);
        }
    }
}

fn create_pin_layout<T, U>(config: LayoutConfig, _: PhantomData<T>) -> Arc<Mutex<T>>
    where
        T: PinLayout<U> + 'static,
        U: ToggleValve + Send + 'static,
{
    Arc::new(Mutex::new(T::new(&config)))
}
