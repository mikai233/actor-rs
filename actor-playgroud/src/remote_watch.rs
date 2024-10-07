use std::net::SocketAddrV4;
use std::time::Duration;

use async_trait::async_trait;
use clap::Parser;
use tracing::info;

use actor_core::{Actor, CodecMessage, DynMessage, Message};
use actor_core::actor::actor_selection::ActorSelectionPath;
use actor_core::actor::actor_system::ActorSystem;
use actor_core::actor::address::{Address, Protocol};
use actor_core::actor::context::{ActorContext1, ActorContext};
use actor_core::actor::props::Props;
use actor_core::actor_path::root_actor_path::RootActorPath;
use actor_core::actor_path::TActorPath;
use actor_core::actor_ref::actor_ref_factory::ActorRefFactory;
use actor_core::actor_ref::ActorRef;
use actor_core::config::actor_setting::ActorSetting;
use actor_core::config::ConfigBuilder;
use actor_core::config::core_config::CoreConfig;
use actor_core::EmptyCodec;
use actor_core::ext::init_logger_with_filter;
use actor_remote::codec::MessageRegistry;
use actor_core::message::terminated::Terminated;
use actor_remote::config::message_buffer::MessageBuffer;
use actor_remote::config::RemoteConfig;
use actor_remote::config::transport::Transport;
use actor_remote::remote_provider::RemoteActorRefProvider;
use actor_remote::config::settings::Settings;

#[derive(Debug)]
struct RemoteActor {
    remote_ref: Option<ActorRef>,
}

#[async_trait]
impl Actor for RemoteActor {
    async fn started(&mut self, context: &mut ActorContext1) -> anyhow::Result<()> {
        info!("{} started", context.myself());
        if let Some(remote_ref) = &self.remote_ref {
            context.watch_with(remote_ref.clone(), RemoteTerminated::new)?;
            info!("{} watch remote ref {}", context.myself(), remote_ref);
        }
        Ok(())
    }

    async fn on_recv(&mut self, context: &mut ActorContext1, message: DynMessage) -> anyhow::Result<()> {
        Self::handle_message(self, context, message).await
    }
}

#[derive(Debug, EmptyCodec)]
struct RemoteTerminated(Terminated);

impl RemoteTerminated {
    pub fn new(terminated: Terminated) -> DynMessage {
        Self(terminated).into_dyn()
    }
}

#[async_trait]
impl Message for RemoteTerminated {
    type A = RemoteActor;

    async fn handle(self: Box<Self>, context: &mut ActorContext1, _actor: &mut Self::A) -> anyhow::Result<()> {
        info!("{} watched {} terminated", context.myself(), self.0);
        Ok(())
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    addr: SocketAddrV4,
    #[arg(short, long)]
    remote_addr: Option<SocketAddrV4>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let arg = Args::parse();
    let addr = arg.addr;
    init_logger_with_filter("debug,actor=debug,actor-core::scheduler=info,h2=info,tower=info,hyper=info");
    let remote_setting = Settings {
        config: RemoteConfig { transport: Transport::tcp(addr, MessageBuffer::default()) },
        reg: MessageRegistry::new(),
    };
    let setting = ActorSetting::new(
        RemoteActorRefProvider::builder(remote_setting),
        CoreConfig::builder().build()?,
    )?;
    let system = ActorSystem::new("mikai233", setting)?;
    match arg.remote_addr {
        None => {
            system.spawn(Props::new(|| {
                Ok(RemoteActor {
                    remote_ref: None,
                })
            }), "watchee")?;
        }
        Some(remote_addr) => {
            let path = RootActorPath::new(Address::new(Protocol::Akka, "mikai233", Some(remote_addr)), "/")
                .child("user")
                .child("watchee");
            let selection = system.actor_selection(ActorSelectionPath::FullPath(path))?;
            let remote = selection.resolve_one(Duration::from_secs(3)).await?;
            system.spawn(Props::new(move || {
                Ok(RemoteActor {
                    remote_ref: Some(remote),
                })
            }), "watcher")?;
        }
    }
    system.await?;
    Ok(())
}