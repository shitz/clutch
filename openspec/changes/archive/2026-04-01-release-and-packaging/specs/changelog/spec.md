## ADDED Requirements

### Requirement: CHANGELOG.md exists at repository root

A `CHANGELOG.md` file SHALL exist at the root of the repository and follow the [keepachangelog](https://keepachangelog.com/en/1.1.0/) format.

#### Scenario: File is present after implementation

- **WHEN** the repository is cloned
- **THEN** `CHANGELOG.md` exists at the root

### Requirement: Unreleased section maintained

The changelog SHALL contain an `[Unreleased]` section at the top that accumulates changes not yet associated with a version.

#### Scenario: Unreleased section appears first

- **WHEN** the changelog is opened
- **THEN** the first versioned section after the header is `[Unreleased]`

### Requirement: Initial v0.10.0 entry

The changelog SHALL contain an entry for `v0.10.0` as the first public release, documenting the initial feature set under the appropriate change type headings (`Added`, `Changed`, `Fixed`, etc.).

#### Scenario: v0.10.0 entry present

- **WHEN** the changelog is inspected
- **THEN** a `[0.10.0]` section with a release date exists below `[Unreleased]`

### Requirement: Change types use standard headings

Each version section SHALL group changes under the standard keepachangelog headings: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.

#### Scenario: Only valid headings used

- **WHEN** a new version entry is added
- **THEN** all entries appear under one of the six standard heading types

### Requirement: Semantic Versioning followed

Version numbers in the changelog SHALL conform to [Semantic Versioning](https://semver.org/) (`MAJOR.MINOR.PATCH`).

#### Scenario: Version numbers are valid semver

- **WHEN** a new release entry is added
- **THEN** the version number is in `MAJOR.MINOR.PATCH` format

### Requirement: Version headings link to GitHub Releases

Each version heading in the changelog SHALL be a Markdown link to its corresponding GitHub Release URL (`https://github.com/shitz/clutch/releases/tag/vX.Y.Z`). The `[Unreleased]` heading SHALL link to the diff between `HEAD` and the latest release tag.

#### Scenario: Released version heading is a link

- **WHEN** the changelog is rendered
- **THEN** each released version heading (e.g., `[0.10.0]`) is a hyperlink to `https://github.com/shitz/clutch/releases/tag/v0.10.0`

#### Scenario: Unreleased heading links to HEAD diff

- **WHEN** the changelog is rendered
- **THEN** the `[Unreleased]` heading links to `https://github.com/shitz/clutch/compare/v0.10.0...HEAD` (or equivalent latest tag)
