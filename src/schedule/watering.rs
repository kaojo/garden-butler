use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::NaiveTime;
use futures::prelude::*;
use tokio::sync::mpsc::Sender;

use crate::communication::get_ctrl_c_future;
use crate::embedded::command::LayoutCommand;
use crate::embedded::ValvePinNumber;
use crate::schedule::configuration::WateringScheduleConfigs;
use crate::schedule::watering_task::WateringTask;
use crate::schedule::{ScheduleConfig, WateringScheduleConfig};

pub struct WateringScheduler {
    senders: Arc<Mutex<HashMap<WateringScheduleConfig, Sender<()>>>>,
    ctrl_c_receiver: tokio::sync::watch::Receiver<String>,
    command_sender: Sender<LayoutCommand>,
}

impl WateringScheduler {
    pub fn new(
        command_sender: Sender<LayoutCommand>,
        ctrl_c_receiver: tokio::sync::watch::Receiver<String>,
    ) -> WateringScheduler {
        let senders = Arc::new(Mutex::new(HashMap::new()));
        WateringScheduler {
            senders,
            command_sender,
            ctrl_c_receiver,
        }
    }

    pub fn start_schedule(&mut self, schedule: &WateringScheduleConfig) -> Result<(), ()> {
        self.spawn_schedule_task(schedule);
        Ok(())
    }

    pub fn stop_schedule(&mut self, schedule: &WateringScheduleConfig) -> Result<(), ()> {
        let sender = self.senders.lock().unwrap().remove(&schedule);
        match sender {
            None => Err(()),
            Some(mut s) => {
                s.try_send(()).map_err(|e| println!("error = {:?}", e))?;
                Ok(())
            }
        }
    }

    pub fn start(&mut self, configs: &Arc<Mutex<WateringScheduleConfigs>>) {
        for schedule in configs.lock().unwrap().get_schedules().iter() {
            if schedule.is_enabled() {
                self.spawn_schedule_task(schedule);
            }
        }
    }

    fn spawn_schedule_task(&mut self, schedule: &WateringScheduleConfig) {
        println!(
            "Creating watering schedule for valve {}",
            schedule.get_valve()
        );
        let schedule_task = create_schedule(
            Arc::clone(&self.senders),
            self.command_sender.clone(),
            *schedule,
            self.ctrl_c_receiver.clone(),
        )
        .boxed()
        .fuse();
        tokio::task::spawn(schedule_task);
    }
}

async fn create_schedule(
    senders: Arc<Mutex<HashMap<WateringScheduleConfig, tokio::sync::mpsc::Sender<()>>>>,
    command_sender: Sender<LayoutCommand>,
    schedule_config: WateringScheduleConfig,
    ctrl_c_receiver: tokio::sync::watch::Receiver<String>,
) {
    let number = ValvePinNumber(schedule_config.get_valve());

    let start_time: NaiveTime = get_schedule_start_time(&schedule_config.get_schedule());
    let end_time = get_schedule_end_time(&schedule_config.get_schedule());

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
    senders.lock().unwrap().insert(schedule_config, sender);
    let mut receiver_future = receiver.recv().boxed().fuse();
    let mut ctrl_c_receiver_future = get_ctrl_c_future(ctrl_c_receiver);

    select! {
        _ = start_task => {},
        _ = end_task => {},
        _ = receiver_future => {}, // TODO test shutoffsenders
        _ = ctrl_c_receiver_future => {}, // TODO test shutoffsenders
    };
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
