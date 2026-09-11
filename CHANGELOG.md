# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.1.9] - 2026-09-11

### Added

- CI workflow running fmt, clippy, tests and the feature-combination builds on
  every push and pull request
- Documentation comments on the public API, `keywords` / `categories` metadata
  for crates.io, and `#![forbid(unsafe_code)]`

### Changed

- `codegen` now emits byte-for-byte identical output for identical input; it
  used to iterate a `HashMap`, so regenerating the models reshuffled every
  entry
- `scripts/release.sh` no longer depends on GNU sed, and only rewrites the
  version in the `[package]` section
- Collapse the thirteen per-feature score lookup helpers into one

### Deprecated

- `Model::total_score()`, which duplicates the public `total_score` field.
  Read the field instead; the method will be removed in the next major release

### Fixed

- Apply the TW2/TW3/TW4 features at the last boundaries of a sentence, matching
  upstream BudouX. They were skipped or looked up with an over-long key near the
  end of the input, which changed where short sentences were split
  (e.g. `来ていた。` was returned as one chunk instead of `来て` / `いた。`)
- Fix a subtraction overflow panic (debug / `overflow-checks` builds) when
  parsing an input of exactly two characters

### Security

- Install `wasm-pack` from a pinned crates.io release in the release workflow,
  instead of piping an unpinned installer script into `sh` in a job that holds
  `contents: write` and the crates.io OIDC token

## [0.1.8] - 2026-07-06

### Changed

- Update `phf` dependency to 0.14.0 and regenerate all model files

### Added

- WASM release workflow and WASM usage documentation in README

### Security

- Set `persist-credentials: false` in GitHub Actions release workflow

## [0.1.7]

- intanal release

## [0.1.6] - 2026-05-08

### Added

- Automated bump-version GitHub Actions workflow

## [0.1.5] - 2026-05-08

### Added

- Release automation script and CI workflow

### Security

- Use SHA-pinned actions in GitHub Actions workflows

## [0.1.4] - 2026-05-08

### Added

- `no_std` support via `parse_with` (heap-free, works without alloc)
- WASM target support
- Test step in CI
- Release workflow

### Changed

- Remove unused features

## [0.1.2] - 2026-02-05

### Fixed

- Fix last character handling in parser

## [0.1.1] - 2026-01-30
