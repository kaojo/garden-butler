extern crate config;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate sysfs_gpio;
extern crate tokio;
extern crate tokio_signal;

use futures::{Future, Stream};
use tokio::runtime::Runtime;

use embedded::{LayoutConfig, PinLayout};

mod embedded;

fn main() {
    println!("Garden buttler starting ...");

    let layout_config = get_layout_config();
    println!("{:?}",layout_config);
    let layout = PinLayout::from_config(&layout_config);

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
        layout.unexport_all().expect("Unexport of GPIO pins failed.");
        Ok(())
    });
    rt.block_on(prog).expect("Error waiting until app is terminated with ctrl+c");

    println!("Exiting garden buttler ...");
}

fn get_layout_config() -> LayoutConfig {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::new("layout", config::FileFormat::Json)).unwrap()
        // Add in settings from the environment (with a prefix of LAYOUT)
        // Eg.. `LAYOUT_POWER=11 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("LAYOUT")).unwrap();
    let layout_config = settings.try_into::<LayoutConfig>().expect("Layout config contains errors");
    layout_config
}
