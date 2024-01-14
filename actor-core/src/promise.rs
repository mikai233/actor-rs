use tokio::sync::oneshot::error::RecvError;
use tokio::sync::oneshot::{channel, Receiver, Sender};

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