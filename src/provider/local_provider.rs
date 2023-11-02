use std::sync::Arc;

use crate::actor::Actor;
use crate::actor_path::{ActorPath, RootActorPath, TActorPath};
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::props::Props;
use crate::provider::{ActorRefFactory, TActorRefProvider};
use crate::root_guardian::RootGuardian;
use crate::system::{make_actor_runtime, ActorSystem};
use crate::system_guardian::SystemGuardian;
use crate::user_guardian::UserGuardian;

#[derive(Debug, Clone)]
pub(crate) struct LocalActorRefProvider {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    root_path: ActorPath,
    root_guardian: LocalActorRef,
    user_guardian: LocalActorRef,
    system_guardian: LocalActorRef,
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
        system.exec_actor_rt(rt)?;
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
        let root_guardian = if let ActorRef::LocalActorRef(l) = root_guardian {
            l
        } else {
            panic!("unreachable branch")
        };
        let system_guardian = if let ActorRef::LocalActorRef(l) = system_guardian {
            l
        } else {
            panic!("unreachable branch")
        };
        let user_guardian = if let ActorRef::LocalActorRef(l) = user_guardian {
            l
        } else {
            panic!("unreachable branch")
        };
        system.exec_actor_rt(rt)?;
        let inner = Inner {
            root_path: root_path.into(),
            root_guardian,
            user_guardian,
            system_guardian,
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
        todo!()
    }

    fn resolve_actor_ref(&self, path: String) -> ActorRef {
        todo!()
    }

    fn resolve_actor_ref_of_path(&self, path: ActorPath) -> ActorRef {
        todo!()
    }
}

#[cfg(test)]
mod local_provider_test {
    use crate::provider::local_provider::LocalActorRefProvider;
    use crate::system::ActorSystem;

    #[test]
    fn test() -> anyhow::Result<()> {
        let system = ActorSystem::new("game".to_string(), "127.0.0.1:12121".parse()?)?;
        let provider = LocalActorRefProvider::new(&system)?;
        Ok(())
    }
}
