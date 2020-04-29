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
///   * The `SpanContext` structure: holding all the common data.
///   * The `ImplContext` trait: holding implementation details.
///
/// Implementations of `ImplContext` are only used by implementations of the
/// `TracerInterface` and are carried around by the `SpanContext`.
/// They contain span and trace identifiers and `Tracer` specific metadata.
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
    fn impl_context(&self) -> &dyn Any;

    /// Clones an `ImplContext` trait object into a new `ImplContext` trait object.
    fn clone(&self) -> Box<dyn ImplContext>;

    /// Allows the `ImplContext` to add references.
    ///
    /// When a reference is added to a `SpanContext` this method will be called
    /// so that the tracer's `ImplContext` can update its internal references.
    fn reference_span(&mut self, reference: &SpanReference);
}


/// Utility structure to create `ImplContext`s.
///
/// Generic structure that creates a wrapper around structures to make them
/// compatible with `ImplContext` trait objects.
///
/// The wrapped structure requires the following traits to be implemented:
///
///   * `Any`
///   * `Clone`
///   * `Send`
///   * `SpanReferenceAware`
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use opentracingrust::ImplContext;
/// use opentracingrust::ImplContextBox;
/// use opentracingrust::SpanReference;
/// use opentracingrust::SpanReferenceAware;
///
/// #[derive(Clone)]
/// struct Context {
///     // ... snip ...
/// }
///
/// impl SpanReferenceAware for Context {
///     fn reference_span(&mut self, reference: &SpanReference) {
///         // ... snip ...
///     }
/// }
///
/// fn main() {
///     let context: Box<ImplContext> = Box::new(ImplContextBox::new(Context{}));
///     // ... snip ...
/// }
/// ```
pub struct ImplContextBox<T: Any + Clone + Send + SpanReferenceAware> {
    inner: T
}

impl<T: Any + Clone + Send + SpanReferenceAware> ImplContextBox<T> {
    /// Wrap a compatible value into a `ImplContextBox`.
    pub fn new(inner: T) -> ImplContextBox<T> {
        ImplContextBox { inner }
    }
}

impl<T: Any + Clone + Send + SpanReferenceAware> ImplContext for ImplContextBox<T> {
    fn impl_context(&self) -> &dyn Any {
        &self.inner
    }

    fn clone(&self) -> Box<dyn ImplContext> {
        Box::new(ImplContextBox {
            inner: self.inner.clone()
        })
    }

    fn reference_span(&mut self, reference: &SpanReference) {
        self.inner.reference_span(reference);
    }
}


/// Trait for structures that want to be wrapped in `ImplContextBox`s.
///
/// See `ImplContext` for more information.
pub trait SpanReferenceAware {
    /// See `ImplContext::reference_span`
    fn reference_span(&mut self, reference: &SpanReference);
}


#[cfg(test)]
mod tests {
    use super::super::super::SpanReference;
    use super::ImplContext;
    use super::ImplContextBox;
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
            let context = ImplContextBox::new(TestContext {
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
        let context = ImplContextBox::new(TestContext { id: "ABC".to_owned() });
        let inner = context.impl_context();
        if let Some(inner) = inner.downcast_ref::<TestContext>() {
            assert_eq!(inner.id, "ABC");
        } else {
            panic!("Failed to downcast inner context");
        }
    }
}
