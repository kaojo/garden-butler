use futures::prelude::*;

pub struct App {}

impl App {
    pub async fn start(ctrc_c_sender: tokio::sync::watch::Sender<String>) -> Result<(), ()> {
        // listen for program termination
        tokio::signal::ctrl_c()
            .map_err(|e| println!("ctrlc-error = {:?}", e))
            .await?;

        // send shut off commands to running tasks
        ctrc_c_sender
            .broadcast("ctrl-c received!".to_string())
            .map_err(|_| {})
    }
}
