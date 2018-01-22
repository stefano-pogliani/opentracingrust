//! This example shows how to trace nested calls in a single thread.
//!
//! To do so, the example uses the Fibonacci recursive implementation and traces
//! the execution of each function call.
//!
//! The steps to use opentracing-rust are:
//!
//!   1. Set up a tracer (there should only be one tracer instance: pass it around as needed).
//!   2. Create spans to represent operations.
//!   3. Pass span contexts around to link spans across the system.
//!   4. Once the operation is complete send each span to the tracing system.
//!
//! Each step is shown and explained in detailed in the code and comments below.
//!
//! As this example aims at showing the basics of opentracing-rust it will use the
//! `FileTracer` implementation to write finished spans to standard error.
//!
//! This example is also not a best practice: some parts (i.e, thread management) are not
//! meant to be used in production systems and need various improvements.
extern crate opentracingrust;

// Standard library imports.
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time;

// Core library imports.
use opentracingrust::SpanContext;
use opentracingrust::StartOptions;
use opentracingrust::Tracer;
use opentracingrust::utils::GlobalTracer;

// Tracer specific imports.
use opentracingrust::tracers::FileTracer;


fn main() {
    println!("Welcome to OpenTracing Rust!");

    // First we create a tracer.
    // To do so we instantiate a tracer implementation and wrap it inside a Tracer.
    // A tracer implementation is any struct that implements the `TracerInterface` trait.
    let (tracer, receiver) = FileTracer::new();
    let tracer = GlobalTracer::init(Tracer::new(tracer));

    // Then we create a thread that will receive finished spans and write them to stderr.
    let looping = Arc::new(AtomicBool::new(true));
    let is_looping = Arc::clone(&looping);
    let writer = thread::spawn(move || {
        let mut stderr = io::stderr();
        while is_looping.load(Ordering::Relaxed) {
            let span = receiver.recv().unwrap();
            FileTracer::write_trace(span, &mut stderr).unwrap();
        }
    });

    // Now that our tracer is set up we can create spans to trace operations.
    let root = tracer.span("main", StartOptions::default());
    let f4 = fibonacci(8, &tracer, root.context().clone());
    println!("fibonacci(8) = {}", f4);

    // Wait 10 seconds to make sure all spans are flushed.
    println!("Waiting to flush all spans ...");
    thread::sleep(time::Duration::new(5, 0));
    looping.store(false, Ordering::Relaxed);

    // We need to finish the span once we are done with it.
    // Finishing a span freezes its state and sends it to the
    // `receiver` channel returned by the tracer constructor.
    root.finish().unwrap();
    writer.join().unwrap();
}

fn fibonacci(n: u64, tracer: &Arc<Tracer>, parent: SpanContext) -> u64 {
    // To create a new span for this operation set the parent span.
    let options = StartOptions::default().child_of(parent);
    if n <= 2 {
        // Since this is the base case we finish the span immediately.
        let span = tracer.span("fibonacci base case", options);
        span.finish().unwrap();
        1
    } else {
        // Since this is the iterative case we recourse passing the new span's
        // context as the new parent span.
        let span = tracer.span("fibonacci iterative case", options);
        let n1 = fibonacci(n - 1, tracer, span.context().clone());
        let n2 = fibonacci(n - 2, tracer, span.context().clone());
        // Once the recoursive operations terminate we can close the current span.
        span.finish().unwrap();
        n1 + n2
    }
}
