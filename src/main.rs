extern crate futures;
extern crate sysfs_gpio;
extern crate tokio;
extern crate tokio_signal;

use futures::{Future, Stream};
use tokio::runtime::Runtime;

use gpio::{PinLayout, ToggleValve};

mod gpio;

fn main() {
    println!("Garden buttler starting ...");
    let layout = PinLayout::new(23, 17, vec![ToggleValve::new(27, 22)]);

    layout.run_start_sequence().expect("StartSequence run.");
    layout.power_on().expect("Power Pin turned on.");

    let mut rt = Runtime::new().unwrap();

    let button_streams = layout.get_button_streams();
    rt.spawn(button_streams);
    println!("Garden buttler started ...");

    // wait until program is terminated
    let ctrl_c = tokio_signal::ctrl_c().flatten_stream().take(1).map_err(|err| println!("error = {:?}", err));

    // Process each ctrl-c as it comes in
    let prog = ctrl_c.for_each(move |()| {
        println!("ctrl-c received!");
        layout.unexport_all().expect("Should unexport all exported gpio pins.");
        Ok(())
    });
    rt.block_on(prog).expect("Should wait until app is terminated");

    println!("Exiting garden buttler ...");
}
