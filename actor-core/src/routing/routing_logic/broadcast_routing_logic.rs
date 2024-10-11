use crate::ext::maybe_ref::MaybeRef;
use crate::message::DynMessage;
use crate::routing::routee::several_routees::SeveralRoutees;
use crate::routing::routee::Routee;
use crate::routing::routing_logic::RoutingLogic;

#[derive(Debug, Clone, Default)]
pub struct BroadcastRoutingLogic;

impl RoutingLogic for BroadcastRoutingLogic {
    fn select<'a>(&self, _message: &DynMessage, routees: &'a Vec<Routee>) -> MaybeRef<'a, Routee> {
        MaybeRef::Own(
            SeveralRoutees {
                routees: routees.clone(),
            }
            .into(),
        )
    }
}
