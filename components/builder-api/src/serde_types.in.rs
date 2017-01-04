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

use std::net::SocketAddr;

use depot;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Public listening net address for HTTP requests
    pub http_addr: SocketAddr,
    /// Depot's configuration
    pub depot: depot::Config,
    /// List of net addresses for routing servers to connect to
    pub routers: Vec<SocketAddr>,
    /// URL to GitHub API
    pub github_url: String,
    /// Client identifier used for GitHub API requests
    pub github_client_id: String,
    /// Client secret used for GitHub API requests
    pub github_client_secret: String,
    /// Path to UI files to host over HTTP. If not set the UI will be disabled.
    pub ui_root: Option<String>,
    /// Whether to log events for funnel metrics
    pub events_enabled: bool,
}

pub mod http {
    // JW TODO: After updating to Rust 1.15, move the types contained in this module back into
    // `http/handlers.rs`

    #[derive(Clone, Serialize, Deserialize)]
    pub struct JobCreateReq {
        pub project_id: String,
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ProjectCreateReq {
        pub origin: String,
        pub plan_path: String,
        pub github: GitHubProject,
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ProjectUpdateReq {
        pub plan_path: String,
        pub github: GitHubProject,
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct GitHubProject {
        pub organization: String,
        pub repo: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename = "push")]
    pub struct GitHubWebhookPush {
        /// The full Git ref that was pushed. Example: "refs/heads/master"
        #[serde(rename = "ref")]
        pub git_ref: String,
        /// The SHA of the most recent commit on ref before the push
        pub before: String,
        /// The SHA of the most recent commit on ref after the push
        pub after: String,
        pub created: bool,
        pub deleted: bool,
        pub forced: bool,
        pub base_ref: Option<String>,
        pub compare: String,
        /// An array of commit objects describing the pushed commits (The array includes a maximum
        /// of 20 commits. If necessary, you can use the Commits API to fetch additional commits.
        /// This limit is applied to timeline events only and isn't applied to webhook deliveries)
        pub commits: Vec<GitHubWebhookCommit>,
        pub head_commit: GitHubWebhookCommit,
        pub repository: GitHubRepository,
        pub pusher: GitHubOwner,
        pub sender: GitHubWebhookSender,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GitHubWebhookCommit {
        pub id: String,
        pub tree_id: String,
        /// Whether this commit is distinct from any that have been pushed before
        pub distinct: bool,
        /// The commit message
        pub message: String,
        pub timestamp: String,
        /// Points to the commit API resource
        pub url: String,
        /// The git author of the commit
        pub author: GitHubAuthor,
        pub committer: GitHubAuthor,
        pub added: Vec<String>,
        pub removed: Vec<String>,
        pub modified: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GitHubAuthor {
        /// Public name of commit author
        pub name: String,
        /// Public email of commit author
        pub email: String,
        /// Display name of commit author
        pub username: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GitHubOwner {
        /// Public name of commit author
        pub name: String,
        /// Public email of commit author
        pub email: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GitHubRepository {
        pub id: u64,
        pub name: String,
        pub full_name: String,
        pub owner: GitHubOwner,
        pub private: bool,
        pub html_url: String,
        pub description: Option<String>,
        pub fork: bool,
        pub url: String,
        pub forks_url: String,
        pub keys_url: String,
        pub collaborators_url: String,
        pub teams_url: String,
        pub hooks_url: String,
        pub issue_events_url: String,
        pub events_url: String,
        pub assignees_url: String,
        pub branches_url: String,
        pub tags_url: String,
        pub blobs_url: String,
        pub git_tags_url: String,
        pub git_refs_url: String,
        pub trees_url: String,
        pub statuses_url: String,
        pub languages_url: String,
        pub stargazers_url: String,
        pub contributors_url: String,
        pub subscribers_url: String,
        pub subscription_url: String,
        pub commits_url: String,
        pub git_commits_url: String,
        pub comments_url: String,
        pub issue_comment_url: String,
        pub contents_url: String,
        pub compare_url: String,
        pub merges_url: String,
        pub archive_url: String,
        pub downloads_url: String,
        pub issues_url: String,
        pub pulls_url: String,
        pub milestones_url: String,
        pub notifications_url: String,
        pub labels_url: String,
        pub releases_url: String,
        pub deployments_url: String,
        pub created_at: u32,
        pub updated_at: String,
        pub pushed_at: u32,
        pub git_url: String,
        pub ssh_url: String,
        pub clone_url: String,
        pub svn_url: String,
        pub homepage: Option<String>,
        pub size: u32,
        pub stargazers_count: u32,
        pub watchers_count: u32,
        pub language: Option<String>,
        pub has_issues: bool,
        pub has_downloads: bool,
        pub has_wiki: bool,
        pub has_pages: bool,
        pub forks_count: u32,
        pub mirror_url: Option<String>,
        pub open_issues_count: u32,
        pub forks: u32,
        pub open_issues: u32,
        pub watchers: u32,
        pub default_branch: String,
        pub stargazers: u32,
        pub master_branch: String,
        pub organization: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GitHubWebhookSender {
        pub login: String,
        pub id: u64,
        pub avatar_url: String,
        pub gravatar_id: Option<String>,
        pub url: String,
        pub html_url: String,
        pub followers_url: String,
        pub following_url: String,
        pub gists_url: String,
        pub starred_url: String,
        pub subscriptions_url: String,
        pub organizations_url: String,
        pub repos_url: String,
        pub events_url: String,
        pub received_events_url: String,
        pub site_admin: bool,
    }
}
