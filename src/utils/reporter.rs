use super::super::FinishedSpan;
use super::super::SpanReceiver;


/// TODO
pub struct ReporterThread {}

impl ReporterThread {
    /// TODO
    pub fn new<Reporter>(receiver: SpanReceiver, reporter: Reporter) -> ReporterThread
        where Reporter: Fn(FinishedSpan) -> ()
    {
        // TODO
        ReporterThread {}
    }

    /// TODO
    pub fn start(&self) {
        // TODO
    }
}
