extern crate rand;
extern crate opentracingrust;

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use rand::random;

use opentracingrust::ExtractFormat;
use opentracingrust::FinishedSpan;
use opentracingrust::ImplContext;
use opentracingrust::InjectFormat;
use opentracingrust::Result;
use opentracingrust::Span;
use opentracingrust::SpanContext;
use opentracingrust::SpanReceiver;
use opentracingrust::SpanReference;
use opentracingrust::SpanSender;
use opentracingrust::StartOptions;
use opentracingrust::Tracer;
use opentracingrust::TracerInterface;

use opentracingrust::utils::GlobalTracer;
use opentracingrust::utils::ReporterThread;


// Because this is an example the full ImplContext is implemented.
// Usually you can avoid most of the boilerplate code by:
//   * `#[derive(Clone)]`
//   * Implement `SpanReferenceAware` for your structure
//   * Wrap instances of your structure in a `ImplContextBox`
struct InnerContext {
    pub trace_id: u64,
    pub span_id: u64,
}

impl ImplContext for InnerContext {
    fn impl_context(&self) -> &Any {
        self
    }

    fn clone(&self) -> Box<ImplContext> {
        Box::new(InnerContext {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
        })
    }

    // The aim of this function is simply to update any trace identifiers.
    // Keeping trak of references is a task for the `SpanContext`, not for the inner context.
    fn reference_span(&mut self, reference: &SpanReference) {
        match *reference {
            SpanReference::ChildOf(ref parent) |
            SpanReference::FollowsFrom(ref parent) => {
                let context = parent.impl_context::<InnerContext>().unwrap();
                self.trace_id = context.trace_id;
            }
        }
    }
}


const BAGGAGE_KEY_PREFIX: &str = "Baggage-";
const SPAN_ID_KEY: &str = "SpanID";
const TRACE_ID_KEY: &str = "TraceID";


struct MemoryTracer {
    sender: SpanSender
}

impl TracerInterface for MemoryTracer {
    fn extract(&self, fmt: ExtractFormat) -> Result<Option<SpanContext>> {
        match fmt {
            ExtractFormat::HttpHeaders(carrier) |
            ExtractFormat::TextMap(carrier) => {
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
                let mut context = SpanContext::new(InnerContext {
                    trace_id,
                    span_id
                });

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

    fn inject(&self, context: &SpanContext, fmt: InjectFormat) -> Result<()> {
        let span_context = context;
        let context = span_context.impl_context::<InnerContext>();
        let context = context.expect("Unsupported span, was it created by MemoryTracer?");
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
        let context = SpanContext::new(InnerContext {
            trace_id,
            span_id
        });
        Span::new(name, context, options, self.sender.clone())
    }
}

impl MemoryTracer {
    pub fn new() -> (Tracer, SpanReceiver, MemoryTracerStore) {
        let (sender, receiver) = mpsc::channel();
        let tracer = MemoryTracer { sender };
        (Tracer::new(tracer), receiver, Mutex::new(HashMap::new()))
    }

    pub fn store(store: &MemoryTracerStore, span: FinishedSpan) {
        let trace_id = {
            let context = span.context().impl_context::<InnerContext>().unwrap();
            context.trace_id
        };
        let mut traces = store.lock().unwrap();
        if traces.contains_key(&trace_id) {
            traces.get_mut(&trace_id).unwrap().push(span);
        } else {
            traces.insert(trace_id, vec![span]);
        }
    }

    pub fn print_store(store: &MemoryTracerStore) {
        let traces = store.lock().unwrap();
        for (trace_id, spans) in traces.iter() {
            println!("TraceID: {}", trace_id);
            for span in spans {
                let context = span.context().impl_context::<InnerContext>();
                let context = context.expect(
                    "Unsupported span, was it created by MemoryTracer?"
                );
                println!("  SpanID: {}", context.span_id);
            }
        }
    }
}


type MemoryTracerStore = Mutex<HashMap<u64, Vec<FinishedSpan>>>;


fn main() {
    // Initialise the tracer.
    let (tracer, receiver, store) = MemoryTracer::new();
    let tracer = GlobalTracer::init(tracer);

    let store = Arc::new(store);
    let inner_store = Arc::clone(&store);
    let mut reporter = ReporterThread::new(receiver, move |span| {
        MemoryTracer::store(&inner_store, span);
    });
    reporter.stop_delay(Duration::from_secs(2));

    // Do some work.
    {
        let span = tracer.span("root");
        // ... snip ...
        span.finish().expect("Unable to finish span");
    }

    // Print the traces.
    drop(reporter);
    MemoryTracer::print_store(&store);
}
