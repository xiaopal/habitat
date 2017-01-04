// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::net::SocketAddr;

use protocol::sharding::ShardId;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// List of net addresses for routing servers to connect to.
    pub routers: Vec<SocketAddr>,
    /// Net address to the persistent datastore.
    pub datastore_addr: SocketAddr,
    /// Connection retry timeout in milliseconds for datastore.
    pub datastore_retry_ms: u64,
    /// Number of database connections to start in pool.
    pub pool_size: u32,
    /// Router's heartbeat port to connect to.
    pub heartbeat_port: u16,
    /// List of shard identifiers serviced by the running service.
    pub shards: Vec<ShardId>,
    /// Number of threads to process queued messages.
    pub worker_threads: usize,
}
