Version 0.3.1 (Unrelease)
=========================
- Implement `AsMut<Span>` for `AutoFinishingSpan`.
- Implement `AsMut<Span>` for `Span`.
- Utility trait to fail spans on `Err`.

Version 0.3.0 (2018-02-09)
==========================

Breaking Changes
----------------
- Don't implement clone for spans and tags.
- GlobalTracer mutually exclude access to the tracer.
- Replace channels with crossbeam-channel.

Features
--------
- Support span logs.


Version 0.2.1 (2018-02-06)
==========================

Features
--------
- Update operation name.
- Support span tags.


Version 0.2.0 (2018-01-31)
==========================

Breaking Changes
----------------
- Extend `Tracer::span` with `StartOptions`.
- `FileTracer::new` returns a `Tracer` and not a `FileTracer`.
- Rename `ImplWrapper` to `ImplContextBox`.
- Replace `MapCarrier::find_items` with `MapCarrier::items`.

Features
--------
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
