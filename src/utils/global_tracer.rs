use std::sync::Mutex;
use std::sync::MutexGuard;

use super::super::Tracer;


static mut GLOBAL_TRACER: Option<Mutex<Tracer>> = None;


/// Utility singleton to store the process's `Tracer`.
///
/// Every thread in the process, may it be application or library, should use
/// the same `Tracer` instance for the entire lifetime of the process.
///
/// > *Applications should initialise the `GlobalTracer::init` as soon as possible!*
/// >
/// > *The `GlobalTracer::init` method is NOT thread safe and MUST be called
/// > before any thread is spawned or threads will panic!*
///
/// The `GlobalTracer` stores a mutually exclusive `Tracer`.
/// This can then be requested by each thread with `GlobalTracer::get`.
///
/// Once initialised, the `GlobalTracer` cannot be changed or dropped.
/// Be aware that the `GlobalTracer` is backed by a static global variable
/// so tracers implementing the `Drop` traits WILL NOT be dropped.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use opentracingrust::tracers::NoopTracer;
/// use opentracingrust::utils::GlobalTracer;
///
///
/// fn main() {
///    let (tracer, _) = NoopTracer::new();
///    GlobalTracer::init(tracer);
///    let span = GlobalTracer::get().span("root");
/// }
/// ```
pub struct GlobalTracer {}
impl GlobalTracer {
    /// Initialises the `GlobalTracer` to store the given `Tracer` instance.
    ///
    /// > *Applications should initialise the `GlobalTracer::init` as soon as possible!*
    /// >
    /// > *The `GlobalTracer::init` method is NOT thread safe and MUST be called
    /// > before any thread is spawned or threads will panic!*
    ///
    /// # Panics
    ///
    /// Panics if the `GlobalTracer` is already initialised with a `Tracer`.
    pub fn init(tracer: Tracer) {
        unsafe {
            match GLOBAL_TRACER {
                None => GLOBAL_TRACER = Some(Mutex::new(tracer)),
                _ => panic!("GlobalTracer already initialised")
            }
        }
    }

    /// Exclusively access the singleton `Tracer` instance.
    ///
    /// # Panics
    ///
    /// Panics if the singleton `Tracer` is requested before the `GlobalTracer` is initialised.
    pub fn get() -> MutexGuard<'static, Tracer> {
        unsafe {
            let tracer = GLOBAL_TRACER.as_ref()
                .expect("GlobalTracer not initialised, call GlobalTracer::init first");
            tracer.lock().expect("Failed to lock GlobalTracer")
        }
    }

    /// Allow tests to clean up before they run.
    #[cfg(test)]
    fn reset() {
        unsafe {
            GLOBAL_TRACER = None
        }
    }
}


#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::super::super::ExtractFormat;
    use super::super::super::InjectFormat;
    use super::super::super::Result;
    use super::super::super::Span;
    use super::super::super::SpanContext;
    use super::super::super::StartOptions;
    use super::super::super::Tracer;
    use super::super::super::TracerInterface;

    use super::GlobalTracer;


    struct DummyTracer {}
    impl TracerInterface for DummyTracer {
        fn extract(&self, _: ExtractFormat) -> Result<Option<SpanContext>> {
            panic!("Not Implemented");
        }

        fn inject(&self, _: &SpanContext, _: InjectFormat) -> Result<()> {
            panic!("Not Implemented");
        }

        fn span(&self, _: &str, _: StartOptions) -> Span {
            panic!("Not Implemented");
        }
    }


    // *** SEQUENTIAL TESTS ***
    // The following tests cannot run in parallel as they (unsafely)
    // manipulate the GLOBAL_TRACER singleton.
    // To avoid forcing all tests to be run serially these tests
    // sleep for increasing 5 ms increments.

    #[test]
    #[should_panic(expected = "GlobalTracer already initialised")]
    fn tracer_cannot_be_set_twice() {
        GlobalTracer::reset();
        GlobalTracer::init(Tracer::new(DummyTracer {}));
        GlobalTracer::init(Tracer::new(DummyTracer {}));
    }

    #[test]
    #[should_panic(expected = "GlobalTracer not initialised, call GlobalTracer::init first")]
    fn tracer_must_be_set() {
        thread::sleep(Duration::from_millis(5));
        GlobalTracer::reset();
        let _tracer = GlobalTracer::get();
    }

    #[test]
    fn tracer_is_returned() {
        thread::sleep(Duration::from_millis(10));
        GlobalTracer::reset();
        GlobalTracer::init(Tracer::new(DummyTracer {}));
        let _tracer = GlobalTracer::get();
    }

    #[test]
    fn tracer_is_returned_to_many_threads() {
        thread::sleep(Duration::from_millis(15));
        GlobalTracer::reset();
        GlobalTracer::init(Tracer::new(DummyTracer {}));
        let t1 = thread::spawn(|| {
            let _tracer = GlobalTracer::get();
            thread::sleep(Duration::from_millis(5));
        });
        let t2 = thread::spawn(|| {
            let _tracer = GlobalTracer::get();
            thread::sleep(Duration::from_millis(5));
        });
        t1.join().unwrap();
        t2.join().unwrap();
    }
}
