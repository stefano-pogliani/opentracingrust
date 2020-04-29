use std::any::Any;
use std::boxed::Box;
use std::collections::HashMap;
use std::collections::hash_map::Iter;
use std::fmt;

mod impl_context;

pub use self::impl_context::ImplContext;
pub use self::impl_context::ImplContextBox;
pub use self::impl_context::SpanReferenceAware;

use super::SpanReference;


/// `Trecer`-specific span and trace identifier and metadata.
///
/// An OpenTracing `SpanContext` is an abstraction that holds:
///
///   * Baggage items: key-value pairs that are propagated though the trace.
///   * Tracer specific details: such as the trace and span ids, flags, and other
///     information that are needed by the concrete tracer (Zipkin, Jaeger, ...).
///
/// OpenTracingRust splits this abstraction into two:
///
///   * The `SpanContext` struct: holding all the common data.
///   * The `ImplContext` trait: holding implementation details.
///
/// The `SpanContext` holds information that is common to all `TracerInterface` implementors.
/// This currently means baggage items only.
///
/// Baggage items are key/value pairs that are propagated through a trace.
/// They are copied to derived spans every time a `SpanContext` is referenced by a `Span`.
/// Baggage items are NOT propagated backwards to parent spans.
///
///
/// # Examples
///
/// ```
/// extern crate crossbeam_channel;
/// extern crate opentracingrust;
///
/// use crossbeam_channel::unbounded;
///
/// use opentracingrust::ImplContextBox;
/// use opentracingrust::Span;
/// use opentracingrust::SpanContext;
/// use opentracingrust::SpanReference;
/// use opentracingrust::SpanReferenceAware;
/// use opentracingrust::StartOptions;
///
/// #[derive(Clone)]
/// struct Context {
///     // ... snip ...
/// }
///
/// impl SpanReferenceAware for Context {
///     fn reference_span(&mut self, _: &SpanReference) {
///         // ... snip ...
///     }
/// }
///
///
/// fn main() {
///     let mut context = SpanContext::new(ImplContextBox::new(Context {}));
///     context.set_baggage_item(String::from("key1"), String::from("value1"));
///
///     let (sender, _) = unbounded();
///     let mut span = Span::new(
///         "test",
///         SpanContext::new(ImplContextBox::new(Context {})),
///         StartOptions::default().child_of(context.clone()),
///         sender
///     );
///     span.set_baggage_item("key2", "value2");
///
///     // The parent context has one item.
///     let items: Vec<(&str, &str)> = context.baggage_items()
///         .map(|(k, v)| (&k[..], &v[..]))
///         .collect();
///     assert_eq!(items, [("key1", "value1")]);
///
///     // The child context has two.
///     let mut items: Vec<(&str, &str)> = span.context().baggage_items()
///         .map(|(k, v)| (&k[..], &v[..]))
///         .collect();
///     items.sort();
///     assert_eq!(items, [("key1", "value1"), ("key2", "value2")]);
/// }
/// ```
pub struct SpanContext {
    baggage: HashMap<String, String>,
    inner: Box<dyn ImplContext>,
}

impl SpanContext {
    /// Creates a new `SpanContext`.
    ///
    /// The new `SpanContext` has no baggage items and holds the given
    /// `ImplContext` trait object.
    pub fn new<Context: ImplContext + 'static>(inner: Context) -> SpanContext {
        SpanContext {
            inner: Box::new(inner),
            baggage: HashMap::new()
        }
    }
}

impl Clone for SpanContext {
    fn clone(&self) -> Self {
        SpanContext {
            inner: self.inner.clone(),
            baggage: self.baggage.clone()
        }
    }
}

impl fmt::Debug for SpanContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f, "SpanContext {{ inner: Box<ImplContext>, baggage: {:?} }}",
            &self.baggage
        )
    }
}

impl SpanContext {
    /// Attempt to access the `SpanContext`'s tracer details.
    ///
    /// This method is for use by `TracerInterface` developers to extract
    /// the tracer-specific span context for inject and referencing operations.
    ///
    /// Since only one `Tracer` implementation should be used throughout the
    /// running process instances of `SpanContext` are expected to hold the
    /// correct concrete `ImplContext`.
    /// 
    /// It is acceptable for `Tracer`s to panic when asked to operate on
    /// `SpanContext`s that do not hold the `Tracer`'s `ImplContext`.
    pub fn impl_context<T: Any>(&self) -> Option<&T> {
        self.inner.impl_context().downcast_ref::<T>()
    }

    /// Iterates over baggage items.
    ///
    /// The method returns an iterator over `(key, value)` tuples.
    pub fn baggage_items(&self) -> Iter<String, String> {
        self.baggage.iter()
    }

    /// Attempt to fetch a baggage item by key.
    ///
    /// If there is no item with the given key this method returns `None`.
    pub fn get_baggage_item(&self, key: &str) -> Option<&String> {
        self.baggage.get(key)
    }

    /// Update this `SpanContext` to reference another span.
    ///
    /// This method should not be called by users directly but is instead
    /// called by the `Span` referencing methods (`child_of`, `follows`).
    ///
    /// This method will call the `ImplContext::reference_span` method.
    pub fn reference_span(&mut self, reference: &SpanReference) {
        self.inner.reference_span(reference);
    }

    /// Adds or updates the baggage items with the given key/value pair.
    ///
    /// Baggage items are forwarded to `Span`s that reference this `SpanContext`
    /// and all the `Span`s that follows them.
    ///
    /// Baggage items are **NOT** propagated backwards to
    /// `Span`s that reference this `SpanContext`.
    pub fn set_baggage_item(&mut self, key: String, value: String) {
        self.baggage.insert(key, value);
    }
}


#[cfg(test)]
mod tests {
    use super::super::SpanReference;
    use super::impl_context::SpanReferenceAware;

    use super::ImplContextBox;
    use super::SpanContext;


    #[derive(Clone)]
    struct TestContext {
        pub id: String
    }
    impl SpanReferenceAware for TestContext {
        fn reference_span(&mut self, _: &SpanReference) {}
    }

    #[test]
    fn clone_span_context() {
        let clone = {
            let inner = ImplContextBox::new(TestContext{id: "A".to_owned()});
            let context = SpanContext::new(inner);
            context.clone()
        };
        let format = format!("{:?}", clone);
        assert_eq!(format, "SpanContext { inner: Box<ImplContext>, baggage: {} }");
    }

    #[test]
    fn debug_formatting() {
        let mut context = SpanContext::new(
            ImplContextBox::new(TestContext{id: "A".to_owned()})
        );
        context.set_baggage_item(String::from("key"), String::from("value"));
        let format = format!("{:?}", context);
        assert_eq!(
            format,
            r#"SpanContext { inner: Box<ImplContext>, baggage: {"key": "value"} }"#
        );
    }

    #[test]
    fn extract_implementation_context() {
        let inner = ImplContextBox::new(TestContext{id: "some-id".to_owned()});
        let context = SpanContext::new(inner);
        match context.impl_context::<TestContext>() {
            Some(ctx) => assert_eq!(ctx.id, "some-id"),
            None => panic!("Failed to downcast context")
        }
    }

    #[test]
    fn set_baggage_item() {
        let inner = ImplContextBox::new(TestContext{id: "some-id".to_owned()});
        let mut context = SpanContext::new(inner);
        context.set_baggage_item(String::from("key"), String::from("value"));
        let baggage: Vec<(String, String)> = context.baggage_items()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let expected = vec![(String::from("key"), String::from("value"))];
        assert_eq!(baggage, expected);
    }
}
