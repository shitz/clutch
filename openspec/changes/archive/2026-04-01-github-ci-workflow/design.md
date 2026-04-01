## Context

The project is a Rust desktop application built with Iced. There is currently no automated CI pipeline — tests and lints are run manually by contributors. The repository is hosted on GitHub.

The Cargo workspace has non-trivial dependencies (Iced GUI framework, tokio async runtime, crypto crates, reqwest HTTP client) that require OS-level libraries on some platforms, making cross-platform build verification valuable.

## Goals / Non-Goals

**Goals:**

- Run `cargo check`, `cargo test --doc`, `cargo test`, and `cargo clippy` on every commit to `main` and every PR
- Validate on Linux, macOS, and Windows using GitHub-hosted runners
- Surface a status badge in `README.md`
- Enforce passing CI as a merge requirement via branch protection

**Non-Goals:**

- Release / publish artifacts or create GitHub releases
- Code coverage reporting
- Caching beyond what's needed for reasonable build times
- Nightly toolchain or benchmark runs
- Dependency auditing (can be a follow-up)

## Decisions

### Single workflow file vs. multiple workflow files

**Decision:** Single `.github/workflows/ci.yml` covering all jobs.

**Rationale:** The scope is small (one crate, no workspace split). A single file is easier to maintain and branch-protection rules target a specific workflow's job names. Splitting into multiple files adds indirection without benefit at this scale.

**Alternative considered:** Separate `check.yml`, `test.yml`, `clippy.yml` — rejected because inter-file dependencies are complex and the badge URL would need to point to one canonical file anyway.

### Matrix strategy

**Decision:** Use a `matrix` with `os: [ubuntu-latest, macos-latest, windows-latest]` and a single stable toolchain. Each job runs all four steps (check, test, clippy, doctest).

**Rationale:** Running all steps on all platforms catches platform-specific failures (e.g., linker differences, OS API availability). A single stable toolchain is sufficient; MSRV enforcement can be added later.

**Alternative considered:** Run check/clippy only on Linux to save runner minutes — rejected because the project uses platform-specific features (window decoration, file dialogs via `rfd`) that should be validated on each platform.

### Clippy configuration

**Decision:** Run `cargo clippy -- -D warnings` to fail the build on any lint warning.

**Rationale:** Treating warnings as errors enforces consistency. This is the standard approach for Rust projects and avoids gradual lint debt accumulation.

### Branch protection

**Decision:** Require the `ci` status check (the job name in the matrix) on `main`. Document this as a manual configuration step in the README or tasks rather than automating it via GitHub API.

**Rationale:** Branch protection rules cannot be set via workflow files; they require a repository admin action or the GitHub API. Documenting it as a one-time repo setup step is the lowest friction approach.

### Cargo caching

**Decision:** Use `actions/cache` to cache `~/.cargo/registry` and `target/` keyed on `os + Cargo.lock` hash.

**Rationale:** Build times for Iced-based projects are substantial. Caching the registry index and compiled dependencies significantly reduces CI time after the first run. Using `Cargo.lock` as the cache key ensures a fresh cache when dependencies change.

## Risks / Trade-offs

- [Iced/GUI tests may need a display server on Linux] → Mitigation: `cargo test` for this project appears to be unit/integration tests that don't spawn a window. If GUI tests are added later, a virtual framebuffer (Xvfb) step will be needed.
- [Windows builds can be slower and flaky] → Mitigation: Windows jobs use the same matrix; flakiness can be addressed by re-running jobs or adding `continue-on-error: false` to surface failures clearly.
- [Cache invalidation] → Mitigation: Cache key includes `Cargo.lock` hash, so dependency updates automatically bust the cache.

## Migration Plan

1. Create `.github/workflows/ci.yml`
2. Update `README.md` with badge
3. After merging, configure branch protection: Settings → Branches → Add rule for `main` → require status check `ci` (or the matrix job names)

No rollback needed — deleting the workflow file disables CI; branch protection can be removed through GitHub settings.
