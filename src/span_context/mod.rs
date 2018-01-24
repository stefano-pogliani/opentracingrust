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


/// TODO
///
/// IDEA:
///   Separate the `SpanContext` into a tracer specific `ImplContext` and
///   a set of common features shared by all tracer implementations.
///
///   The `ImplContext` uses the `std::any::Any` features to define a generic
///   wrapper for implementation specific code.
pub struct SpanContext {
    baggage: HashMap<String, String>,
    inner: Box<ImplContext>,
}

impl SpanContext {
    /// TODO
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
    /// TODO
    pub fn impl_context<T: Any>(&self) -> Option<&T> {
        self.inner.impl_context().downcast_ref::<T>()
    }

    /// TODO
    pub fn baggage_items(&self) -> Iter<String, String> {
        self.baggage.iter()
    }

    /// TODO
    pub fn get_baggage_item(&self, key: &str) -> Option<&String> {
        self.baggage.get(key)
    }

    /// TODO
    pub fn reference_span(&mut self, reference: &SpanReference) {
        self.inner.reference_span(reference);
    }

    /// TODO
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
