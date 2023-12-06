use std::fmt::Debug;

use crate::Actor;

#[derive(Debug)]
pub struct ClusterSingletonManager;

impl Actor for ClusterSingletonManager {}
