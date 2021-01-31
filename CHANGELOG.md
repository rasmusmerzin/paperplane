# Changelog

All notable changes to paperplane will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## [Unreleased]

## [0.3.2] - 2021-01-31

### Fixed

- Session dropping on server kick.

## [0.3.1] - 2021-01-31

### Added

- Methods `filter_sessions` and `filter_and_update_sessions`.

## [0.3.0] - 2021-01-20

### Added

- Implemented sessions.

### Changed

- Updated `async-std` to `1.9` and switched `futures::channel::mpsc::channel` to `async_std::channel::bounded`.

[unreleased]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.3.2...master
[0.3.2]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.3.1...v0.3.2
[0.3.1]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.3.0...v0.3.1
[0.3.0]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.2.2...v0.3.0
