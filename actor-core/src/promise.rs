use tokio::sync::oneshot::{channel, Receiver, Sender};
use tokio::sync::oneshot::error::RecvError;

pub struct PromiseTx<R> {
    tx: Sender<R>,
}

impl<R> PromiseTx<R> {
    pub fn complete(self, result: R) -> Result<(), R> {
        self.tx.send(result)
    }
}

pub struct PromiseRx<R> {
    rx: Receiver<R>,
}

impl<R> PromiseRx<R> {
    async fn get(self) -> Result<R, RecvError> {
        self.rx.await
    }
}

pub fn promise<R>() -> (PromiseTx<R>, PromiseRx<R>) {
    let (tx, rx) = channel();
    let tx = PromiseTx { tx };
    let rx = PromiseRx { rx };
    (tx, rx)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use anyhow::anyhow;

    use crate::promise::promise;

    #[tokio::test]
    async fn test_promise() -> anyhow::Result<()> {
        let (tx, rx) = promise::<()>();
        tx.complete(()).map_err(|_| anyhow!("channel closed"))?;
        tokio::time::timeout(Duration::from_secs(1), rx.get()).await??;
        Ok(())
    }
}