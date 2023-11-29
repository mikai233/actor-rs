use async_trait::async_trait;

use crate::Actor;

#[derive(Debug)]
pub struct ShardRegion;

#[async_trait]
impl Actor for ShardRegion {}
