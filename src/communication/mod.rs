use std::time::Duration;

use crossbeam::Receiver;
use futures::future::FusedFuture;
use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::{FutureExt, StreamExt};
use std::pin::Pin;

pub struct ReceiverFuture {
    inner: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl ReceiverFuture {
    pub fn new<T>(receiver: crossbeam::Receiver<T>) -> ReceiverFuture
    where
        T: Sized + Send + 'static,
    {
        let inner = tokio::time::interval(Duration::from_secs(1))
            .map(move |_| match receiver.try_recv() {
                Ok(m) => {
                    return Some(m);
                }
                Err(_) => {
                    return None;
                }
            })
            .take_while(|m| match m {
                None => future::ready(true),
                Some(_) => future::ready(false),
            })
            .for_each(|_| future::ready(()))
            .boxed();
        ReceiverFuture { inner }
    }
}

impl Future for ReceiverFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}

pub struct ReceiverStream {
    inner: Pin<Box<dyn Stream<Item = ()> + Send>>,
}

impl ReceiverStream {
    pub fn new<T>(receiver: crossbeam::Receiver<T>) -> ReceiverStream
    where
        T: Sized + Send + 'static,
    {
        let inner = tokio::time::interval(Duration::from_secs(1))
            .map(move |_| match receiver.try_recv() {
                Ok(_) => {
                    return Some(());
                }
                Err(_) => {
                    return None;
                }
            })
            .filter(|o| future::ready(o.is_some()))
            .map(|o| o.unwrap())
            .boxed();
        ReceiverStream { inner }
    }
}

impl Stream for ReceiverStream {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

pub async fn create_abortable_task(
    mut task: impl Future<Output = ()> + Sized + Send + FusedFuture + Unpin,
    r: Receiver<String>,
) {
    let mut receiver = ReceiverFuture::new(r.clone()).fuse();
    select! {
                     _ = task => {},
                    _ = receiver => {},
    };
}
