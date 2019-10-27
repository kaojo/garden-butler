use std::collections::HashMap;
use std::time::Duration;

use chrono::{Local, NaiveTime, Utc, Timelike};
use crossbeam::Sender;
use futures::{Future, Stream};
use tokio_timer::{Interval};

use communication::ReceiverFuture;
use embedded::command::LayoutCommand;
use embedded::ValvePinNumber;
use schedule::configuration::{WateringScheduleConfig, WateringScheduleConfigs};
use schedule::ScheduleConfig;

pub struct WateringScheduler {
    configs: WateringScheduleConfigs,
    senders: HashMap<ValvePinNumber, crossbeam::Sender<()>>,
    command_sender: Sender<LayoutCommand>,
}

impl WateringScheduler {
    pub fn new(configs: WateringScheduleConfigs, command_sender: Sender<LayoutCommand>) -> WateringScheduler {
        let senders = HashMap::new();
        WateringScheduler {
            senders,
            command_sender,
            configs,
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

    pub fn get_config(&self) -> &WateringScheduleConfigs {
        &self.configs
    }

    pub fn start(&mut self) -> Vec<impl Future<Item=(), Error=()> + Send> {
        let mut schedules = Vec::new();
        for config in self.configs.get_schedules().iter() {
            if config.is_enabled() {
                println!(
                    "Creating watering schedule for valve {}",
                    config.get_valve()
                );
                if let Ok(result) = create_schedule(&mut self.senders, &self.command_sender, config)
                {
                    schedules.push(result);
                }
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

    let start_time: NaiveTime = get_schedule_start_time(schedule_config.get_schedule());
    let end_time = get_schedule_end_time(schedule_config.get_schedule());
    let command_sender_clone = command_sender.clone();
    let task = Interval::new_interval(Duration::from_secs(1))
        .filter(move|_| {
            let now = Utc::now().time();
            let truncated_now = NaiveTime::from_hms(now.hour(), now.minute(), now.second());
            return truncated_now.eq(&start_time);
        })
        .map_err(|_| ())
        .for_each(move |_| {
            println!(
                "{}: Send open command for valve {}.",
                Local::now().format("%Y-%m-%d][%H:%M:%S"),
                valve_pin_num
            );
            let turn_on_send_result = command_sender_clone.send(LayoutCommand::Open(ValvePinNumber(valve_pin_num)));
            let command_sender_clone_clone = command_sender_clone.clone();
            let turn_off = Interval::new_interval(Duration::from_secs(1))
                .map_err(|e| println!("error = {}", e))
                .filter(move |_| {
                    let now = Utc::now().time();
                    let truncated_now = NaiveTime::from_hms(now.hour(), now.minute(), now.second());
                    return truncated_now.eq(&end_time);
                })
                .take(1)
                .for_each(move |_| {
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
        .select2(ReceiverFuture::new(receiver)) // TODO test shutoffsenders
        .map(|_| ())
        .map_err(|_| ());
    Ok(task)
}

fn get_schedule_start_time(config: &ScheduleConfig) -> NaiveTime {
    NaiveTime::from_hms(*config.get_start_hour() as u32, *config.get_start_minute() as u32, 0)
}

fn get_schedule_end_time(config: &ScheduleConfig) -> NaiveTime {
    NaiveTime::from_hms(*config.get_end_hour() as u32, *config.get_end_minute() as u32, 0)
}
