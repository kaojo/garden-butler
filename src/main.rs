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

use serde::export::PhantomData;

#[cfg(not(feature = "gpio"))]
use crate::embedded::fake::{FakePinLayout, FakeToggleValve};
#[cfg(feature = "gpio")]
use crate::embedded::gpio::{GpioPinLayout, GpioToggleValve};
use crate::embedded::{PinLayout, ToggleValve};
use app::App;
use embedded::configuration::LayoutConfig;
use mqtt::configuration::MqttConfig;
use mqtt::MqttSession;
use schedule::WateringScheduleConfigs;

mod app;
mod communication;
mod embedded;
mod mqtt;
mod schedule;

#[cfg(feature = "gpio")]
pub const LAYOUT_TYPE: PhantomData<GpioPinLayout> = PhantomData;
#[cfg(feature = "gpio")]
pub const VALVE_TYPE: PhantomData<GpioToggleValve> = PhantomData;
#[cfg(not(feature = "gpio"))]
pub const LAYOUT_TYPE: PhantomData<FakePinLayout> = PhantomData;
#[cfg(not(feature = "gpio"))]
pub const VALVE_TYPE: PhantomData<FakeToggleValve> = PhantomData;

#[tokio::main]
async fn main() -> Result<(), ()> {
    println!("Garden buttler starting ...");

    let layout_config: Arc<Mutex<LayoutConfig>> = Arc::new(Mutex::new(LayoutConfig::default()));
    let layout = create_pin_layout(Arc::clone(&layout_config), LAYOUT_TYPE);

    let mqtt_config = MqttConfig::default();
    let mqtt_session: Arc<Mutex<MqttSession>> = MqttSession::from_config(mqtt_config.clone());

    let watering_schedule_config: WateringScheduleConfigs = WateringScheduleConfigs::default();

    let mut app = App::new(
        layout_config,
        layout,
        mqtt_config,
        mqtt_session,
        watering_schedule_config,
        VALVE_TYPE,
    );

    app.report_layout_config();
    app.report_pin_layout_status();
    app.report_watering_configuration();

    app.listen_to_layout_commands();

    #[cfg(feature = "gpio")]
    {
        app.listen_to_button_presses();
    }

    app.listen_to_mqtt_commands();

    app.start_watering_schedules();

    app.report_online();

    tokio::spawn(app.wait_for_termination()).await.unwrap()
}

fn create_pin_layout<T, U>(config: Arc<Mutex<LayoutConfig>>, _: PhantomData<T>) -> Arc<Mutex<T>>
where
    T: PinLayout<U> + 'static,
    U: ToggleValve + Send + 'static,
{
    Arc::new(Mutex::new(T::new(&config.lock().unwrap())))
}
