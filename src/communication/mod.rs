use std::time::Duration;

use futures::{future, Future, Stream};
use tokio::prelude::Async;

pub struct ReceiverFuture {
    pub inner: Box<dyn Future<Item=(), Error=()> + Send>,
}

impl ReceiverFuture {
    pub fn new<T>(receiver: crossbeam::Receiver<T>) -> ReceiverFuture
        where
            T: Sized + Send + 'static,
    {
        let inner = tokio_timer::Interval::new_interval(Duration::from_secs(1))
            .map(move |_| match receiver.try_recv() {
                Ok(m) => {
                    return Some(m);
                }
                Err(_) => {
                    return None;
                }
            })
            .take_while(|m| match m {
                None => future::ok(true),
                Some(_) => future::ok(false),
            })
            .for_each(|_| Ok(()))
            .map_err(|_| ());
        ReceiverFuture {
            inner: Box::new(inner),
        }
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

pub struct ReceiverStream {
    inner: Box<dyn Stream<Item=(), Error=()> + Send>
}

impl ReceiverStream {
    pub fn new<T>(receiver: crossbeam::Receiver<T>) -> ReceiverStream
        where
            T: Sized + Send + 'static {
        let inner = tokio_timer::Interval::new_interval(Duration::from_secs(1))
            .map(move |_| match receiver.try_recv() {
                Ok(_) => {
                    return Some(());
                }
                Err(_) => {
                    return None;
                }
            })
            .filter(|o| o.is_some())
            .map(|o| o.unwrap())
            .map_err(|_| ());
        ReceiverStream {
            inner: Box::new(inner)
        }
    }
}

impl Stream for ReceiverStream {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }
}
