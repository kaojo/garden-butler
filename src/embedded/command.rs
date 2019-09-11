use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam::TryRecvError;
use futures::{Async, Future, Stream};

use embedded::{PinLayout, ToggleValve, ValvePinNumber};

#[derive(Debug)]
pub enum LayoutCommand {
    Open(ValvePinNumber),
    Close(ValvePinNumber),
}

pub struct LayoutCommandListener
{
    inner: Box<Future<Item=(), Error=()> + Send>,
}

impl LayoutCommandListener {
    pub fn new<T, U>(
        layout: Arc<Mutex<T>>,
        receiver: crossbeam::Receiver<LayoutCommand>) -> Self
        where
            T: PinLayout<U> + Send + 'static,
            U: ToggleValve + Send + 'static,
    {
        let inner = Box::new(
            tokio_timer::Interval::new_interval(Duration::from_secs(1))
                .map_err(|_| ())
                .map(move |_| {
                    receiver
                        .try_recv()
                        .map_err(|e| {
                            match e {
                                TryRecvError::Empty => {}
                                TryRecvError::Disconnected => { println!("error = {}", e) }
                            }
                        })
                })
                .filter(|r| r.is_ok())
                .map(|r| r.unwrap())
                .inspect(|n| println!("{:?}", n))
                .for_each(move |command| {
                    match command {
                        LayoutCommand::Open(pin_num) => {
                            if let Ok(valve) = layout.lock().unwrap().find_pin(pin_num) {
                                valve.lock().unwrap().turn_on().map_err(|e| println!("command execution error = {:?}", e))?;
                            }
                        }
                        LayoutCommand::Close(pin_num) => {
                            if let Ok(valve) = layout.lock().unwrap().find_pin(pin_num) {
                                valve.lock().unwrap().turn_off().map_err(|e| println!("command execution error = {:?}", e))?;
                            }
                        }
                    }
                    Ok(())
                })
        );
        LayoutCommandListener { inner }
    }
}

impl Future for LayoutCommandListener
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }
}
