use futures::{Future, Async, Stream};
use embedded::command::LayoutCommand;
use chrono::{NaiveTime, Utc, Timelike};
use crossbeam::Sender;
use tokio_timer::Interval;
use std::time::Duration;

pub struct WateringTask {
    inner: Box<dyn Future<Item=(), Error=()> + Send>,
}

impl WateringTask {

    pub fn new(layout_command: LayoutCommand, execution_time: NaiveTime, command_sender: Sender<LayoutCommand> ) -> WateringTask {
        let task = Interval::new_interval(Duration::from_secs(1))
            .filter(move|_| {
                let now = Utc::now().time();
                let now_seconds_precision = NaiveTime::from_hms(now.hour(), now.minute(), now.second());
                return now_seconds_precision.eq(&execution_time);
            })
            .map_err(|_| ())
            .for_each(move |_| {
                command_sender.send(layout_command)
                    .map_err(|e| println!("error = {}", e))
            });
        WateringTask{
            inner: Box::new(task)
        }
    }
}


impl Future for WateringTask {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }
}
