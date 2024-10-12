use crate::actor::behavior::Behavior;
use crate::actor::receive::Receive;
use crate::actor::root_guardian::RootGuardian;
use crate::actor::system_guardian::SystemGuardian;
use crate::actor::user_guardian::UserGuardian;
use crate::actor::Actor;
use crate::actor_ref::actor_ref_factory::ActorRefFactory;
use crate::actor_ref::ActorRef;
use crate::message::handler::MessageHandler;
use actor_derive::Message;

#[derive(Debug, Message, derive_more::Display)]
#[display("StopChild {{ child: {child} }}")]
pub(crate) struct StopChild {
    pub(crate) child: ActorRef,
}

impl MessageHandler<RootGuardian> for StopChild {
    fn handle(
        _: &mut RootGuardian,
        ctx: &mut <RootGuardian as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<RootGuardian>,
    ) -> anyhow::Result<Behavior<RootGuardian>> {
        ctx.stop(&message.child);
        Ok(Behavior::same())
    }
}

impl MessageHandler<UserGuardian> for StopChild {
    fn handle(
        _: &mut UserGuardian,
        ctx: &mut <UserGuardian as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<UserGuardian>,
    ) -> anyhow::Result<Behavior<UserGuardian>> {
        ctx.stop(&message.child);
        Ok(Behavior::same())
    }
}

impl MessageHandler<SystemGuardian> for StopChild {
    fn handle(
        _: &mut SystemGuardian,
        ctx: &mut <SystemGuardian as Actor>::Context,
        message: Self,
        _: Option<ActorRef>,
        _: &Receive<SystemGuardian>,
    ) -> anyhow::Result<Behavior<SystemGuardian>> {
        ctx.stop(&message.child);
        Ok(Behavior::same())
    }
}
