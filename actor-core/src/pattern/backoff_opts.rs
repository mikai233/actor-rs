use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail};

use crate::actor::props::{Props, PropsBuilder};
use crate::actor_ref::ActorRef;
use crate::ext::maybe_ref::MaybeRef;
use crate::message::DynMessage;

#[derive(Debug)]
pub struct BackoffOpts;

impl BackoffOpts {
    #[allow(unused_variables)]
    fn on_failure(
        child_props: Props,
        child_name: impl Into<String>,
        min_backoff: Duration,
        max_backoff: Duration,
        random_factor: f64,
    ) {
        todo!()
    }

    #[allow(unused_variables)]
    fn on_stop(
        child_props: Props,
        child_name: impl Into<String>,
        min_backoff: Duration,
        max_backoff: Duration,
        random_factor: f64,
    ) {
        todo!()
    }
}

trait ExtendedBackoffOptions {
    fn with_auto_reset(&self, reset_backoff: Duration) -> Self;

    fn with_manual_reset(&self) -> Self;

    fn with_max_nr_of_retries(&self, max_nr_of_retries: usize) -> Self;

    fn with_reply_while_stopped(&self, reply_while_stopped: DynMessage) -> Self;

    fn with_handler_while_stopped(&self, handler: ActorRef) -> Self;

    fn props(&self) -> Props;
}

trait BackoffOnStopOptions: ExtendedBackoffOptions {
    fn with_default_stopping_strategy(&self) -> Self;

    fn with_final_stop_message(
        &self,
        is_final_stop_message: Box<dyn Fn(DynMessage) -> bool>,
    ) -> Self;
}

trait BackoffOnFailureOptions: ExtendedBackoffOptions {}

#[derive(Clone, derive_more::Debug)]
struct BackoffOnStopOptionsImpl {
    child_props: Arc<PropsBuilder<()>>,
    child_name: String,
    min_backoff: Duration,
    max_backoff: Duration,
    random_factor: f64,
    reset: Option<BackoffReset>,
    handling_while_stopped: HandlingWhileStopped,
    #[debug(skip)]
    final_stop_message: Option<Arc<Box<dyn Fn(DynMessage) -> bool>>>,
}

impl BackoffOnStopOptionsImpl {
    fn backoff_reset(&self) -> MaybeRef<BackoffReset> {
        match &self.reset {
            None => MaybeRef::Own(BackoffReset::AutoReset {
                reset_backoff: self.max_backoff,
            }),
            Some(reset) => MaybeRef::Ref(reset),
        }
    }

    fn with_auto_reset(&self, reset_backoff: Duration) -> Self {
        let mut myself = self.clone();
        myself.reset = Some(BackoffReset::AutoReset { reset_backoff });
        myself
    }

    fn with_manual_reset(&self) -> Self {
        let mut myself = self.clone();
        myself.reset = Some(BackoffReset::ManualReset);
        myself
    }

    fn with_reply_while_stopped(&self, reply_while_stopped: DynMessage) -> anyhow::Result<Self> {
        match reply_while_stopped.clone_box() {
            Some(msg) => {
                let mut myself = self.clone();
                myself.handling_while_stopped = HandlingWhileStopped::ReplyWith { msg };
                Ok(myself)
            }
            None => {
                bail!(
                    "message {} require cloneable",
                    reply_while_stopped.signature()
                );
            }
        }
    }

    fn with_handler_while_stopped(&self, handler_while_stopped: ActorRef) -> Self {
        let mut myself = self.clone();
        myself.handling_while_stopped = HandlingWhileStopped::ForwardTo {
            handler: handler_while_stopped,
        };
        myself
    }

    #[allow(unused_variables)]
    fn with_max_nr_of_retries(&self, max_nr_of_retries: i32) -> Self {
        let myself = self.clone();
        myself
    }

    fn with_default_stopping_strategy(&self) -> Self {
        let myself = self.clone();
        myself
    }

    fn with_final_stop_message<A>(&self, action: A) -> Self
    where
        A: Fn(DynMessage) -> bool + 'static,
    {
        let mut myself = self.clone();
        myself.final_stop_message = Some(Arc::new(Box::new(action)));
        myself
    }

    fn props(&self) -> anyhow::Result<Props> {
        if !(self.min_backoff > Duration::ZERO) {
            return Err(anyhow!("min backoff must be > 0"));
        }
        if !(self.max_backoff >= self.min_backoff) {
            return Err(anyhow!("max backoff must be >= min backoff"));
        }
        if !(self.random_factor >= 0.0 && self.random_factor <= 1.0) {
            return Err(anyhow!("random factor must be between 0.0 and 1.0"));
        }
        if let Some(BackoffReset::AutoReset { reset_backoff }) = self.reset {
            if !(self.min_backoff <= reset_backoff && self.max_backoff >= reset_backoff) {
                return Err(anyhow!(
                    "auto reset {:?} must in min backoff {:?} and max backoff {:?}",
                    reset_backoff,
                    self.min_backoff,
                    self.max_backoff
                ));
            }
        }
        todo!()
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum BackoffReset {
    ManualReset,
    AutoReset { reset_backoff: Duration },
}

#[derive(Debug)]
pub(crate) enum HandlingWhileStopped {
    ForwardDeathLetters,
    ForwardTo { handler: ActorRef },
    ReplyWith { msg: DynMessage },
}

impl Clone for HandlingWhileStopped {
    fn clone(&self) -> Self {
        match self {
            HandlingWhileStopped::ForwardDeathLetters => Self::ForwardDeathLetters,
            HandlingWhileStopped::ForwardTo { handler } => Self::ForwardTo {
                handler: handler.clone(),
            },
            HandlingWhileStopped::ReplyWith { msg } => Self::ReplyWith {
                msg: msg.clone_box().expect("message cannot be cloned"),
            },
        }
    }
}
