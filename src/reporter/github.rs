use std::time::Duration;

use crate::core::{Indication, Indications, Reporter};
use log::{debug, warn};
use octocrab::Octocrab;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

pub struct GithubReporter {
    client: Octocrab,
    owner: String,
    repository: String,
    pull_number: u64,
    token: Secret<String>,
}

impl Reporter for GithubReporter {
    async fn report(&self, indications: Indications) {
        for indication in indications.exclude_cancel().exclude_warnings().values {
            self.comment(indication).await;
            sleep(Duration::from_secs(1)).await;
        }
    }
}

impl GithubReporter {
    pub fn new(
        token: Secret<String>,
        owner: String,
        repository: String,
        pull_number: u64,
    ) -> GithubReporter {
        // TODO: Secret 活用できなてくない…？
        let secret = token.expose_secret();
        GithubReporter {
            client: Octocrab::builder()
                .personal_token(secret.clone())
                .build()
                .unwrap(),
            owner,
            repository,
            pull_number,
            token: Secret::new(secret.to_string()),
        }
    }

    async fn comment(&self, indication: Indication) {
        let comment_request = &CommentRequest {
            body: self.add_suffix(indication.message.clone()),
            commit_id: self.fetch_latest_commit_sha().await,
            path: indication.location.path.clone(),
            start_line: if indication.location.is_single_line() {
                None
            } else {
                Some(indication.location.start_line)
            },
            start_side: if indication.location.is_single_line() {
                None
            } else {
                Some("RIGHT".to_string())
            },
            line: indication.location.end_line,
            side: "RIGHT".to_string(),
        };

        let response = reqwest::Client::new()
            .post(&format!(
                "https://api.github.com/repos/{}/{}/pulls/{}/comments",
                self.owner, self.repository, self.pull_number
            ))
            .bearer_auth(self.token.expose_secret())
            // https://github.com/seanmonstar/reqwest/issues/918
            .header("User-Agent", "Rust")
            .header("Accept", "application/vnd.github.v3+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(comment_request)
            .send()
            .await
            .unwrap_or_else(|e| panic!("Failed to send request: {}", e));

        if response.status().is_client_error() || response.status().is_server_error() {
            warn!("GitHub API returned an error: {:?}", response.text().await);
            warn!(
                "indication: {:?}",
                serde_json::to_string(&indication).unwrap()
            );
        }
    }

    async fn fetch_latest_commit_sha(&self) -> String {
        let pull = self
            .client
            .pulls(self.owner.clone(), self.repository.clone())
            .get(self.pull_number)
            .await
            .unwrap();
        debug!("pull.head.sha: {:?}", pull.head.sha);
        pull.head.sha
    }

    fn add_suffix(&self, message: String) -> String {
        format!(
            "{}\n\n{}",
            message, "Reported by [LLM linter](https://github.com/HosokawaR/LLM-linter)"
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CommentRequest {
    body: String,
    commit_id: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_side: Option<String>,
    line: u64,
    side: String,
}
