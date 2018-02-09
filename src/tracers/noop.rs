use crossbeam_channel::unbounded;
use rand::random;

use super::super::ImplContextBox;
use super::super::Result;

use super::super::FinishedSpan;
use super::super::Span;
use super::super::SpanContext;
use super::super::SpanReceiver;
use super::super::SpanReference;
use super::super::SpanReferenceAware;
use super::super::SpanSender;
use super::super::StartOptions;

use super::super::ExtractFormat;
use super::super::InjectFormat;
use super::super::Tracer;
use super::super::TracerInterface;


/// A tracer that discards spans.
///
/// Like for other tracers, spans are still collected in full and are cost memory
/// until they are read from the `FinishedSpan` receiver and dropped.
///
/// Unlike other tracers, the `NoopTracer` will not propagate or extract tracing
/// information and will discard all `FinishedSpan`s when they are `NoopTracer::report`ed.
///
/// This is especially useful as a way to effectively disable tracing in applications built
/// with OpenTracing support when the end user does not wish to collect tracing information.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use opentracingrust::tracers::NoopTracer;
///
///
/// fn main() {
///     let (tracer, receiver) = NoopTracer::new();
///     // ... snip ...
/// }
/// ```
pub struct NoopTracer {
    sender: SpanSender
}

impl TracerInterface for NoopTracer {
    fn extract(&self, _fmt: ExtractFormat) -> Result<Option<SpanContext>> {
        // This method intentionality does nothing.
        Ok(None)
    }

    fn inject(&self, _context: &SpanContext, _fmt: InjectFormat) -> Result<()> {
        // This method intentionality does nothing.
        Ok(())
    }

    fn span(&self, name: &str, options: StartOptions) -> Span {
        let trace_id = random::<[u8; 16]>();
        let span_id = random::<u64>();
        let context = SpanContext::new(ImplContextBox::new(NoopTracerContext {
            trace_id,
            span_id
        }));
        Span::new(name, context, options, self.sender.clone())
    }
}

impl NoopTracer {
    /// Instantiate a new `NoopTracer`.
    pub fn new() -> (Tracer, SpanReceiver) {
        let (sender, receiver) = unbounded();
        let tracer = NoopTracer { sender };
        (Tracer::new(tracer), receiver)
    }


    /// "Reports" the finished span into nowere.
    ///
    /// `FinishedSpan`s passed to this method are simply dropped.
    /// This method is to provide a common interface with other tracers.
    pub fn report(span: FinishedSpan) {
        drop(span);
    }
}

/// Inner NoopTracer context.
#[derive(Clone, Debug)]
struct NoopTracerContext {
    trace_id: [u8; 16],
    span_id: u64
}

impl SpanReferenceAware for NoopTracerContext {
    fn reference_span(&mut self, reference: &SpanReference) {
        match reference {
            &SpanReference::ChildOf(ref parent) |
            &SpanReference::FollowsFrom(ref parent) => {
                let context = parent.impl_context::<NoopTracerContext>();
                let context = context.expect(
                    "Unsupported span context, was it created by NoopTracer?"
                );
                self.trace_id = context.trace_id;
            }
        }
    }
}


#[cfg(test)]
mod tests {
    mod inner_context {
        use super::super::super::super::ImplContextBox;
        use super::super::super::super::SpanContext;
        use super::super::super::super::SpanReference;
        use super::super::super::super::SpanReferenceAware;

        use super::super::NoopTracer;
        use super::super::NoopTracerContext;

        #[derive(Clone)]
        struct OtherContext {}
        impl SpanReferenceAware for OtherContext {
            fn reference_span(&mut self, _: &SpanReference) {}
        }

        #[test]
        fn child_of_updates_trace_id() {
            let (tracer, _) = NoopTracer::new();
            let parent = tracer.span("test1");
            let mut span = tracer.span("test2");
            span.child_of(parent.context().clone());

            let inner_parent = parent.context().impl_context::<NoopTracerContext>().unwrap();
            let inner_span = span.context().impl_context::<NoopTracerContext>().unwrap();
            assert_eq!(inner_parent.trace_id, inner_span.trace_id);
        }

        #[test]
        fn follows_updates_trace_id() {
            let (tracer, _) = NoopTracer::new();
            let parent = tracer.span("test1");
            let mut span = tracer.span("test2");
            span.follows(parent.context().clone());

            let inner_parent = parent.context().impl_context::<NoopTracerContext>().unwrap();
            let inner_span = span.context().impl_context::<NoopTracerContext>().unwrap();
            assert_eq!(inner_parent.trace_id, inner_span.trace_id);
        }

        #[test]
        #[should_panic(expected = "Unsupported span context, was it created by NoopTracer?")]
        fn panics_if_invalid_context() {
            let (tracer, _) = NoopTracer::new();
            let parent = SpanContext::new(ImplContextBox::new(OtherContext{}));
            let mut span = tracer.span("test");
            span.child_of(parent);
        }
    }
}
