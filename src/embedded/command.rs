use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::FutureExt;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::embedded::{PinLayout, ToggleValve, ValvePinNumber};

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
        receiver: Receiver<LayoutCommand>,
        mut layout_status_sender: Sender<()>,
    ) -> Self
    where
        T: PinLayout<U> + Send + 'static,
        U: ToggleValve + Send + 'static,
    {
        let inner =
            receiver
                .inspect(|n| println!("{:?}", n))
                .then(move |command| {
                    match command {
                        LayoutCommand::Open(pin_num) => {
                            if let Err(_e) =
                                layout.lock().unwrap().turn_on(pin_num).map_err(|e| {
                                    println!("turn on: command execution error = {:?}", e)
                                })
                            {
                                return future::err(());
                            }
                        }
                        LayoutCommand::Close(pin_num) => {
                            if let Err(_e) = layout.lock().unwrap().turn_off(pin_num).map_err(|e| {
                                println!("turn off: command execution error = {:?}", e)
                            }) {
                                return future::err(());
                            }
                        }
                    }
                    future::ok(())
                })
                .for_each(move |_| {
                    let _ = layout_status_sender.try_send(()).map_err(|e| {
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
