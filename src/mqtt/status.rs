use core::time::Duration;
use std::sync::{Arc, Mutex};

use chrono::Local;
use futures::{Async, Future, Stream};
use rumqtt::QoS;
use tokio_timer::Interval;

use communication::ReceiverStream;
use embedded::{PinLayout, ToggleValve};
use embedded::configuration::LayoutConfig;
use mqtt::configuration::MqttConfig;
use mqtt::MqttSession;
use schedule::WateringScheduler;

pub struct PinLayoutStatus {
    inner: Box<Future<Item=(), Error=()> + Send>,
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
        let interval = Interval::new_interval(
            Duration::from_secs(mqtt_config.status_publish_interval_secs.unwrap_or(60)),
        );
        let inner = Box::new(
            interval
                .map(|_| ()).map_err(|_| ())
                .select(ReceiverStream::new(send_layout_status_receiver))
                .map(move |_| layout.lock().unwrap().get_layout_status())
                .inspect(|status| println!("{}: {:?}", Local::now().format("%Y-%m-%d][%H:%M:%S"), status))
                .fold(mqtt_session, move |mqtt_session, status| {
                    let topic = format!("{}/garden-butler/status/layout", mqtt_config.client_id);
                    let message = serde_json::to_string(&status).unwrap();
                    match mqtt_session.lock().unwrap().publish(topic, QoS::AtMostOnce, true, message) {
                        Ok(_) => {}
                        Err(e) => { println!("mqtt publish error = {:?}", e) }
                    }
                    Ok(mqtt_session)
                })
                .map(|_| ())
                .map_err(|e| println!("error = {:?}", e))
        );

        PinLayoutStatus { inner }
    }
}

impl Future for PinLayoutStatus {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }
}

pub struct WateringScheduleConfigStatus {
    inner: Box<Future<Item=(), Error=()> + Send>,
}

impl WateringScheduleConfigStatus {
    pub fn new(
        watering_scheduler: Arc<Mutex<WateringScheduler>>,
        mqtt_session: Arc<Mutex<MqttSession>>) -> Self {
        let inner = Box::new(
            {
                let mut session = mqtt_session.lock().unwrap();
                let topic = format!("{}/garden-butler/status/watering-schedule", session.get_client_id());
                let message = serde_json::to_string(watering_scheduler.lock().unwrap().get_config()).unwrap();
                match session.publish(topic, QoS::ExactlyOnce, true, message) {
                    Ok(_) => { futures::future::ok(()) }
                    Err(e) => {
                        println!("error = {:?}", e);
                        futures::future::err(())
                    }
                }
            }
        );

        WateringScheduleConfigStatus { inner }
    }
}

impl Future for WateringScheduleConfigStatus {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }
}


pub struct LayoutConfigStatus {
    inner: Box<Future<Item=(), Error=()> + Send>,
}


impl LayoutConfigStatus {
    pub fn new(
        layout: &LayoutConfig,
        mqtt_session: Arc<Mutex<MqttSession>>) -> Self {
        let inner = Box::new(
            {
                let mut session = mqtt_session.lock().unwrap();
                let topic = format!("{}/garden-butler/status/layout-config", session.get_client_id());
                let message = serde_json::to_string(layout).unwrap();
                match session.publish(topic, QoS::ExactlyOnce, true, message) {
                    Ok(_) => { futures::future::ok(()) }
                    Err(e) => {
                        println!("error = {:?}", e);
                        futures::future::err(())
                    }
                }
            }
        );

        LayoutConfigStatus { inner }
    }
}

impl Future for LayoutConfigStatus {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }
}
