use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam::TryRecvError;
use futures::prelude::*;
use futures::task::{Context, Poll};

use crate::embedded::{PinLayout, ToggleValve, ValvePinNumber};
use futures::FutureExt;
use std::pin::Pin;

#[derive(Debug, Copy, Clone)]
pub enum LayoutCommand {
    Open(ValvePinNumber),
    Close(ValvePinNumber),
}

pub struct LayoutCommandListener {
    inner: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl LayoutCommandListener {
    pub fn new<T, U>(
        layout: Arc<Mutex<T>>,
        receiver: crossbeam::Receiver<LayoutCommand>,
        sender: crossbeam::Sender<Result<(), ()>>,
    ) -> Self
    where
        T: PinLayout<U> + Send + 'static,
        U: ToggleValve + Send + 'static,
    {
        let inner = tokio::time::interval(Duration::from_secs(1))
            .map(move |_| {
                receiver.try_recv().map_err(|e| match e {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => {
                        println!("error receiving signals for layout command listener= {}", e)
                    }
                })
            })
            .filter(|r| future::ready(r.is_ok()))
            .inspect(|n| println!("{:?}", n))
            .and_then(move |command| {
                match command {
                    LayoutCommand::Open(pin_num) => {
                        if let Ok(valve) = layout.lock().unwrap().find_pin(pin_num) {
                            if let Err(e) = valve
                                .lock()
                                .unwrap()
                                .turn_on()
                                .map_err(|e| println!("command execution error = {:?}", e))
                            {
                                return future::err(e);
                            }
                        }
                    }
                    LayoutCommand::Close(pin_num) => {
                        if let Ok(valve) = layout.lock().unwrap().find_pin(pin_num) {
                            if let Err(e) = valve
                                .lock()
                                .unwrap()
                                .turn_off()
                                .map_err(|e| println!("command execution error = {:?}", e))
                            {
                                return future::err(e);
                            }
                        }
                    }
                }
                future::ok(())
            })
            .for_each(move |_| {
                let _ = sender.send(Ok(())).map_err(|e| {
                    println!("error sending signal for layout status update. = {}", e)
                });
                future::ready(())
            })
            .boxed();
        LayoutCommandListener { inner }
    }
}

impl Future for LayoutCommandListener {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}
