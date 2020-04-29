use std::io;
use std::io::Write;
use std::time::UNIX_EPOCH;

use crossbeam_channel::unbounded;
use rand::random;

use super::super::ImplContextBox;
use super::super::Result;

use super::super::FinishedSpan;
use super::super::LogValue;
use super::super::Span;
use super::super::SpanContext;
use super::super::SpanReceiver;
use super::super::SpanReference;
use super::super::SpanReferenceAware;
use super::super::SpanSender;
use super::super::StartOptions;
use super::super::TagValue;

use super::super::ExtractFormat;
use super::super::InjectFormat;
use super::super::Tracer;
use super::super::TracerInterface;


const BAGGAGE_KEY_PREFIX: &str = "Baggage-";
const SPAN_ID_KEY: &str = "SpanID";
const TRACE_ID_KEY: &str = "TraceID";


/// A tracer that writes spans to an `std::io::Write`.
///
/// Useful for local testing, experiments, tests.
/// **NOT suited for production use!**
///
/// Intended to write spans to stderr but can also be used to write spans to stdout or files.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use std::io;
/// use std::time::Duration;
///
/// use opentracingrust::FinishedSpan;
/// use opentracingrust::tracers::FileTracer;
/// use opentracingrust::utils::GlobalTracer;
/// use opentracingrust::utils::ReporterThread;
///
///
/// fn main() {
///     let (tracer, receiver) = FileTracer::new();
///     GlobalTracer::init(tracer);
///
///     let reporter = ReporterThread::new_with_duration(
///         receiver, Duration::from_millis(50), |span| {
///             let mut stderr = io::stderr();
///             FileTracer::write_trace(span, &mut stderr).unwrap();
///         }
///     );
///
///     // ... snip ...
/// }
/// ```
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
                let mut context = SpanContext::new(ImplContextBox::new(
                    FileTracerContext {
                        trace_id,
                        span_id
                    }
                ));

                // Decode baggage items.
                for (key, value) in carrier.items() {
                    if key.starts_with(BAGGAGE_KEY_PREFIX) {
                        context.set_baggage_item(key.clone(), value.clone());
                    }
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
                for (key, value) in span_context.baggage_items() {
                    let key = format!("{}{}", BAGGAGE_KEY_PREFIX, key);
                    carrier.set(&key, value);
                }
                Ok(())
            },
            _ => panic!("Unsupported injection format")
        }
    }

    fn span(&self, name: &str, options: StartOptions) -> Span {
        let trace_id = random::<u64>();
        let span_id = random::<u64>();
        let context = SpanContext::new(ImplContextBox::new(FileTracerContext {
            trace_id,
            span_id
        }));
        Span::new(name, context, options, self.sender.clone())
    }
}

impl FileTracer {
    /// Instantiate a new file tracer.
    pub fn new() -> (Tracer, SpanReceiver) {
        let (sender, receiver) = unbounded();
        let tracer = FileTracer { sender };
        (Tracer::new(tracer), receiver)
    }

    /// Function to write a `FinishedSpan` to a stream.
    ///
    /// Used to send `FinishedSpan`s to an `std::io::Write` stream.
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
        let start = span.start_time();
        let duration = finish.duration_since(*start).unwrap();
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
                SpanReference::ChildOf(_) => "Child of span ID",
                SpanReference::FollowsFrom(_) => "Follows from span ID"
            };
            buffer.push_str(&format!("===>   * {}: {}\n", ref_type, ref_id));
        }
        buffer.push_str("===> ]\n");

        buffer.push_str("===> Baggage items: [\n");
        for (key, value) in span.context().baggage_items() {
            buffer.push_str(&format!("===>   * {}: {}\n", key, value));
        }
        buffer.push_str("===> ]\n");

        let mut tags: Vec<(&String, &TagValue)> = span.tags().iter().collect();
        tags.sort_by_key(|&(k, _)| k);
        buffer.push_str("===> Tags: [\n");
        for (tag, value) in tags {
            let value = match value {
                TagValue::Boolean(v) => v.to_string(),
                TagValue::Float(v) => v.to_string(),
                TagValue::Integer(v) => v.to_string(),
                TagValue::String(ref v) => v.clone(),
            };
            buffer.push_str(&format!("===>   * {}: {}\n", tag, value));
        }
        buffer.push_str("===> ]\n");

        buffer.push_str("===> Logs: [\n");
        for log in span.logs().iter() {
            let timestamp = log.timestamp().unwrap()
                .duration_since(UNIX_EPOCH).unwrap()
                .as_secs();
            buffer.push_str(&format!("===>   - {}:\n", timestamp));

            let mut fields: Vec<(&String, &LogValue)> = log.iter().collect();
            fields.sort_by_key(|&(k, _)| k);
            for (key, value) in fields {
                let value = match value {
                    LogValue::Boolean(v) => v.to_string(),
                    LogValue::Float(v) => v.to_string(),
                    LogValue::Integer(v) => v.to_string(),
                    LogValue::String(ref v) => v.clone(),
                };
                buffer.push_str(&format!("===>     * {}: {}\n", key, value));
            }
        }
        buffer.push_str("===> ]\n");
        file.write_all(buffer.as_bytes())
    }
}


/// Inner `SpanContext` for `FileTracer`.
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
    use super::super::super::ImplContextBox;
    use super::super::super::SpanContext;
    use super::super::super::SpanReceiver;
    use super::super::super::Tracer;

    use super::FileTracer;
    use super::FileTracerContext;

    fn make_context(trace_id: u64, span_id: u64) -> SpanContext {
        SpanContext::new(ImplContextBox::new(FileTracerContext {
            trace_id,
            span_id
        }))
    }

    fn make_tracer() -> (Tracer, SpanReceiver) {
        FileTracer::new()
    }


    mod span {
        use std::time::UNIX_EPOCH;
        use std::time::Duration;

        use super::super::super::super::Log;

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
                    context.get_baggage_item("Baggage-Item1").unwrap()
                );
                assert_eq!(
                    "cd",
                    context.get_baggage_item("Baggage-Item2").unwrap()
                );
            }
        }


        mod inject {
            use std::collections::HashMap;
            use std::io;

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
                context.set_baggage_item(String::from("Item1"), String::from("ab"));
                context.set_baggage_item(String::from("Item2"), String::from("cd"));
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
                context.set_baggage_item(String::from("Item1"), String::from("ab"));
                context.set_baggage_item(String::from("Item2"), String::from("cd"));
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
            let span = tracer.span("test1");
            let context = span.context().impl_context::<FileTracerContext>();
            context.unwrap();
        }

        #[test]
        fn write() {
            let (tracer, receiver) = make_tracer();
            let mut span = tracer.span("test1");
            span.child_of(make_context(123456, 123));
            span.follows(make_context(123456, 456));
            span.set_baggage_item("TestKey", "Test Value");
            span.tag("test.bool", true);
            span.tag("test.float", 0.5);
            span.tag("test.int", 5);
            span.tag("test.string", "hello");

            span.log(Log::new()
                .log("bool", false)
                .log("float", 0.66)
                .at(UNIX_EPOCH + Duration::from_secs(123456))
            );
            span.log(Log::new()
                .log("int", 66)
                .log("string", "message")
                .at(UNIX_EPOCH + Duration::from_secs(654321))
            );
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
                "===> Tags: [",
                "===>   * test.bool: true",
                "===>   * test.float: 0.5",
                "===>   * test.int: 5",
                "===>   * test.string: hello",
                "===> ]",
                "===> Logs: [",
                "===>   - 123456:",
                "===>     * bool: false",
                "===>     * float: 0.66",
                "===>   - 654321:",
                "===>     * int: 66",
                "===>     * string: message",
                "===> ]",
                ""
            ]);
        }
    }
}
