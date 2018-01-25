use super::super::FinishedSpan;
use super::super::SpanReceiver;


/// TODO
pub struct ReporterThread {}

impl ReporterThread {
    /// TODO
    pub fn new<Reporter>(_receiver: SpanReceiver, _reporter: Reporter) -> ReporterThread
        where Reporter: FnMut(FinishedSpan) -> ()
    {
        // TODO
        ReporterThread {}
    }

    /// TODO
    pub fn start(&self) {
        // TODO
    }
}
