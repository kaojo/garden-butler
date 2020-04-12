use crate::embedded::command::LayoutCommand;
use chrono::{NaiveTime, Timelike, Utc};
use crossbeam::Sender;
use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::FutureExt;
use std::pin::Pin;
use std::time::Duration;

pub struct WateringTask {
    inner: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl WateringTask {
    pub fn new(
        layout_command: LayoutCommand,
        execution_time: NaiveTime,
        command_sender: Sender<LayoutCommand>,
    ) -> WateringTask {
        let task = tokio::time::interval(Duration::from_secs(1))
            .filter(move |_| {
                let now = Utc::now().time();
                let now_seconds_precision =
                    NaiveTime::from_hms(now.hour(), now.minute(), now.second());
                return future::ready(now_seconds_precision.eq(&execution_time));
            })
            .for_each(move |_| {
                command_sender
                    .send(layout_command)
                    .map_err(|e| println!("error = {}", e));
                future::ready(())
            })
            .boxed();
        WateringTask { inner: task }
    }
}

impl Future for WateringTask {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}
