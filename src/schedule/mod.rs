use std::collections::HashMap;
use std::time::Duration;

use futures::{Future, Stream};
use tokio::runtime::Runtime;
use tokio_channel::oneshot::{channel, Sender};
use tokio_timer::Interval;

use embedded::LayoutConfig;

#[derive(Default)]
pub struct Scheduler {
    senders: HashMap<u8, Sender<()>>
}

impl Scheduler {
    pub fn new(layout: &LayoutConfig, runtime: &mut Runtime) -> Scheduler {
        let valves = layout.get_valves();

        let mut senders = HashMap::new();

        for v in valves {
            let valve_pin = v.get_valve_pin_num();
            let (sender, receiver) = channel::<()>();
            senders.insert(valve_pin, sender);

            println!("Creating new schedule for valve pin num {}.", valve_pin);
            let schedule = Interval::new_interval(Duration::from_secs(10)).for_each(move |instant| {
                println!("{:?}: Interval event for {}.", instant, valve_pin);
                Ok(())
            });
            let turn_off_switch = receiver.map(|_| ());
            let task = schedule.select2(turn_off_switch).map(|_| ()).map_err(|_| ());
            runtime.spawn(task);
        }
        Scheduler {
            senders
        }
    }
}
