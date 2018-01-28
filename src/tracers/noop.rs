use std::sync::mpsc;

use rand::random;

use super::super::Error;
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


/// TODO
pub struct NoopTracer {
    sender: SpanSender
}

impl TracerInterface for NoopTracer {
    fn extract(&self, _fmt: ExtractFormat) -> Result<Option<SpanContext>> {
        Err(Error::Msg(String::from("TODO")))
    }

    fn inject(&self, _context: &SpanContext, _fmt: InjectFormat) -> Result<()> {
        Err(Error::Msg(String::from("TODO")))
    }

    fn span(&self, name: &str, options: StartOptions) -> Span {
        let trace_id = random::<u64>();
        let span_id = random::<u64>();
        let context = SpanContext::new(ImplContextBox::new(NoopTracerContext {
            trace_id,
            span_id
        }));
        Span::new(name, context, options, self.sender.clone())
    }
}

impl NoopTracer {
    /// TODO
    pub fn new() -> (Tracer, SpanReceiver) {
        let (sender, receiver) = mpsc::channel();
        let tracer = NoopTracer { sender };
        (Tracer::new(tracer), receiver)
    }


    /// TODO
    pub fn report(_span: FinishedSpan) {
        // TODO
    }
}

/// TODO
#[derive(Clone, Debug)]
struct NoopTracerContext {
    trace_id: u64,
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
    // TODO
}
