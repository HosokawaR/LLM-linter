use crate::core::{PatchReader, Patches};
use anyhow::Result;
use octocrab::Octocrab;
use secrecy::Secret;

pub struct Github {
    client: Octocrab,
    owner: String,
    repository: String,
    pull_number: u64,
}

impl Github {
    pub fn new(
        token: Secret<String>,
        owner: String,
        repository: String,
        pull_number: u64,
    ) -> Github {
        Github {
            client: Octocrab::builder().personal_token(token).build().unwrap(),
            owner,
            repository,
            pull_number,
        }
    }
}

impl PatchReader for Github {
    async fn read(&self) -> Result<Patches> {
        let patch = self
            .client
            .pulls(self.owner.clone(), self.repository.clone())
            .get_diff(self.pull_number)
            .await?;
        Patches::parse(&patch)
    }
}
