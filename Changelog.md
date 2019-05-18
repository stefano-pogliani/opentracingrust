# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] - 2019-05-18
### Changed
- Updated dependencies to latest release.

### Fixed
- `Error` now implements `std::error::Error`.

## [0.3.1] - 2018-02-26
### Added
- Implement `AsMut<Span>` for `AutoFinishingSpan`.
- Implement `AsMut<Span>` for `Span`.
- Make `GlobalTracer::reset` public for tests.
- Make `TracerInterface` and `Tracer` `Sync`.
- Utility trait to fail spans on `Err`.

## [0.3.0] - 2018-02-09
### Added
- Support span logs.

### Changed
- **BREACKING** Don't implement clone for spans and tags.
- **BREACKING** GlobalTracer mutually exclude access to the tracer.
- **BREACKING** Replace channels with crossbeam-channel.

## [0.2.1] - 2018-02-06
### Added
- Update operation name.
- Support span tags.

## [0.2.0] - 2018-01-31
### Added
- Auto finishing spans: spans can finish themselves when dropped.
- Custom tracer example.
- Documentation.
- FileTracer prints durations.
- GlobalTracer signleton.
- NoopTracer implementation.
- Pass span initial references.
- Pass span start time.
- ReporterThread: a `FinishedSpan`s reported based on a background thread.
- Span stores finish time.
- Span stores start time.

### Changed
- **BREACKING** Extend `Tracer::span` with `StartOptions`.
- **BREACKING** `FileTracer::new` returns a `Tracer` and not a `FileTracer`.
- **BREACKING** Rename `ImplWrapper` to `ImplContextBox`.
- **BREACKING** Replace `MapCarrier::find_items` with `MapCarrier::items`.

## 0.1.1 - 2018-01-16
### Changed
- Added links to crates.io and docs.rs


## 0.1.0 - 2018-01-16
### Added
- Crate metadata (and name reservation).
- Essential library interface.
- FileTracer (for tests and development).
- Quickstart example.
- Repository setup (LICENSE, Changelog, Readme).
- Span references.


[Unreleased]: https://github.com/stefano-pogliani/opentracingrust-zipkin/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/stefano-pogliani/opentracingrust-zipkin/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/stefano-pogliani/opentracingrust-zipkin/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/stefano-pogliani/opentracingrust-zipkin/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/stefano-pogliani/opentracingrust-zipkin/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/stefano-pogliani/opentracingrust-zipkin/compare/v0.1.1...v0.2.0
