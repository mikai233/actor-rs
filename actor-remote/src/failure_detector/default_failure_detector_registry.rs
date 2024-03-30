use std::any::type_name;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;

use crate::failure_detector::failure_detector_registry::FailureDetectorRegistry;
use crate::failure_detector::FailureDetector;

pub struct DefaultFailureDetectorRegistry<A> {
    _phantom: PhantomData<A>,
    pub detector_factory: Box<dyn Fn() -> Box<dyn FailureDetector> + Send>,
    pub resource_to_failure_detector: HashMap<A, Box<dyn FailureDetector>>,
}

impl<A> DefaultFailureDetectorRegistry<A> {
    pub fn new<F>(factory: F) -> DefaultFailureDetectorRegistry<A>
        where
            F: Fn() -> Box<dyn FailureDetector> + Send + 'static,
    {
        let detector_factory = Box::new(factory);
        Self {
            _phantom: Default::default(),
            detector_factory,
            resource_to_failure_detector: Default::default(),
        }
    }
}

impl<A> Debug for DefaultFailureDetectorRegistry<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let struct_name = format!("DefaultFailureDetectorRegistry<{}>", type_name::<A>());
        f.debug_struct(&struct_name)
            .finish_non_exhaustive()
    }
}

impl<A> FailureDetectorRegistry for DefaultFailureDetectorRegistry<A> where A: Send + Hash + Eq {
    type A = A;

    fn is_available(&self, resource: &Self::A) -> bool {
        match self.resource_to_failure_detector.get(resource) {
            None => true,
            Some(r) => r.is_available(),
        }
    }

    fn is_monitoring(&self, resource: &Self::A) -> bool {
        match self.resource_to_failure_detector.get(resource) {
            None => true,
            Some(r) => r.is_monitoring(),
        }
    }

    fn heartbeat(&mut self, resource: Self::A) {
        match self.resource_to_failure_detector.entry(resource) {
            Entry::Occupied(mut o) => {
                o.get_mut().heartbeat();
            }
            Entry::Vacant(v) => {
                let mut r = (self.detector_factory)();
                r.heartbeat();
                v.insert(r);
            }
        }
    }

    fn remove(&mut self, resource: &Self::A) {
        self.resource_to_failure_detector.remove(resource);
    }

    fn reset(&mut self) {
        self.resource_to_failure_detector.clear();
    }
}