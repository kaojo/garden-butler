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

use std::sync::{Arc, Mutex};

use crossbeam::{Receiver, Sender};
use futures::future::FusedFuture;
use futures::{select, Future, FutureExt};
use rumqtt::QoS;
use serde::export::PhantomData;

use app::App;
use communication::ReceiverFuture;
use embedded::command::{LayoutCommand, LayoutCommandListener};
use embedded::configuration::LayoutConfig;
#[cfg(not(feature = "gpio"))]
use embedded::fake::FakePinLayout;
#[cfg(feature = "gpio")]
use embedded::gpio::GpioPinLayout;
use embedded::{PinLayout, ToggleValve};
use mqtt::command::MqttCommandListener;
use mqtt::configuration::MqttConfig;
use mqtt::status::{LayoutConfigStatus, PinLayoutStatus, WateringScheduleConfigStatus};
use mqtt::MqttSession;
use schedule::{WateringScheduleConfigs, WateringScheduler};

mod app;
mod communication;
mod embedded;
mod mqtt;
mod schedule;

#[cfg(feature = "gpio")]
pub const LAYOUT_TYPE: PhantomData<GpioPinLayout> = PhantomData;
#[cfg(not(feature = "gpio"))]
pub const LAYOUT_TYPE: PhantomData<FakePinLayout> = PhantomData;

#[tokio::main]
async fn main() -> Result<(), ()> {
    println!("Garden buttler starting ...");

    let layout_config = LayoutConfig::default();
    let layout = create_pin_layout(layout_config.clone(), LAYOUT_TYPE);

    // this vec stores all channels, this is needed so the sender and receiver don't become
    // disconnected if any data is dropped
    let ctrl_c_channels = Arc::new(Mutex::new(Vec::new()));

    // command listening
    let (layout_command_sender, layout_command_receiver): (
        Sender<LayoutCommand>,
        Receiver<LayoutCommand>,
    ) = crossbeam::unbounded();
    let (layout_status_send_sender, layout_status_send_receiver): (
        Sender<Result<(), ()>>,
        Receiver<Result<(), ()>>,
    ) = crossbeam::unbounded();

    let layout_command_listener = LayoutCommandListener::new(
        Arc::clone(&layout),
        layout_command_receiver.clone(),
        layout_status_send_sender.clone(),
    )
    .fuse();

    spawn_task(Arc::clone(&ctrl_c_channels), layout_command_listener);

    // listen for physical button presses
    #[cfg(feature = "gpio")]
    {
        let clone = layout.lock().unwrap();
        clone.spawn_button_streams(Arc::clone(&ctrl_c_channels));
    }

    let mqtt_config = MqttConfig::default();
    let mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(mqtt_config.clone());

    {
        let mqtt_command_listener =
            MqttCommandListener::new(Arc::clone(&mqtt_session), layout_command_sender.clone())
                .fuse();
        spawn_task(Arc::clone(&ctrl_c_channels), mqtt_command_listener);
    }

    // spawn preconfigured automatic watering tasks
    let mut scheduler = WateringScheduler::new(
        WateringScheduleConfigs::default(),
        layout_command_sender.clone(),
    );

    let schedule_channels = scheduler.start();
    let ctrl_c_channels_for_closure = Arc::clone(&ctrl_c_channels);
    schedule_channels.into_iter().for_each(move |channel| {
        ctrl_c_channels_for_closure.lock().unwrap().push(channel);
    });

    let watering_scheduler = Arc::new(Mutex::new(scheduler));

    // report layout status
    {
        let pin_layout_status = PinLayoutStatus::new(
            Arc::clone(&layout),
            Arc::clone(&mqtt_session),
            mqtt_config.clone(),
            layout_status_send_receiver,
        )
        .boxed()
        .fuse()
        .map(|_| ());
        spawn_task(Arc::clone(&ctrl_c_channels), pin_layout_status);
    }

    // report automatic watering configuration
    let watering_schedule_config_status = WateringScheduleConfigStatus::new(
        Arc::clone(&watering_scheduler),
        Arc::clone(&mqtt_session),
    )
    .fuse();
    spawn_task(
        Arc::clone(&ctrl_c_channels),
        watering_schedule_config_status,
    );

    // report layout configuration
    let layout_config_status = LayoutConfigStatus::new(&layout_config, Arc::clone(&mqtt_session))
        .fuse()
        .map(|_| ());
    spawn_task(Arc::clone(&ctrl_c_channels), layout_config_status);

    report_online(Arc::clone(&mqtt_session));

    tokio::spawn(App::start(Arc::clone(&ctrl_c_channels)))
        .await
        .unwrap()
}

fn spawn_task(
    ctrl_c_channels: Arc<Mutex<Vec<(Sender<String>, Receiver<String>)>>>,
    mut task: impl Future<Output = ()> + Sized + Send + FusedFuture + Unpin + 'static,
) {
    let (s, r) = crossbeam::unbounded();
    ctrl_c_channels.lock().unwrap().push((s.clone(), r.clone()));
    tokio::task::spawn(async move {
        let mut receiver = ReceiverFuture::new(r.clone()).fuse();
        select! {
                         _ = task => {},
                        _ = receiver => {},
        }
    });
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
