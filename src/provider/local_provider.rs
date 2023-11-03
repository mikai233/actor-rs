use anyhow::Error;
use std::sync::Arc;

use crate::actor::Actor;
use crate::actor_path::{ActorPath, RootActorPath, TActorPath};
use crate::actor_ref::dead_letter_ref::DeadLetterActorRef;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::ext::random_actor_name;
use crate::props::Props;
use crate::provider::{ActorRefFactory, TActorRefProvider};
use crate::root_guardian::RootGuardian;
use crate::system::{make_actor_runtime, ActorSystem};
use crate::system_guardian::SystemGuardian;
use crate::user_guardian::UserGuardian;

#[derive(Debug, Clone)]
pub struct LocalActorRefProvider {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    root_path: ActorPath,
    root_guardian: LocalActorRef,
    user_guardian: LocalActorRef,
    system_guardian: LocalActorRef,
    dead_letters: ActorRef,
}

impl LocalActorRefProvider {
    pub(crate) fn new(system: &ActorSystem) -> anyhow::Result<Self> {
        let root_path = RootActorPath::new(system.address().clone(), "/".to_string());
        let system_path = root_path.child("system".to_string());
        let user_path = root_path.child("user".to_string());
        let rt = make_actor_runtime(
            system,
            RootGuardian,
            (),
            Props::default(),
            root_path.clone().into(),
            None,
        )?;
        let root_guardian = rt.myself.clone();
        let rt = make_actor_runtime(
            system,
            SystemGuardian,
            (),
            Props::default(),
            system_path,
            Some(root_guardian.clone()),
        )?;
        let system_guardian = rt.myself.clone();
        system.exec_actor_rt(rt)?;
        let rt = make_actor_runtime(
            system,
            UserGuardian,
            (),
            Props::default(),
            user_path,
            Some(root_guardian.clone()),
        )?;
        let user_guardian = rt.myself.clone();
        let root_guardian = root_guardian.local_or_panic().clone();
        let system_guardian = system_guardian.local_or_panic().clone();
        let user_guardian = user_guardian.local_or_panic().clone();
        system.exec_actor_rt(rt)?;
        let dead_letters = DeadLetterActorRef {
            system: system.clone(),
            path: root_path.child("deadLetters".to_string()),
        };
        let inner = Inner {
            root_path: root_path.into(),
            root_guardian,
            user_guardian,
            system_guardian,
            dead_letters: dead_letters.into(),
        };
        let provider = Self {
            inner: inner.into(),
        };
        Ok(provider)
    }
}

impl TActorRefProvider for LocalActorRefProvider {
    fn root_guardian(&self) -> &LocalActorRef {
        &self.inner.root_guardian
    }

    fn guardian(&self) -> &LocalActorRef {
        &self.inner.user_guardian
    }

    fn system_guardian(&self) -> &LocalActorRef {
        &self.inner.system_guardian
    }

    fn root_path(&self) -> &ActorPath {
        &self.inner.root_path
    }

    fn temp_path(&self) -> &ActorPath {
        todo!()
    }

    fn temp_path_of(&self) -> &ActorPath {
        todo!()
    }

    fn register_temp_actor(&self, actor: ActorRef, path: ActorPath) {
        todo!()
    }

    fn unregister_temp_actor(&self, path: ActorPath) {
        todo!()
    }

    fn actor_of<T>(
        &self,
        actor: T,
        arg: T::A,
        props: Props,
        supervisor: &ActorRef,
        path: ActorPath,
    ) -> anyhow::Result<ActorRef>
    where
        T: Actor,
    {
        let rt = make_actor_runtime(
            &self.guardian().system,
            actor,
            arg,
            props,
            path,
            Some(supervisor.clone()),
        )?;
        let actor_ref = rt.myself.clone();
        self.guardian().system.exec_actor_rt(rt)?;
        Ok(actor_ref)
    }

    fn resolve_actor_ref(&self, path: String) -> ActorRef {
        match path.parse::<ActorPath>() {
            Ok(actor_path) => self.resolve_actor_ref_of_path(actor_path),
            Err(_) => self.dead_letters().clone(),
        }
    }

    fn resolve_actor_ref_of_path(&self, path: ActorPath) -> ActorRef {
        if path.address() == self.root_path().address() {
            self.root_guardian()
                .get_child(path.elements())
                .unwrap_or_else(|| self.dead_letters().clone())
        } else {
            self.dead_letters().clone()
        }
    }

    fn dead_letters(&self) -> &ActorRef {
        &self.inner.dead_letters
    }
}

#[cfg(test)]
mod local_provider_test {
    use crate::actor::context::ActorContext;
    use crate::actor::Actor;
    use crate::actor_ref::{ActorRef, ActorRefExt};
    use crate::cell::envelope::UserEnvelope;
    use crate::props::Props;
    use crate::provider::local_provider::LocalActorRefProvider;
    use crate::provider::{ActorRefFactory, TActorRefProvider};
    use crate::system::ActorSystem;
    use std::time::Duration;
    use tracing::info;

    #[derive(Debug)]
    struct ActorA;

    impl Actor for ActorA {
        type M = ();
        type S = ();
        type A = ();

        fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor a {} pre start", ctx.myself);
            ctx.actor_of(ActorB, (), Props::default(), None)?;
            Ok(())
        }

        fn on_recv(
            &self,
            ctx: &mut ActorContext<Self>,
            state: &mut Self::S,
            envelope: UserEnvelope<Self::M>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct ActorB;

    impl Actor for ActorB {
        type M = ();
        type S = ();
        type A = ();

        fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor b {} pre start", ctx.myself);
            ctx.actor_of(ActorC, (), Props::default(), None)?;
            Ok(())
        }

        fn on_recv(
            &self,
            ctx: &mut ActorContext<Self>,
            state: &mut Self::S,
            envelope: UserEnvelope<Self::M>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct ActorC;

    impl Actor for ActorC {
        type M = ();
        type S = ();
        type A = ();

        fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
            info!("actor c {} pre start", ctx.myself);
            Ok(())
        }

        fn on_recv(
            &self,
            ctx: &mut ActorContext<Self>,
            state: &mut Self::S,
            envelope: UserEnvelope<Self::M>,
        ) -> anyhow::Result<()> {
            info!("actor c {} recv message", ctx.myself);
            Ok(())
        }
    }

    #[test]
    fn test() -> anyhow::Result<()> {
        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?)?;
        let actor_a = system.actor_of(ActorA, (), Props::default(), None)?;
        let actor_c = system
            .provider()
            .resolve_actor_ref("tcp://game@127.0.0.1:12121/user/$a/$b/$c".to_string());
        actor_c.tell_local((), None);
        std::thread::park();
        Ok(())
    }
}
