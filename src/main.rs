use core::{LlmClient, PatchReader, Reporter};
use std::env;


mod core;
mod llm_clients;
mod patches;
mod reporter;
mod rules;

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optopt("r", "rules", "Path to the rules markdown file", "RULES");
    opts.optopt("o", "owner", "Owner of the repository", "OWNER");
    opts.optopt("p", "repository", "Repository name", "REPOSITORY");
    opts.optopt("n", "pull", "Pull request number", "PULL_NUMBER");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };
    let config = Config {
        rules_markdown_path: matches
            .opt_str("rules")
            .unwrap_or_else(|| panic!("--rules must be set")),
        owner: matches
            .opt_str("owner")
            .unwrap_or_else(|| panic!("--owner must be set")),
        repository: matches
            .opt_str("repository")
            .unwrap_or_else(|| panic!("--repository must be set")),
        pull_number: matches
            .opt_str("pull")
            .map(|s| {
                s.parse()
                    .unwrap_or_else(|_| panic!("Failed to parse pull number"))
            })
            .unwrap_or_else(|| panic!("--pull must be set")),
    };

    let rules = rules::markdown::read(&config.rules_markdown_path);
    let llm_client = llm_clients::openai::OpenAI::new(
        env::var("OPENAI_API_KEY").unwrap_or_else(|_| panic!("OPENAI_API_KEY must be set")),
        env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string()),
    );
    let linter = core::Linter::new(llm_client, rules);
    let github_patches_client = patches::github::Github::new(
        secrecy::Secret::new(
            env::var("GITHUB_TOKEN").unwrap_or_else(|_| panic!("GITHUB_TOKEN must be set")),
        ),
        config.owner.clone(),
        config.repository.clone(),
        config.pull_number,
    );
    let patches = github_patches_client.read().await.unwrap_or_else(|e| {
        panic!("Failed to read patches: {}", e);
    });

    let indications = linter.lint(patches).await;
    let reporter = reporter::github::GithubReporter::new(
        secrecy::Secret::new(
            env::var("GITHUB_TOKEN").unwrap_or_else(|_| panic!("GITHUB_TOKEN must be set")),
        ),
        config.owner,
        config.repository,
        config.pull_number,
    );
    // let reporter = reporter::stdout::StdoutReporter::new();
    reporter.report(indications).await;
}

struct Config {
    rules_markdown_path: String,
    owner: String,
    repository: String,
    pull_number: u64,
}
