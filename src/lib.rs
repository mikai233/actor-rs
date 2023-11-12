#![feature(exclusive_range_pattern)]

pub mod actor_ref;
pub(crate) mod message2;
pub mod system;
pub(crate) mod state;
mod props;
pub(crate) mod actor_path;
pub(crate) mod address;
pub(crate) mod provider;
mod ext;
mod net;
mod cell;
mod cluster;
mod actor;
mod root_guardian;
pub(crate) mod system_guardian;
pub(crate) mod user_guardian;
mod decoder;
mod delegate;
mod message;

#[cfg(test)]
mod actor_test {
    use tracing::Level;
    use crate::ext::init_logger;

    #[ctor::ctor]
    fn init() {
        init_logger(Level::DEBUG)
    }
}


// #[cfg(test)]
// mod actor_test {
//     use std::time::Duration;
//
//     use anyhow::anyhow;
//     use futures::future::try_join_all;
//     use crate::Actor;
//     use crate::actor_ref::ActorRefImpl;
//     use crate::config::ActorConfig;
//     use crate::context::ActorContext;
//     use crate::provider::ActorRefFactory;
//
//     #[derive(Debug)]
//     struct TestActor;
//
//     #[derive(Debug)]
//     enum TestMessage {
//         Ask,
//         RecvMessage(tokio::sync::oneshot::Sender<()>),
//         SpawnChild(tokio::sync::oneshot::Sender<ActorRefImpl>),
//         Stop(tokio::sync::oneshot::Sender<()>),
//     }
//
//     impl Actor for TestActor {
//         type M = TestMessage;
//         type S = Option<tokio::sync::oneshot::Sender<()>>;
//         type A = ();
//
//         fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
//             Ok(None)
//         }
//
//         fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
//             match message {
//                 TestMessage::Ask => {
//                     ctx.sender.as_ref().unwrap().tell((), None);
//                 }
//                 TestMessage::RecvMessage(responder) => {
//                     responder.send(()).unwrap();
//                 }
//                 TestMessage::SpawnChild(responder) => {
//                     let child = ctx.actor_of(ActorConfig::default(), ChildActor, ())?;
//                     responder.send(child).unwrap();
//                 }
//                 TestMessage::Stop(responder) => {
//                     *state = Some(responder);
//                     ctx.stop();
//                 }
//             }
//             Ok(())
//         }
//
//         fn post_stop(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S) -> anyhow::Result<()> {
//             assert!(ctx.children.is_empty());
//             state.take().unwrap().send(()).unwrap();
//             Ok(())
//         }
//     }
//
//     #[derive(Debug)]
//     struct ChildActor;
//
//     #[derive(Debug)]
//     enum ChildMessage {
//         Ping,
//     }
//
//     impl Actor for ChildActor {
//         type M = ChildMessage;
//         type S = ();
//         type A = ();
//
//         fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
//             Ok(())
//         }
//
//         fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
//             match message {
//                 ChildMessage::Ping => {
//                     ctx.sender.as_ref().unwrap().tell((), None);
//                 }
//             }
//             Ok(())
//         }
//     }
//
//     #[derive(Debug)]
//     struct ExceptionActor;
//
//     impl Actor for ExceptionActor {
//         type M = ();
//         type S = ();
//         type A = Vec<tokio::sync::oneshot::Sender<()>>;
//
//         fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
//             for tx in arg {
//                 ctx.actor_of(ActorConfig::default(), ExceptionChildActor, tx)?;
//             }
//             Err(anyhow!("test error"))
//         }
//
//         fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
//             Ok(())
//         }
//     }
//
//     #[derive(Debug)]
//     struct ExceptionChildActor;
//
//     impl Actor for ExceptionChildActor {
//         type M = ();
//         type S = Option<tokio::sync::oneshot::Sender<()>>;
//         type A = tokio::sync::oneshot::Sender<()>;
//
//         fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
//             Ok(Some(arg))
//         }
//
//         fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
//             Ok(())
//         }
//
//         fn post_stop(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S) -> anyhow::Result<()> {
//             state.take().unwrap().send(()).unwrap();
//             Ok(())
//         }
//     }
//
//     #[derive(Debug)]
//     struct StashActor;
//
//     #[derive(Debug)]
//     enum StashMessage {
//         Test,
//     }
//
//     impl Actor for StashActor {
//         type M = StashMessage;
//         type S = (usize, usize, Option<tokio::sync::oneshot::Sender<()>>);
//         type A = (usize, tokio::sync::oneshot::Sender<()>);
//
//         fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
//             let (expect, tx) = arg;
//             Ok((0, expect, Some(tx)))
//         }
//
//         fn on_recv(&self, ctx: &mut ActorContext<Self>, state: &mut Self::S, message: Self::M) -> anyhow::Result<()> {
//             state.0 += 1;
//             if state.0 <= state.1 {
//                 ctx.stash(message);
//             }
//             if state.0 == state.1 {
//                 ctx.unstash_all();
//                 assert!(ctx.stash.is_empty());
//             }
//             if state.0 == state.1 * 2 {
//                 state.2.take().unwrap().send(()).unwrap();
//             }
//             Ok(())
//         }
//     }
//
//     #[tokio::test]
//     async fn test_actor_handle_message() -> anyhow::Result<()> {
//         let mut rxs = vec![];
//         let actor = actor_of(ActorConfig::default(), TestActor, ())?;
//         for _ in 0..1000 {
//             let (tx, rx) = tokio::sync::oneshot::channel();
//             rxs.push(tokio::time::timeout(Duration::from_secs(3), rx));
//             actor.tell(TestMessage::RecvMessage(tx), None);
//         }
//         assert!(try_join_all(rxs).await?.into_iter().all(|t| t.is_ok()));
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_actor_ask() -> anyhow::Result<()> {
//         let actor = actor_of(ActorConfig::default(), TestActor, ())?;
//         for _ in 0..1000 {
//             let ans: () = ask(&actor, TestMessage::Ask, Duration::from_secs(3)).await?;
//         }
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_actor_spawn_child() -> anyhow::Result<()> {
//         let actor = actor_of(ActorConfig::default(), TestActor, ())?;
//         let (tx, rx) = tokio::sync::oneshot::channel();
//         actor.tell(TestMessage::SpawnChild(tx), None);
//         let child = tokio::time::timeout(Duration::from_secs(3), rx).await??;
//         let _: () = ask(&child, ChildMessage::Ping, Duration::from_secs(3)).await?;
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_actor_pre_start_error() -> anyhow::Result<()> {
//         let mut txs = vec![];
//         let mut rxs = vec![];
//         for _ in 0..10 {
//             let (tx, rx) = tokio::sync::oneshot::channel();
//             txs.push(tx);
//             rxs.push(tokio::time::timeout(Duration::from_secs(3), rx));
//         }
//         let actor = actor_of(ActorConfig::default(), ExceptionActor, txs)?;
//         assert!(try_join_all(rxs).await?.into_iter().all(|t| t.is_ok()));
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_actor_stash() -> anyhow::Result<()> {
//         let (tx, rx) = tokio::sync::oneshot::channel();
//         let message_num = 10;
//         let actor = actor_of(ActorConfig::default(), StashActor, (message_num, tx))?;
//         for _ in 0..message_num {
//             actor.tell(StashMessage::Test, None);
//         }
//         let _ = tokio::time::timeout(Duration::from_secs(3), rx).await??;
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_actor_stop() -> anyhow::Result<()> {
//         let actor = actor_of(ActorConfig::default(), TestActor, ())?;
//         let (tx, rx) = tokio::sync::oneshot::channel();
//         actor.tell(TestMessage::SpawnChild(tx), None);
//         let _ = tokio::time::timeout(Duration::from_secs(3), rx).await??;
//         let (tx, rx) = tokio::sync::oneshot::channel();
//         actor.tell(TestMessage::Stop(tx), None);
//         let _: () = tokio::time::timeout(Duration::from_secs(3), rx).await??;
//         Ok(())
//     }
// }