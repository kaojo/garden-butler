use core::time::Duration;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use chrono::Local;
use futures::prelude::*;
use rumqtt::QoS;
use tokio::sync::mpsc;

use crate::embedded::configuration::LayoutConfig;
use crate::embedded::{LayoutStatus, PinLayout, ToggleValve};
use crate::mqtt::configuration::MqttConfig;
use crate::mqtt::MqttSession;
use crate::schedule::WateringScheduleConfigs;

pub struct PinLayoutStatus {}

impl PinLayoutStatus {
    pub async fn report<T, U>(
        layout: Arc<Mutex<T>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
        mqtt_config: MqttConfig,
        send_layout_status_receiver: mpsc::Receiver<Result<(), ()>>,
    ) -> ()
    where
        T: PinLayout<U> + Send + 'static,
        U: ToggleValve + Send + 'static,
    {
        let interval = tokio::time::interval(Duration::from_secs(
            mqtt_config.status_publish_interval_secs.unwrap_or(60),
        ))
        .map(|_| ());
        let mut interval_or_receiver =
            stream::select(interval, send_layout_status_receiver.map(|_| ()));

        while let Some(_) = interval_or_receiver.next().await {
            let status = PinLayoutStatus::get_current_layout_status(&layout);
            PinLayoutStatus::log_status(&status);
            PinLayoutStatus::publish_status(&mqtt_session, &mqtt_config, &status);
        }
    }

    fn get_current_layout_status<T, U>(layout: &Arc<Mutex<T>>) -> LayoutStatus
    where
        T: PinLayout<U> + Send + 'static,
        U: ToggleValve + Send + 'static,
    {
        layout.lock().unwrap().get_layout_status()
    }

    fn log_status(status: &LayoutStatus) {
        println!(
            "{}: {:?}",
            Local::now().format("%Y-%m-%d][%H:%M:%S"),
            status
        )
    }

    fn publish_status(
        mqtt_session: &Arc<Mutex<MqttSession>>,
        mqtt_config: &MqttConfig,
        status: &LayoutStatus,
    ) -> () {
        let topic = format!("{}/garden-butler/status/layout", mqtt_config.client_id);
        let message = serde_json::to_string(&status).unwrap();
        match mqtt_session
            .lock()
            .unwrap()
            .publish(topic, QoS::AtMostOnce, true, message)
        {
            Ok(_) => {}
            Err(e) => println!("mqtt publish error = {:?}", e),
        }
    }
}

pub struct WateringScheduleConfigStatus {}

impl WateringScheduleConfigStatus {
    pub async fn report(
        watering_schedule_configs: Arc<Mutex<WateringScheduleConfigs>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
    ) -> () {
        let mut session = mqtt_session.lock().unwrap();
        let topic = format!(
            "{}/garden-butler/status/watering-schedule",
            session.get_client_id()
        );
        let message =
            serde_json::to_string(watering_schedule_configs.lock().unwrap().deref()).unwrap();
        session
            .publish(topic, QoS::ExactlyOnce, true, message)
            .map(|_| ())
            .map_err(|e| println!("error = {:?}", e))
            .unwrap_or_default()
    }
}

pub struct LayoutConfigStatus {}

impl LayoutConfigStatus {
    pub async fn report(
        layout: Arc<Mutex<LayoutConfig>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
    ) -> () {
        let mut session = mqtt_session.lock().unwrap();
        let topic = format!(
            "{}/garden-butler/status/layout-config",
            session.get_client_id()
        );
        let guard = layout.lock().unwrap();
        let message = serde_json::to_string(guard.deref()).unwrap();
        session
            .publish(topic, QoS::ExactlyOnce, true, message)
            .map(|_| ())
            .map_err(|e| println!("error = {:?}", e))
            .unwrap_or_default()
    }
}
