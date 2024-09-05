use crate::core::{Rule, Rules};
use regex::Regex;
use std::fs::read_to_string;

pub fn read(path: &str) -> Rules {
    let markdown = read_to_string(path).unwrap_or_else(|_| panic!("Could not read file: {}", path));
    parse(&markdown)
}

fn parse(raw: &str) -> Rules {
    let mut rules_by_globs: Vec<(String, String)> = Vec::new();

    let re = Regex::new(r"<!--\s*llm-lint-glob: (.*?)\s*-->").unwrap();
    let mut current_glob = None;
    for line in raw.split('\n') {
        match re.captures(line) {
            Some(c) => {
                let globs = c
                    .get(1)
                    .unwrap_or_else(|| panic!("Could not parse glob: {}", line))
                    .as_str();
                current_glob = Some(globs);
            }
            None => {
                if let Some(glob) = current_glob {
                    let rules_opt = rules_by_globs.iter_mut().find(|(g, _)| g == glob);
                    if let Some(rules) = rules_opt {
                        rules.1.push_str(format!("\n{}", line).as_str());
                    } else {
                        rules_by_globs.push((glob.to_string(), String::new()));
                        rules_by_globs.last_mut().unwrap().1.push_str(line);
                    }
                }
            }
        }
    }

    Rules {
        all: rules_by_globs
            .into_iter()
            .map(|(glob, rules)| Rule {
                target_file_glob: glob,
                content: rules,
            })
            .collect::<Vec<Rule>>(),
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn test_parse() {
        let markdown = indoc! {"
            # Rules 

            <!-- llm-lint-glob: src/**/*.rs -->
            ## for Rust

            - Do not use `unwrap` in production code
            - Do not use `expect` in production code

            <!-- llm-lint-glob: src/**/*.md -->
            ## for Markdown

            - Must have a title
            - Should use native markdown syntax
        "};

        let rules = parse(markdown);

        assert_eq!(rules.all.len(), 2);
        assert_eq!(rules.all[0].target_file_glob, "src/**/*.rs");
        assert_eq!(
            rules.all[0].content,
            indoc! {"
                ## for Rust

                - Do not use `unwrap` in production code
                - Do not use `expect` in production code
            "}
        );
        assert_eq!(rules.all[1].target_file_glob, "src/**/*.md");
        assert_eq!(
            rules.all[1].content,
            indoc! {"
                ## for Markdown

                - Must have a title
                - Should use native markdown syntax
            "}
        );
    }
}
