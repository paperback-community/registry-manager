# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Features

- Pretty print the versioning file (@celarye)
- Added repository info to the commit message (@celarye)
- Sorted the extensions alphabetically in both the versioning and metadata files (@celarye)

### Bug Fixes

- Fixed GitHub API file size limit issue with object fetching (@celarye)

### Miscellaneous

- Fixed a broken link in the changelog (@celarye)
- Switched to Clippy (@celarye)

## [v0.2.1] - 2025-03-10

### Bug fixes

- Extension deletion fixes (#3, @Celarye)

### Miscellaneous

- Added a GitHub test workflow for pull requests (#3, @Celarye)

## [v0.2.0] - 2025-03-09

### Features

- Ensure a clean exit (exit code 0) when no extensions need updating, display a warning instead (#2, @Celarye)
- Added extension deletion support by rewriting the versioning update logic (#2, @Celarye)

### Bug fixes

- Extension duplication (#2, @Celarye)

### Miscellaneous

- General code structure improvements (#2, @Celarye)

## [v0.1.0] - 2025-02-24

### Features

- Base registry manager tool (@Celarye)
- A GitHub workflow which releases a binary of the tool on pushed tags (@Celarye)
- `action.yml` to make the tool available through a GitHub Action (@Celarye)
- GitHub Issue templates (@Celarye)
- GitHub support, security and contributing guidelines (@Celarye)

[Unreleased]: https://github.com/paperback-community/registry-manager/compare/v0.2.1...HEAD
[v0.2.1]: https://github.com/paperback-community/registry-manager/compare/v0.2.0...v0.2.1
[v0.2.0]: https://github.com/paperback-community/registry-manager/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/paperback-community/registry-manager/releases/tag/v0.1.0

