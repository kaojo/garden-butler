
use futures::Future;
use tokio::prelude::Async;

pub struct ReceiverFuture<T> {
    pub receiver: crossbeam::Receiver<T>,
}

impl<T> Future for ReceiverFuture<T> {
    type Item = T;
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let result = self.receiver.try_recv();

        match result {
            Ok(s) => { Ok(Async::Ready(s)) }
            Err(_) => { Ok(Async::NotReady) }
        }
    }
}
