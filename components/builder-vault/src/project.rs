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

use hab_core::package::Plan;
use protocol::vault as proto;
use regex::Regex;

use error::{Error, Result};

lazy_static! {
    static ref GITHUB_REPO_URL_RGX: Regex =
        Regex::new(r"http|https):\/\/[a-zA-Z0-9]*\.[a-zA-Z]{2,}\/(.*)\/(.*)\.git").unwrap();
}

pub trait ProjectId {
    fn project_id(&self) -> String;
}

impl ProjectId for Plan {
    fn project_id(&self) -> String {
        format!("{}/{}", self.origin, self.name)
    }
}

/// Identifies an implementor as a repository identifiable by a `String` value.
pub trait RepoIdent {
    fn repo_ident(&self) -> Result<String>;
}

impl RepoIdent for proto::Project {
    fn repo_ident(&self) -> Result<String> {
        if self.has_git() {
            return self.get_git().repo_ident();
        }
        Err(Error::UnknownVCS)
    }
}

impl RepoIdent for proto::VCSGit {
    fn repo_ident(&self) -> Result<String> {
        let captures = try!(GITHUB_REPO_URL_RGX.captures(self.get_url())
            .ok_or(Error::BadGitHubCloneURL(self.get_url().to_string())));
        Ok(format!("{}:{}", &captures[1], &captures[2]))
    }
}
