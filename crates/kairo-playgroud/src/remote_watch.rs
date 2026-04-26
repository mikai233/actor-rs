use std::net::SocketAddrV4;
use std::time::Duration;

use async_trait::async_trait;
use clap::Parser;
use tracing::info;

use kairo_core::EmptyCodec;
use kairo_core::actor::actor_selection::ActorSelectionPath;
use kairo_core::actor::actor_system::ActorSystem;
use kairo_core::actor::address::Address;
use kairo_core::actor::context::{ActorContext, Context};
use kairo_core::actor::props::Props;
use kairo_core::actor_path::TActorPath;
use kairo_core::actor_path::root_actor_path::RootActorPath;
use kairo_core::actor_ref::ActorRef;
use kairo_core::actor_ref::actor_ref_factory::ActorRefFactory;
use kairo_core::config::ConfigBuilder;
use kairo_core::config::actor_setting::ActorSetting;
use kairo_core::config::core_config::CoreConfig;
use kairo_core::ext::init_logger_with_filter;
use kairo_core::message::message_registry::MessageRegistry;
use kairo_core::message::terminated::Terminated;
use kairo_core::{Actor, CodecMessage, DynMessage, Message};
use kairo_remote::config::RemoteConfig;
use kairo_remote::config::buffer::Buffer;
use kairo_remote::config::transport::Transport;
use kairo_remote::remote_provider::RemoteActorRefProvider;
use kairo_remote::remote_setting::RemoteSetting;

#[derive(Debug)]
struct RemoteActor {
    remote_ref: Option<ActorRef>,
}

#[async_trait]
impl Actor for RemoteActor {
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        info!("{} started", context.myself());
        if let Some(remote_ref) = &self.remote_ref {
            context.watch(remote_ref.clone(), RemoteTerminated::new)?;
            info!("{} watch remote ref {}", context.myself(), remote_ref);
        }
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

    async fn handle(
        self: Box<Self>,
        context: &mut ActorContext,
        _actor: &mut Self::A,
    ) -> anyhow::Result<()> {
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
    init_logger_with_filter(
        "debug,actor=debug,kairo-core::scheduler=info,h2=info,tower=info,hyper=info",
    );
    let remote_setting = RemoteSetting {
        config: RemoteConfig {
            transport: Transport::tcp(addr, Buffer::default()),
        },
        reg: MessageRegistry::new(),
    };
    let setting = ActorSetting::new(
        RemoteActorRefProvider::builder(remote_setting),
        CoreConfig::builder().build()?,
    )?;
    let system = ActorSystem::new("mikai233", setting)?;
    match arg.remote_addr {
        None => {
            system.spawn(
                Props::new(|| Ok(RemoteActor { remote_ref: None })),
                "watchee",
            )?;
        }
        Some(remote_addr) => {
            let path = RootActorPath::new(Address::new("tcp", "mikai233", Some(remote_addr)), "/")
                .child("user")
                .child("watchee");
            let selection = system.actor_selection(ActorSelectionPath::FullPath(path))?;
            let remote = selection.resolve_one(Duration::from_secs(3)).await?;
            system.spawn(
                Props::new(move || {
                    Ok(RemoteActor {
                        remote_ref: Some(remote),
                    })
                }),
                "watcher",
            )?;
        }
    }
    system.await?;
    Ok(())
}
