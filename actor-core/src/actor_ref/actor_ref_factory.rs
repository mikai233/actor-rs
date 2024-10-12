use crate::actor::actor_selection::{ActorSelection, ActorSelectionPath};
use crate::actor::actor_system::ActorSystem;
use crate::actor::props::Props;
use crate::actor_path::TActorPath;
use crate::actor_ref::local_ref::LocalActorRef;
use crate::actor_ref::{ActorRef, TActorRef};
use crate::provider::ActorRefProvider;

pub trait ActorRefFactory {
    fn system(&self) -> &ActorSystem;

    fn provider(&self) -> &ActorRefProvider;

    fn guardian(&self) -> &LocalActorRef;

    fn lookup_root(&self) -> &dyn TActorRef;

    fn spawn(&self, props: Props, name: impl Into<String>) -> anyhow::Result<ActorRef>;

    fn spawn_anonymous(&self, props: Props) -> anyhow::Result<ActorRef>;

    fn actor_selection(&self, path: ActorSelectionPath) -> anyhow::Result<ActorSelection> {
        match path {
            ActorSelectionPath::RelativePath(rp) => {
                let mut elements = rp.split("/").peekable();
                match elements.peek() {
                    None => ActorSelection::new(
                        dyn_clone::clone_box(self.provider().dead_letters()).into(),
                        vec![""],
                    ),
                    Some(n) => {
                        if n.is_empty() {
                            elements.next();
                            ActorSelection::new(
                                self.provider().root_guardian().clone().into(),
                                elements.into_iter(),
                            )
                        } else {
                            ActorSelection::new(
                                self.provider().root_guardian().clone().into(),
                                elements.into_iter(),
                            )
                        }
                    }
                }
            }
            ActorSelectionPath::FullPath(fp) => {
                let root_guardian = self.provider().root_guardian_at(fp.address());
                ActorSelection::new(root_guardian, fp.elements().iter().map(|e| e.as_str()))
            }
        }
    }

    fn stop(&self, actor: &ActorRef);
}
