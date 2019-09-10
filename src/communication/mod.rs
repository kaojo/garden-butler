use futures::{Future, Stream, future};
use tokio::prelude::Async;
use std::time::Duration;

pub struct ReceiverFuture {
    pub inner: Box<Future<Item = (), Error = ()> + Send>,
}

impl ReceiverFuture {
    pub fn new<T>(receiver: crossbeam::Receiver<T>) -> ReceiverFuture where T: Sized + Send + 'static {
        let inner = tokio_timer::Interval::new_interval(Duration::from_secs(1))
            .map(move |_| {
                match receiver.try_recv() {
                    Ok(m) => { return Some(m); }
                    Err(_) => { return None; }
                }
            })
            .take_while(|m| {
                match m {
                    None => {future::ok(true)},
                    Some(_) => {future::ok(false)},
                }
            })
            .for_each(|_| Ok(()))
            .map_err(|_| ());
        ReceiverFuture {inner : Box::new(inner)}
    }
}

impl Future for ReceiverFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }
}
