use std::fmt::Debug;
use actor_core::Actor;


#[derive(Debug)]
pub struct ClusterSingletonManager;

impl Actor for ClusterSingletonManager {}
