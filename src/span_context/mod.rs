use std::any::Any;
use std::boxed::Box;
use std::fmt;

mod baggage;
mod impl_context;

pub use self::baggage::BaggageItem;
pub use self::impl_context::ImplContext;
pub use self::impl_context::ImplWrapper;


/// TODO
///
/// IDEA:
///   Separate the `SpanContext` into a tracer specific `ImplContext` and
///   a set of common features shared by all tracer implementations.
///
///   The `ImplContext` uses the `std::any::Any` features to define a generic
///   wrapper for implementation specific code.
pub struct SpanContext {
    inner: Box<ImplContext>,
    baggage: Vec<BaggageItem>
}

impl SpanContext {
    /// TODO
    pub fn new<Context: 'static + ImplContext>(inner: Context) -> SpanContext {
        SpanContext {
            inner: Box::new(inner),
            baggage: vec![]
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
    pub fn baggage_items(&self) -> &[BaggageItem] {
        &self.baggage
    }

    /// TODO
    pub fn set_baggage_item(&mut self, item: BaggageItem) {
        self.baggage.retain(|x| x.key() != item.key());
        self.baggage.push(item);
    }
}


#[cfg(test)]
mod tests {
    use super::BaggageItem;
    use super::ImplWrapper;
    use super::SpanContext;

    #[derive(Clone)]
    struct TestContext {
        pub id: String
    }

    #[test]
    fn clone_span_context() {
        let clone = {
            let inner = ImplWrapper::new(TestContext{id: "A".to_owned()});
            let context = SpanContext::new(inner);
            context.clone()
        };
        let format = format!("{:?}", clone);
        assert_eq!(format, "SpanContext { inner: Box<ImplContext>, baggage: [] }");
    }

    #[test]
    fn debug_formatting() {
        let mut context = SpanContext::new(
            ImplWrapper::new(TestContext{id: "A".to_owned()})
        );
        context.set_baggage_item(BaggageItem::new("key", "value"));
        let format = format!("{:?}", context);
        assert_eq!(
            format,
            r#"SpanContext { inner: Box<ImplContext>, baggage: [BaggageItem { key: "key", value: "value" }] }"#
        );
    }

    #[test]
    fn extract_implementation_context() {
        let inner = ImplWrapper::new(TestContext{id: "some-id".to_owned()});
        let context = SpanContext::new(inner);
        match context.impl_context::<TestContext>() {
            Some(ctx) => assert_eq!(ctx.id, "some-id"),
            None => panic!("Failed to downcast context")
        }
    }

    #[test]
    fn set_baggage_item() {
        let inner = ImplWrapper::new(TestContext{id: "some-id".to_owned()});
        let mut context = SpanContext::new(inner);
        context.set_baggage_item(BaggageItem::new("key", "value"));
        let baggage = context.baggage_items();
        let expected = vec![BaggageItem::new("key", "value")];
        assert_eq!(baggage, &expected[..]);
    }
}
