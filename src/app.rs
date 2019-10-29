use futures::{Future, Async, Stream};
use crossbeam::{Sender, Receiver};

pub struct App {
    inner: Box<dyn Future<Item=(), Error=()> + Send>,
}

impl App {
    pub fn new(ctrl_c_channels: Vec<(Sender<String>, Receiver<String>)>) -> App {
        // listen for program termination
        let prog = tokio_signal::ctrl_c()
            .flatten_stream()
            .take(1)
            .map_err(|e| println!("ctrlc-error = {:?}", e))
            .for_each(move |_| {
            println!(
                "ctrl-c received! Sending message to {} futures.",
                ctrl_c_channels.len()
            );
            ctrl_c_channels.iter().for_each(|ctrl_c_channel| {
                ctrl_c_channel.0
                    .send("ctrl-c received!".to_string())
                    .map_err(|e| println!("send error = {}", e.0))
                    .unwrap_or_default();
            });
            Ok(())
        });
        let inner = Box::new(prog);

        App{inner}
    }
}

impl Future for App {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let value = try_ready!(self.inner.poll());
        Ok(Async::Ready(value))
    }

}
