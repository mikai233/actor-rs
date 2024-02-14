use std::fmt::Debug;

use dyn_clone::DynClone;

pub trait ShardAllocationStrategy: Send + Sync + DynClone + Debug {}

dyn_clone::clone_trait_object!(ShardAllocationStrategy);
