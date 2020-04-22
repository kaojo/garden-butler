use std::collections::HashMap;

use chrono::NaiveTime;
use crossbeam::{Receiver, Sender};
use futures::prelude::*;

use crate::communication::{create_abortable_task, ReceiverFuture};
use crate::embedded::command::LayoutCommand;
use crate::embedded::ValvePinNumber;
use crate::schedule::configuration::WateringScheduleConfigs;
use crate::schedule::watering_task::WateringTask;
use crate::schedule::ScheduleConfig;

use std::sync::{Arc, Mutex};

pub struct WateringScheduler {
    configs: WateringScheduleConfigs,
    senders: Arc<Mutex<HashMap<ValvePinNumber, crossbeam::Sender<()>>>>,
    command_sender: Sender<LayoutCommand>,
}

impl WateringScheduler {
    pub fn new(
        configs: WateringScheduleConfigs,
        command_sender: Sender<LayoutCommand>,
    ) -> WateringScheduler {
        let senders = Arc::new(Mutex::new(HashMap::new()));
        WateringScheduler {
            senders,
            command_sender,
            configs,
        }
    }

    /// TODO test method
    pub fn delete_schedule(&mut self, valve_pin: &ValvePinNumber) -> Result<(), ()> {
        let sender = self.senders.lock().unwrap().remove(valve_pin);
        match sender {
            None => Err(()),
            Some(s) => {
                s.try_send(()).map_err(|e| println!("error = {:?}", e))?;
                Ok(())
            }
        }
    }

    pub fn get_config(&self) -> &WateringScheduleConfigs {
        &self.configs
    }

    pub fn start(&mut self) -> Vec<(Sender<String>, Receiver<String>)> {
        let mut ctrl_c_channels = Vec::new();
        for schedule in self.configs.get_schedules().iter() {
            if schedule.is_enabled() {
                println!(
                    "Creating watering schedule for valve {}",
                    schedule.get_valve()
                );
                let schedule_task = create_schedule(
                    self.senders.clone(),
                    self.command_sender.clone(),
                    schedule.get_valve().clone(),
                    schedule.get_schedule().clone(),
                )
                .boxed()
                .fuse();

                let (s, r) = crossbeam::unbounded();
                ctrl_c_channels.push((s.clone(), r.clone()));

                tokio::task::spawn(create_abortable_task(schedule_task, r));
            }
        }
        ctrl_c_channels
    }
}

async fn create_schedule(
    senders: Arc<Mutex<HashMap<ValvePinNumber, crossbeam::Sender<()>>>>,
    command_sender: Sender<LayoutCommand>,
    valve_pin_num: u8,
    schedule_config: ScheduleConfig,
) -> () {
    let number = ValvePinNumber(valve_pin_num);

    let start_time: NaiveTime = get_schedule_start_time(&schedule_config);
    let end_time = get_schedule_end_time(&schedule_config);

    let mut start_task = WateringTask::new(
        LayoutCommand::Open(number),
        start_time,
        command_sender.clone(),
    )
    .fuse();
    let mut end_task = WateringTask::new(
        LayoutCommand::Close(number),
        end_time,
        command_sender.clone(),
    )
    .fuse();

    let (sender, receiver) = crossbeam::unbounded();
    senders.lock().unwrap().insert(number, sender);
    let mut shut_off = ReceiverFuture::new(receiver).fuse();
    let task = select! {
        _ = start_task => {},
        _ = end_task => {},
        _ = shut_off => {}, // TODO test shutoffsenders
    };
    task
}

fn get_schedule_start_time(config: &ScheduleConfig) -> NaiveTime {
    NaiveTime::from_hms(
        *config.get_start_hour() as u32,
        *config.get_start_minute() as u32,
        0,
    )
}

fn get_schedule_end_time(config: &ScheduleConfig) -> NaiveTime {
    NaiveTime::from_hms(
        *config.get_end_hour() as u32,
        *config.get_end_minute() as u32,
        0,
    )
}
