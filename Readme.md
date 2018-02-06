OpenTracing Rust
================
A rust crate implementing the [OpenTracing](http://opentracing.io/) API.

This crate must be complemented by a tracer implementation to send
information to a supported distributed tracer.

Alpha notice
------------
This library is in alpha state!

### Feature completeness
The library is almost feature-complete with OpenTracing v1.1

Feature completeness with OpenTracing 1.1 is expected in
version 0.2.2 of OpenTracingRust.

### Stability
Most features are just out the door and not extensively
tested in production environments.


Why OpenTracing?
----------------
If you are not yet convinced that distributed tracing is a useful tool see:
http://opentracing.io/documentation/

OpenTracing is a language and tracer independent specification, different
libraries exist for many languages to integrate any software in the ecosystem.

The OpenTracing specification also allows library users to pick whichever
distributed tracer they want (and is supported by the language ecosystem).


Why not rustracing?
-------------------
Rust has already an opentracing implementation called
[rustracing](https://crates.io/crates/rustracing) so why create another one?

The main difference is that opentracingrust uses traits and trait objects
to abstract tracer specific details.
These abstractions are then wrapped by structs and functions that implement
the common logic shared by all tracer implementations.

This means that:

  * Changing the distributed tracer means changing the tracer initialisation.
  * The distributed tracer can be configured at run time by library users.
  * Supporting an new distributed tracer requires implementing a handfull of traits.


References
----------

  * Documentation: https://docs.rs/opentracingrust/
  * Crates.io page: https://crates.io/crates/opentracingrust
  * Repository: https://github.com/stefano-pogliani/opentracingrust
  * Available tracers: https://crates.io/search?q=opentracingrust


Quickstart
----------
To use opentracingrust add a dependency to this crate.

For development and testing the core library provides two tracers:

  * The `NoopTracer` is a tracer that drops all received information.
    This tracer is a useful default for unit tests and project bootstapping.
  * The `FileTracer` is a test only tracer that writes span information to
    a rust `Write` stream (usually stdout/stderr).
    This tracer is useful to test that all desired information is collected
    and experiment with opentracingrust.

For production use you will need to add a dependency on
all tracers you would like to support:
```toml
# These crates may not exist yet but tracer crates will look like this
opentracingrust_appdash = "^0.1.0"
opentracingrust_zipkin = "^0.1.0"
```

Usage examples with step by step explainations can be found in the `examples/`
directory and you should start with `examples/0-quickstart.rs`.
Regardless of the tracer though there are four steps to using opentracingrust:

  1. Set up a tracer (there should only be one tracer instance: pass it around as needed).
  2. Create spans to represent operations.
  3. Pass span contexts around to link spans across the system.
  4. Once the operation is complete send each span to the tracing system.
