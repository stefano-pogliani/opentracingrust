use std::sync::mpsc;
use std::time::SystemTime;

use super::Error;
use super::Result;

use super::SpanContext;
use super::span_context::BaggageItem;


/// TODO
///
/// Structure invariant: An AutoFinishingSpan *always* contains a `Span`.
///   An AutoFinishingSpan is only created with a `Some(span)`.
///   The `Drop::drop` method is the only method allowed to leave
///   the AutoFinishingSpan with an inner `None`.
#[derive(Clone, Debug)]
pub struct AutoFinishingSpan(Option<Span>);

impl AutoFinishingSpan {
    pub fn new(span: Span) -> AutoFinishingSpan {
        AutoFinishingSpan(Some(span))
    }
}

impl Drop for AutoFinishingSpan {
    fn drop(&mut self) {
        if let Some(span) = self.0.take() {
            span.finish().expect("Failed to auto-finish span");
        }
    }
}


/// TODO
#[derive(Clone, Debug)]
pub struct FinishedSpan {
    context: SpanContext,
    finish_time: SystemTime,
    name: String,
    references: Vec<SpanReference>,
    start_time: SystemTime,
}

impl FinishedSpan {
    /// TODO
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// TODO
    pub fn finish_time(&self) -> &SystemTime {
        &self.finish_time
    }

    /// TODO
    pub fn name(&self) -> &String {
        &self.name
    }

    /// TODO
    pub fn references(&self) -> &Vec<SpanReference> {
        &self.references
    }

    /// TODO
    pub fn start_time(&self) -> &SystemTime {
        &self.start_time
    }
}


/// TODO
#[derive(Clone, Debug)]
pub struct Span {
    context: SpanContext,
    finish_time: Option<SystemTime>,
    name: String,
    references: Vec<SpanReference>,
    sender: SpanSender,
    start_time: SystemTime,
}

impl Span {
    /// TODO
    pub fn new(
        name: &str, context: SpanContext, options: StartOptions,
        sender: SpanSender
    ) -> Span {
        let mut span = Span {
            context,
            finish_time: None,
            name: String::from(name),
            references: Vec::new(),
            sender,
            start_time: options.start_time.unwrap_or_else(SystemTime::now),
        };
        for reference in options.references {
            span.reference_span(reference);
        }
        span
    }
}

impl Span {
    /// TODO
    pub fn auto_finish(self) -> AutoFinishingSpan {
        AutoFinishingSpan::new(self)
    }

    /// TODO
    pub fn child_of(&mut self, parent: SpanContext) {
        self.reference_span(SpanReference::ChildOf(parent));
    }

    /// TODO
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// TODO
    pub fn finish_time(&mut self, finish_time: SystemTime) {
        self.finish_time = Some(finish_time);
    }

    /// TODO
    pub fn finish(self) -> Result<()> {
        let finished = FinishedSpan {
            context: self.context,
            finish_time: self.finish_time.unwrap_or_else(SystemTime::now),
            name: self.name,
            references: self.references,
            start_time: self.start_time,
        };
        self.sender.send(finished).map_err(|e| Error::SendError(e))?;
        Ok(())
    }

    /// TODO
    pub fn follows(&mut self, parent: SpanContext) {
        self.reference_span(SpanReference::FollowsFrom(parent));
    }

    /// TODO
    pub fn get_baggage_item(&self, key: &str) -> Option<&BaggageItem> {
        self.context.get_baggage_item(key)
    }

    /// TODO
    pub fn references(&self) -> &[SpanReference] {
        &self.references
    }

    /// TODO
    pub fn set_baggage_item(&mut self, key: &str, value: &str) {
        self.context.set_baggage_item(BaggageItem::new(key, value));
    }
}

impl Span {
    /// TODO
    fn reference_span(&mut self, reference: SpanReference) {
        self.context.reference_span(&reference);
        match reference {
            SpanReference::ChildOf(ref parent) |
            SpanReference::FollowsFrom(ref parent) => {
                for item in parent.baggage_items() {
                    self.context.set_baggage_item(item.clone())
                }
            }
        }
        self.references.push(reference);
    }
}


/// TODO
#[derive(Clone, Debug)]
pub enum SpanReference {
    ChildOf(SpanContext),
    FollowsFrom(SpanContext)
}


/// TODO
pub type SpanReceiver = mpsc::Receiver<FinishedSpan>;

/// TODO
pub type SpanSender = mpsc::Sender<FinishedSpan>;


/// TODO
pub struct StartOptions {
    references: Vec<SpanReference>,
    start_time: Option<SystemTime>,
}

impl StartOptions {
    /// TODO
    pub fn child_of(self, parent: SpanContext) -> Self {
        self.reference_span(SpanReference::ChildOf(parent))
    }

    /// TODO
    pub fn follows(self, parent: SpanContext) -> Self {
        self.reference_span(SpanReference::FollowsFrom(parent))
    }

    /// TODO
    pub fn reference_span(mut self, reference: SpanReference) -> Self {
        self.references.push(reference);
        self
    }

    /// TODO
    pub fn start_time(mut self, start_time: SystemTime) -> Self {
        self.start_time = Some(start_time);
        self
    }
}

impl Default for StartOptions {
    fn default() -> StartOptions {
        StartOptions {
            references: Vec::new(),
            start_time: None,
        }
    }
}


#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::super::ImplWrapper;
    use super::super::SpanContext;
    use super::super::SpanReferenceAware;
    use super::super::StartOptions;
    use super::super::span_context::BaggageItem;

    use super::FinishedSpan;
    use super::Span;
    use super::SpanReceiver;
    use super::SpanReference;


    #[derive(Debug, Clone)]
    struct TestContext {
        pub id: String
    }
    impl TestContext {
        fn new(options: StartOptions) -> (Span, SpanReceiver) {
            let (sender, receiver) = mpsc::channel();
            let context = SpanContext::new(ImplWrapper::new(TestContext {
                id: String::from("test-id")
            }));
            (Span::new("test-span", context, options, sender), receiver)
        }
    }
    impl SpanReferenceAware for TestContext {
        fn reference_span(&mut self, _: &SpanReference) {}
    }


    #[test]
    fn autoclose_finish_span_on_drop() {
        let options = StartOptions::default();
        let (span, receiver) = TestContext::new(options);
        {
            span.auto_finish();
        }
        receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    }

    #[test]
    fn start_span_on_creation() {
        let (_span, _): (Span, _) = TestContext::new(StartOptions::default());
    }

    #[test]
    fn send_span_on_finish() {
        let (sender, receiver) = mpsc::channel();
        let context = SpanContext::new(ImplWrapper::new(TestContext {
            id: String::from("test-id")
        }));
        let options = StartOptions::default();
        let span: Span = Span::new("test-span", context, options, sender);
        span.finish().unwrap();
        let _finished: FinishedSpan = receiver.recv().unwrap();
    }

    #[test]
    fn span_child_of_another() {
        let (sender, _) = mpsc::channel();
        let context = SpanContext::new(ImplWrapper::new(TestContext {
            id: String::from("test-id-1")
        }));
        let options = StartOptions::default();
        let mut span = Span::new("test-span", context, options, sender);
        let mut context = SpanContext::new(ImplWrapper::new(TestContext {
            id: String::from("test-id-2")
        }));
        context.set_baggage_item(BaggageItem::new("a", "b"));
        span.child_of(context.clone());
        match span.references().get(0).unwrap() {
            &SpanReference::ChildOf(ref context) => {
                let span = context.impl_context::<TestContext>().unwrap();
                assert_eq!(span.id, "test-id-2");
            },
            _ => panic!("Invalid span reference")
        }
        let item = span.get_baggage_item("a").unwrap();
        assert_eq!(item.value(), "b");
    }

    #[test]
    fn span_follows_another() {
        let (sender, _) = mpsc::channel();
        let context = SpanContext::new(ImplWrapper::new(TestContext {
            id: String::from("test-id-1")
        }));
        let options = StartOptions::default();
        let mut span = Span::new("test-span", context, options, sender);
        let mut context = SpanContext::new(ImplWrapper::new(TestContext {
            id: String::from("test-id-2")
        }));
        context.set_baggage_item(BaggageItem::new("a", "b"));
        span.follows(context.clone());
        match span.references().get(0).unwrap() {
            &SpanReference::FollowsFrom(ref context) => {
                let span = context.impl_context::<TestContext>().unwrap();
                assert_eq!(span.id, "test-id-2");
            },
            _ => panic!("Invalid span reference")
        }
        let item = span.get_baggage_item("a").unwrap();
        assert_eq!(item.value(), "b");
    }

    mod references {
        use super::super::super::ImplWrapper;

        use super::super::SpanContext;
        use super::super::SpanReference;
        use super::super::StartOptions;

        use super::TestContext;


        #[test]
        fn child_of() {
            let parent = SpanContext::new(ImplWrapper::new(TestContext {
                id: String::from("test-id")
            }));
            let options = StartOptions::default()
                .child_of(parent);
            let (span, _) = TestContext::new(options);
            match span.references().get(0) {
                Some(&SpanReference::ChildOf(_)) => (),
                Some(_) => panic!("Invalid span reference"),
                None => panic!("Missing span reference")
            }
        }

        #[test]
        fn follows() {
            let parent = SpanContext::new(ImplWrapper::new(TestContext {
                id: String::from("test-id")
            }));
            let options = StartOptions::default()
                .follows(parent);
            let (span, _) = TestContext::new(options);
            match span.references().get(0) {
                Some(&SpanReference::FollowsFrom(_)) => (),
                Some(_) => panic!("Invalid span reference"),
                None => panic!("Missing span reference")
            }
        }

        #[test]
        fn multi_refs() {
            let parent = SpanContext::new(ImplWrapper::new(TestContext {
                id: String::from("test-id")
            }));
            let options = StartOptions::default()
                .child_of(parent.clone())
                .follows(parent);
            let (span, _) = TestContext::new(options);
            match span.references().get(0) {
                Some(&SpanReference::ChildOf(_)) => (),
                Some(_) => panic!("Invalid span reference"),
                None => panic!("Missing span reference")
            }
            match span.references().get(1) {
                Some(&SpanReference::FollowsFrom(_)) => (),
                Some(_) => panic!("Invalid span reference"),
                None => panic!("Missing span reference")
            }
        }
    }

    mod times {
        use std::time::Duration;
        use std::time::SystemTime;

        use super::super::StartOptions;
        use super::TestContext;


        #[test]
        fn finish_span_on_finish() {
            // Can't mock SystemTime::now() to a fixed value.
            // Check finish time is in a range [now, now + ten minutes].
            let about_now = SystemTime::now();
            let options = StartOptions::default();
            let (span, receiver) = TestContext::new(options);
            let about_soon = about_now + Duration::from_secs(600);
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            assert!(about_now <= span.finish_time, "Finish time too old");
            assert!(span.finish_time <= about_soon, "Finish time too new");
        }

        #[test]
        fn finish_span_at_finish_time() {
            let in_ten_minutes = SystemTime::now() + Duration::from_secs(600);
            let options = StartOptions::default();
            let (mut span, receiver) = TestContext::new(options);
            span.finish_time(in_ten_minutes);
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            assert_eq!(span.finish_time, in_ten_minutes);
        }

        #[test]
        fn starts_now_by_default() {
            // Can't mock SystemTime::now() to a fixed value.
            // Check start time is in a range [now, now + ten minutes].
            let about_now = SystemTime::now();
            let options = StartOptions::default();
            let (span, _) = TestContext::new(options);
            let about_soon = about_now + Duration::from_secs(600);
            assert!(about_now <= span.start_time, "Start time too old");
            assert!(span.start_time <= about_soon, "Start time too new");
        }

        #[test]
        fn start_time_set() {
            let ten_minutes_ago = SystemTime::now() - Duration::from_secs(600);
            let options = StartOptions::default()
                .start_time(ten_minutes_ago.clone());
            let (span, _) = TestContext::new(options);
            assert_eq!(span.start_time, ten_minutes_ago);
        }
    }
}
