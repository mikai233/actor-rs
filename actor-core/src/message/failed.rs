use async_trait::async_trait;

use actor_derive::SystemEmptyCodec;

use crate::{Actor, SystemMessage};
use crate::actor::context::{ActorContext, Context};
use crate::actor::directive::Directive;
use crate::actor_ref::{ActorRef, ActorRefSystemExt};
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::ext::option_ext::OptionExt;

#[derive(Debug, SystemEmptyCodec)]
pub(crate) struct Failed {
    pub(crate) child: ActorRef,
    pub(crate) error: eyre::Error,
}

#[async_trait]
impl SystemMessage for Failed {
    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut dyn Actor) -> eyre::Result<()> {
        let Self { child: failed_child, error } = *self;
        let directive = actor.on_child_failure(context, &failed_child, &error);
        match directive {
            Directive::Resume => {
                failed_child.resume();
            }
            Directive::Stop => {
                debug_assert!(context.children().iter().find(|child| child == &&failed_child).is_some());
                context.stop(&failed_child);
            }
            Directive::Escalate => {
                context.parent().foreach(|parent| {
                    parent.cast_system(Failed { child: failed_child, error }, ActorRef::no_sender());
                });
            }
        }
        Ok(())
    }
}