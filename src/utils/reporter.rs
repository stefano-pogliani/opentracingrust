use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::RecvTimeoutError;

use std::thread;
use std::thread::Builder;
use std::thread::JoinHandle;
use std::time::Duration;

use super::super::FinishedSpan;
use super::super::SpanReceiver;


const STOP_DEALY_SEC_DEFAULT: u64 = 2;
const RECV_TIMEOUT_MSEC_DEFAULT: u64 = 50;


/// A basic span reporter backed by a background thread.
///
/// The reporter spawns a thread that loops until stopped and waits for `FinishedSpan`s.
/// Every time a finished span is received the `ReporterFn` closure is called with it.
/// The `ReporterFn` closure is responsible for shipping the received spans.
///
/// The `ReporterThread` also supports clean shutdown of the receiver thread.
/// When `ReporterThread::stop` is called or an instance is dropped:
///
///   1. The calling thread is paused for the `stop_delay` duration.
///      This allows the reporter thread to process any `FinishedSpan`s still in the channel.
///   2. The background thread is informend to shutdown and the calling thread joins it.
///   3. As soon as any `FinishedSpan` is processed or receiving times out the thread is stopped.
///      Receiving spans times out every 50 milliseconds.
// If https://github.com/rust-lang/rust/issues/27800 leads to a stable API
// rework this to be more efficient with shutdowns.
pub struct ReporterThread {
    stop_delay: Duration,
    stopping: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl ReporterThread {
    /// Creates a new reporter waiting for spans on the `receiver`.
    ///
    /// The reporter starts with a spawned thread and runs until stopped or dropped.
    pub fn new<ReporterFn>(receiver: SpanReceiver, mut reporter: ReporterFn) -> ReporterThread
        where ReporterFn: FnMut(FinishedSpan) -> () + Send + 'static
    {
        // Stopping flag.
        let stopping = Arc::new(AtomicBool::new(false));
        let inner_stopping = Arc::clone(&stopping);

        // Reporter thread loop.
        let thread = Builder::new().name("OpenTracingReporter".into()).spawn(move || {
            while !inner_stopping.load(Ordering::Relaxed) {
                let timeout = Duration::from_millis(RECV_TIMEOUT_MSEC_DEFAULT);
                let span = receiver.recv_timeout(timeout);
                match span {
                    Ok(span) => { reporter(span); },
                    Err(RecvTimeoutError::Timeout) => continue,
                    _ => panic!("Failed to receive span")
                }
            }
        }).expect("Failed to spawn reporter thread");

        // Return a wrapper around the thread.
        ReporterThread {
            stop_delay: Duration::from_secs(STOP_DEALY_SEC_DEFAULT),
            stopping: stopping,
            thread_handle: Some(thread),
        }
    }

    /// Version of `new` that also sets the `stop_delay`.
    pub fn new_with_duration<ReporterFn>(
        receiver: SpanReceiver, stop_delay: Duration, reporter: ReporterFn
    ) -> ReporterThread
        where ReporterFn: FnMut(FinishedSpan) -> () + Send + 'static
    {
        let mut reporter = ReporterThread::new(receiver, reporter);
        reporter.stop_delay(stop_delay);
        reporter
    }

    /// Updates the `stop_delay` for when the thread is stopped.
    pub fn stop_delay(&mut self, stop_delay: Duration) {
        self.stop_delay = stop_delay;
    }

    /// Stops the background thread and joins it.
    pub fn stop(&mut self) {
        if let Some(thread) = self.thread_handle.take() {
            thread::sleep(self.stop_delay);
            self.stopping.store(true, Ordering::Relaxed);
            thread.join().expect("Failed to join reporter thread");
        }
    }
}

impl Drop for ReporterThread {
    fn drop(&mut self) {
        self.stop()
    }
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;

    use super::super::super::FinishedSpan;
    use super::super::super::tracers::NoopTracer;

    use super::ReporterThread;

    #[test]
    fn receive_span() {
        // Tracer and shared span store.
        let (tracer, receiver) = NoopTracer::new();
        let spans: Arc<Mutex<Vec<FinishedSpan>>> = Arc::new(Mutex::new(Vec::new()));

        // Create the reporter closure.
        let inner_spans = Arc::clone(&spans);
        let mut reporter = ReporterThread::new_with_duration(
            receiver, Duration::from_millis(50), move |span| {
                inner_spans.lock().unwrap().push(span);
            }
        );

        // Finish a span and stop the reporter (join the thread).
        tracer.span("test").finish().unwrap();
        reporter.stop();

        // Check the span was received.
        assert_eq!(1, spans.lock().unwrap().len());
    }
}
