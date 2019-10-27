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
use schedule::watering_task::WateringTask;

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
        for schedule in self.configs.get_schedules().iter() {
            if schedule.is_enabled() {
                println!(
                    "Creating watering schedule for valve {}",
                    schedule.get_valve()
                );
                if let Ok(result) = create_schedule(&mut self.senders, &self.command_sender, schedule)
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
    let number = ValvePinNumber(valve_pin_num);
    let schedule = schedule_config.get_schedule();

    let start_time: NaiveTime = get_schedule_start_time(schedule);
    let end_time = get_schedule_end_time(schedule);

    let start_task = WateringTask::new(LayoutCommand::Open(number), start_time, command_sender.clone());
    let end_task = WateringTask::new(LayoutCommand::Close(number), end_time, command_sender.clone());

    let (sender, receiver) = crossbeam::unbounded();
    senders.insert(number, sender);
    let task = start_task
        .join(end_task)
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
