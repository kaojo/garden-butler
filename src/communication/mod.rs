use futures::future::Fuse;
use futures::prelude::*;
use std::pin::Pin;
use tokio::sync::watch::Receiver;

pub async fn create_abortable_task(
    task: impl Future<Output = ()> + Sized + Send,
    task_name: String,
    rx: tokio::sync::watch::Receiver<String>,
) {
    let mut receiver = get_ctrl_c_future(rx);
    let mut tmp_task = task
        .then(|_| {
            println!("task {} finished", task_name);
            future::ready(())
        })
        .boxed()
        .fuse();
    select! {
                     _ = tmp_task => {},
                    _ = receiver => {},
    };
}

pub fn get_ctrl_c_future(rx: Receiver<String>) -> Fuse<Pin<Box<dyn Future<Output = ()> + Send>>> {
    futures::StreamExt::take(rx, 2)
        .for_each(|_| future::ready(()))
        .then(|_| {
            println!("ctrl_c received");
            future::ready(())
        })
        .boxed()
        .fuse()
}
