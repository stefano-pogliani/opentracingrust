use super::ExtractFormat;
use super::InjectFormat;

use super::Result;
use super::Span;
use super::SpanContext;
use super::StartOptions;


/// Smallest set of operations that a concrete tracer must implement.
///
/// While OpenTracingRust users develop against the `Tracer` structure,
/// the logic of a tracer is implemented through this trait.
///
/// `Tracer` is therefore a wrapper around `TracerInterface` trait objects
/// and provides some helper methods that are useful for all tracers.
///
/// # Implementing tracers
///
/// OpenTracingRust aims to minimise the amount of work to implement tracers.
/// This is achieved by `Box`ing traits into structures that are passed around my clients.
///
/// The following elements must be provided by tracer implementations:
///
///   * An inner context that implements `ImplContext` to store a tracer specific span id.
///   * An inner tracer that implements `TracerInterface` to inject/extract/create spans.
///   * A function that sends `FinishedSpan`s to the distributed tracer.
///
/// How these elements are implemented and what they do is up to the tracer implementation
/// with one exception: `FinishedSpan`s are sent over an `crossbeam_channel::unbounded`
/// so `ImplContext` has to be `Send`.
///
/// # Examples
///
/// If you are looking to implement your tracer checkout the following first:
///
///   * The `FileTracer` implementation that is part of OpenTracingRust.
///   * Example `1-custom-tracer.rs`, which implements an in-memory tracer.
pub trait TracerInterface : Send + Sync {
    /// Attempt to extract a SpanContext from a carrier.
    fn extract(&self, fmt: ExtractFormat) -> Result<Option<SpanContext>>;

    /// Inject tracing information into a carrier.
    fn inject(&self, context: &SpanContext, fmt: InjectFormat) -> Result<()>;

    /// Create a new `Span` with the given operation name and starting options.
    fn span(&self, name: &str, options: StartOptions) -> Span;
}


/// The library users interface to tracing.
///
/// This structure is the focus point for clients to use in combination with `SpanContext`.
/// The configured tracer is stored in this structure and backs the available methods.
///
/// The `Tracer` structure also provides some utility methods to make common operations easier.
pub struct Tracer {
    tracer: Box<TracerInterface>
}

impl Tracer {
    /// Creates a new `Tracer` for a concrete tracer.
    pub fn new<T: TracerInterface + 'static>(tracer: T) -> Tracer {
        Tracer {
            tracer: Box::new(tracer)
        }
    }
}

impl Tracer {
    /// Attempt to extract a SpanContext from a carrier.
    ///
    /// If the carrier (i.e, HTTP Request, RPC Message, ...) includes tracing information
    /// this method returns `Ok(Some(context))`, otherwise `Ok(None)` is returned.
    ///
    /// If the method fails to extract a context because the carrier fails or because
    /// the tracing information is incorrectly formatted an `Error` is returned.
    pub fn extract(&self, fmt: ExtractFormat) -> Result<Option<SpanContext>> {
        self.tracer.extract(fmt)
    }

    /// Inject tracing information into a carrier.
    ///
    /// If the method fails to inject the context because the carrier fails.
    pub fn inject(
        &self, context: &SpanContext, fmt: InjectFormat
    ) -> Result<()> {
        self.tracer.inject(context, fmt)
    }

    /// Create a new `Span` with the given operation name and default starting options.
    pub fn span(&self, name: &str) -> Span {
        self.span_with_options(name, StartOptions::default())
    }

    /// Create a new `Span` with the given operation name and starting options.
    pub fn span_with_options(&self, name: &str, options: StartOptions) -> Span {
        self.tracer.span(name, options)
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io;
    use std::io::BufRead;

    use crossbeam_channel::unbounded;

    use super::super::ExtractFormat;
    use super::super::InjectFormat;

    use super::super::ImplContextBox;
    use super::super::Result;
    use super::super::Span;
    use super::super::SpanContext;
    use super::super::SpanReference;
    use super::super::SpanReferenceAware;
    use super::super::SpanSender;
    use super::super::StartOptions;

    use super::Tracer;
    use super::TracerInterface;


    #[derive(Debug, Clone)]
    struct TestContext {
        pub name: String
    }
    impl SpanReferenceAware for TestContext {
        fn reference_span(&mut self, _: &SpanReference) {}
    }

    struct TestTracer {
        sender: SpanSender
    }
    impl TracerInterface for TestTracer {
        fn extract(&self, fmt: ExtractFormat) -> Result<Option<SpanContext>> {
            match fmt {
                ExtractFormat::Binary(carrier) => {
                    let mut reader = self::io::BufReader::new(carrier);
                    let mut name = String::new();
                    reader.read_line(&mut name)?;

                    let mut context = SpanContext::new(ImplContextBox::new(
                        TestContext { name: name.trim().to_owned() }
                    ));
                    for line in reader.lines() {
                        let line = line?;
                        let cells: Vec<&str> = line.split(':').collect();
                        context.set_baggage_item(String::from(cells[0]), String::from(cells[1]));
                    }
                    Ok(Some(context))
                }

                ExtractFormat::HttpHeaders(carrier) => {
                    let mut context = SpanContext::new(ImplContextBox::new(
                        TestContext { name: carrier.get("Span-Name").unwrap() }
                    ));
                    for (key, value) in carrier.items() {
                        if key.starts_with("Baggage-") {
                            context.set_baggage_item(String::from(&key[8..]), value.clone());
                        }
                    }
                    Ok(Some(context))
                }

                ExtractFormat::TextMap(carrier) => {
                    let mut context = SpanContext::new(ImplContextBox::new(
                        TestContext { name: carrier.get("span-name").unwrap() }
                    ));
                    for (key, value) in carrier.items() {
                        if key.starts_with("baggage-") {
                            context.set_baggage_item(String::from(&key[8..]), value.clone());
                        }
                    }
                    Ok(Some(context))
                }
            }
        }

        fn inject(
            &self, context: &SpanContext, fmt: InjectFormat
        ) -> Result<()> {
            match fmt {
                InjectFormat::Binary(carrier) => {
                    let inner = context.impl_context::<TestContext>().unwrap();
                    carrier.write_fmt(format_args!("TraceId: {}\n", "123"))?;
                    carrier.write_fmt(
                        format_args!("Span Name: {}\n", &inner.name)
                    )?;
                    for (key, value) in context.baggage_items() {
                        carrier.write_fmt(format_args!("Baggage-{}: {}\n", key, value))?;
                    }
                    Ok(())
                }

                InjectFormat::HttpHeaders(carrier) => {
                    let inner = context.impl_context::<TestContext>().unwrap();
                    carrier.set("Trace-Id", "123");
                    carrier.set("Span-Name", &inner.name);
                    for (key, value) in context.baggage_items() {
                        let key = format!("Baggage-{}", key);
                        carrier.set(&key, value);
                    }
                    Ok(())
                }

                InjectFormat::TextMap(carrier) => {
                    let inner = context.impl_context::<TestContext>().unwrap();
                    carrier.set("trace-id", "123");
                    carrier.set("span-name", &inner.name);
                    for (key, value) in context.baggage_items() {
                        let key = format!("baggage-{}", key);
                        carrier.set(&key, value);
                    }
                    Ok(())
                }
            }
        }

        fn span(&self, name: &str, options: StartOptions) -> Span {
            let context = SpanContext::new(ImplContextBox::new(TestContext {
                name: String::from("test-span")
            }));
            Span::new(name, context, options, self.sender.clone())
        }
    }


    #[test]
    fn create_span() {
        let (sender, _) = unbounded();
        let tracer = Tracer::new(TestTracer {sender});
        let _span: Span = tracer.span("test-span");
    }

    #[test]
    fn extract_binary() {
        let mut buffer = io::Cursor::new("test-span\na:b\n");
        let (sender, _) = unbounded();
        let tracer = Tracer::new(TestTracer {sender});
        let context = tracer.extract(
            ExtractFormat::Binary(Box::new(&mut buffer))
        ).unwrap().unwrap();
        let inner = context.impl_context::<TestContext>().unwrap();
        assert_eq!("test-span", inner.name);
        let items: Vec<(String, String)> = context.baggage_items()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_eq!(items, vec![(String::from("a"), String::from("b"))]);
    }

    #[test]
    fn extract_http_headers() {
        let mut map = HashMap::new();
        map.insert(String::from("Span-Name"), String::from("2"));
        map.insert(String::from("Baggage-a"), String::from("b"));
        let (sender, _) = unbounded();
        let tracer = Tracer::new(TestTracer {sender});
        let context = tracer.extract(ExtractFormat::HttpHeaders(Box::new(&map))).unwrap().unwrap();
        let inner = context.impl_context::<TestContext>().unwrap();
        assert_eq!("2", inner.name);
        let items: Vec<(String, String)> = context.baggage_items()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_eq!(items, vec![(String::from("a"), String::from("b"))]);
    }

    #[test]
    fn extract_textmap() {
        let mut map = HashMap::new();
        map.insert(String::from("span-name"), String::from("2"));
        map.insert(String::from("baggage-a"), String::from("b"));
        let (sender, _) = unbounded();
        let tracer = Tracer::new(TestTracer {sender});
        let context = tracer.extract(ExtractFormat::TextMap(Box::new(&map))).unwrap().unwrap();
        let inner = context.impl_context::<TestContext>().unwrap();
        assert_eq!("2", inner.name);
        let items: Vec<(String, String)> = context.baggage_items()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_eq!(items, vec![(String::from("a"), String::from("b"))]);
    }

    #[test]
    fn inject_binary() {
        let (sender, _) = unbounded();
        let tracer = Tracer::new(TestTracer {sender});
        let mut span = tracer.span("test-span");
        span.set_baggage_item("a", "b");

        let mut buffer: Vec<u8> = Vec::new();
        tracer.inject(span.context(), InjectFormat::Binary(Box::new(&mut buffer))).unwrap();
        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "TraceId: 123\nSpan Name: test-span\nBaggage-a: b\n"
        );
    }

    #[test]
    fn inject_http_headers() {
        let (sender, _) = unbounded();
        let tracer = Tracer::new(TestTracer {sender});
        let mut span = tracer.span("test-span");
        span.set_baggage_item("a", "b");

        let mut map = HashMap::new();
        tracer.inject(span.context(), InjectFormat::HttpHeaders(Box::new(&mut map))).unwrap();

        let mut items: Vec<(String, String)> = map.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        items.sort();
        assert_eq!(items, [
            (String::from("Baggage-a"), String::from("b")),
            (String::from("Span-Name"), String::from("test-span")),
            (String::from("Trace-Id"), String::from("123"))
        ]);
    }

    #[test]
    fn inject_textmap() {
        let (sender, _) = unbounded();
        let tracer = Tracer::new(TestTracer {sender});
        let mut span = tracer.span("test-span");
        span.set_baggage_item("a", "b");

        let mut map = HashMap::new();
        tracer.inject(span.context(), InjectFormat::TextMap(Box::new(&mut map))).unwrap();

        let mut items: Vec<(String, String)> = map.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        items.sort();
        assert_eq!(items, [
            (String::from("baggage-a"), String::from("b")),
            (String::from("span-name"), String::from("test-span")),
            (String::from("trace-id"), String::from("123"))
        ]);
    }
}
