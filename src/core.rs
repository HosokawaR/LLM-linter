use std::time::Duration;

use anyhow::{anyhow, Result};
use globset::Glob;
use indoc::formatdoc;
use patch::Patch as Patch_;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

pub struct Linter<L: LlmClient> {
    llm_client: L,
    rules: Rules,
}

const SLEEP: Duration = Duration::from_secs(3);

impl<L: LlmClient> Linter<L> {
    pub fn new(llm_client: L, rules: Rules) -> Linter<L> {
        Linter { llm_client, rules }
    }

    pub async fn lint(&self, patches: Patches) -> Indications {
        let mut patch_indications = Vec::new();

        for patch in patches.all {
            let prompt = self.generate_prompt(patch.clone());
            match prompt {
                None => {
                    continue;
                }
                Some(prompt) => {
                    sleep(SLEEP).await;

                    match self.llm_client.check(patch.path.clone(), prompt).await {
                        Ok(result) => {
                            patch_indications.extend(result);
                        }
                        Err(e) => {
                            panic!("Failed to lint: {}", e);
                        }
                    }
                }
            }
        }

        Indications {
            all: patch_indications,
        }
    }

    fn generate_prompt(&self, patch: Patch) -> Option<String> {
        let rules = self.extract_rules_for(&patch);
        if rules.is_empty() {
            return None;
        }

        Some(formatdoc! {r#"
            以下は Git のパッチです。

            {}

            このパッチに対して以下のルールに違反している点がないか確認してください。
            以下のレビューで言及されたルールについてのみ指摘を行いなさい。
            また指摘後には revaluation にその指摘が適切であるかどうかを記載しなさい。

            指摘がいかなる場合も適切ならば kind を "error" にしなさい。
            指摘が場合によっては適切であるかもしれない場合は kind を "warning" にしなさい。
            指摘が適切でないことが分かった場合は kind を "cancel" に変更しなさい。

            ソースコードの左端の番号は行番号を表しています。
            また「+」で始まる行は追加された行、「-」で始まる行は削除された行を表しています。
            適切に指摘箇所の行番号を指定しなさい。
            
            {}
            
            返答は JSON のみを返すようにしなさい。
            json は以下の形式に従いなさい。

            {{
                messages: {{
                    "message": string
                    "location": {{
                        "start_line": number
                        "end_line": number
                    }}
                    "kind": "error" | "warning" | "cancel"
                }}[]
            }}

            例)

            {{
                messages: [
                    {{
                        message: "XXX の箇所は YYY に変更してください。",
                        reevalution: "実際に XXX が使用されているので、この指摘は適切である。",
                        location: {{ start_line: 10, end_line: 10 }},
                        kind: "error"
                    }},
                    {{
                        message: "XXX には必ず ZZZ をつけるようにしてください。",
                        reevalution: "実際に ZZZ はついているので、この指摘は不適切である。",
                        location: {{ start_line: 20, end_line: 30 }},
                        kind: "cancel"
                    }},
                ]
            }}
            "#,
            patch.content_with_path(),
            rules
        })
    }

    fn extract_rules_for(&self, patch: &Patch) -> String {
        self.rules
            .all
            .iter()
            .filter(|rule| {
                Glob::new(&rule.target_file_glob)
                    .unwrap_or_else(|_| panic!("Invalid glob: {}", rule.target_file_glob))
                    .compile_matcher()
                    .is_match(&patch.path)
            })
            .map(|rule| rule.content.clone())
            .collect::<Vec<String>>()
            .join("\n")
            .trim()
            .to_string()
    }
}

pub trait LlmClient {
    fn new(api_key: String, model: String) -> Self;
    async fn check(&self, path: String, prompt: String) -> Result<Vec<Indication>>;
}

pub trait Reporter {
    async fn report(&self, indications: Indications);
}

pub trait PatchReader {
    async fn read(&self) -> Result<Patches>;
}

impl Patches {
    pub fn parse(content: &str) -> Result<Patches> {
        // TODO: show original error message
        let patches =
            Patch_::from_multiple(content).map_err(|_| anyhow!("Failed to parse patches."))?;
        Ok(Patches {
            all: patches
                .iter()
                .flat_map(|patch| {
                    patch.hunks.iter().map(|hunk| Patch {
                        path: patch.new.path.to_string().replace("b/", ""),
                        content: hunk
                            .lines
                            .iter()
                            .map(|line| match line {
                                patch::Line::Context(s) => s.to_string(),
                                patch::Line::Add(s) => format!("+{}", s),
                                patch::Line::Remove(s) => format!("-{}", s),
                            })
                            .enumerate()
                            .map(|(i, line)| {
                                format!("{:4} {}", hunk.new_range.start + i as u64, line)
                            })
                            .collect::<Vec<String>>()
                            .join("\n"),
                        start_line: hunk.new_range.start,
                        end_line: hunk.new_range.start + hunk.new_range.count,
                    })
                })
                .collect(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Patches {
    pub all: Vec<Patch>,
}

#[derive(Clone, Debug)]
pub struct Patch {
    pub path: String,
    pub content: String,
    pub start_line: u64,
    pub end_line: u64,
}

impl Patch {
    pub fn content_with_path(&self) -> String {
        formatdoc! {r#"path:{}\n{}"#, self.path, self.content}
    }
}

pub struct Rules {
    pub all: Vec<Rule>,
}

pub struct Rule {
    pub target_file_glob: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct Indications {
    pub all: Vec<Indication>,
}

impl Indications {
    pub fn exclude_cancel(self) -> Vec<Indication> {
        self.all
            .into_iter()
            .filter(|indication| indication.kind != IndicationKind::Cancel)
            .collect()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Indication {
    pub kind: IndicationKind,
    pub message: String,
    pub location: Location,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum IndicationKind {
    Error,
    Warning,
    Cancel,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Location {
    pub path: String,
    pub start_line: u64,
    pub end_line: u64,
}

impl Location {
    pub fn is_single_line(&self) -> bool {
        self.start_line == self.end_line
    }
}
