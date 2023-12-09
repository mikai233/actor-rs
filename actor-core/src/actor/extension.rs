use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

use dashmap::DashMap;
use dashmap::mapref::one::{MappedRef, MappedRefMut};

use crate::ext::as_any::AsAny;

pub trait Extension: Any + AsAny + Send + Sync {}

impl<T> Extension for T where T: Any + AsAny + Send + Sync {}

#[derive(Default)]
pub struct ActorSystemExtension {
    extensions: DashMap<&'static str, Box<dyn Extension>>,
}

impl ActorSystemExtension {
    pub fn register<E>(&self, extension: E) where E: Extension {
        let name = std::any::type_name::<E>();
        if !self.extensions.contains_key(name) {
            self.extensions.insert(name, Box::new(extension));
        }
    }

    pub fn get<E>(&self) -> Option<MappedRef<&'static str, Box<dyn Extension>, E>> where E: Extension {
        let name = std::any::type_name::<E>();
        let extension = self.extensions
            .get(name)
            .and_then(|e| {
                let e = e.try_map::<_, E>(|e| {
                    e.deref().as_any_ref().downcast_ref::<E>()
                });
                match e {
                    Ok(r) => Some(r),
                    Err(_) => None,
                }
            });
        extension
    }

    pub fn get_mut<E>(&self) -> Option<MappedRefMut<&'static str, Box<dyn Extension>, E>> where E: Extension {
        let name = std::any::type_name::<E>();
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

impl Deref for ActorSystemExtension {
    type Target = DashMap<&'static str, Box<dyn Extension>>;

    fn deref(&self) -> &Self::Target {
        &self.extensions
    }
}

impl DerefMut for ActorSystemExtension {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.extensions
    }
}

impl Debug for ActorSystemExtension {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let extensions = self.extensions.iter().map(|e| *e.key()).collect::<Vec<_>>();
        f.debug_struct("ActorExtension")
            .field("extensions", &extensions)
            .finish()
    }
}

#[cfg(test)]
mod test {
    use crate::actor::extension::ActorSystemExtension;

    struct ExtensionA;

    struct ExtensionB;

    #[test]
    fn test_extension() {
        let extensions = ActorSystemExtension::default();
        assert!(extensions.get::<ExtensionA>().is_none());
        extensions.register(ExtensionA);
        extensions.register(ExtensionB);
        extensions.register(ExtensionB);
        assert!(extensions.get::<ExtensionA>().is_some());
        assert!(extensions.get::<ExtensionB>().is_some());
    }
}