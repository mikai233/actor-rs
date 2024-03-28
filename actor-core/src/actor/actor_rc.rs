use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use crate::actor::context::{ActorContext, Context};

pub struct Arc<T: ?Sized> {
    id: usize,
    value: Rc<T>,
}

impl<T> Arc<T> {
    pub fn new(ctx: &ActorContext, value: T) -> Arc<T> {
        Self {
            id: ctx.id,
            value: Rc::new(value),
        }
    }

    pub fn clone(&self, ctx: &ActorContext) -> Arc<T> {
        assert_eq!(self.id, ctx.id, "{} not own this value", ctx.myself());
        Self {
            id: self.id,
            value: self.value.clone(),
        }
    }

    pub fn get(&self, ctx: &ActorContext) -> &Rc<T> {
        assert_eq!(self.id, ctx.id, "{} not own this value", ctx.myself());
        &self.value
    }
}

impl<T: ?Sized> Debug for Arc<T> where T: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arc")
            .field("id", &self.id)
            .field("value", &self.value)
            .finish()
    }
}

impl<T: ?Sized> Display for Arc<T> where T: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Arc {{id: {}, value: {} }}", self.id, self.value)
    }
}

unsafe impl<T: ?Sized> Send for Arc<T> {}

#[cfg(test)]
mod arc_test {
    use std::cell::RefCell;
    use std::time::Duration;

    use async_trait::async_trait;

    use actor_derive::{EmptyCodec, OrphanEmptyCodec};

    use crate::{Actor, Message};
    use crate::actor::actor_rc::Arc;
    use crate::actor::actor_system::ActorSystem;
    use crate::actor::context::{ActorContext, Context};
    use crate::actor::props::Props;
    use crate::actor_ref::actor_ref_factory::ActorRefFactory;
    use crate::actor_ref::ActorRefExt;
    use crate::pattern::patterns::PatternsExt;

    #[derive(Debug)]
    struct RefActor {
        value: Arc<RefCell<i64>>,
    }

    impl RefActor {
        fn new(ctx: &ActorContext) -> Self {
            Self { value: Arc::new(ctx, RefCell::new(0)) }
        }
    }

    impl Actor for RefActor {}

    #[derive(Debug, EmptyCodec)]
    struct Incr;

    #[async_trait]
    impl Message for Incr {
        type A = RefActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            *actor.value.get(context).borrow_mut() += 1;
            Ok(())
        }
    }

    #[derive(Debug, EmptyCodec)]
    struct Get;

    #[derive(Debug, OrphanEmptyCodec)]
    struct GetRsp(i64);

    #[async_trait]
    impl Message for Get {
        type A = RefActor;

        async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()> {
            context.sender().unwrap().resp(GetRsp(*actor.value.get(context).borrow()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let system = ActorSystem::new("mikai233", Default::default())?;
        let ref_actor = system.spawn_anonymous(Props::new_with_ctx(|ctx| { Ok(RefActor::new(ctx)) }))?;
        for _ in 0..1000 {
            ref_actor.cast_ns(Incr);
        }
        let rsp: GetRsp = ref_actor.ask(Get, Duration::from_secs(1)).await?;
        assert_eq!(1000, rsp.0);
        Ok(())
    }
}