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
pub mod admin;
mod metrics;
pub mod node;
pub mod proxy;
pub mod raft;
pub mod root;

use std::{sync::Arc, time::Duration};

use engula_client::{ClientOptions, EngulaClient};

use crate::{
    node::{resolver::AddressResolver, Node},
    root::Root,
    Provider,
};

#[derive(Clone)]
pub struct Server {
    pub node: Arc<Node>,
    pub root: Root,
    pub address_resolver: Arc<AddressResolver>,
}

#[derive(Clone)]
pub struct ProxyServer {
    pub client: engula_client::EngulaClient,
}

impl ProxyServer {
    pub(crate) fn new(provider: &Provider) -> Self {
        let opts = ClientOptions {
            connect_timeout: Some(Duration::from_millis(250)),
            timeout: None,
        };
        ProxyServer {
            client: EngulaClient::build(
                opts,
                provider.router.clone(),
                provider.root_client.clone(),
                provider.conn_manager.clone(),
            ),
        }
    }
}
