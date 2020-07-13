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
use tokio::time::Interval;

pub struct PinLayoutStatus {}

impl PinLayoutStatus {
    pub async fn report<T, U>(
        layout: Arc<Mutex<T>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
        mqtt_config: Arc<Mutex<MqttConfig>>,
        report_status_rx: mpsc::Receiver<()>,
    ) where
        T: PinLayout<U> + Send + 'static,
        U: ToggleValve + Send + 'static,
    {
        let interval = get_publish_interval(&mqtt_config).map(|_| ());
        let mut interval_or_receiver = stream::select(interval, report_status_rx.map(|_| ()));

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
        mqtt_config: &Arc<Mutex<MqttConfig>>,
        status: &LayoutStatus,
    ) {
        let topic = format!(
            "{}/garden-butler/status/layout",
            mqtt_config.lock().unwrap().client_id
        );
        let message = serde_json::to_string(&status).unwrap();
        match mqtt_session
            .lock()
            .unwrap()
            .publish(topic, QoS::AtMostOnce, true, message)
        {
            Ok(_) => println!("layout status published"),
            Err(e) => println!("mqtt publish error = {:?}", e),
        }
    }
}

pub struct WateringScheduleConfigStatus {}

impl WateringScheduleConfigStatus {
    pub async fn report(
        watering_schedule_configs: Arc<Mutex<WateringScheduleConfigs>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
        mqtt_config: Arc<Mutex<MqttConfig>>,
        report_status_rx: mpsc::Receiver<()>,
    ) {
        let interval = get_publish_interval(&mqtt_config).map(|_| ());
        let mut interval_or_receiver = stream::select(interval, report_status_rx.map(|_| ()));

        while let Some(_) = interval_or_receiver.next().await {
            let guard = watering_schedule_configs.lock().unwrap();
            WateringScheduleConfigStatus::publish_status(&mqtt_session, &mqtt_config, guard.deref())
        }
    }

    fn publish_status(
        mqtt_session: &Arc<Mutex<MqttSession>>,
        mqtt_config: &Arc<Mutex<MqttConfig>>,
        status: &WateringScheduleConfigs,
    ) {
        let topic = format!(
            "{}/garden-butler/status/watering-schedule",
            mqtt_config.lock().unwrap().client_id
        );
        let message = serde_json::to_string(status).unwrap();

        let mut session = mqtt_session.lock().unwrap();
        session
            .publish(topic, QoS::ExactlyOnce, true, message)
            .map(|_| println!("watering configuration published"))
            .map_err(|e| println!("error = {:?}", e))
            .unwrap_or_default()
    }
}

pub struct LayoutConfigStatus {}

impl LayoutConfigStatus {
    pub async fn report(layout: Arc<Mutex<LayoutConfig>>, mqtt_session: Arc<Mutex<MqttSession>>) {
        let mut session = mqtt_session.lock().unwrap();
        let topic = format!(
            "{}/garden-butler/status/layout-config",
            session.get_client_id()
        );
        let guard = layout.lock().unwrap();
        let message = serde_json::to_string(guard.deref()).unwrap();
        session
            .publish(topic, QoS::ExactlyOnce, true, message)
            .map(|_| println!("layout configuration published"))
            .map_err(|e| println!("error = {:?}", e))
            .unwrap_or_default()
    }
}

fn get_publish_interval(mqtt_config: &Arc<Mutex<MqttConfig>>) -> Interval {
    tokio::time::interval(Duration::from_secs(
        mqtt_config
            .lock()
            .unwrap()
            .status_publish_interval_secs
            .unwrap_or(60),
    ))
}
