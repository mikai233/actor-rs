# actor-rs

本项目的目标是移植一个最小功能的java actor
框架（[akka](https://doc.akka.io/docs/akka/current/typed/guide/introduction.html)）
到 rust 这边，大部分逻辑参考 akka 的实现方式，部分地方受限于 不同语言之间的差异以及自己的理解，采用了不同的逻辑实现相同的功能。
akka 采用的是 gossip 协议来保持集群状态的一致性， 这边没有使用 gossip ，而是采用了 etcd 来作为整个集群的配置中心，逻辑更简单一些。

# 计划实现的核心功能

- [x] actor
- [x] router
- [ ] cluster router
- [x] remote
- [x] cluster (unstable)
- [x] cluster-sharding
- [x] cluster singleton
- [ ] distributed pubsub
- [x] circuit breaker

# 使用

## 声明一个Actor

```rust
use actor_core::Actor;

struct MyActor;

impl Actor for MyActor {}
```

## 处理Actor消息

```rust
#[derive(Debug, EmptyCodec)]
struct MyMessage {
    name: String,
}

#[async_trait]
impl Message for MyMessage {
    type A = MyActor;

    async fn handle(self: Box<Self>, _context: &mut ActorContext, _actor: &mut Self::A) -> eyre::Result<()> {
        println!("{:?}", self.name);
        Ok(())
    }
}
```

## 创建Actor

```rust
#[tokio::main]
async fn main() -> eyre::Result<()> {
    let system = ActorSystem::create("mikai233", ActorSetting::default())?;
    let my_actor = system.spawn(Props::create(|_| Ok(MyActor)), "my_actor")?;
    system.await?;
    Ok(())
}
```

## 向Actor发送消息

```rust
my_actor.tell(DynMessage::user(MyMessage { name: "hello".to_string() }), ActorRef::no_sender());
```

## 几个核心的trait

### Actor

```rust
#[async_trait]
pub trait Actor: Send + Any {
    #[allow(unused_variables)]
    async fn started(&mut self, context: &mut ActorContext) -> eyre::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn stopped(&mut self, context: &mut ActorContext) -> eyre::Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_child_failure(&mut self, context: &mut ActorContext, child: &ActorRef, error: &eyre::Error) -> Directive {
        Directive::Resume
    }

    #[allow(unused_variables)]
    async fn on_recv(&mut self, context: &mut ActorContext, message: DynMessage) -> eyre::Result<Option<DynMessage>> {
        Ok(Some(message))
    }
}
```

要启动一个actor，则需要定义一个结构实现此 `trait`

### Message

```rust
pub trait CodecMessage: Any + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn into_codec(self: Box<Self>) -> Box<dyn CodecMessage>;

    fn decoder() -> Option<Box<dyn MessageDecoder>> where Self: Sized;

    fn encode(self: Box<Self>, reg: &MessageRegistry) -> eyre::Result<Vec<u8>>;

    fn clone_box(&self) -> eyre::Result<Box<dyn CodecMessage>>;

    fn cloneable(&self) -> bool;

    fn into_dyn(self) -> DynMessage;
}
```

actor消息需要实现的顶层 `trait` ，用于决定此消息需不需要序列化（如果一条消息只是本地处理，那么不需要实现序列化）以及可否进行复制

```rust
#[async_trait]
pub trait Message: CodecMessage {
    type A: Actor;

    async fn handle(self: Box<Self>, context: &mut ActorContext, actor: &mut Self::A) -> eyre::Result<()>;
}
```

发送给actor的消息需要实现 `CodecMessage` 这个 `trait` 之外还需要实现 `Message` 这个 `trait` ，这个 `trait`
决定此消息在actor中的处理逻辑。

```rust
pub trait OrphanMessage: CodecMessage {}
```

actor与actor之间的通信方式除了fire and forget（tell）方式之外，还支持request
response（ask）模式，这个时候返回的消息需要实现 `OrphanMessage`
这个 `trait`， 这个 `trait` 仅仅时作为标记作用，以确保使用者正确的使用框架的接口。 `CodeMessage` 这个 `trait`
提供了过程宏来快速实现这个 `trait` ，
以减少样板代码（参见下文）。

## 序列化与DyMessage

`DyMessage` 代表着一条actor消息，actor 收到此消息之后会根据自身的类型把 `DyMessage`
向下转型成具体的消息，然后调用 `handle` 方法处理此消息，如果此消息向下转型失败，那么就代表这个消息不属于此actor

在声明actor消息的时候，需要派生一个宏属性，来确定这个消息是否要支持序列化以及复制，例如 `EmptyCodec` 表示此消息不需要序列化以及不可以复制，
只能发送给本地的actor处理， `CEmptyCodec` 表示此消息不需要序列化但是可以进行复制，需要同时添加 `Clone` 宏。`MessageCodec`
表示此消息需要进行序列化，消息发送到远程的actor处理时需要把消息进行序列化和反序列化，此过程宏默认使用 `bincode`
进行序列化反序列化，所以需要额外添加 `bincode::Serailize` `bincode::Deserialize` 两个过程宏，如果有其它自定义的序列化需求，可以自行实现
`CodecMessage` 这个 `trait`

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

同样的，还有 `SystemCodec` `CSystemCode` `OrphanCodec` 等不同的类型， `system`
开头的属于actor的系统消息，业务中一般不使用，`orphan` 开头的属于 `ask` 消息的返回消息（response）需要派生的宏。

# Cluster Sharding

此模块是本项目比较核心的一个功能，通过 Cluster Sharding ，可以实现一个大型的集群系统，每个节点上的 actor 都可以通过一个唯一的
id 来访问，而不需要关心这个 actor 在哪个节点上。并且可以实现动态的扩容与缩容，实现的逻辑也参考 akka 的实现方式。

## 使用

sharding 的例子可以参考[sharding.rs](actor-playgroud/src/sharding.rs)

大致的使用流程为：

1. 构建 `ActorSetting` 时，向 `MessageRegistry` 中注册所有的 cluster sharding 需要的内部消息
2. 向 `ActorSystem` 中注册 `ClusterSharding` 的拓展模块
3. 定义消息路由 `MessageExtractor` ，每条消息都会调用 `MessageExtractor` 的 `entity_id` 方法，根据返回的 id
   来路由消息， `shard_id`
   用来确定这个Actor属于哪个Shard管理
4. 通过 `ClusterSharding` 的 `start` 方法启动一个 `ShardRegion` ， `ShardRegion` 是一个特殊的 Actor
   ，用来管理一组 `Shard` ， `Shard`
   是一组具有相同 `shard_id` 的 Actor

然后通过向 `ShardRegion` 发送消息， `ShardRegion` 会根据 `MessageExtractor` 的 `entity_id`
方法来路由消息，如果这个 `entity_id`
对应的 `Shard` 不存在，那么会创建一个新的 `Shard` ，然后再创建一个新的 Actor 来处理这个消息。

# 更多的例子可以参考[actor-playground](actor-playgroud/src)

## 后续计划

- [ ] 完善测试用例
- [ ] 完善文档
- [ ] 清理todo
- [ ] 优化实现逻辑

## 鸣谢

特别感谢 [JetBrains](https://www.jetbrains.com/?from=actor-rs) 为开源项目提供免费的 IDE 授权

![JetBrains logo](https://resources.jetbrains.com/storage/products/company/brand/logos/jetbrains.png)