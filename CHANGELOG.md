# Changelog

All notable changes to paperplane will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0),
and this project adheres to [Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## [Unreleased]

### Added

- `connections` method
- `next_transform` & `send_transform` methods

### Changed

- `Event` generic over `Message`
- `next`, `send` & `kick` methods

### Removed

- Sessions
- `send_all`, `kick_all`, `send_map` & `kick_map` methods

### Fixed

- Utilized multiproducer part of mpsc

## [0.3.2] - 2021-01-31

### Fixed

- Session dropping on server kick

## [0.3.1] - 2021-01-31

### Added

- Methods `filter_sessions` and `filter_and_update_sessions`

## [0.3.0] - 2021-01-20

### Added

- Sessions

### Changed

- `futures::channel::mpsc::channel` to `async_std::channel::bounded`

[unreleased]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.3.2...master
[0.3.2]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.3.1...v0.3.2
[0.3.1]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.3.0...v0.3.1
[0.3.0]: https://gitlab.com/rasmusmerzin/paperplane/compare/v0.2.2...v0.3.0
