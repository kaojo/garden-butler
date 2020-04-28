use std::sync::{Arc, Mutex};

use futures::StreamExt;
use tokio::sync::mpsc;

use crate::schedule::{WateringScheduleConfig, WateringScheduleConfigs, WateringScheduler};

#[derive(Debug, Copy, Clone)]
pub enum WateringConfigCommand {
    Enable(WateringScheduleConfig),
    Disable(WateringScheduleConfig),
    Delete(WateringScheduleConfig),
    Create(WateringScheduleConfig),
}

pub struct WateringConfigCommandListener {}

impl WateringConfigCommandListener {
    pub async fn listen_to_commands(
        watering_config: Arc<Mutex<WateringScheduleConfigs>>,
        watering_schedule: Arc<Mutex<WateringScheduler>>,
        mut receiver: mpsc::Receiver<WateringConfigCommand>,
        mut watering_config_status_tx: mpsc::Sender<()>,
    ) -> () {
        while let Some(command) = receiver.next().await {
            println!("{:?}", command);
            let result: Result<(), ()> =
                handle_command(&watering_config, &watering_schedule, command);
            match result {
                Ok(_) => {
                    let _ = watering_config_status_tx
                        .try_send(())
                        .map_err(|e| println!("schedule command status send error: {}", e));
                }
                Err(_) => println!(),
            }
        }
    }
}

fn handle_command(
    watering_config: &Arc<Mutex<WateringScheduleConfigs>>,
    watering_schedule: &Arc<Mutex<WateringScheduler>>,
    command: WateringConfigCommand,
) -> Result<(), ()> {
    match command {
        WateringConfigCommand::Enable(schedule) => {
            let result: Result<WateringScheduleConfig, ()> =
                watering_config.lock().unwrap().enable_schedule(&schedule);
            result.and_then(|s| watering_schedule.lock().unwrap().start_schedule(&s))
        }
        WateringConfigCommand::Disable(schedule) => {
            let result: Result<WateringScheduleConfig, ()> =
                watering_config.lock().unwrap().disable_schedule(&schedule);
            result.and_then(|s| watering_schedule.lock().unwrap().stop_schedule(&s))
        }
        WateringConfigCommand::Delete(schedule) => {
            let result: Result<WateringScheduleConfig, ()> =
                watering_config.lock().unwrap().delete_schedule(&schedule);
            result.and_then(|s| watering_schedule.lock().unwrap().stop_schedule(&s))
        }
        WateringConfigCommand::Create(schedule) => {
            let result: Result<WateringScheduleConfig, ()> =
                watering_config.lock().unwrap().create_schedule(schedule);
            result.and_then(|s| watering_schedule.lock().unwrap().start_schedule(&s))
        }
    }
}
