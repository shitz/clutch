### Requirement: CI status badge in README

The `README.md` SHALL include a GitHub Actions status badge that displays the current CI workflow status for the `main` branch.

#### Scenario: Badge visible in README

- **WHEN** a user views the repository on GitHub
- **THEN** the CI status badge is visible near the top of the README

#### Scenario: Badge reflects current CI state

- **WHEN** the CI workflow on `main` is passing
- **THEN** the badge displays a passing/green state

#### Scenario: Badge reflects failure state

- **WHEN** the CI workflow on `main` is failing
- **THEN** the badge displays a failing/red state
