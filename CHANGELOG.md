# Changelog

All notable changes to this project will be documented in this file.

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
