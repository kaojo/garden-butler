use std::collections::HashMap;
use std::ops::Add;
use std::str::FromStr;
use std::time::Duration;

use chrono::Local;
use cron::Schedule;
use futures::{Future, Stream};
use tokio::runtime::Runtime;
use tokio_channel::oneshot::{channel, Sender};
use tokio_chrono::CronInterval;
use tokio_timer::clock::now;
use tokio_timer::Delay;

use embedded::{PinLayout, ToggleValve};
use schedule::settings::{WateringScheduleConfig, WateringScheduleConfigs};

pub struct WateringScheduler {
    // key = valve pin number, value = sender that shuts off the schedule
    configs: WateringScheduleConfigs,
    senders: HashMap<u64, Sender<()>>,
    layout: PinLayout,
    pub enabled: bool
}

impl WateringScheduler {
    pub fn new(configs: WateringScheduleConfigs, layout: PinLayout) -> WateringScheduler {
        let senders = HashMap::new();
        let enabled = configs.enabled.unwrap_or(true);
        WateringScheduler { senders, layout, configs, enabled }
    }

    /// TODO test method
    pub fn delete_schedule(&mut self, valve_pin: &u64) -> Result<(), ()> {
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
            runtime.spawn(create_schedule(&mut self.senders, &self.layout, config)?);
        }
        Ok(())
    }
}

fn find_pin(valve_pin_num: u64, layout: &PinLayout) -> Result<&ToggleValve, ()> {
    match layout.get_valve_pins().iter().find(|ref valve_pin| valve_pin_num == valve_pin.get_valve_pin().get_pin()) {
        None => Err(()),
        Some(pin) => Ok(pin),
    }
}

fn create_schedule(senders: &mut HashMap<u64, Sender<()>>, layout: &PinLayout, schedule_config: &WateringScheduleConfig) -> Result<impl Future<Item=(), Error=()> + Send, ()> {
    let valve_pin_num = schedule_config.get_valve();
    println!("Creating new schedule for valve pin num {}.", valve_pin_num);

    let toggle_valve = find_pin(valve_pin_num, layout)?.clone();

    let (sender, receiver) = channel::<()>();
    senders.insert(valve_pin_num, sender);

    let watering_duration = schedule_config.get_schedule().get_duration_seconds().clone();
    let cron_expression = schedule_config.get_schedule().get_cron_expression();
    let cron = Schedule::from_str(cron_expression).map_err(|err| println!("{:?}", err))?;
    let task = CronInterval::new(cron)
        .map_err(|_| ())
        .for_each(move |_| {
            println!("{}: Turning on valve {}.", Local::now().format("%Y-%m-%d][%H:%M:%S"), valve_pin_num);
            toggle_valve.turn_on().map_err(|_| ())?;
            let clone = toggle_valve.clone();
            let turn_off = Delay::new(now().add(Duration::from_secs(watering_duration)))
                .map_err(|_| ())
                .and_then(move |_| {
                    println!("{}: Turning off valve {}.", Local::now().format("%Y-%m-%d][%H:%M:%S"), valve_pin_num);
                    clone.turn_off().map_err(|_| ())?;
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
