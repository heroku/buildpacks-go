# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.1.6] - 2025-12-04

- Added go1.24.11 (linux-amd64), go1.24.11 (linux-arm64), go1.25.5 (linux-amd64), go1.25.5 (linux-arm64).

## [2.1.5] - 2025-11-06

- Added go1.24.10 (linux-amd64), go1.24.10 (linux-arm64), go1.25.4 (linux-amd64), go1.25.4 (linux-arm64).

## [2.1.4] - 2025-10-14

- Added go1.24.9 (linux-amd64), go1.24.9 (linux-arm64), go1.25.3 (linux-amd64), go1.25.3 (linux-arm64).

## [2.1.3] - 2025-10-07

- Added go1.24.8 (linux-amd64), go1.24.8 (linux-arm64), go1.25.2 (linux-amd64), go1.25.2 (linux-arm64).

## [2.1.2] - 2025-09-08

- Added go1.24.7 (linux-amd64), go1.24.7 (linux-arm64), go1.25.1 (linux-amd64), go1.25.1 (linux-arm64).

## [2.1.1] - 2025-08-14

- Added go1.25.0 (linux-amd64), go1.25.0 (linux-arm64).

## [2.1.0] - 2025-08-08

- Added go1.23.12 (linux-amd64), go1.23.12 (linux-arm64), go1.24.6 (linux-amd64), go1.24.6 (linux-arm64), go1.25rc3 (linux-amd64), go1.25rc3 (linux-arm64).

## [2.0.0] - 2025-08-04

### Changed

- The buildpack now passes user-provided environment variables to subprocesses such as `go install`. ([#388](https://github.com/heroku/buildpacks-go/pull/388))

## [1.0.4] - 2025-07-11

- The buildpack will now retry downloading Go up to 4 times ([#386](https://github.com/heroku/buildpacks-go/pull/386))

## [1.0.3] - 2025-07-09

- Added go1.23.11 (linux-amd64), go1.23.11 (linux-arm64), go1.24.5 (linux-amd64), go1.24.5 (linux-arm64), go1.25rc2 (linux-amd64), go1.25rc2 (linux-arm64).

## [1.0.2] - 2025-06-13

- Added go1.23.10 (linux-amd64), go1.23.10 (linux-arm64), go1.24.4 (linux-amd64), go1.24.4 (linux-arm64), go1.25rc1 (linux-amd64), go1.25rc1 (linux-arm64).

## [1.0.1] - 2025-05-14

- Added go1.23.9 (linux-amd64), go1.23.9 (linux-arm64), go1.24.3 (linux-amd64), go1.24.3 (linux-arm64).

## [1.0.0] - 2025-04-24

### Changed

- Update build output style ([#368](https://github.com/heroku/buildpacks-go/pull/368))

## [0.5.8] - 2025-04-02

- Added go1.23.8 (linux-amd64), go1.23.8 (linux-arm64), go1.24.2 (linux-amd64), go1.24.2 (linux-arm64).

## [0.5.7] - 2025-03-26

- No changes.

## [0.5.6] - 2025-03-04

- Added go1.23.7 (linux-amd64), go1.23.7 (linux-arm64), go1.24.1 (linux-amd64), go1.24.1 (linux-arm64).

## [0.5.5] - 2025-03-03

### Changed

- Updated libcnb to 0.27.0, which includes opentelemetry 0.28 and the new custom OTLP File Exporter. ([#345](https://github.com/heroku/buildpacks-go/pull/345))

## [0.5.4] - 2025-02-12

### Added

- Added go1.24.0 (linux-amd64), go1.24.0 (linux-arm64).

## [0.5.3] - 2025-02-06

### Added

- Added go1.22.12 (linux-amd64), go1.22.12 (linux-arm64), go1.23.6 (linux-amd64), go1.23.6 (linux-arm64), go1.24rc3 (linux-amd64), go1.24rc3 (linux-arm64).

## [0.5.2] - 2025-01-21

### Added

- Added go1.22.11 (linux-amd64), go1.22.11 (linux-arm64), go1.23.5 (linux-amd64), go1.23.5 (linux-arm64), go1.24rc2 (linux-amd64), go1.24rc2 (linux-arm64).

## [0.5.1] - 2025-01-06

### Added

- Added go1.24rc1 (linux-amd64), go1.24rc1 (linux-arm64).

## [0.5.0] - 2024-12-13

### Changed

- Now prefers processes set by Procfile, and no longer adds it's own processes if a Procfile is present.

## [0.4.7] - 2024-12-06

### Added

- Added go1.22.10 (linux-amd64), go1.22.10 (linux-arm64), go1.23.4 (linux-amd64), go1.23.4 (linux-arm64).

## [0.4.6] - 2024-11-12

### Added

- Added go1.22.9 (linux-amd64), go1.22.9 (linux-arm64), go1.23.3 (linux-amd64), go1.23.3 (linux-arm64).

## [0.4.5] - 2024-10-01

### Added

- Added go1.22.8 (linux-amd64), go1.22.8 (linux-arm64), go1.23.2 (linux-amd64), go1.23.2 (linux-arm64).

## [0.4.4] - 2024-09-05

### Added

- Added go1.22.7 (linux-amd64), go1.22.7 (linux-arm64), go1.23.1 (linux-amd64), go1.23.1 (linux-arm64).

## [0.4.3] - 2024-08-13

### Added

- Added go1.23.0 (linux-amd64), go1.23.0 (linux-arm64).

## [0.4.2] - 2024-08-07

### Added

- Added go1.21.13 (linux-arm64), go1.21.13 (linux-amd64), go1.22.6 (linux-amd64), go1.22.6 (linux-arm64).
- Added go1.23rc2 (linux-arm64), go1.23rc2 (linux-amd64).

## [0.4.1] - 2024-07-15

### Added

- Added go1.21.12 (linux-amd64), go1.21.12 (linux-arm64), go1.22.5 (linux-amd64), go1.22.5 (linux-arm64), go1.23rc1 (linux-arm64), go1.23rc1 (linux-amd64).

## [0.4.0] - 2024-06-04

### Added

- Added go1.21.11 (linux-amd64), go1.21.11 (linux-arm64).
- Added go1.22.4 (linux-arm64), go1.22.4 (linux-amd64).

### Changed

- The build cache is now invalidated when the target distribution changes. ([#267](https://github.com/heroku/buildpacks-go/pull/267))
- The build cache is no longer invalidated on minor go version changes. ([#267](https://github.com/heroku/buildpacks-go/pull/267))

## [0.3.1] - 2024-05-07

### Added

- Added go1.21.10 (linux-amd64), go1.21.10 (linux-arm64), go1.22.3 (linux-amd64), go1.22.3 (linux-arm64).

## [0.3.0] - 2024-05-02

### Added

- Support for `arm64` and multi-arch images. ([#261](https://github.com/heroku/buildpacks-go/pull/261))

## [0.2.1] - 2024-04-05

### Added

- Added go1.21.9 (linux-aarch64), go1.21.9 (linux-x86_64), go1.22.2 (linux-aarch64), go1.22.2 (linux-x86_64).
- Added linux aarch64 artifacts for >= go1.8.5. ([#230](https://github.com/heroku/buildpacks-go/pull/230))
- Added linux/arm64 buildpack target. ([#233](https://github.com/heroku/buildpacks-go/pull/233))

### Changed

- Implement Buildpack API 0.10. ([#231](https://github.com/heroku/buildpacks-go/pull/231))

## [0.2.0] - 2024-03-06

### Added

- Added go1.21.8, go1.22.1.
- Added go1.9.2rc2, go1.6rc2, go1.6rc1, go1.6beta2. ([#216](https://github.com/heroku/buildpacks-go/pull/216))

### Changed

- Changed inventory utils to use upstream release feed. ([#216](https://github.com/heroku/buildpacks-go/pull/216))
- The buildpack now installs Go from upstream (rather than mirrored) binaries. ([#216](https://github.com/heroku/buildpacks-go/pull/216))

### Removed

- Removed go1.21rc1, go1.8.5rc4, go1.7.2, go1.5.2, go1.5.1, go1.5, go1.4.3, go1.4.2, go1.4.1, go1.4, go1.3.3, go1.3.2, go1.3.1, go1.3, go1.2.2, go1.2.1, go1.2, go1.1.2, go1.1.1, go1.1, go1.0.3, go1.0.2, go1.0.1. ([#216](https://github.com/heroku/buildpacks-go/pull/216))

## [0.1.16] - 2024-02-08

### Added

- Added go1.20.14, go1.21.7, go1.22.0.

## [0.1.15] - 2024-02-06

### Added

- Added go1.22rc2.

## [0.1.14] - 2024-01-16

### Added

- Added go1.20.13, go1.21.6.

### Changed

- Separated buildpack and supplementary binaries into independent crates. ([#200](https://github.com/heroku/buildpacks-go/pull/200))

## [0.1.13] - 2024-01-03

### Added

- Enabled tracing/telemetry via libcnb `trace` flag. ([#198](https://github.com/heroku/buildpacks-go/pull/198))
- Added go1.20.12, go1.21.5, and go1.22rc1. ([#196](https://github.com/heroku/buildpacks-go/pull/196))

## [0.1.12] - 2023-11-13

### Added

- Added go1.20.11 and go1.21.4. ([#168](https://github.com/heroku/buildpacks-go/pull/184))

## [0.1.11] - 2023-10-24

### Added

- Added buildpack description to metadata used by CNB registry. ([#178](https://github.com/heroku/buildpack-go/pull/178))

## [0.1.10] - 2023-10-12

### Added

- Added go1.20.10, go1.20.9, go1.21.2, go1.21.3. ([#168](https://github.com/heroku/buildpacks-go/pull/168))

## [0.1.9] - 2023-09-18

### Added

- Added go1.19.13, go1.20.8, go1.21.1. ([#154](https://github.com/heroku/buildpacks-go/pull/154))

## [0.1.8] - 2023-08-15

### Added

- Added go1.21.0. ([#136](https://github.com/heroku/buildpacks-go/pull/136))

### Fixed

- The `$GOROOT/go.env` file is now correctly installed. ([#137](https://github.com/heroku/buildpacks-go/pull/137))

## [0.1.7] - 2023-08-07

### Added

- Added go1.19.12, go1.20.7, go1.21rc4. ([#132](https://github.com/heroku/buildpacks-go/pull/132))

## [0.1.6] - 2023-08-01

### Added

- Added go1.19.11, go1.20.6, go1.21rc1, go1.21rc2, go1.21rc3. ([#105](https://github.com/heroku/buildpacks-go/pull/105))

## [0.1.5] - 2023-06-27

### Added

- Added go1.19.10, go1.20.5. ([#102](https://github.com/heroku/buildpacks-go/pull/102))

### Changed

- The buildpack now implements Buildpack API 0.9 instead of 0.8, and so requires `lifecycle` 0.15.x or newer. ([#101](https://github.com/heroku/buildpacks-go/pull/101))

## [0.1.4] - 2023-05-09

### Added

- Added go1.19.9, go1.20.4. ([#92](https://github.com/heroku/buildpacks-go/pull/92))

## [0.1.3] - 2023-04-11

### Added

- Added go1.19.8, go1.20.3. ([#83](https://github.com/heroku/buildpacks-go/pull/83))
- Added go1.19.6, go1.19.7, go1.20.1, go1.20.2. ([#75](https://github.com/heroku/buildpacks-go/pull/75))

## [0.1.2] - 2023-02-06

### Added

- Added go1.20. ([#65](https://github.com/heroku/buildpacks-go/pull/65))

## [0.1.1] - 2023-01-23

### Added

- Added go1.19.5, go1.19.4, go1.19.3, go1.19.2, go1.19.1, go1.19. ([#57](https://github.com/heroku/buildpacks-go/pull/57))
- Added go1.18.10, go1.18.9, go1.18.7, go1.18.6, go1.18.5, go1.18.4. ([#57](https://github.com/heroku/buildpacks-go/pull/57))
- Added go1.17.13, go1.17.12. ([#57](https://github.com/heroku/buildpacks-go/pull/57))

## [0.1.0] - 2022-12-01

### Added

- Initial implementation using libcnb.rs. ([#1](https://github.com/heroku/buildpacks-go/pull/1))

[unreleased]: https://github.com/heroku/buildpacks-go/compare/v2.1.6...HEAD
[2.1.6]: https://github.com/heroku/buildpacks-go/compare/v2.1.5...v2.1.6
[2.1.5]: https://github.com/heroku/buildpacks-go/compare/v2.1.4...v2.1.5
[2.1.4]: https://github.com/heroku/buildpacks-go/compare/v2.1.3...v2.1.4
[2.1.3]: https://github.com/heroku/buildpacks-go/compare/v2.1.2...v2.1.3
[2.1.2]: https://github.com/heroku/buildpacks-go/compare/v2.1.1...v2.1.2
[2.1.1]: https://github.com/heroku/buildpacks-go/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/heroku/buildpacks-go/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/heroku/buildpacks-go/compare/v1.0.4...v2.0.0
[1.0.4]: https://github.com/heroku/buildpacks-go/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/heroku/buildpacks-go/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/heroku/buildpacks-go/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/heroku/buildpacks-go/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/heroku/buildpacks-go/compare/v0.5.8...v1.0.0
[0.5.8]: https://github.com/heroku/buildpacks-go/compare/v0.5.7...v0.5.8
[0.5.7]: https://github.com/heroku/buildpacks-go/compare/v0.5.6...v0.5.7
[0.5.6]: https://github.com/heroku/buildpacks-go/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/heroku/buildpacks-go/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/heroku/buildpacks-go/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/heroku/buildpacks-go/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/heroku/buildpacks-go/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/heroku/buildpacks-go/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/heroku/buildpacks-go/compare/v0.4.7...v0.5.0
[0.4.7]: https://github.com/heroku/buildpacks-go/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/heroku/buildpacks-go/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/heroku/buildpacks-go/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/heroku/buildpacks-go/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/heroku/buildpacks-go/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/heroku/buildpacks-go/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/heroku/buildpacks-go/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/heroku/buildpacks-go/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/heroku/buildpacks-go/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/heroku/buildpacks-go/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/heroku/buildpacks-go/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/heroku/buildpacks-go/compare/v0.1.16...v0.2.0
[0.1.16]: https://github.com/heroku/buildpacks-go/compare/v0.1.15...v0.1.16
[0.1.15]: https://github.com/heroku/buildpacks-go/compare/v0.1.14...v0.1.15
[0.1.14]: https://github.com/heroku/buildpacks-go/compare/v0.1.13...v0.1.14
[0.1.13]: https://github.com/heroku/buildpacks-go/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/heroku/buildpacks-go/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/heroku/buildpacks-go/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/heroku/buildpacks-go/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/heroku/buildpacks-go/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/heroku/buildpacks-go/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/heroku/buildpacks-go/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/heroku/buildpacks-go/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/heroku/buildpacks-go/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/heroku/buildpacks-go/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/heroku/buildpacks-go/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/heroku/buildpacks-go/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/heroku/buildpacks-go/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/heroku/buildpacks-go/releases/tag/v0.1.0
