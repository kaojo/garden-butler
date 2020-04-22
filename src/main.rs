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

use futures::future::FusedFuture;
use futures::{Future, FutureExt};
use rumqtt::QoS;
use serde::export::PhantomData;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::watch;

use app::App;
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

use crate::communication::create_abortable_task;

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

    let layout_config = Arc::new(Mutex::new(LayoutConfig::default()));
    let layout = create_pin_layout(Arc::clone(&layout_config), LAYOUT_TYPE);

    let (ctrl_c_sender, mut ctrl_c_receiver) = watch::channel("hello".to_string());
    let _ = ctrl_c_receiver.recv().await;
    // command listening
    let (layout_command_sender, layout_command_receiver): (
        Sender<LayoutCommand>,
        Receiver<LayoutCommand>,
    ) = tokio::sync::mpsc::channel(16);
    let (layout_status_send_sender, layout_status_send_receiver): (
        Sender<Result<(), ()>>,
        Receiver<Result<(), ()>>,
    ) = tokio::sync::mpsc::channel(16);

    let layout_command_listener = LayoutCommandListener::new(
        Arc::clone(&layout),
        layout_command_receiver,
        layout_status_send_sender.clone(),
    )
    .fuse();

    spawn_task(ctrl_c_receiver.clone(), layout_command_listener);

    // listen for physical button presses
    #[cfg(feature = "gpio")]
    {
        layout
            .lock()
            .unwrap()
            .spawn_button_streams(ctrl_c_receiver.clone());
    }

    let mqtt_config = MqttConfig::default();
    let mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(mqtt_config.clone());

    {
        let mqtt_command_listener =
            MqttCommandListener::new(Arc::clone(&mqtt_session), layout_command_sender.clone())
                .fuse();
        spawn_task(ctrl_c_receiver.clone(), mqtt_command_listener);
    }

    //spawn preconfigured automatic watering tasks
    let mut scheduler = WateringScheduler::new(
        WateringScheduleConfigs::default(),
        layout_command_sender.clone(),
    );
    scheduler.start(ctrl_c_receiver.clone());

    let watering_scheduler = Arc::new(Mutex::new(scheduler));

    // report layout status
    {
        let pin_layout_status = PinLayoutStatus::report(
            Arc::clone(&layout),
            Arc::clone(&mqtt_session),
            mqtt_config.clone(),
            layout_status_send_receiver,
        )
        .boxed()
        .fuse()
        .map(|_| ());
        spawn_task(ctrl_c_receiver.clone(), pin_layout_status);
    }

    // report automatic watering configuration
    let watering_schedule_config_status = WateringScheduleConfigStatus::report(
        Arc::clone(&watering_scheduler),
        Arc::clone(&mqtt_session),
    )
    .boxed()
    .fuse();
    spawn_task(ctrl_c_receiver.clone(), watering_schedule_config_status);

    // report layout configuration
    let layout_config_status = LayoutConfigStatus::report(layout_config, Arc::clone(&mqtt_session))
        .boxed()
        .fuse();
    spawn_task(ctrl_c_receiver.clone(), layout_config_status);

    report_online(Arc::clone(&mqtt_session));

    tokio::spawn(App::start(ctrl_c_sender)).await.unwrap()
}

fn spawn_task(
    ctrl_c_receiver: watch::Receiver<String>,
    task: impl Future<Output = ()> + Sized + Send + FusedFuture + Unpin + 'static,
) {
    let task1 = create_abortable_task(task, ctrl_c_receiver);
    tokio::task::spawn(task1);
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

fn create_pin_layout<T, U>(config: Arc<Mutex<LayoutConfig>>, _: PhantomData<T>) -> Arc<Mutex<T>>
where
    T: PinLayout<U> + 'static,
    U: ToggleValve + Send + 'static,
{
    Arc::new(Mutex::new(T::new(&config.lock().unwrap())))
}
