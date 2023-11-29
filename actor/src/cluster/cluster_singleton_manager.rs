use std::fmt::Debug;

use async_trait::async_trait;

use crate::Actor;

#[derive(Debug)]
pub struct ClusterSingletonManager;

#[async_trait]
impl Actor for ClusterSingletonManager {}
