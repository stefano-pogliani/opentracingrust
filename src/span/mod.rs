use std::time::SystemTime;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;

use super::Result;
use super::SpanContext;

pub mod log;
pub mod tag;

use self::log::Log;
use self::tag::SpanTags;
use self::tag::TagValue;


/// A `Span` wrapper that finishes a span when dropped.
///
/// # Panics
///
/// If the inner span fails to `Span::finish` correctly the `AutoFinishingSpan`
/// will cause the current thread to panic when it is dropped.
// Structure invariant: An AutoFinishingSpan *always* contains a `Span`.
//   An AutoFinishingSpan is only created with a `Some(span)`.
//   The `Drop::drop` method is the only method allowed to leave
//   the AutoFinishingSpan with an inner `None`.
#[derive(Debug)]
pub struct AutoFinishingSpan(Option<Span>);

impl AutoFinishingSpan {
    pub fn new(span: Span) -> AutoFinishingSpan {
        AutoFinishingSpan(Some(span))
    }
}

impl AutoFinishingSpan {
    /// Access the `SpanContext` for the inner `Span`.
    pub fn context(&self) -> &SpanContext {
        self.0.as_ref().unwrap().context()
    }

    /// Attach a log event to the span.
    pub fn log(&mut self, log: Log) {
        self.0.as_mut().unwrap().log(log);
    }
}

impl Drop for AutoFinishingSpan {
    fn drop(&mut self) {
        if let Some(span) = self.0.take() {
            span.finish().expect("Failed to auto-finish span");
        }
    }
}


/// A `Span` that represents a finished operation.
///
/// The span can no longer be altered since the operation is finished.
/// `Tracer`s must provide a way to submit `FinishedSpan`a to the distributed tracer.
#[derive(Debug)]
pub struct FinishedSpan {
    context: SpanContext,
    finish_time: SystemTime,
    logs: Vec<Log>,
    name: String,
    references: Vec<SpanReference>,
    start_time: SystemTime,
    tags: SpanTags,
}

impl FinishedSpan {
    /// Access the operation's `SpanContext`.
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// Access the `SystemTime` the `Span` was finished.
    pub fn finish_time(&self) -> &SystemTime {
        &self.finish_time
    }

    /// Access the logs attached to this span.
    pub fn logs(&self) -> &Vec<Log> {
        &self.logs
    }

    /// Access the name of the operation.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Access all the `SpanContext`s and their relationship with this span.
    pub fn references(&self) -> &Vec<SpanReference> {
        &self.references
    }

    /// Access the `SystemTime` the `Span` was started.
    pub fn start_time(&self) -> &SystemTime {
        &self.start_time
    }

    /// Access the tags attached to this span.
    pub fn tags(&self) -> &SpanTags {
        &self.tags
    }
}


/// Model of an in progress operation.
///
/// A `Span` is to a distributed trace what a stack frame is to a stack trace.
///
/// `Span`s are created by `Tracer`s with `Tracer::span`.
/// `Span`s can be populated with `StartOptions` passed to `Tracer::span` and
/// with the mutating methods described below.
///
/// Once an operation is complete the span should be finished with `Span::finished`.
#[derive(Debug)]
pub struct Span {
    context: SpanContext,
    finish_time: Option<SystemTime>,
    logs: Vec<Log>,
    name: String,
    references: Vec<SpanReference>,
    sender: SpanSender,
    start_time: SystemTime,
    tags: SpanTags,
}

impl Span {
    /// Creates a new `Span` instance and initialises any passed `StartOptions`.
    ///
    /// This function is for use by `TracerInterface` implementations in their
    /// `TracerInterface::span` method.
    ///
    /// The `sender` argument is the sending end of an `crossbeam_channel::unbounded`.
    /// The receiving end of this channel, usually returned by the tracer's initialisation
    /// routine, will gather `FinishedSpan`s so they can be shipped to the distributed tracer.
    pub fn new(
        name: &str, context: SpanContext, options: StartOptions,
        sender: SpanSender
    ) -> Span {
        let mut span = Span {
            context,
            finish_time: None,
            logs: Vec::new(),
            name: String::from(name),
            references: Vec::new(),
            sender,
            start_time: options.start_time.unwrap_or_else(SystemTime::now),
            tags: SpanTags::new(),
        };
        for reference in options.references {
            span.reference_span(reference);
        }
        span
    }
}

impl Span {
    /// Convert the running `Span` into an `AutoFinishingSpan`.
    ///
    /// `Span`s instances need to be `finished` for the information to be sent.
    /// If a `Span` goes out of scope the information in it is lost and the span
    /// is never sent to the distributed tracer.
    ///
    /// The `AutoFinishingSpan` wrapper allows a `Span` to be finished when it goes out of scope.
    ///
    /// # Panics
    /// While this function never panics, keep in mind that the `AutoFinishingSpan`
    /// panics if `Span::finish` fails.
    pub fn auto_finish(self) -> AutoFinishingSpan {
        AutoFinishingSpan::new(self)
    }

    /// Marks this span as a child of the given context.
    pub fn child_of(&mut self, parent: SpanContext) {
        self.reference_span(SpanReference::ChildOf(parent));
    }

    /// Access the `SpanContext` of this span.
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// Set the span finish time.
    /// 
    /// This method allows to set the finish time of an operation explicitly
    /// and still manipulate the span further.
    /// This allows to time the operation first and the populate the span with
    /// any available detail without obfuscating the duration of the real operation.
    pub fn finish_time(&mut self, finish_time: SystemTime) {
        self.finish_time = Some(finish_time);
    }

    /// Finished a span and sends it to the tracer's receiver..
    ///
    /// Consumes a `Span` to create a `FinishedSpan`.
    /// The finished span is then send to the tracer's `crossbeam_channel::Receiver`
    /// associated with the span at the time of creation.
    ///
    /// Any error sending the span is returned to the caller.
    pub fn finish(self) -> Result<()> {
        let finished = FinishedSpan {
            context: self.context,
            finish_time: self.finish_time.unwrap_or_else(SystemTime::now),
            logs: self.logs,
            name: self.name,
            references: self.references,
            start_time: self.start_time,
            tags: self.tags,
        };
        self.sender.send(finished)?;
        Ok(())
    }

    /// Marks this span as a follower of the given context.
    pub fn follows(&mut self, parent: SpanContext) {
        self.reference_span(SpanReference::FollowsFrom(parent));
    }

    /// Attempt to fetch a baggage item by key.
    ///
    /// If there is no item with the given key this method returns `None`.
    pub fn get_baggage_item(&self, key: &str) -> Option<&String> {
        self.context.get_baggage_item(key)
    }

    /// Attach a log event to the span.
    pub fn log(&mut self, mut log: Log) {
        log.at_or_now();
        self.logs.push(log);
    }

    /// Returns the operation name.
    pub fn operation_name(&self) -> &str {
        &self.name
    }

    /// Adds a reference to a `SpanContext`.
    pub fn reference_span(&mut self, reference: SpanReference) {
        self.context.reference_span(&reference);
        match reference {
            SpanReference::ChildOf(ref parent) |
            SpanReference::FollowsFrom(ref parent) => {
                for (key, value) in parent.baggage_items() {
                    self.context.set_baggage_item(key.clone(), value.clone())
                }
            }
        }
        self.references.push(reference);
    }

    /// Access all referenced span contexts and their relationship.
    pub fn references(&self) -> &[SpanReference] {
        &self.references
    }

    /// Adds or updates the baggage items with the given key/value pair.
    ///
    /// Baggage items are forwarded to `Span`s that reference this `Span`
    /// and all the `Span`s that reference them.
    ///
    /// Baggage items are **NOT** propagated backwards to `Span`s that reference this `Span`.
    pub fn set_baggage_item(&mut self, key: &str, value: &str) {
        self.context.set_baggage_item(String::from(key), String::from(value));
    }

    /// Updates the operation name.
    pub fn set_operation_name(&mut self, name: &str) {
        self.name = String::from(name);
    }

    /// Append a tag to the span.
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
    ///     let (tracer, _) = NoopTracer::new();
    ///     let mut span = tracer.span("some_work");
    ///     span.tag("client.name", "some-tracing-client");
    ///     span.tag("client.version", 3.4);
    ///     // ... snip ...
    /// }
    /// ```
    pub fn tag<TV: Into<TagValue>>(&mut self, tag: &str, value: TV) {
        self.tags.tag(tag, value.into());
    }
}


/// Enumerates all known relationships among `SpanContext`s.
///
/// Each relationship also carries the `SpanContext` it relates to.
#[derive(Clone, Debug)]
pub enum SpanReference {
    ChildOf(SpanContext),
    FollowsFrom(SpanContext)
}


/// Type alias for an `crossbeam_channel::Receiver` of `FinishedSpan`s.
pub type SpanReceiver = Receiver<FinishedSpan>;

/// Type alias for an `crossbeam_channel::Sender` of `FinishedSpan`s.
pub type SpanSender = Sender<FinishedSpan>;


/// Additional options that are passed to `Tracer::span`.
///
/// These options specify initial attributes of a span.
/// All values are optional.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use std::time::SystemTime;
///
/// use opentracingrust::StartOptions;
/// use opentracingrust::tracers::NoopTracer;
///
///
/// fn main() {
///     let (tracer, _) = NoopTracer::new();
///     let parent = tracer.span("parent");
///
///     let now = SystemTime::now();
///     let options = StartOptions::default()
///         .child_of(parent.context().clone())
///         .start_time(now);
///     let span = tracer.span_with_options("test", options);
/// }
/// ```
pub struct StartOptions {
    references: Vec<SpanReference>,
    start_time: Option<SystemTime>,
}

impl StartOptions {
    /// Declares a `ChildOf` relationship for the `Span` to be.
    pub fn child_of(self, parent: SpanContext) -> Self {
        self.reference_span(SpanReference::ChildOf(parent))
    }

    /// Declares a `FollowsFrom` relationship for the `Span` to be.
    pub fn follows(self, parent: SpanContext) -> Self {
        self.reference_span(SpanReference::FollowsFrom(parent))
    }

    /// Declares any of the `SpanReference`s for the `Span` to be.
    pub fn reference_span(mut self, reference: SpanReference) -> Self {
        self.references.push(reference);
        self
    }

    /// Sets the start time for the operation.
    pub fn start_time(mut self, start_time: SystemTime) -> Self {
        self.start_time = Some(start_time);
        self
    }
}

impl Default for StartOptions {
    /// Returns a default set of `StartOptions`.
    ///
    /// By default the `Span` will:
    ///
    ///   * Have no references, which will make it a root span.
    ///   * Have have a start time of when `Tracer::span` is called.
    fn default() -> StartOptions {
        StartOptions {
            references: Vec::new(),
            start_time: None,
        }
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crossbeam_channel::unbounded;

    use super::super::ImplContextBox;
    use super::super::SpanContext;
    use super::super::SpanReferenceAware;
    use super::super::StartOptions;

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
            let (sender, receiver) = unbounded();
            let context = SpanContext::new(ImplContextBox::new(TestContext {
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
        let (sender, receiver) = unbounded();
        let context = SpanContext::new(ImplContextBox::new(TestContext {
            id: String::from("test-id")
        }));
        let options = StartOptions::default();
        let span: Span = Span::new("test-span", context, options, sender);
        span.finish().unwrap();
        let _finished: FinishedSpan = receiver.recv().unwrap();
    }

    #[test]
    fn set_span_name() {
        let (sender, _) = unbounded();
        let context = SpanContext::new(ImplContextBox::new(TestContext {
            id: String::from("test-id")
        }));
        let options = StartOptions::default();
        let mut span = Span::new("test-span", context, options, sender);
        span.set_operation_name("some-other-name");
        assert_eq!("some-other-name", span.operation_name());
    }

    #[test]
    fn span_child_of_another() {
        let (sender, _) = unbounded();
        let context = SpanContext::new(ImplContextBox::new(TestContext {
            id: String::from("test-id-1")
        }));
        let options = StartOptions::default();
        let mut span = Span::new("test-span", context, options, sender);
        let mut context = SpanContext::new(ImplContextBox::new(TestContext {
            id: String::from("test-id-2")
        }));
        context.set_baggage_item(String::from("a"), String::from("b"));
        span.child_of(context.clone());
        match span.references().get(0).unwrap() {
            &SpanReference::ChildOf(ref context) => {
                let span = context.impl_context::<TestContext>().unwrap();
                assert_eq!(span.id, "test-id-2");
            },
            _ => panic!("Invalid span reference")
        }
        let item = span.get_baggage_item("a").unwrap();
        assert_eq!(item, "b");
    }

    #[test]
    fn span_follows_another() {
        let (sender, _) = unbounded();
        let context = SpanContext::new(ImplContextBox::new(TestContext {
            id: String::from("test-id-1")
        }));
        let options = StartOptions::default();
        let mut span = Span::new("test-span", context, options, sender);
        let mut context = SpanContext::new(ImplContextBox::new(TestContext {
            id: String::from("test-id-2")
        }));
        context.set_baggage_item(String::from("a"), String::from("b"));
        span.follows(context.clone());
        match span.references().get(0).unwrap() {
            &SpanReference::FollowsFrom(ref context) => {
                let span = context.impl_context::<TestContext>().unwrap();
                assert_eq!(span.id, "test-id-2");
            },
            _ => panic!("Invalid span reference")
        }
        let item = span.get_baggage_item("a").unwrap();
        assert_eq!(item, "b");
    }

    mod references {
        use super::super::super::ImplContextBox;

        use super::super::SpanContext;
        use super::super::SpanReference;
        use super::super::StartOptions;

        use super::TestContext;


        #[test]
        fn child_of() {
            let parent = SpanContext::new(ImplContextBox::new(TestContext {
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
            let parent = SpanContext::new(ImplContextBox::new(TestContext {
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
            let parent = SpanContext::new(ImplContextBox::new(TestContext {
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

    mod logs {
        // TODO: add and get logs with time.
        // TODO: add and get logs without time.
        // TODO: reject logs with time older then start.
        // TODO: reject logs with time newer then finish.
    }

    mod tags {
        use super::super::StartOptions;
        use super::super::TagValue;

        use super::TestContext;

        #[test]
        fn add_generic_tag() {
            let (mut span, receiver) = TestContext::new(StartOptions::default());
            span.tag("key", TagValue::String(String::from("value")));
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            match span.tags().get("key") {
                Some(&TagValue::String(ref v)) => assert_eq!(v, "value"),
                Some(_) => panic!("Invalid tag type"),
                None => panic!("Tag not found")
            }
        }

        #[test]
        fn add_bool_tag() {
            let (mut span, receiver) = TestContext::new(StartOptions::default());
            span.tag("key", true);
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            match span.tags().get("key") {
                Some(&TagValue::Boolean(v)) => assert_eq!(v, true),
                Some(_) => panic!("Invalid tag type"),
                None => panic!("Tag not found")
            }
        }

        #[test]
        fn add_float_tag() {
            let (mut span, receiver) = TestContext::new(StartOptions::default());
            span.tag("key", 1.2);
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            match span.tags().get("key") {
                Some(&TagValue::Float(v)) => assert_eq!(v, 1.2),
                Some(_) => panic!("Invalid tag type"),
                None => panic!("Tag not found")
            }
        }

        #[test]
        fn add_integer_tag() {
            let (mut span, receiver) = TestContext::new(StartOptions::default());
            span.tag("key", -2);
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            match span.tags().get("key") {
                Some(&TagValue::Integer(v)) => assert_eq!(v, -2),
                Some(_) => panic!("Invalid tag type"),
                None => panic!("Tag not found")
            }
        }

        #[test]
        fn add_str_tag() {
            let (mut span, receiver) = TestContext::new(StartOptions::default());
            span.tag("key", "value");
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            match span.tags().get("key") {
                Some(&TagValue::String(ref v)) => assert_eq!(v, "value"),
                Some(_) => panic!("Invalid tag type"),
                None => panic!("Tag not found")
            }
        }

        #[test]
        fn add_string_tag() {
            let (mut span, receiver) = TestContext::new(StartOptions::default());
            span.tag("key", String::from("value"));
            span.finish().unwrap();
            let span = receiver.recv().unwrap();
            match span.tags().get("key") {
                Some(&TagValue::String(ref v)) => assert_eq!(v, "value"),
                Some(_) => panic!("Invalid tag type"),
                None => panic!("Tag not found")
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
