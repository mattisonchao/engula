// Copyright 2022 The Engula Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::{sync::Arc, time::Duration};

use engula_api::server::v1::*;
use tracing::{debug, error, info};

use super::ActionTaskWithLocks;
use crate::schedule::{
    actions::{AddLearners, CreateReplicas, ReplaceVoters},
    provider::GroupProviders,
    scheduler::ScheduleContext,
    task::{Task, TaskState},
    tasks::{ActionTask, PROMOTE_GROUP_TASK_ID},
};

pub struct PromoteGroup {
    required_replicas: usize,
    providers: Arc<GroupProviders>,
}

impl PromoteGroup {
    pub fn new(providers: Arc<GroupProviders>) -> Self {
        PromoteGroup {
            required_replicas: 3,
            providers,
        }
    }

    async fn setup(
        &self,
        num_acquire: usize,
        former_replica_id: u64,
        ctx: &mut ScheduleContext<'_>,
    ) -> bool {
        let group_id = ctx.group_id;
        let replica_id = ctx.replica_id;

        let replicas = match self
            .alloc_addition_replicas(ctx, "promoting_group", num_acquire)
            .await
        {
            Some(replicas) => replicas,
            None => return false,
        };

        let incoming_peers = replicas.iter().map(|r| r.id).collect::<Vec<_>>();
        let mut locked_replicas = vec![former_replica_id];
        locked_replicas.extend(incoming_peers.iter());
        let new_task_id = ctx.next_task_id();
        let epoch = ctx.replica.epoch();
        let locks = ctx
            .group_lock_table
            .config_change(new_task_id, epoch, &locked_replicas, &replicas, &[])
            .expect("Check conflicts in before steps");
        let create_replicas = Box::new(CreateReplicas::new(replicas.clone()));
        let add_learners = Box::new(AddLearners {
            providers: self.providers.clone(),
            learners: replicas.clone(),
        });
        let replace_voters = Box::new(ReplaceVoters {
            providers: self.providers.clone(),
            incoming_voters: replicas,
            demoting_voters: vec![],
        });
        let promoting_task = ActionTask::new(
            new_task_id,
            vec![create_replicas, add_learners, replace_voters],
        );
        ctx.delegate(Box::new(ActionTaskWithLocks::new(locks, promoting_task)));

        info!("group {group_id} replica {replica_id} promote group by add {incoming_peers:?}");

        true
    }

    async fn alloc_addition_replicas(
        &self,
        ctx: &mut ScheduleContext<'_>,
        who: &str,
        num_required: usize,
    ) -> Option<Vec<ReplicaDesc>> {
        let group_id = ctx.group_id;
        let replica_id = ctx.replica_id;
        let req = AllocReplicaRequest {
            group_id,
            epoch: ctx.replica.epoch(),
            current_term: ctx.current_term,
            leader_id: ctx.replica_id,
            num_required: num_required as u64,
        };
        match ctx.provider.root_client.alloc_replica(req).await {
            Ok(resp) => Some(resp.replicas),
            Err(
                e @ (engula_client::Error::ResourceExhausted(_)
                | engula_client::Error::EpochNotMatch(_)),
            ) => {
                debug!(
                    "group {group_id} replica {replica_id} alloc addition replicas for {who}: {e}",
                );
                None
            }
            Err(e) => {
                error!(
                    "group {group_id} replica {replica_id} alloc addition replicas for {who}: {e}",
                );
                None
            }
        }
    }
}

#[crate::async_trait]
impl Task for PromoteGroup {
    fn id(&self) -> u64 {
        PROMOTE_GROUP_TASK_ID
    }

    async fn poll(&mut self, ctx: &mut ScheduleContext<'_>) -> TaskState {
        if ctx.group_lock_table.has_config_change() {
            return TaskState::Pending(Some(Duration::from_secs(1)));
        }

        let replicas = self.providers.descriptor.replicas();
        if replicas.len() > 1 {
            return TaskState::Terminated;
        } else if replicas.is_empty() {
            return TaskState::Pending(Some(Duration::from_secs(1)));
        }

        let former_replica_id = replicas[0].id;
        if ctx.group_lock_table.is_replica_locked(former_replica_id) {
            return TaskState::Pending(Some(Duration::from_secs(1)));
        }

        let num_online_nodes = self.providers.node.num_online_nodes();
        if 1 < self.required_replicas && self.required_replicas <= num_online_nodes {
            let num_acquire = self.required_replicas - 1;
            self.setup(num_acquire, former_replica_id, ctx).await;
        }

        TaskState::Pending(Some(Duration::from_secs(10)))
    }
}
