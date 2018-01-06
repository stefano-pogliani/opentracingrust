use std::sync::mpsc;

use super::Error;
use super::Result;

use super::SpanContext;
use super::span_context::BaggageItem;


/// TODO
pub type SpanReceiver = mpsc::Receiver<FinishedSpan>;

/// TODO
pub type SpanSender = mpsc::Sender<FinishedSpan>;


/// TODO
#[derive(Clone, Debug)]
pub struct FinishedSpan {
    context: SpanContext,
    name: String
}


/// TODO
#[derive(Clone, Debug)]
pub struct Span {
    context: SpanContext,
    name: String,
    sender: SpanSender
}

impl Span {
    /// TODO
    pub fn new(name: &str, context: SpanContext, sender: SpanSender) -> Span {
        Span {
            context,
            name: String::from(name),
            sender: sender
        }
    }
}

impl Span {
    /// TODO
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// TODO
    pub fn finish(self) -> Result<()> {
        let finished = FinishedSpan {
            context: self.context,
            name: self.name
        };
        self.sender.send(finished).map_err(|e| Error::SendError(e))?;
        Ok(())
    }

    /// TODO
    pub fn set_baggage_item(&mut self, key: &str, value: &str) {
        self.context.set_baggage_item(BaggageItem::new(key, value));
    }
}


#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::super::ImplWrapper;
    use super::super::SpanContext;

    use super::Span;
    use super::FinishedSpan;


    #[derive(Debug, Clone)]
    struct TestContext {
        pub id: String
    }

    #[test]
    fn start_span_on_creation() {
        let (sender, _) = mpsc::channel();
        let context = SpanContext::new(ImplWrapper::new(TestContext {
            id: String::from("test-id")
        }));
        let _span: Span = Span::new("test-span", context, sender);
    }

    #[test]
    fn send_span_on_finish() {
        let (sender, receiver) = mpsc::channel();
        let context = SpanContext::new(ImplWrapper::new(TestContext {
            id: String::from("test-id")
        }));
        let span: Span = Span::new("test-span", context, sender);
        span.finish().unwrap();
        let _finished: FinishedSpan = receiver.recv().unwrap();
    }
}
