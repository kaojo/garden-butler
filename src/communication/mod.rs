use futures::future::FusedFuture;
use futures::prelude::*;

pub async fn create_abortable_task(
    mut task: impl Future<Output = ()> + Sized + Send + FusedFuture + Unpin,
    mut rx: tokio::sync::watch::Receiver<String>,
) {
    let mut receiver = rx.recv().boxed().fuse();
    select! {
                     _ = task => {},
                    _ = receiver => {},
    };
}
