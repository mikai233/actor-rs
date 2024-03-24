use actor_core::actor_ref::ActorRef;

use crate::shard_region::ImShardId;

#[derive(Debug)]
pub(super) enum StateUpdate {
    // ShardHomeDeallocated {
    //     shard: ImShardId,
    // },
    ShardRegionProxyTerminated {
        region_proxy: ActorRef,
    },
    ShardCoordinatorInitialized,
    ShardRegionTerminated {
        region: ActorRef,
    },
    ShardRegionProxyRegistered {
        region_proxy: ActorRef,
    },
    ShardHomeAllocated {
        shard: ImShardId,
        region: ActorRef,
    },
    ShardRegionRegistered {
        region: ActorRef,
    },
}