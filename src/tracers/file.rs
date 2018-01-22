use std::io;
use std::io::Write;
use std::sync::mpsc;

use rand::random;

use super::super::BaggageItem;
use super::super::ImplWrapper;
use super::super::Result;

use super::super::FinishedSpan;
use super::super::Span;
use super::super::SpanContext;
use super::super::SpanReceiver;
use super::super::SpanReference;
use super::super::SpanReferenceAware;
use super::super::SpanSender;
use super::super::StartOptions;

use super::super::ExtractFormat;
use super::super::InjectFormat;
use super::super::Tracer;
use super::super::TracerInterface;


const BAGGAGE_KEY_PREFIX: &str = "Baggage-";
const SPAN_ID_KEY: &str = "SpanID";
const TRACE_ID_KEY: &str = "TraceID";


/// TODO
pub struct FileTracer {
    sender: SpanSender
}

impl TracerInterface for FileTracer {
    /// Extract a span context from a text map or HTTP headers.
    ///
    /// Note that the binary extraction format is not supported by `FileTracer`.
    fn extract(&self, fmt: ExtractFormat) -> Result<Option<SpanContext>> {
        match fmt {
            ExtractFormat::HttpHeaders(carrier) => {
                // Decode trace and span IDs.
                let trace_id = carrier.get(TRACE_ID_KEY);
                if trace_id.is_none() {
                    return Ok(None);
                }
                let trace_id = trace_id.unwrap().parse::<u64>()?;

                let span_id = carrier.get(SPAN_ID_KEY);
                if span_id.is_none() {
                    return Ok(None);
                }
                let span_id = span_id.unwrap().parse::<u64>()?;

                // Create a mutable context to load baggage items.
                let mut context = SpanContext::new(ImplWrapper::new(
                    FileTracerContext {
                        trace_id,
                        span_id
                    }
                ));

                // Decode baggage items.
                let items = carrier.find_items(Box::new(
                    |k| k.starts_with(BAGGAGE_KEY_PREFIX)
                ));
                for (key, value) in items {
                    context.set_baggage_item(BaggageItem::new(key, value));
                }
                Ok(Some(context))
            },
            _ => panic!("Unsupported extraction format")
        }
    }

    /// Inject the span context into a text map or HTTP headers.
    ///
    /// Note that the binary injection format is not supported by `FileTracer`.
    fn inject(&self, context: &SpanContext, fmt: InjectFormat) -> Result<()> {
        let span_context = context;
        let context = span_context.impl_context::<FileTracerContext>();
        let context = context.expect(
            "Unsupported span, was it created by FileTracer?"
        );
        match fmt {
            InjectFormat::HttpHeaders(carrier) |
            InjectFormat::TextMap(carrier) => {
                carrier.set(TRACE_ID_KEY, &context.trace_id.to_string());
                carrier.set(SPAN_ID_KEY, &context.span_id.to_string());
                for item in span_context.baggage_items() {
                    let key = format!("{}{}", BAGGAGE_KEY_PREFIX, item.key());
                    carrier.set(&key, item.value());
                }
                Ok(())
            },
            _ => panic!("Unsupported injection format")
        }
    }

    fn span(&self, name: &str, options: StartOptions) -> Span {
        let trace_id = random::<u64>();
        let span_id = random::<u64>();
        let context = SpanContext::new(ImplWrapper::new(FileTracerContext {
            trace_id,
            span_id
        }));
        Span::new(name, context, options, self.sender.clone())
    }
}

impl FileTracer {
    /// TODO
    pub fn new() -> (Tracer, SpanReceiver) {
        let (sender, receiver) = mpsc::channel();
        let tracer = FileTracer { sender };
        (Tracer::new(tracer), receiver)
    }

    /// TODO
    pub fn write_trace<W: Write>(
        span: FinishedSpan, file: &mut W
    ) -> io::Result<()> {
        let context = span.context().impl_context::<FileTracerContext>();
        let context = context.expect(
            "Unsupported span, was it created by FileTracer?"
        );
        let mut buffer = String::new();
        buffer.push_str(&format!("==>> Trace ID: {}\n", context.trace_id));
        buffer.push_str(&format!("===> Span ID: {}\n", context.span_id));

        let finish = span.finish_time();
        let start = span.start_time().clone();
        let duration = finish.duration_since(start).unwrap();
        let secs = duration.as_secs() as f64;
        let delta = secs + duration.subsec_nanos() as f64 * 1e-9;
        buffer.push_str(&format!("===> Span Duration: {}\n", delta));

        buffer.push_str("===> References: [\n");
        for reference in span.references() {
            let ref_id = match reference {
                &SpanReference::ChildOf(ref parent) |
                &SpanReference::FollowsFrom(ref parent) => {
                    let context = parent.impl_context::<FileTracerContext>();
                    let context = context.expect(
                        "Unsupported span context, was it created by FileTracer?"
                    );
                    context.span_id
                }
            };
            let ref_type = match reference {
                &SpanReference::ChildOf(_) => "Child of span ID",
                &SpanReference::FollowsFrom(_) => "Follows from span ID"
            };
            buffer.push_str(&format!("===>   * {}: {}\n", ref_type, ref_id));
        }
        buffer.push_str("===> ]\n");

        buffer.push_str("===> Baggage items: [\n");
        for item in span.context().baggage_items() {
            buffer.push_str(&format!("===>   * {}: {}\n", item.key(), item.value()));
        }
        buffer.push_str("===> ]\n");
        file.write_all(buffer.as_bytes())
    }
}


/// TODO
#[derive(Clone, Debug)]
struct FileTracerContext {
    trace_id: u64,
    span_id: u64
}

impl SpanReferenceAware for FileTracerContext {
    fn reference_span(&mut self, reference: &SpanReference) {
        match reference {
            &SpanReference::ChildOf(ref parent) |
            &SpanReference::FollowsFrom(ref parent) => {
                let context = parent.impl_context::<FileTracerContext>();
                let context = context.expect(
                    "Unsupported span context, was it created by FileTracer?"
                );
                self.trace_id = context.trace_id;
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::super::super::ImplWrapper;
    use super::super::super::SpanContext;
    use super::super::super::SpanReceiver;
    use super::super::super::Tracer;

    use super::FileTracer;
    use super::FileTracerContext;

    fn make_context(trace_id: u64, span_id: u64) -> SpanContext {
        SpanContext::new(ImplWrapper::new(FileTracerContext {
            trace_id,
            span_id
        }))
    }

    fn make_tracer() -> (Tracer, SpanReceiver) {
        FileTracer::new()
    }


    mod span {
        use super::super::super::super::StartOptions;

        use super::super::FileTracer;
        use super::super::FileTracerContext;
        use super::make_context;
        use super::make_tracer;


        mod extract {
            use std::collections::HashMap;
            use std::io;

            use super::super::super::super::super::Error;
            use super::super::super::super::super::ExtractFormat;

            use super::FileTracerContext;
            use super::make_tracer;


            mod invalid {
                use std::collections::HashMap;

                use super::Error;
                use super::ExtractFormat;
                use super::make_tracer;

                #[test]
                fn fails_if_invalid_span_id() {
                    let (tracer, _) = make_tracer();
                    let mut map: HashMap<String, String> = HashMap::new();
                    map.insert(String::from("TraceID"), String::from("123"));
                    map.insert(String::from("SpanID"), String::from("abc"));
                    let context = tracer.extract(
                        ExtractFormat::HttpHeaders(Box::new(&map))
                    );
                    match context {
                        Err(Error::ParseIntError(_)) => {},
                        Err(err) => panic!("Unexpected error: {:?}", err),
                        Ok(success) => panic!("Unexpected ok: {:?}", success)
                    }
                }

                #[test]
                fn fails_if_invalid_trace_id() {
                    let (tracer, _) = make_tracer();
                    let mut map: HashMap<String, String> = HashMap::new();
                    map.insert(String::from("TraceID"), String::from("abc"));
                    let context = tracer.extract(
                        ExtractFormat::HttpHeaders(Box::new(&map))
                    );
                    match context {
                        Err(Error::ParseIntError(_)) => {},
                        Err(err) => panic!("Unexpected error: {:?}", err),
                        Ok(success) => panic!("Unexpected ok: {:?}", success)
                    }
                }

                #[test]
                fn returns_none_without_trace_id() {
                    let (tracer, _) = make_tracer();
                    let map: HashMap<String, String> = HashMap::new();
                    let context = tracer.extract(
                        ExtractFormat::HttpHeaders(Box::new(&map))
                    );
                    match context {
                        Err(err) => panic!("Unexpected error: {:?}", err),
                        Ok(Some(success)) => panic!(
                            "Unexpected some: {:?}", success
                        ),
                        Ok(None) => {}
                    }
                }

                #[test]
                fn returns_none_without_span_id() {
                    let (tracer, _) = make_tracer();
                    let mut map: HashMap<String, String> = HashMap::new();
                    map.insert(String::from("TraceID"), String::from("123"));
                    let context = tracer.extract(
                        ExtractFormat::HttpHeaders(Box::new(&map))
                    );
                    match context {
                        Err(err) => panic!("Unexpected error: {:?}", err),
                        Ok(Some(success)) => panic!(
                            "Unexpected some: {:?}", success
                        ),
                        Ok(None) => {}
                    }
                }
            }

            #[test]
            #[should_panic(expected = "Unsupported extraction format")]
            fn binary_not_supported() {
                let (tracer, _) = make_tracer();
                let mut stdin = io::stdin();
                tracer.extract(
                    ExtractFormat::Binary(Box::new(&mut stdin))
                ).unwrap();
            }

            #[test]
            fn http_headers() {
                let (tracer, _) = make_tracer();
                let mut map: HashMap<String, String> = HashMap::new();
                map.insert(String::from("TraceID"), String::from("1234"));
                map.insert(String::from("SpanID"), String::from("5678"));
                map.insert(String::from("Baggage-Item1"), String::from("ab"));
                map.insert(String::from("Baggage-Item2"), String::from("cd"));

                let context = tracer.extract(
                    ExtractFormat::HttpHeaders(Box::new(&map))
                ).unwrap().unwrap();
                let inner = context.impl_context::<FileTracerContext>();
                let inner = inner.unwrap();

                assert_eq!(1234, inner.trace_id);
                assert_eq!(5678, inner.span_id);
                assert_eq!(
                    "ab",
                    context.get_baggage_item("Baggage-Item1").unwrap().value()
                );
                assert_eq!(
                    "cd",
                    context.get_baggage_item("Baggage-Item2").unwrap().value()
                );
            }
        }


        mod inject {
            use std::collections::HashMap;
            use std::io;

            use super::super::super::super::super::span_context::BaggageItem;
            use super::super::super::super::super::InjectFormat;
            use super::make_context;
            use super::make_tracer;


            #[test]
            #[should_panic(expected = "Unsupported injection format")]
            fn binary_not_supported() {
                let (tracer, _) = make_tracer();
                let context = make_context(1234, 1234);
                let mut stdout = io::stdout();
                tracer.inject(
                    &context,
                    InjectFormat::Binary(Box::new(&mut stdout))
                ).unwrap();
            }

            #[test]
            fn http_headers() {
                let (tracer, _) = make_tracer();
                let mut context = make_context(1234, 5678);
                let mut map: HashMap<String, String> = HashMap::new();
                context.set_baggage_item(BaggageItem::new("Item1", "ab"));
                context.set_baggage_item(BaggageItem::new("Item2", "cd"));
                tracer.inject(
                    &context,
                    InjectFormat::HttpHeaders(Box::new(&mut map))
                ).unwrap();

                assert_eq!("1234", map.get("TraceID").unwrap());
                assert_eq!("5678", map.get("SpanID").unwrap());
                assert_eq!("ab", map.get("Baggage-Item1").unwrap());
                assert_eq!("cd", map.get("Baggage-Item2").unwrap());
            }

            #[test]
            fn text_map() {
                let (tracer, _) = make_tracer();
                let mut context = make_context(1234, 5678);
                let mut map: HashMap<String, String> = HashMap::new();
                context.set_baggage_item(BaggageItem::new("Item1", "ab"));
                context.set_baggage_item(BaggageItem::new("Item2", "cd"));
                tracer.inject(
                    &context,
                    InjectFormat::TextMap(Box::new(&mut map))
                ).unwrap();

                assert_eq!("1234", map.get("TraceID").unwrap());
                assert_eq!("5678", map.get("SpanID").unwrap());
                assert_eq!("ab", map.get("Baggage-Item1").unwrap());
                assert_eq!("cd", map.get("Baggage-Item2").unwrap());
            }
        }


        #[test]
        fn create() {
            let (tracer, _) = make_tracer();
            let span = tracer.span("test1", StartOptions::default());
            let context = span.context().impl_context::<FileTracerContext>();
            context.unwrap();
        }

        #[test]
        fn write() {
            let (tracer, receiver) = make_tracer();
            let mut span = tracer.span("test1", StartOptions::default());
            span.child_of(make_context(123456, 123));
            span.follows(make_context(123456, 456));
            span.set_baggage_item("TestKey", "Test Value");
            span.finish().unwrap();

            let mut buffer = Vec::new();
            let span = receiver.recv().unwrap();
            FileTracer::write_trace::<Vec<u8>>(span, &mut buffer).unwrap();

            let buffer = String::from_utf8(buffer).unwrap();
            let mut buffer = buffer.split('\n');
            assert_eq!(buffer.next().unwrap(), "==>> Trace ID: 123456");

            let buffer: Vec<&str> = buffer.skip(2).collect();
            assert_eq!(buffer, [
                "===> References: [",
                "===>   * Child of span ID: 123",
                "===>   * Follows from span ID: 456",
                "===> ]",
                "===> Baggage items: [",
                "===>   * TestKey: Test Value",
                "===> ]",
                ""
            ]);
        }
    }
}
