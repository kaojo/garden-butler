use std::marker::PhantomData;
#[cfg(feature = "gpio")]
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use futures::future::FusedFuture;
use futures::prelude::*;
use rumqtt::QoS;
use tokio::sync::{mpsc, watch};

use crate::communication::create_abortable_task;
use crate::embedded::command::{LayoutCommand, LayoutCommandListener};
use crate::embedded::configuration::LayoutConfig;
#[cfg(feature = "gpio")]
use crate::embedded::gpio::{GpioPinLayout, GpioToggleValve};
use crate::embedded::{PinLayout, ToggleValve};
use crate::mqtt::command::MqttCommandListener;
use crate::mqtt::configuration::MqttConfig;
use crate::mqtt::status::{LayoutConfigStatus, PinLayoutStatus, WateringScheduleConfigStatus};
use crate::mqtt::MqttSession;
use crate::schedule::{WateringScheduleConfigs, WateringScheduler};

pub struct App<T, U>
where
    T: PinLayout<U> + Send + 'static,
    U: ToggleValve + Send + 'static,
{
    ctrl_c_sender: watch::Sender<String>,
    ctrl_c_receiver: watch::Receiver<String>,

    layout_command_sender: Option<mpsc::Sender<LayoutCommand>>,
    layout_status_send_sender: Option<mpsc::Sender<Result<(), ()>>>,

    layout_config: Arc<Mutex<LayoutConfig>>,
    layout: Arc<Mutex<T>>,
    valve_type: PhantomData<U>,

    mqtt_config: MqttConfig,
    mqtt_session: Arc<Mutex<MqttSession>>,

    watering_schedule_config: Arc<Mutex<WateringScheduleConfigs>>,
}

#[cfg(feature = "gpio")]
impl App<GpioPinLayout, GpioToggleValve> {
    pub fn listen_to_button_presses(&self) -> () {
        let guard = self.layout.lock().unwrap();
        guard
            .deref()
            .spawn_button_streams(self.ctrl_c_receiver.clone());
    }
}

impl<T, U> App<T, U>
where
    T: PinLayout<U> + Send + 'static,
    U: ToggleValve + Send + 'static,
{
    pub fn new(
        layout_config: Arc<Mutex<LayoutConfig>>,
        layout: Arc<Mutex<T>>,
        mqtt_config: MqttConfig,
        mqtt_session: Arc<Mutex<MqttSession>>,
        watering_schedule_config: WateringScheduleConfigs,
        valve_type: PhantomData<U>,
    ) -> Self
    where
        T: PinLayout<U> + Send + 'static,
        U: ToggleValve + Send + 'static,
    {
        let (ctrl_c_sender, mut ctrl_c_receiver) = watch::channel("hello".to_string());
        let _ = async {
            ctrl_c_receiver.recv().await;
        }; // empty channel

        App {
            ctrl_c_sender,
            ctrl_c_receiver,
            layout_command_sender: None,
            layout_status_send_sender: None,

            layout_config,
            layout,
            valve_type,

            mqtt_config,
            mqtt_session,

            watering_schedule_config: Arc::new(Mutex::new(watering_schedule_config)),
        }
    }

    pub fn report_layout_config(&self) -> () {
        let layout_config_status = LayoutConfigStatus::report(
            Arc::clone(&self.layout_config),
            Arc::clone(&self.mqtt_session),
        )
        .boxed()
        .fuse();
        spawn_task(self.ctrl_c_receiver.clone(), layout_config_status);
    }

    pub fn report_pin_layout_status(&mut self) -> () {
        let (layout_status_send_sender, layout_status_send_receiver): (
            mpsc::Sender<Result<(), ()>>,
            mpsc::Receiver<Result<(), ()>>,
        ) = tokio::sync::mpsc::channel(16);

        self.layout_status_send_sender = Some(layout_status_send_sender);

        let pin_layout_status = PinLayoutStatus::report(
            Arc::clone(&self.layout),
            Arc::clone(&self.mqtt_session),
            self.mqtt_config.clone(),
            layout_status_send_receiver,
        )
        .boxed()
        .fuse()
        .map(|_| ());
        spawn_task(self.ctrl_c_receiver.clone(), pin_layout_status);
    }

    pub fn report_watering_configuration(&mut self) -> () {
        let (watering_configuration_sender, watering_configuration_receiver): (
            mpsc::Sender<()>,
            mpsc::Receiver<()>,
        ) = tokio::sync::mpsc::channel(16);

        let watering_schedule_config_status = WateringScheduleConfigStatus::report(
            Arc::clone(&self.watering_schedule_config),
            Arc::clone(&self.mqtt_session),
        )
        .boxed()
        .fuse();
        spawn_task(
            self.ctrl_c_receiver.clone(),
            watering_schedule_config_status,
        );
    }

    pub fn listen_to_layout_commands(&mut self) -> () {
        let (layout_command_sender, layout_command_receiver): (
            mpsc::Sender<LayoutCommand>,
            mpsc::Receiver<LayoutCommand>,
        ) = tokio::sync::mpsc::channel(16);

        self.layout_command_sender = Some(layout_command_sender.clone());

        if let Some(layout_status_tx) = &self.layout_status_send_sender {
            let layout_command_listener = LayoutCommandListener::new(
                Arc::clone(&self.layout),
                layout_command_receiver,
                layout_status_tx.clone(),
            )
            .fuse();

            spawn_task(self.ctrl_c_receiver.clone(), layout_command_listener);
        }
    }

    pub fn report_online(&self) {
        let mut session = self.mqtt_session.lock().unwrap();
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

    pub fn listen_to_mqtt_commands(&self) -> () {
        if let Some(layout_command_tx) = &self.layout_command_sender {
            let mqtt_command_listener =
                MqttCommandListener::new(Arc::clone(&self.mqtt_session), layout_command_tx.clone())
                    .fuse();
            spawn_task(self.ctrl_c_receiver.clone(), mqtt_command_listener);
        }
    }

    pub fn start_watering_schedules(&self) -> () {
        //spawn preconfigured automatic watering tasks
        if let Some(layout_command_tx) = &self.layout_command_sender {
            let mut scheduler = WateringScheduler::new(
                Arc::clone(&self.watering_schedule_config),
                layout_command_tx.clone(),
            );
            scheduler.start(self.ctrl_c_receiver.clone());
        }
    }

    pub async fn wait_for_termination(self) -> Result<(), ()> {
        // listen for program termination
        tokio::signal::ctrl_c()
            .map_err(|e| println!("ctrlc-error = {:?}", e))
            .await?;

        // send shut off commands to running tasks
        self.ctrl_c_sender
            .broadcast("ctrl-c received!".to_string())
            .map_err(|_| {})
    }
}

fn spawn_task(
    ctrl_c_receiver: watch::Receiver<String>,
    task: impl Future<Output = ()> + Sized + Send + FusedFuture + Unpin + 'static,
) {
    let task1 = create_abortable_task(task, ctrl_c_receiver);
    tokio::task::spawn(task1);
}
