// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

#[macro_use]
extern crate habitat_core as hab_core;
#[macro_use]
extern crate log;
extern crate serde;
extern crate statsd;
extern crate toml;

pub mod build_config;
pub mod channel;
pub mod metrics;

mod types {
    include!(concat!(env!("OUT_DIR"), "/serde_types.rs"));
}
