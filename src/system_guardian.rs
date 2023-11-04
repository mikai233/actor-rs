use crate::actor::context::{ActorContext, Context};
use crate::actor::Actor;
use crate::cell::envelope::UserEnvelope;
use tracing::debug;
use crate::actor_ref::ActorRef;
use crate::provider::ActorRefFactory;

#[derive(Debug)]
pub(crate) struct SystemGuardian;

#[derive(Debug)]
pub(crate) enum SystemGuardianMessage {
    StopChild {
        child: ActorRef,
    }
}

impl Actor for SystemGuardian {
    type M = SystemGuardianMessage;
    type S = ();
    type A = ();

    fn pre_start(&self, ctx: &mut ActorContext<Self>, arg: Self::A) -> anyhow::Result<Self::S> {
        debug!("SystemGuardian {} pre start", ctx.myself());
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
                    SystemGuardianMessage::StopChild { child } => {
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
