use core::time::Duration;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::Local;
use futures::{Async, Future, Stream};
use rumqtt::QoS;
use tokio_timer::Interval;

use embedded::{PinLayout, ToggleValve};
use mqtt::configuration::MqttConfig;
use mqtt::MqttSession;

pub struct PinLayoutStatus {
    inner: Box<Future<Item=(), Error=()> + Send>,
}

impl PinLayoutStatus {
    pub fn new<T, U>(
        layout: Arc<Mutex<T>>,
        mqtt_session: Arc<Mutex<MqttSession>>,
        mqtt_config: MqttConfig) -> Self
        where
            T: PinLayout<U> + Send + 'static,
            U: ToggleValve + Send + 'static,
    {
        let interval = Interval::new(
            Instant::now().add(Duration::from_secs(5)),
            Duration::from_secs(mqtt_config.status_publish_interval_secs.unwrap_or(60)),
        );
        let inner = Box::new(interval
            .map(move |_| layout.lock().unwrap().get_layout_status())
            .inspect(|status| println!("{}: {:?}", Local::now().format("%Y-%m-%d][%H:%M:%S"), status))
            .fold(mqtt_session, move |mut mqtt_session, status| {
                let topic = format!("{}/garden-butler/layout/status", mqtt_config.client_id);
                let message = serde_json::to_string(&status).unwrap();
                mqtt_session.lock().unwrap().publish(topic, QoS::AtMostOnce, true, message);
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
