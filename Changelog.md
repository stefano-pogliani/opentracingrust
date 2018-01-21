Version 0.2.0 (Unreleased)
==========================

Breaking Changes
----------------
- Extend `Tracer::span` with `StartOptions`.

Features
--------
- Auto finishing spans: spans can finish themselves when dropped.
- Documentation.
- FileTracer prints durations.
- Pass span initial references.
- Pass span start time.
- Span stores finish time.
- Span stores start time.


Version 0.1.1 (2018-01-16)
==========================

Changed
--------
- Added links to crates.io and docs.rs


Version 0.1.0 (2018-01-16)
==========================

Features
--------
- Crate metadata (and name reservation).
- Essential library interface.
- FileTracer (for tests and development).
- Quickstart example.
- Repository setup (LICENSE, Changelog, Readme).
- Span references.
