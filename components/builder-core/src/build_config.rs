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

use hab_core::config::{ConfigError, ConfigFile};

pub use types::{BuildCfg, PublishCfg};
use channel::DEFAULT_CHANNEL;

/// Postprocessing config file name
pub const BUILD_CFG_FILENAME: &'static str = "builder.toml";

impl ConfigFile for BuildCfg {
    type Error = ConfigError;
}

impl Default for BuildCfg {
    fn default() -> Self {
        BuildCfg {
            triggers: vec!["./*".to_string()],
            publish: PublishCfg::default(),
        }
    }
}

impl Default for PublishCfg {
    fn default() -> Self {
        PublishCfg {
            channel: DEFAULT_CHANNEL.to_string(),
            enabled: true,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;
    use hab_core::config::ConfigFile;

    #[test]
    fn from_contents() {
        let raw = r#"
        triggers = [
            "components/builder-api",
            "components/builder-core/builder.toml"
        ]
        [publish]
        channel = "stable"
        enabled = false
        "#;
        let cfg = BuildCfg::from_str(raw).unwrap();
        assert_eq!(&cfg.triggers,
                   &["components/builder-api", "components/builder-core/builder.toml"]);
        assert_eq!(cfg.publish.channel, "stable");
        assert_eq!(cfg.publish.enabled, false);
    }
}
