# LLM linter

This is a general-purpose linter with rules base natural language powered by Large Language Model (LLM).

TODO: embed a demo video here.

## Usage

### Installation and build

```console
git clone git@github.com:HosokawaR/LLM-linter.git
cargo build --release
```

### Write rules

You need to create linter rules file. For example `rules.md` is like this.
`llm-lint-glob` is a special comment that specifies the target files. The range from the comment to the next comment is the target of the rule.

**The performance of the LLM linter depends on rules quality**. You should write rules specifically, explicitly, and concisely. It's important to make anyones understand the rules without any previous knowledge.

```md
# Rules

<!-- llm-lint-glob: src/domain/**/*.ts -->
## Domain

1. Keep business logic independent of external services.
2. Use functional programming principles where possible.

<!-- llm-lint-glob: src/repository/**/*.ts -->
## Repository

1. Ensure all database transactions are atomic.
2. Always use parameterized queries to prevent SQL injection.

<!-- llm-lint-glob: src/ui/**/*.tsx -->
## UI

1. Keep components stateless and reusable.
2. Use `useEffect` only for side effects, avoid unnecessary re-renders.
```

### Set secrets

LLM linter needs following environment variables.
**Recommend to limit the Open AI usage by web console.**

- `OPENAI_API_KEY`
- `OPENAI_MODEL` (default: `gpt-4o`)
- `GITHUB_TOKEN` (optional: only if you want to report the GitHub PR)

### Run

TODO: describe how to run on GitHub Actions.
TODO: describe how to run for local source code.

```console
./target/release/llm-linter -r rules.md --owner hosokawar --repo LLM-linter --pr 1
```
