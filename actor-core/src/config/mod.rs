use std::any::Any;
use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use config::Source;
use dashmap::DashMap;
use dashmap::mapref::one::MappedRef;
use dyn_clone::DynClone;

use crate::ext::as_any::AsAny;

pub mod actor_setting;
pub mod core_config;
pub mod mailbox;

pub trait Config: Debug + Send + Sync + Any + AsAny + DynClone {}

dyn_clone::clone_trait_object!(Config);

pub trait ConfigBuilder: Sized {
    type C: Config;

    fn add_source<T>(self, source: T) -> anyhow::Result<Self>
        where
            T: Source + Send + Sync + 'static;

    fn build(self) -> anyhow::Result<Self::C>;
}

#[derive(Default)]
pub struct ActorConfig {
    configs: DashMap<&'static str, Box<dyn Config>>,
}

impl ActorConfig {
    pub fn add<C>(&self, config: C) -> anyhow::Result<()> where C: Config {
        let name = type_name::<C>();
        if !self.configs.contains_key(name) {
            self.configs.insert(name, Box::new(config));
        } else {
            return Err(anyhow!("actor config {} already exists", name));
        }
        Ok(())
    }

    pub fn get<C>(&self) -> Option<MappedRef<&'static str, Box<dyn Config>, C>> where C: Config {
        let name = type_name::<C>();
        let config = self.configs
            .get(name)
            .and_then(|e| {
                let e = e.try_map::<_, C>(|e| {
                    e.deref().as_any().downcast_ref::<C>()
                });
                match e {
                    Ok(r) => Some(r),
                    Err(_) => None,
                }
            });
        config
    }
}

impl Deref for ActorConfig {
    type Target = DashMap<&'static str, Box<dyn Config>>;

    fn deref(&self) -> &Self::Target {
        &self.configs
    }
}

impl DerefMut for ActorConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.configs
    }
}

impl Debug for ActorConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let configs = self.configs.iter().map(|e| *e.key()).collect::<Vec<_>>();
        f.debug_struct("ActorConfig")
            .field("configs", &configs)
            .finish()
    }
}
