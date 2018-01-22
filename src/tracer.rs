use super::ExtractFormat;
use super::InjectFormat;

use super::Result;
use super::Span;
use super::SpanContext;
use super::StartOptions;


/// TODO
pub trait TracerInterface : Send {
    /// TODO
    fn extract(&self, fmt: ExtractFormat) -> Result<Option<SpanContext>>;

    /// TODO
    fn inject(&self, context: &SpanContext, fmt: InjectFormat) -> Result<()>;

    /// TODO
    fn span(&self, name: &str, options: StartOptions) -> Span;
}


/// TODO
pub struct Tracer {
    tracer: Box<TracerInterface>
}

impl Tracer {
    /// TODO
    pub fn new<T: TracerInterface + 'static>(tracer: T) -> Tracer {
        Tracer {
            tracer: Box::new(tracer)
        }
    }
}

impl Tracer {
    /// TODO
    pub fn extract(&self, fmt: ExtractFormat) -> Result<Option<SpanContext>> {
        self.tracer.extract(fmt)
    }

    /// TODO
    pub fn inject(
        &self, context: &SpanContext, fmt: InjectFormat
    ) -> Result<()> {
        self.tracer.inject(context, fmt)
    }

    /// TODO
    pub fn span(&self, name: &str, options: StartOptions) -> Span {
        self.tracer.span(name, options)
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io;
    use std::io::BufRead;
    use std::sync::mpsc;


    use super::super::ExtractFormat;
    use super::super::InjectFormat;

    use super::super::ImplWrapper;
    use super::super::Result;
    use super::super::Span;
    use super::super::SpanContext;
    use super::super::SpanReference;
    use super::super::SpanReferenceAware;
    use super::super::SpanSender;
    use super::super::StartOptions;
    use super::super::span_context::BaggageItem;

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

                    let mut context = SpanContext::new(ImplWrapper::new(
                        TestContext { name: name.trim().to_owned() }
                    ));
                    for line in reader.lines() {
                        let line = line?;
                        let cells: Vec<&str> = line.split(':').collect();
                        context.set_baggage_item(BaggageItem::new(cells[0], cells[1]));
                    }
                    Ok(Some(context))
                }

                ExtractFormat::HttpHeaders(carrier) => {
                    let mut context = SpanContext::new(ImplWrapper::new(
                        TestContext { name: carrier.get("Span-Name").unwrap() }
                    ));
                    let items = carrier.find_items(Box::new(
                        |k| k.starts_with("Baggage-")
                    ));
                    for (key, value) in items {
                        context.set_baggage_item(
                            BaggageItem::new(&key[8..], value)
                        );
                    }
                    Ok(Some(context))
                }

                ExtractFormat::TextMap(carrier) => {
                    let mut context = SpanContext::new(ImplWrapper::new(
                        TestContext { name: carrier.get("span-name").unwrap() }
                    ));
                    let items = carrier.find_items(Box::new(
                        |k| k.starts_with("baggage-")
                    ));
                    for (key, value) in items {
                        context.set_baggage_item(
                            BaggageItem::new(&key[8..], value)
                        );
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
                    for item in context.baggage_items() {
                        carrier.write_fmt(format_args!(
                            "Baggage-{}: {}\n", item.key(), item.value()
                        ))?;
                    }
                    Ok(())
                }

                InjectFormat::HttpHeaders(carrier) => {
                    let inner = context.impl_context::<TestContext>().unwrap();
                    carrier.set("Trace-Id", "123");
                    carrier.set("Span-Name", &inner.name);
                    for item in context.baggage_items() {
                        let key = format!("Baggage-{}", item.key());
                        carrier.set(&key, item.value());
                    }
                    Ok(())
                }

                InjectFormat::TextMap(carrier) => {
                    let inner = context.impl_context::<TestContext>().unwrap();
                    carrier.set("trace-id", "123");
                    carrier.set("span-name", &inner.name);
                    for item in context.baggage_items() {
                        let key = format!("baggage-{}", item.key());
                        carrier.set(&key, item.value());
                    }
                    Ok(())
                }
            }
        }

        fn span(&self, name: &str, options: StartOptions) -> Span {
            let context = SpanContext::new(ImplWrapper::new(TestContext {
                name: String::from("test-span")
            }));
            Span::new(name, context, options, self.sender.clone())
        }
    }


    #[test]
    fn create_span() {
        let (sender, _) = mpsc::channel();
        let tracer = Tracer::new(TestTracer {sender});
        let _span: Span = tracer.span("test-span", StartOptions::default());
    }

    #[test]
    fn extract_binary() {
        let mut buffer = io::Cursor::new("test-span\na:b\n");
        let (sender, _) = mpsc::channel();
        let tracer = Tracer::new(TestTracer {sender});
        let context = tracer.extract(
            ExtractFormat::Binary(Box::new(&mut buffer))
        ).unwrap().unwrap();
        let inner = context.impl_context::<TestContext>().unwrap();
        assert_eq!("test-span", inner.name);
        assert_eq!(context.baggage_items(), [BaggageItem::new("a", "b")]);
    }

    #[test]
    fn extract_http_headers() {
        let mut map = HashMap::new();
        map.insert(String::from("Span-Name"), String::from("2"));
        map.insert(String::from("Baggage-a"), String::from("b"));
        let (sender, _) = mpsc::channel();
        let tracer = Tracer::new(TestTracer {sender});
        let context = tracer.extract(ExtractFormat::HttpHeaders(Box::new(&map))).unwrap().unwrap();
        let inner = context.impl_context::<TestContext>().unwrap();
        assert_eq!("2", inner.name);
        assert_eq!(context.baggage_items(), [BaggageItem::new("a", "b")]);
    }

    #[test]
    fn extract_textmap() {
        let mut map = HashMap::new();
        map.insert(String::from("span-name"), String::from("2"));
        map.insert(String::from("baggage-a"), String::from("b"));
        let (sender, _) = mpsc::channel();
        let tracer = Tracer::new(TestTracer {sender});
        let context = tracer.extract(ExtractFormat::TextMap(Box::new(&map))).unwrap().unwrap();
        let inner = context.impl_context::<TestContext>().unwrap();
        assert_eq!("2", inner.name);
        assert_eq!(context.baggage_items(), [BaggageItem::new("a", "b")]);
    }

    #[test]
    fn inject_binary() {
        let (sender, _) = mpsc::channel();
        let tracer = Tracer::new(TestTracer {sender});
        let mut span = tracer.span("test-span", StartOptions::default());
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
        let (sender, _) = mpsc::channel();
        let tracer = Tracer::new(TestTracer {sender});
        let mut span = tracer.span("test-span", StartOptions::default());
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
        let (sender, _) = mpsc::channel();
        let tracer = Tracer::new(TestTracer {sender});
        let mut span = tracer.span("test-span", StartOptions::default());
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
