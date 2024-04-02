use std::any::type_name;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

use eyre::anyhow;
use dashmap::DashMap;
use dashmap::mapref::one::{MappedRef, MappedRefMut};

use crate::ext::as_any::AsAny;

pub trait Extension: AsAny + Send + Sync + 'static {
    fn init(&self) -> eyre::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct SystemExtension {
    extensions: DashMap<&'static str, Box<dyn Extension>>,
}

impl SystemExtension {
    pub fn register<E>(&self, extension: E) -> eyre::Result<()> where E: Extension {
        let name = type_name::<E>();
        if !self.extensions.contains_key(name) {
            self.extensions.insert(name, Box::new(extension));
            self.extensions.get(name).unwrap().init()?;
        } else {
            return Err(anyhow!("actor extension {} already registered", name));
        }
        Ok(())
    }

    pub fn get<E>(&self) -> Option<E> where E: Extension + Clone {
        self.get_ref::<E>().map(|e| { e.value().clone() })
    }

    pub fn get_ref<E>(&self) -> Option<MappedRef<&'static str, Box<dyn Extension>, E>> where E: Extension {
        let name = type_name::<E>();
        let extension = self.extensions
            .get(name)
            .and_then(|e| {
                let e = e.try_map::<_, E>(|e| {
                    e.deref().as_any().downcast_ref::<E>()
                });
                match e {
                    Ok(r) => Some(r),
                    Err(_) => None,
                }
            });
        extension
    }

    pub fn get_mut<E>(&self) -> Option<MappedRefMut<&'static str, Box<dyn Extension>, E>> where E: Extension {
        let name = type_name::<E>();
        let extension = self.extensions
            .get_mut(name)
            .and_then(|e| {
                let e = e.try_map::<_, E>(|e| {
                    e.deref_mut().as_any_mut().downcast_mut::<E>()
                });
                match e {
                    Ok(r) => Some(r),
                    Err(_) => None,
                }
            });
        extension
    }
}

impl Deref for SystemExtension {
    type Target = DashMap<&'static str, Box<dyn Extension>>;

    fn deref(&self) -> &Self::Target {
        &self.extensions
    }
}

impl DerefMut for SystemExtension {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.extensions
    }
}

impl Debug for SystemExtension {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let extensions = self.extensions.iter().map(|e| *e.key()).collect::<Vec<_>>();
        f.debug_struct("SystemExtension")
            .field("extensions", &extensions)
            .finish()
    }
}

#[cfg(test)]
mod test {
    use actor_derive::AsAny;

    use crate::actor::extension::{Extension, SystemExtension};

    #[derive(Clone, AsAny)]
    struct ExtensionA;

    impl Extension for ExtensionA {}

    #[derive(Clone, AsAny)]
    struct ExtensionB;

    impl Extension for ExtensionB {}

    #[test]
    fn test_extension() -> eyre::Result<()> {
        let extensions = SystemExtension::default();
        assert!(extensions.get::<ExtensionA>().is_none());
        extensions.register(ExtensionA)?;
        extensions.register(ExtensionB)?;
        assert!(extensions.get::<ExtensionA>().is_some());
        assert!(extensions.get::<ExtensionB>().is_some());
        Ok(())
    }
}