use async_trait::async_trait;

use crate::Actor;

pub struct RouterActor;

#[async_trait]
impl Actor for RouterActor {}