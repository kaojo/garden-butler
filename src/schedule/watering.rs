use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Add;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use cron::Schedule;
use futures::{Future, Stream};
use tokio::runtime::Runtime;
use tokio_channel::oneshot::{channel, Sender};
use tokio_chrono::CronInterval;
use tokio_timer::clock::now;
use tokio_timer::Delay;

use embedded::{PinLayout, ToggleValve, ValvePinNumber};
use schedule::configuration::{WateringScheduleConfig, WateringScheduleConfigs};

pub struct WateringScheduler<T, U> where T: PinLayout<U> + 'static, U: ToggleValve + Send + 'static {
    configs: WateringScheduleConfigs,
    senders: HashMap<ValvePinNumber, Sender<()>>,
    layout: Arc<Mutex<T>>,
    toggle_valve_type: PhantomData<U>,
    pub enabled: bool,
}

impl<T, U> WateringScheduler<T, U> where T: PinLayout<U> + 'static, U: ToggleValve + Send + 'static {
    pub fn new(
        configs: WateringScheduleConfigs,
        layout: Arc<Mutex<T>>,
    ) -> WateringScheduler<T, U> {
        let senders = HashMap::new();
        let enabled = configs.enabled.unwrap_or(true);
        let toggle_valve_type = PhantomData;
        WateringScheduler {
            senders,
            layout,
            configs,
            toggle_valve_type,
            enabled,
        }
    }

    /// TODO test method
    pub fn delete_schedule(&mut self, valve_pin: &ValvePinNumber) -> Result<(), ()> {
        let sender = self.senders.remove(valve_pin);
        match sender {
            None => Err(()),
            Some(s) => {
                s.send(())?;
                Ok(())
            }
        }
    }

    pub fn start(&mut self, runtime: &mut Runtime) -> Result<(), ()> {
        for config in self.configs.get_schedules().iter() {
            println!("Creating watering schedule for valve {}", config.get_valve());
            runtime.spawn(create_schedule(
                &mut self.senders,
                Arc::clone(&self.layout),
                config,
            )?);
        }
        Ok(())
    }
}

fn create_schedule<T, U>(
    senders: &mut HashMap<ValvePinNumber, Sender<()>>,
    layout: Arc<Mutex<T>>,
    schedule_config: &WateringScheduleConfig,
) -> Result<impl Future<Item=(), Error=()> + Send, ()>
    where T: PinLayout<U> + 'static, U: ToggleValve + Send + 'static
{
    let valve_pin_num = schedule_config.get_valve();

    let toggle_valve = Arc::clone(layout.lock().unwrap().find_pin(ValvePinNumber(valve_pin_num))?);
    println!("Creating new schedule for valve pin num {}.", toggle_valve.lock().unwrap().get_valve_pin_num().0);

    let (sender, receiver) = channel::<()>();
    senders.insert(ValvePinNumber(valve_pin_num), sender);

    let watering_duration = *schedule_config
        .get_schedule()
        .get_duration_seconds();
    let cron_expression = schedule_config.get_schedule().get_cron_expression();
    let cron = Schedule::from_str(cron_expression).map_err(|err| println!("{:?}", err))?;
    let task = CronInterval::new(cron)
        .map_err(|_| ())
        .for_each(move |_| {
            println!(
                "{}: Turning on valve {}.",
                Local::now().format("%Y-%m-%d][%H:%M:%S"),
                valve_pin_num
            );
            toggle_valve.lock().unwrap().turn_on().map_err(|_| ())?;
            let clone = Arc::clone(&toggle_valve);
            let turn_off = Delay::new(now().add(Duration::from_secs(watering_duration)))
                .map_err(|_| ())
                .and_then(move |_| {
                    println!(
                        "{}: Turning off valve {}.",
                        Local::now().format("%Y-%m-%d][%H:%M:%S"),
                        valve_pin_num
                    );
                    clone.lock().unwrap().turn_off().map_err(|_| ())?;
                    Ok(())
                });
            tokio::spawn(turn_off);
            Ok(())
        })
        .select2(receiver) // TODO test shutoff
        .map(|_| ())
        .map_err(|_| ());
    Ok(task)
}
