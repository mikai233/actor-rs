# actor-rs

> ### ⚠️ Project Status: On Hold / Blocked
>
> This project is currently blocked and will not be updated for the time being. I am dissatisfied with the current code architecture and believe it is unreasonable. A complete redesign is needed, which will involve significant code changes. I currently do not have enough time to dedicate to this major overhaul.

The goal of this project is to port a minimal functional Scala Actor framework ([akka](https://doc.akka.io/docs/akka/current/typed/guide/introduction.html)) to Rust. Most of the logic refers to Akka's implementation, while some parts adopt different logic to achieve the same functionality due to language differences and my own understanding.

Akka uses the Gossip protocol to maintain cluster state consistency. Currently, this project uses etcd as the configuration center for the entire cluster instead of Gossip. The Gossip protocol will be implemented in the future.

# Core Planned Features

- [x] actor
- [x] router
- [ ] cluster router
- [x] remote
- [x] cluster (unstable)
- [x] cluster-sharding
- [x] cluster singleton
- [ ] distributed pubsub
- [x] circuit breaker

# Usage

## Declaring an Actor

```rust
use actor_core::Actor;

struct MyActor;

impl Actor for MyActor {}
```

## Handling Actor Messages

```rust
#[derive(Debug, EmptyCodec)]
struct MyMessage {
    name: String,
}

#[async_trait]
impl Message for MyMessage {
    type A = MyActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> anyhow::Result<()> {
        println!("{:?}", self.name);
        Ok(())
    }
}
```

## Creating an Actor

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let system = ActorSystem::create("mikai233", ActorSetting::default())?;
    let my_actor = system.spawn(Props::create(|_| Ok(MyActor)), "my_actor")?;
    system.await?;
    Ok(())
}
```

## Sending Messages to an Actor

```rust
my_actor.tell(DynMessage::user(MyMessage { name: "hello".to_string() }), ActorRef::no_sender());
```

## Core Traits

### Actor

```rust
#[async_trait]
pub trait Actor: Send + Any {
    #[allow(unused_variables)]
    async fn started(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn stopped(&mut self, context: &mut ActorContext) -> anyhow::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_child_failure(&mut self, context: &mut ActorContext, child: &ActorRef, error: &anyhow::Error) -> Directive {
        Directive::Resume
    }

    #[allow(unused_variables)]
    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> anyhow::Result<Option<DynMessage>> {
        Ok(Some(message))
    }
}
```

To start an actor, you need to define a struct that implements this `trait`.

### Message

```rust
pub trait CodecMessage: Any + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn into_codec(self: Box<Self>) -> Box<dyn CodecMessage>;

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized;

    fn encode(self: Box<Self>, reg: &MessageRegistry) -> anyhow::Result<Vec<u8>>;

    fn clone_box(&self) -> anyhow::Result<Box<dyn CodecMessage>>;

    fn cloneable(&self) -> bool;

    fn into_dyn(self) -> DynMessage;
}
```

The top-level `trait` that actor messages need to implement. It determines whether the message needs serialization (if a message is only handled locally, serialization is not required) and whether it can be cloned.

```rust
#[async_trait]
pub trait Message: CodecMessage {
    type A: Actor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> anyhow::Result<()>;
}
```

Messages sent to an actor must implement the `Message` trait in addition to the `CodecMessage` trait. This trait determines the handling logic of the message within the actor.

```rust
pub trait OrphanMessage: CodecMessage {}
```

In addition to fire-and-forget (tell), communication between actors also supports the request-response (ask) pattern. In this case, the returned message needs to implement the `OrphanMessage` trait. This trait serves only as a marker to ensure the user correctly uses the framework's interface. The `CodecMessage` trait provides procedural macros to quickly implement this trait and reduce boilerplate code (see below).

## Serialization and DynMessage

`DynMessage` represents an actor message. After receiving this message, the actor will downcast `DynMessage` to a specific message type based on its own type and then call the `handle` method. If the downcast fails, it means the message does not belong to this actor.

When declaring an actor message, you need to derive a macro attribute to determine whether the message supports serialization and cloning. For example, `EmptyCodec` indicates that the message does not need serialization and cannot be cloned (can only be sent to local actors). `CEmptyCodec` indicates no serialization but supports cloning (requires the `Clone` macro). `MessageCodec` indicates that serialization is required; when sent to a remote actor, the message will be serialized and deserialized. This procedural macro uses `bincode` by default, so you need to add `bincode::Serialize` and `bincode::Deserialize`. For other custom serialization needs, you can implement the `CodecMessage` trait yourself.

```rust
pub trait CodecMessage: Any + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized;

    fn encode(&self, reg: &MessageRegistry) -> Result<Vec<u8>, EncodeError>;

    fn dyn_clone(&self) -> Option<DynMessage>;

    fn is_cloneable(&self) -> bool;
}
```

Similarly, there are different types like `SystemCodec`, `CSystemCodec`, `OrphanCodec`, etc. Those starting with `system` are actor system messages (generally not used in business logic). Those starting with `orphan` are messages returned by `ask` (response) that need to be derived.

# Cluster Sharding

This module is a core feature of this project. Through Cluster Sharding, a large-scale cluster system can be achieved. Actors on each node can be accessed through a unique ID without worrying about which node the actor is on. It also enables dynamic expansion and contraction. The implementation logic refers to Akka.

## Usage

For sharding examples, please refer to [sharding.rs](actor-playgroud/src/sharding.rs).

The general usage flow is:

1. When building `ActorSetting`, register all internal messages required for cluster sharding in the `MessageRegistry`.
2. Register the `ClusterSharding` extension module in the `ActorSystem`.
3. Define the message router `MessageExtractor`. Each message will call the `entity_id` method of `MessageExtractor`. Messages are routed based on the returned ID. `shard_id` is used to determine which Shard this Actor belongs to.
4. Start a `ShardRegion` via the `start` method of `ClusterSharding`. `ShardRegion` is a special Actor that manages a group of `Shard`s. A `Shard` is a group of Actors with the same `shard_id`.

Then, by sending messages to `ShardRegion`, it routes messages according to the `entity_id` method of `MessageExtractor`. If the `Shard` corresponding to the `entity_id` does not exist, a new `Shard` will be created, followed by a new Actor to handle the message.

# More examples can be found in [actor-playground](actor-playgroud/src)

## Future Plans

- [ ] Improve test cases
- [ ] Improve documentation
- [ ] Clean up TODOs
- [ ] Optimize implementation logic

## Acknowledgments

Special thanks to [JetBrains](https://www.jetbrains.com/?from=actor-rs) for providing free IDE licenses for open-source projects.

![JetBrains logo](https://resources.jetbrains.com/storage/products/company/brand/logos/jetbrains.png)