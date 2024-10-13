use std::any::type_name;
use std::any::Any;
use std::fmt::Display;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use dashmap::mapref::one::MappedRef;
use dashmap::DashMap;
use dyn_clone::DynClone;

use crate::ext::as_any::AsAny;

pub mod actor;
pub mod circuit_breaker;
pub mod coordinated_shutdown;
pub mod debug;
pub mod duration;
pub mod mailbox;
pub mod phase;
pub mod settings;

pub trait Config: Debug + Display + Send + Sync + Any + AsAny + DynClone {}

dyn_clone::clone_trait_object!(Config);

#[derive(Default)]
pub struct ActorConfigs {
    configs: DashMap<&'static str, Box<dyn Config>>,
}

impl ActorConfigs {
    pub fn add<C>(&self, config: C) -> anyhow::Result<()>
    where
        C: Config,
    {
        let name = type_name::<C>();
        if !self.configs.contains_key(name) {
            self.configs.insert(name, Box::new(config));
        } else {
            return Err(anyhow!("actor config {} already exists", name));
        }
        Ok(())
    }

    pub fn get<C>(&self) -> Option<MappedRef<&'static str, Box<dyn Config>, C>>
    where
        C: Config,
    {
        let name = type_name::<C>();
        let config = self.configs.get(name).and_then(|e| {
            let e = e.try_map::<_, C>(|e| e.deref().as_any().downcast_ref::<C>());
            match e {
                Ok(r) => Some(r),
                Err(_) => None,
            }
        });
        config
    }
}

impl Deref for ActorConfigs {
    type Target = DashMap<&'static str, Box<dyn Config>>;

    fn deref(&self) -> &Self::Target {
        &self.configs
    }
}

impl DerefMut for ActorConfigs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.configs
    }
}

impl Debug for ActorConfigs {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let configs = self.configs.iter().map(|e| *e.key()).collect::<Vec<_>>();
        f.debug_struct("ActorConfigs")
            .field("configs", &configs)
            .finish()
    }
}
