### Requirement: CI workflow triggers

The CI workflow SHALL run automatically on every push to the `main` branch and on every pull request targeting any branch.

#### Scenario: Push to main triggers CI

- **WHEN** a commit is pushed to the `main` branch
- **THEN** the CI workflow is triggered and all jobs run

#### Scenario: Pull request triggers CI

- **WHEN** a pull request is opened, updated, or synchronized
- **THEN** the CI workflow is triggered and all jobs run

### Requirement: Cross-platform matrix

The CI workflow SHALL execute all jobs on Linux (`ubuntu-latest`), macOS (`macos-latest`), and Windows (`windows-latest`) GitHub-hosted runners.

#### Scenario: All three platforms run

- **WHEN** the CI workflow is triggered
- **THEN** jobs run in parallel on ubuntu-latest, macos-latest, and windows-latest

### Requirement: Cargo check

The CI workflow SHALL run `cargo check` to verify the project compiles without errors on all platforms.

#### Scenario: Compilation errors fail CI

- **WHEN** the project contains a compilation error
- **THEN** the `cargo check` step fails and the overall job fails

### Requirement: Cargo test including doctests

The CI workflow SHALL run `cargo test` to execute all unit, integration, and documentation tests on all platforms.

#### Scenario: Failing test fails CI

- **WHEN** any test (unit, integration, or doctest) fails
- **THEN** the test step fails and the overall job fails

#### Scenario: All tests pass

- **WHEN** all tests pass
- **THEN** the test step succeeds

### Requirement: Cargo clippy with warnings as errors

The CI workflow SHALL run `cargo clippy -- -D warnings` to enforce lint rules, treating any warning as a build failure.

#### Scenario: Clippy warning fails CI

- **WHEN** `cargo clippy` reports one or more warnings
- **THEN** the clippy step fails and the overall job fails

#### Scenario: Clean clippy output passes

- **WHEN** `cargo clippy` reports no warnings
- **THEN** the clippy step succeeds

### Requirement: Dependency caching

The CI workflow SHALL cache Cargo registry and build artifacts keyed on the runner OS and `Cargo.lock` hash to reduce build times on repeat runs.

#### Scenario: Cache restored on second run

- **WHEN** the CI workflow runs a second time with unchanged dependencies
- **THEN** the cache is restored and build time is reduced compared to a cold build

#### Scenario: Cache invalidated on dependency change

- **WHEN** `Cargo.lock` changes
- **THEN** a new cache entry is created reflecting the updated dependencies

### Requirement: Branch protection enforces CI

The `main` branch SHALL be configured with a branch protection rule requiring the CI workflow jobs to pass before a pull request can be merged.

#### Scenario: Failing CI blocks merge

- **WHEN** the CI workflow fails on a pull request
- **THEN** the pull request cannot be merged into `main`

#### Scenario: Passing CI allows merge

- **WHEN** all CI workflow jobs pass
- **THEN** the pull request is eligible for merging
