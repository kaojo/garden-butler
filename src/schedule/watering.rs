use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::NaiveTime;
use futures::prelude::*;
use tokio::sync::mpsc::Sender;

use crate::embedded::command::LayoutCommand;
use crate::embedded::ValvePinNumber;
use crate::schedule::configuration::WateringScheduleConfigs;
use crate::schedule::watering_task::WateringTask;
use crate::schedule::ScheduleConfig;

pub struct WateringScheduler {
    configs: WateringScheduleConfigs,
    senders: Arc<Mutex<HashMap<ValvePinNumber, Sender<()>>>>,
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
            Some(mut s) => {
                s.try_send(()).map_err(|e| println!("error = {:?}", e))?;
                Ok(())
            }
        }
    }

    pub fn get_config(&self) -> &WateringScheduleConfigs {
        &self.configs
    }

    pub fn start(&mut self, ctrl_c_receiver: tokio::sync::watch::Receiver<String>) -> () {
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
                    ctrl_c_receiver.clone(),
                )
                .boxed()
                .fuse();

                tokio::task::spawn(schedule_task);
            }
        }
    }
}

async fn create_schedule(
    senders: Arc<Mutex<HashMap<ValvePinNumber, tokio::sync::mpsc::Sender<()>>>>,
    command_sender: Sender<LayoutCommand>,
    valve_pin_num: u8,
    schedule_config: ScheduleConfig,
    mut ctrl_c_receiver: tokio::sync::watch::Receiver<String>,
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

    let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
    senders.lock().unwrap().insert(number, sender);
    let mut receiver_future = receiver.recv().boxed().fuse();
    let mut ctrl_c_receiver_future = ctrl_c_receiver.recv().boxed().fuse();

    let task = select! {
        _ = start_task => {},
        _ = end_task => {},
        _ = receiver_future => {}, // TODO test shutoffsenders
        _ = ctrl_c_receiver_future => {}, // TODO test shutoffsenders
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
