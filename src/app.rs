use crossbeam::{Receiver, Sender};
use futures::prelude::*;
use std::sync::{Arc, Mutex};

pub struct App {}

impl App {
    pub async fn start(
        ctrl_c_channels: Arc<Mutex<Vec<(Sender<String>, Receiver<String>)>>>,
        (_, _): (Sender<Result<(), ()>>, Receiver<Result<(), ()>>),
    ) -> Result<(), ()> {
        // listen for program termination
        tokio::signal::ctrl_c()
            .map_err(|e| println!("ctrlc-error = {:?}", e))
            .await?;
        println!(
            "ctrl-c received! Sending message to {} shut off channels.",
            ctrl_c_channels.lock().unwrap().len()
        );
        // send shut off commands to running tasks
        ctrl_c_channels
            .lock()
            .unwrap()
            .iter()
            .for_each(|ctrl_c_channel| {
                let _ = ctrl_c_channel
                    .0
                    .send("ctrl-c received!".to_string())
                    .map_err(|e| println!("send error = {}", e.0));
            });
        Ok(())
    }
}