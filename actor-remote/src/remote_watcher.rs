use async_trait::async_trait;

use actor_core::Actor;

#[derive(Debug)]
pub struct RemoteWatcher {}

#[async_trait]
impl Actor for RemoteWatcher {}