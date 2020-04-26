use futures::future::FusedFuture;
use futures::prelude::*;
use tokio::stream::StreamExt;

pub async fn create_abortable_task(
    mut task: impl Future<Output = ()> + Sized + Send + FusedFuture + Unpin,
    mut rx: tokio::sync::watch::Receiver<String>,
) {
    let mut receiver = futures::StreamExt::take(rx, 2)
        .for_each(|_| future::ready(()))
        .then(|_| {
            println!("ctrl_c received");
            future::ready(())
        })
        .boxed()
        .fuse();
    let mut tmp_task = task.then(|_| {
        println!("task fnished");
        future::ready(())
    });
    select! {
                     _ = tmp_task => {},
                    _ = receiver => {},
    };
}
