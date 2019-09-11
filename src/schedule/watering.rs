use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Add;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use cron::Schedule;
use crossbeam::Sender;
use futures::{Future, Stream};
use tokio_chrono::CronInterval;
use tokio_timer::clock::now;
use tokio_timer::Delay;

use communication::CancelReceiverFuture;
use embedded::{PinLayout, ToggleValve, ValvePinNumber};
use embedded::command::LayoutCommand;
use schedule::configuration::{WateringScheduleConfig, WateringScheduleConfigs};

pub struct WateringScheduler {
    configs: WateringScheduleConfigs,
    senders: HashMap<ValvePinNumber, crossbeam::Sender<()>>,
    command_sender: Sender<LayoutCommand>,
    pub enabled: bool,
}

impl WateringScheduler {
    pub fn new(configs: WateringScheduleConfigs, command_sender: Sender<LayoutCommand>) -> WateringScheduler {
        let senders = HashMap::new();
        let enabled = configs.enabled.unwrap_or(true);
        WateringScheduler {
            senders,
            command_sender,
            configs,
            enabled,
        }
    }

    /// TODO test method
    pub fn delete_schedule(&mut self, valve_pin: &ValvePinNumber) -> Result<(), ()> {
        let sender = self.senders.remove(valve_pin);
        match sender {
            None => Err(()),
            Some(s) => {
                s.send(()).map_err(|e| println!("error = {:?}", e))?;
                Ok(())
            }
        }
    }

    pub fn start(&mut self) -> Vec<impl Future<Item=(), Error=()> + Send> {
        let mut schedules = Vec::new();
        for config in self.configs.get_schedules().iter() {
            println!(
                "Creating watering schedule for valve {}",
                config.get_valve()
            );
            if let Ok(result) = create_schedule(&mut self.senders, &self.command_sender, config)
            {
                schedules.push(result);
            }
        }
        schedules
    }
}

fn create_schedule(
    senders: &mut HashMap<ValvePinNumber, crossbeam::Sender<()>>,
    command_sender: &Sender<LayoutCommand>,
    schedule_config: &WateringScheduleConfig,
) -> Result<impl Future<Item=(), Error=()> + Send, ()> {
    let valve_pin_num = schedule_config.get_valve();

    let (sender, receiver) = crossbeam::unbounded();
    senders.insert(ValvePinNumber(valve_pin_num), sender);

    let watering_duration = *schedule_config.get_schedule().get_duration_seconds();
    let cron_expression = schedule_config.get_schedule().get_cron_expression();
    let cron = Schedule::from_str(cron_expression).map_err(|err| println!("{:?}", err))?;
    let command_sender_clone = command_sender.clone();
    let task = CronInterval::new(cron)
        .map_err(|_| ())
        .for_each(move |_| {
            println!(
                "{}: Send open command for valve {}.",
                Local::now().format("%Y-%m-%d][%H:%M:%S"),
                valve_pin_num
            );
            let turn_on_send_result = command_sender_clone.send(LayoutCommand::Open(ValvePinNumber(valve_pin_num)));
            let command_sender_clone_clone = command_sender_clone.clone();
            let turn_off = Delay::new(now().add(Duration::from_secs(watering_duration)))
                .map_err(|e| println!("turn off stream error = {:?}", e))
                .and_then(move |_| {
                    println!(
                        "{}: Send close command for valve {}.",
                        Local::now().format("%Y-%m-%d][%H:%M:%S"),
                        valve_pin_num
                    );
                    command_sender_clone_clone.send(LayoutCommand::Close(ValvePinNumber(valve_pin_num)))
                        .map_err(|e| println!("error = {}", e))
                });
            match turn_on_send_result {
                Ok(_) => {
                    tokio::spawn(turn_off);
                }
                Err(e) => {
                    return {
                        println!("error = {}", e);
                        Err(())
                    };
                }
            }
            Ok(())
        })
        .select2(CancelReceiverFuture::new(receiver)) // TODO test shutoff
        .map(|_| ())
        .map_err(|_| ());
    Ok(task)
}
