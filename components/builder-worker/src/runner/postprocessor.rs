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

use std::path::{Path, PathBuf};

use builder_core::build_config::{BUILD_CFG_FILENAME, BuildCfg, PublishCfg};
use hab_core::package::archive::PackageArchive;
use hab_core::config::{ConfigError, ConfigFile};

use depot_client;
use super::workspace::Workspace;

pub struct PostProcessor<'a> {
    config_path: PathBuf,
    depot: &'a depot_client::Client,
}

impl<'a> PostProcessor<'a> {
    pub fn new(depot: &'a depot_client::Client, workspace: &Workspace) -> Self {
        let parent_path = Path::new(workspace.job.get_project().get_plan_path()).parent().unwrap();
        let file_path = workspace.src().join(parent_path.join(BUILD_CFG_FILENAME));
        PostProcessor {
            config_path: file_path,
            depot: depot,
        }
    }

    pub fn run(&mut self, archive: &mut PackageArchive, auth_token: &str) -> bool {
        let cfg = match BuildCfg::from_file(&self.config_path) {
            Ok(value) => value,
            Err(ConfigError::IO(_)) => BuildCfg::default(),
            Err(err) => {
                debug!("Invalid build configuration file, {}", err);
                return false;
            }
        };
        debug!("starting post processing");
        if !self.publish(&cfg.publish, archive, &auth_token) {
            return false;
        }
        true
    }

    fn publish(&self, cfg: &PublishCfg, archive: &mut PackageArchive, auth_token: &str) -> bool {
        if !cfg.enabled {
            return false;
        }
        debug!("post process: publish (url: {})", self.depot.endpoint);
        // JW TODO: publish to a channel and not to the global package namespace
        if let Some(err) = self.depot.x_put_package(archive, auth_token).err() {
            error!("post processing unable to put package, {}", err);
            return false;
        };
        true
    }
}
