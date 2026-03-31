## 1. GitHub Actions Workflow

- [x] 1.1 Create `.github/workflows/ci.yml` with the workflow definition
- [x] 1.2 Configure workflow triggers: `push` on `main`, `pull_request` on all branches
- [x] 1.3 Define a matrix strategy with `os: [ubuntu-latest, macos-latest, windows-latest]`
- [x] 1.4 Add `actions/cache` step for `~/.cargo/registry` and `target/` keyed on OS + `Cargo.lock` hash
- [x] 1.5 Add `cargo check` step
- [x] 1.6 Add `cargo test` step (includes doctests by default)
- [x] 1.7 Add `cargo clippy -- -D warnings` step

## 2. README Badge

- [x] 2.1 Add GitHub Actions status badge for the `ci.yml` workflow targeting `main` to `README.md`

## 3. Branch Protection

- [x] 3.1 In GitHub repository settings, configure branch protection for `main`: require the CI status checks to pass before merging
- [x] 3.2 Enable "Require branches to be up to date before merging" for the `main` protection rule
