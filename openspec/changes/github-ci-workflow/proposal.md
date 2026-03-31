## Why

The project currently has no automated CI pipeline, meaning code quality and correctness are only verified locally. Adding a GitHub Actions workflow ensures every commit to `main` and every pull request is automatically built and checked, catching regressions early and giving contributors confidence before merging.

## What Changes

- Add a GitHub Actions CI workflow that runs on Linux, macOS, and Windows runners
- Workflow runs `cargo check`, `cargo test` (including doctests), and `cargo clippy` on each trigger
- Workflow triggers on all pushes to `main` and on all pull request events
- Configure branch protection on `main` to require the CI workflow to pass before merging
- Add a CI status shield to `README.md`

## Capabilities

### New Capabilities

- `github-ci`: GitHub Actions workflow for continuous integration — build, check, test, and lint the Rust codebase across Linux, macOS, and Windows

### Modified Capabilities

- `readme`: Add CI status badge to the project README

## Impact

- New file: `.github/workflows/ci.yml`
- Modified file: `README.md` (add CI badge)
- GitHub repository branch protection rules must be updated to require the CI workflow
