# Contributing to ShellQL

Thanks for your interest in contributing ❤️

ShellQL is in **beta**, and contributions are very welcome.

## Ground rules

- Be respectful and constructive.
- Keep PRs focused and reasonably small.
- Prefer incremental improvements over large rewrites.
- If behavior changes, update docs/help text in the same PR when possible.

## Development setup

### Prerequisites

- Rust (stable)
- Cargo
- Docker + Docker Compose (for SQL integration tests)

### Run locally

```bash
cargo run
```

## Quality checks (before opening a PR)

Please run:

```bash
cargo fmt --all
cargo check --all-targets
cargo test --lib
cargo test --test connections --test validation
```

If your change touches SQL execution/DB behavior, also run:

```bash
docker compose -f tests/docker-compose.yml up -d
cargo test --test sql_integration --test sql_prebuilt_queries_integration
docker compose -f tests/docker-compose.yml down -v
```

## Pull request process

1. Create a branch from `main`.
2. Make your changes (code + tests/docs as needed).
3. Run the checks above.
4. Open a PR with:
   - clear summary
   - motivation/context
   - screenshots/gifs for TUI UX changes (if applicable)
   - testing notes

CI must pass before merge.

## Coding notes

- Follow existing Rust style and module boundaries.
- Keep user-facing behavior consistent unless explicitly changing UX.
- Prefer descriptive error messages.
- Avoid unrelated refactors in feature/fix PRs.

## Reporting issues

When filing a bug, include:

- OS + terminal
- ShellQL version/commit
- reproduction steps
- expected vs actual behavior
- logs/error output

Thanks again for helping improve ShellQL 🚀
