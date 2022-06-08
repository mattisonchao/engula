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

mod cmd_delete;
mod cmd_get;
mod cmd_put;

use engula_api::server::v1::ShardDesc;

pub use self::{cmd_delete::delete, cmd_get::get, cmd_put::put};
use crate::serverpb::v1::EvalResult;

pub fn add_shard(shard: ShardDesc) -> EvalResult {
    use crate::serverpb::v1::{AddShard, SyncOp};

    #[allow(clippy::needless_update)]
    EvalResult {
        op: Some(SyncOp {
            add_shard: Some(AddShard { shard: Some(shard) }),
            ..Default::default()
        }),
        ..Default::default()
    }
}
