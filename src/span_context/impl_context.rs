use std::any::Any;
use std::boxed::Box;

use super::super::SpanReference;


/// TODO
pub trait ImplContext {
    /// TODO
    fn impl_context(&self) -> &Any;

    /// TODO
    fn clone(&self) -> Box<ImplContext>;

    /// TODO
    fn reference_span(&mut self, _reference: &SpanReference);
}


/// TODO
pub struct ImplWrapper<T: Any + Clone> {
    inner: T
}

impl<T: Any + Clone> ImplWrapper<T> {
    /// TODO
    pub fn new(inner: T) -> ImplWrapper<T> {
        ImplWrapper { inner }
    }
}

impl<T: Any + Clone + SpanReferenceAware> ImplContext for ImplWrapper<T> {
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
    /// TODO
    fn reference_span(&mut self, _reference: &SpanReference);
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
