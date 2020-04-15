use core::time::Duration;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use chrono::Local;
use futures::prelude::*;
use futures::task::{Context, Poll};
use rumqtt::QoS;

use crate::communication::ReceiverStream;
use crate::embedded::configuration::LayoutConfig;
use crate::embedded::{PinLayout, ToggleValve};
use crate::mqtt::configuration::MqttConfig;
use crate::mqtt::MqttSession;
use crate::schedule::WateringScheduler;
use futures::FutureExt;

pub struct PinLayoutStatus {
    inner: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl PinLayoutStatus {
    pub fn new<T, U>(
        layout: Arc<Mutex<T>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
        mqtt_config: MqttConfig,
        send_layout_status_receiver: crossbeam::Receiver<Result<(), ()>>,
    ) -> Self
    where
        T: PinLayout<U> + Send + 'static,
        U: ToggleValve + Send + 'static,
    {
        let interval = tokio::time::interval(Duration::from_secs(
            mqtt_config.status_publish_interval_secs.unwrap_or(60),
        ))
        .map(|_| ())
        .fuse();
        let receiver = ReceiverStream::new(send_layout_status_receiver).fuse();
        let interval_or_receiver = stream::select(interval, receiver);
        let inner = interval_or_receiver
            .map(move |_| layout.lock().unwrap().get_layout_status())
            .inspect(|status| {
                println!(
                    "{}: {:?}",
                    Local::now().format("%Y-%m-%d][%H:%M:%S"),
                    status
                )
            })
            .fold(mqtt_session, move |mqtt_session, status| {
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
                future::ready(mqtt_session)
            })
            .map(|_| ())
            .boxed();

        PinLayoutStatus { inner }
    }
}

impl Future for PinLayoutStatus {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}

pub struct WateringScheduleConfigStatus {
    inner: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl WateringScheduleConfigStatus {
    pub fn new(
        watering_scheduler: Arc<Mutex<WateringScheduler>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
    ) -> Self {
        let inner = {
            let mut session = mqtt_session.lock().unwrap();
            let topic = format!(
                "{}/garden-butler/status/watering-schedule",
                session.get_client_id()
            );
            let message =
                serde_json::to_string(watering_scheduler.lock().unwrap().get_config()).unwrap();
            match session.publish(topic, QoS::ExactlyOnce, true, message) {
                Ok(_) => futures::future::ready(()),
                Err(e) => {
                    println!("error = {:?}", e);
                    futures::future::ready(())
                }
            }
        }
        .boxed();

        WateringScheduleConfigStatus { inner }
    }
}

impl Future for WateringScheduleConfigStatus {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}

pub struct LayoutConfigStatus {
    inner: Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>,
}

impl LayoutConfigStatus {
    pub fn new(layout: &LayoutConfig, mqtt_session: Arc<Mutex<MqttSession>>) -> Self {
        let inner = {
            let mut session = mqtt_session.lock().unwrap();
            let topic = format!(
                "{}/garden-butler/status/layout-config",
                session.get_client_id()
            );
            let message = serde_json::to_string(layout).unwrap();
            match session.publish(topic, QoS::ExactlyOnce, true, message) {
                Ok(_) => futures::future::ok(()),
                Err(e) => {
                    println!("error = {:?}", e);
                    futures::future::err(())
                }
            }
        }
        .boxed();

        LayoutConfigStatus { inner }
    }
}

impl Future for LayoutConfigStatus {
    type Output = Result<(), ()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}
