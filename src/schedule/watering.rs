use std::collections::HashMap;
use std::ops::Add;
use std::str::FromStr;
use std::time::Duration;

use chrono::Local;
use cron::Schedule;
use futures::Future;
use tokio_channel::oneshot::{channel, Sender};
use tokio_timer::clock::now;
use tokio_timer::Delay;

use embedded::PinLayout;
use schedule::settings::WateringScheduleConfig;

pub struct WateringScheduler {
    // key = valve pin number, value = sender that shuts off the schedule
    senders: HashMap<u8, Sender<()>>,
    layout: PinLayout,
}

impl WateringScheduler {
    pub fn new(layout: PinLayout) -> WateringScheduler {
        let senders = HashMap::new();
        WateringScheduler { senders, layout }
    }

    pub fn delete_schedule(&mut self, valve_pin: &u8) -> Result<(), ()> {
        let sender = self.senders.remove(valve_pin);
        match sender {
            None => Err(()),
            Some(s) => {
                s.send(())?;
                Ok(())
            }
        }
    }

    pub fn create_schedule(
        &mut self,
        schedule_config: WateringScheduleConfig,
    ) -> Result<impl Future<Item = (), Error = ()> + Send, ()> {
        let valve_pin = schedule_config.get_valve();
        let (sender, receiver) = channel::<()>();
        self.senders.insert(valve_pin, sender);

        println!("Creating new schedule for valve pin num {}.", valve_pin);
        let cron_expression = schedule_config.get_schedule().get_chron_expression();
        let cron = Schedule::from_str(cron_expression).map_err(|err| println!("{:?}", err))?;
        match cron.upcoming(Local).next() {
            Some(next_date) => {
                let next_millis = next_date.timestamp_millis() - Local::now().timestamp_millis();
                let next_instant = now().add(Duration::from_millis(next_millis as u64));
                let turn_on = Delay::new(next_instant)
                    .and_then(move |_| {
                    println!("{}: Turning on valve {}.", Local::now().format("%Y-%m-%d][%H:%M:%S"), valve_pin);
                    Ok(())
                });
                let turn_off = Delay::new(
                    next_instant.add(Duration::from_secs(schedule_config.get_schedule().get_duration().clone()))
                )
                .and_then(move |_| {
                    println!("{}: Turning off valve {}.", Local::now().format("%Y-%m-%d][%H:%M:%S"), valve_pin);
                    Ok(())
                })
                .map_err(|_| ());
                let abort_schedule = receiver;
                Ok(turn_on
                    .select2(abort_schedule)
                    .map(|_| ())
                    .map_err(|_| ())
                    .join(turn_off)
                    .map(|_| ())
                    .map_err(|_| ()))
            }
            None => Err(()),
        }
    }
}
