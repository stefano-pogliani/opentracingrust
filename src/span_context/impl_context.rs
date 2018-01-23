use std::any::Any;
use std::boxed::Box;
use std::marker::Send;

use super::super::SpanReference;


/// Tracer implementation's context details.
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
/// Implementations of `ImplContext` are only used by implementations of the
/// `TracerInterface` and are carried around by the `SpanContext`.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use std::any::Any;
/// use opentracingrust::ImplContext;
/// use opentracingrust::SpanContext;
/// use opentracingrust::SpanReference;
///
///
/// pub struct SomeTracerContext {
///     span_id: u64,
///     trace_id: u64,
/// }
///
/// impl ImplContext for SomeTracerContext {
///     fn impl_context(&self) -> &Any {
///         self
///     }
///
///     fn clone(&self) -> Box<ImplContext> {
///         Box::new(SomeTracerContext {
///             span_id: self.span_id,
///             trace_id: self.trace_id,
///         })
///     }
///
///     fn reference_span(&mut self, reference: &SpanReference) {
///         match *reference {
///             SpanReference::ChildOf(ref parent) |
///             SpanReference::FollowsFrom(ref parent) => {
///                 let context = parent.impl_context::<SomeTracerContext>().unwrap();
///                 self.trace_id = context.trace_id;
///             }
///         }
///     }
/// }
///
/// fn main() {
///     // ... snip ...
///
///     let span_context = SpanContext::new(SomeTracerContext {
///         span_id: 21,
///         trace_id: 42,
///     });
///
///     // ... snip ...
/// }
/// ```
pub trait ImplContext : Send {
    /// Allow runtime downcasting with the `Any` interface.
    ///
    /// `SpanContext`s store implementations `ImplContext`s using `Box`es.  
    /// This method is used by the `SpanContext::impl_context` generic method
    /// to downcast the boxed `ImplContext` using the `std::any::Any` interface.
    fn impl_context(&self) -> &Any;

    /// Clones an `ImplContext` trait object into a new `ImplContext` trait object.
    fn clone(&self) -> Box<ImplContext>;

    /// Allows the `ImplContext` to add references.
    ///
    /// When a reference is added to a `SpanContext` this method will be called
    /// so that the tracer's `ImplContext` can update its internal references.
    fn reference_span(&mut self, reference: &SpanReference);
}


/// TODO
pub struct ImplWrapper<T: Any + Clone + Send> {
    inner: T
}

impl<T: Any + Clone + Send> ImplWrapper<T> {
    /// TODO
    pub fn new(inner: T) -> ImplWrapper<T> {
        ImplWrapper { inner }
    }
}

impl<T: Any + Clone + Send + SpanReferenceAware> ImplContext for ImplWrapper<T> {
    fn impl_context(&self) -> &Any {
        &self.inner
    }

    fn clone(&self) -> Box<ImplContext> {
        Box::new(ImplWrapper {
            inner: self.inner.clone()
        })
    }

    fn reference_span(&mut self, reference: &SpanReference) {
        self.inner.reference_span(reference);
    }
}


/// TODO
pub trait SpanReferenceAware {
    /// See `ImplContext::reference_span`
    fn reference_span(&mut self, reference: &SpanReference);
}


#[cfg(test)]
mod tests {
    use super::super::super::SpanReference;
    use super::ImplContext;
    use super::ImplWrapper;
    use super::SpanReferenceAware;

    #[derive(Debug, Clone)]
    struct TestContext {
        pub id: String
    }
    impl SpanReferenceAware for TestContext {
        fn reference_span(&mut self, _: &SpanReference) {}
    }

    #[test]
    fn clone_context() {
        let clone = {
            let context = ImplWrapper::new(TestContext {
                id: "ABC".to_owned()
            });
            context.clone()
        };
        let inner = clone.impl_context();
        if let Some(inner) = inner.downcast_ref::<TestContext>() {
            assert_eq!(inner.id, "ABC");
        } else {
            panic!("Failed to downcast inner context");
        }
    }

    #[test]
    fn unwrap_context() {
        let context = ImplWrapper::new(TestContext { id: "ABC".to_owned() });
        let inner = context.impl_context();
        if let Some(inner) = inner.downcast_ref::<TestContext>() {
            assert_eq!(inner.id, "ABC");
        } else {
            panic!("Failed to downcast inner context");
        }
    }
}
