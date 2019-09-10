extern crate chrono;
extern crate config;
extern crate core;
extern crate cron;
extern crate crossbeam;
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

use futures::{Future, Stream};
use futures::future::lazy;
use rumqtt::QoS;
use serde::export::PhantomData;

use communication::ReceiverFuture;
use embedded::{PinLayout, ToggleValve};
use embedded::configuration::LayoutConfig;
#[cfg(not(feature = "gpio"))]
use embedded::fake::FakePinLayout;
#[cfg(feature = "gpio")]
use embedded::gpio::GpioPinLayout;
use mqtt::command::command_listener;
use mqtt::configuration::MqttConfig;
use mqtt::MqttSession;
use schedule::{WateringScheduleConfigs, WateringScheduler};

mod embedded;
mod schedule;
mod mqtt;
mod communication;

#[cfg(feature = "gpio")]
pub const LAYOUT_TYPE: PhantomData<GpioPinLayout> = PhantomData;
#[cfg(not(feature = "gpio"))]
pub const LAYOUT_TYPE: PhantomData<FakePinLayout> = PhantomData;

fn main() {
    println!("Garden buttler starting ...");

    tokio::run(lazy(|| {
        let mut ctrl_c_senders = Vec::new();

        let layout = create_pin_layout(LayoutConfig::default(), LAYOUT_TYPE);
        let mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(MqttConfig::default());
        let mut scheduler =
            WateringScheduler::new(WateringScheduleConfigs::default(), Arc::clone(&layout));

        #[cfg(feature = "gpio")]
            {
                let (s, r) = crossbeam::unbounded();
                ctrl_c_senders.push((s.clone(), r.clone()));
                let button_streams = layout.lock().unwrap().get_button_streams();
                tokio::spawn(button_streams
                    .select2(ReceiverFuture::new(r.clone()))
                    .map(|_| ())
                    .map_err(|_| ())
                );
            }

        let mqtt_config = MqttConfig::default();
        // TODO replace global subscription with only relevant topics for reception of commands
        mqtt_session.lock().unwrap().client.subscribe(format!("{}/garden-butler/#", &mqtt_config.client_id), QoS::AtLeastOnce).unwrap();
        tokio::spawn({
            let (s, r) = crossbeam::unbounded();
            ctrl_c_senders.push((s.clone(), r.clone()));
            command_listener(&mqtt_session, &layout)
                .select2(ReceiverFuture::new(r.clone()))
                .map(|_| ())
                .map_err(|_| ())
        }
        );

        if scheduler.enabled {
            scheduler
                .start().into_iter()
                .filter(|res| res.is_ok())
                .map(|res| res.unwrap())
                .for_each(|schedule_future| {
                    let (s, r) = crossbeam::unbounded();
                    ctrl_c_senders.push((s.clone(), r.clone()));

                    tokio::spawn(
                        schedule_future
                            .select2(ReceiverFuture::new(r.clone()))
                            .map(|_| ())
                            .map_err(|_| ())
                    );
                });
        }

        println!("Garden buttler started ...");
        let ctrl_c = tokio_signal::ctrl_c()
            .flatten_stream().take(1).map_err(|e| println!("ctrlc-error = {:?}", e));
        let prog = ctrl_c.for_each(move |_| {
            println!("ctrl-c received! Sending message to {} futures.", ctrl_c_senders.len());
            ctrl_c_senders.iter().for_each(|sender| {
                println!("ctrl-c received!");
                sender.0.send("ctrl-c received!".to_string())
                    .map_err(|e| println!("send error = {}", e.0))
                    .unwrap_or_default();
            });
            Ok(())
        });
        prog
    }));
    println!("Exiting garden buttler ...");
}

fn create_pin_layout<T, U>(config: LayoutConfig, _: PhantomData<T>) -> Arc<Mutex<T>>
    where T: PinLayout<U> + 'static, U: ToggleValve + Send + 'static {
    Arc::new(Mutex::new(T::new(&config)))
}
