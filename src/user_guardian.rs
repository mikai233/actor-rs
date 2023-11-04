use tracing::debug;

use crate::actor::context::{ActorContext, Context};
use crate::actor::Actor;
use crate::actor_ref::ActorRef;
use crate::cell::envelope::UserEnvelope;
use crate::provider::ActorRefFactory;

#[derive(Debug)]
pub(crate) struct UserGuardian;

#[derive(Debug)]
pub(crate) enum UserGuardianMessage {
    StopChild {
        child: ActorRef,
    }
}

impl Actor for UserGuardian {
    type M = UserGuardianMessage;
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("UserGuardian {} pre start", ctx.myself());
        Ok(())
    }

    fn on_recv(
        &self,
        ctx: &mut ActorContext<Self>,
        state: &mut Self::S,
        message: UserEnvelope<Self::M>,
    ) -> anyhow::Result<()> {
        match message {
            UserEnvelope::Local(l) => {
                match l {
                    UserGuardianMessage::StopChild { child } => {
                        ctx.stop(&child);
                    }
                }
            }
            UserEnvelope::Remote { .. } => { todo!() }
            UserEnvelope::Unknown { .. } => { todo!() }
        }
        Ok(())
    }
}
