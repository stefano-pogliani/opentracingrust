//! An [OpenTracing](http://opentracing.io/) implementation for rust.
//!
//! This crate provides a generic Tracer interface following the OpenTracing
//! specification that allows users (libraries, framework, applications) to
//! implement distributed tracing across ecosystems and without committing to
//! a specific distributed tracer.
//!
//! This means that:
//!
//!   * Frameworks don't have to impose a tracer on applications.
//!   * Libraries can integrate their user's traces.
//!   * Applications lock end-users into a distributed tracer.
//!
//!
//! # Architecture
//!
//! At the core of this library are three types:
//!
//!   * `Tracer`: an interface to create and serialise `Span`s.
//!   * `Span`: each instance represents an operation and its metadata.
//!   * `SpanContext`: a tracer-specific identifier of a `Span`.
//!
//!
//! ## Configuraing a `Tracer`
//! 
//! <b>
//!     Application developers MUST read this, library and framework
//!     developers SHOULD still read it for completeness.
//! </b>
//!
//! Every application and all the libraries and frameworks it uses
//! share a single `Tracer` instance, unique to the entire process.
//! The instance should be passed around using a dependency injection
//! technique, of which there are many.
//! This crate provides a `GlobalTracer` utility singleton for cases
//! where dependency injections are not usable.
//!
//! Configuration of the `Tracer` instance used across the process is the
//! responsibility of the application and should be performed as soon as
//! possible in the initialisation phase.
//!
//! Tracers are implemented in separate crates and each tracers can be
//! implemented as desired but there are two requirements of concrete tracers:
//!
//!   * Initialisation returns instance of `Tracer`.
//!   * A `std::sync::mpsc::channel` is used by the tracer to send
//!     `FinishedSpan`s to a reporting thread.
//!
//! The reporting thread is responsible for pushing the spans to the
//! distributed tracer of choice.
//!
//! In code:
//!
//! ```
//! extern crate opentracingrust;
//!
//! use std::time::Duration;
//!
//! use opentracingrust::tracers::NoopTracer;
//! use opentracingrust::utils::GlobalTracer;
//! use opentracingrust::utils::ReporterThread;
//!
//!
//! fn main() {
//!     let (tracer, receiver) = NoopTracer::new();
//!     let reporter = ReporterThread::new_with_duration(
//!         receiver, Duration::from_millis(50), NoopTracer::report
//!     );
//!     GlobalTracer::init(tracer);
//!
//!     // ... snip ...
//! }
//! ```
//!
//!
//! ## Tracing an operation
//!
//! Now that a `Tracer` is configured and we are sending `FinishedSpan`s to a
//! distributed tracing software, it is possible to trace operations.
//!
//! Tracing operations is done by:
//!
//!   * Creating a named `Span` that represents the operation.
//!   * Attaching causality information with `SpanContext`s.
//!   * Adding any needed metadata.
//!   * Finishing the span once the operation is done.
//!
//! ```
//! extern crate opentracingrust;
//!
//! use std::time::Duration;
//!
//! use opentracingrust::SpanContext;
//! use opentracingrust::StartOptions;
//!
//! use opentracingrust::tracers::NoopTracer;
//! use opentracingrust::utils::GlobalTracer;
//! use opentracingrust::utils::ReporterThread;
//!
//!
//! fn main() {
//!     let (tracer, receiver) = NoopTracer::new();
//!     let reporter = ReporterThread::new_with_duration(
//!         receiver, Duration::from_millis(50), NoopTracer::report
//!     );
//!     GlobalTracer::init(tracer);
//!     // Once the tracer is configured we can start working.
//!     start_working();
//! }
//!
//! fn start_working() {
//!     let mut root_span = GlobalTracer::get().span("start_working");
//!     // The actual work is done in a sub-operation.
//!     do_work(root_span.context().clone());
//!     root_span.finish();
//! }
//!
//! fn do_work(context: SpanContext) {
//!     let mut span = GlobalTracer::get().span_with_options(
//!         "do_work", StartOptions::default().child_of(context)
//!     );
//!     // ... do work ...
//!     span.finish();
//! }
//! ```
//!
//! The `examples/` directoy includes many more working
//! end-to-end examples of different use cases.
//!
//!
//! ## The `NoopTracer`
//!
//! As mentioned above, the crate does not provide concrete `Tracer`s
//! but rather a standard interface for projects to bind against.
//!
//! The `NoopTracer` is the perfect tool to write tests with and a good default
//! for examples and projects that do not yet implement full tracing support.
extern crate rand;

mod carrier;
mod errors;
mod span;
mod span_context;
mod tracer;

pub mod tracers;
pub mod utils;


pub use self::carrier::ExtractFormat;
pub use self::carrier::InjectFormat;
pub use self::carrier::MapCarrier;

pub use self::errors::Error;
pub use self::errors::Result;

pub use self::span_context::ImplContext;
pub use self::span_context::ImplContextBox;
pub use self::span_context::SpanContext;
pub use self::span_context::SpanReferenceAware;

pub use self::span::FinishedSpan;
pub use self::span::Span;
pub use self::span::SpanReceiver;
pub use self::span::SpanReference;
pub use self::span::SpanSender;
pub use self::span::StartOptions;

pub use self::span::log::Log;
pub use self::span::log::LogValue;
pub use self::span::tag::TagValue;

pub use self::tracer::Tracer;
pub use self::tracer::TracerInterface;
