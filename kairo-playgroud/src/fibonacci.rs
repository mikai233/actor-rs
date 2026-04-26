use std::time::Duration;

use async_trait::async_trait;
use rand::Rng;
use tracing::info;

use kairo_core::CEmptyCodec;
use kairo_core::actor::actor_system::ActorSystem;
use kairo_core::actor::context::{ActorContext, Context};
use kairo_core::actor::props::Props;
use kairo_core::actor::timers::Timers;
use kairo_core::actor_ref::actor_ref_factory::ActorRefFactory;
use kairo_core::config::actor_setting::ActorSetting;
use kairo_core::ext::init_logger_with_filter;
use kairo_core::{Actor, DynMessage, Message};

pub fn fibonacci(n: i32) -> u64 {
    if n < 0 {
        panic!("{} is negative!", n);
    } else if n == 0 {
        panic!("zero is not a right argument to fibonacci()!");
    } else if n == 1 {
        return 1;
    }

    let mut sum = 0;
    let mut last = 0;
    let mut curr = 1;
    for _i in 1..n {
        sum = last + curr;
        last = curr;
        curr = sum;
    }
    sum
}

struct FibActor {
    timers: Timers,
}

impl FibActor {
    fn new(context: &mut ActorContext) -> anyhow::Result<Self> {
        let timers = Timers::new(context)?;
        Ok(Self { timers })
    }
}

#[async_trait]
impl Actor for FibActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        let n = rand::thread_rng().gen_range(1..=50);
        self.timers.start_timer_with_fixed_delay(
            None,
            Duration::from_millis(100),
            Fib(n),
            context.myself().clone(),
        );
        info!("{} started", context.myself());
        Ok(())
    }

    async fn on_recv(
        &mut self,
        context: &mut ActorContext,
        message: DynMessage,
    ) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

#[derive(Debug, Clone, CEmptyCodec)]
struct Fib(i32);

#[async_trait]
impl Message for Fib {
    type A = FibActor;

    async fn handle(
        self: Box<Self>,
        _context: &mut ActorContext,
        _actor: &mut Self::A,
    ) -> anyhow::Result<()> {
        fibonacci(self.0);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger_with_filter("actor=info");
    let system = ActorSystem::new("mikai233", ActorSetting::default())?;
    system.spawn_anonymous(Props::new_with_ctx(FibActor::new))?;
    system.await?;
    Ok(())
}
