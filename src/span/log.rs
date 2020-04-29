use std::collections::HashMap;
use std::collections::hash_map::Iter;

use std::time::SystemTime;


/// Structured logging information to attach to spans.
///
/// Each log has a set of vaules idenfied by strings.
/// Values stored in fields can be of any type convertible to a `LogValue`.
///
/// Logs also have an optional timestamp.
/// If set, the timestamp must be between the start and the end of the span.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use std::thread;
/// use std::time::Duration;
/// use std::time::SystemTime;
///
/// use opentracingrust::Log;
/// use opentracingrust::tracers::NoopTracer;
///
///
/// fn main() {
///     let (tracer, _) = NoopTracer::new();
///     let mut span = tracer.span("example");
///
///     let time = SystemTime::now();
///     thread::sleep(Duration::from_millis(50));
///
///     let log = Log::new()
///         .log("error", false)
///         .log("event", "some-event")
///         .log("line", 26)
///         .at(time);
///     span.log(log);
/// }
/// ```
#[derive(Debug, Default)]
pub struct Log {
    fields: LogFileds,
    timestamp: Option<SystemTime>,
}

impl Log {
    /// Creates an empty structured log.
    pub fn new() -> Log {
        Log {
            fields: LogFileds::new(),
            timestamp: None,
        }
    }
}

impl Log {
    /// Sets the timestamp associated with the log.
    pub fn at(mut self, timestamp: SystemTime) -> Log {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the timestamp to now if not set.
    pub fn at_or_now(&mut self) {
        if self.timestamp.is_none() {
            self.timestamp = Some(SystemTime::now())
        }
    }

    /// Extend the log fields with the given value.
    ///
    /// If a value with the same key is already in the log the value is replaced.
    pub fn log<LV: Into<LogValue>>(mut self, key: &str, value: LV) -> Log {
        self.fields.log(key.into(), value.into());
        self
    }

    /// Access an iterator over stored fields.
    pub fn iter(&self) -> Iter<String, LogValue> {
        self.fields.iter()
    }

    /// Access the (optional) timestamp for the log.
    pub fn timestamp(&self) -> Option<&SystemTime> {
        self.timestamp.as_ref()
    }
}


/// Structured log fields container.
#[derive(Debug, Default)]
struct LogFileds(HashMap<String, LogValue>);

impl LogFileds {
    /// Creates an empty
    pub fn new() -> LogFileds {
        LogFileds(HashMap::new())
    }

    /// Insert/update a field.
    pub fn log(&mut self, key: String, value: LogValue) {
        self.0.insert(key, value);
    }

    /// Access an iterator over fields.
    pub fn iter(&self) -> Iter<String, LogValue> {
        self.0.iter()
    }
}


/// Enumeration of valid types for log values.
#[derive(Debug, PartialEq)]
pub enum LogValue {
    Boolean(bool),
    Float(f64),
    Integer(i64),
    String(String),
}

impl From<bool> for LogValue {
    fn from(value: bool) -> LogValue {
        LogValue::Boolean(value)
    }
}

impl From<f64> for LogValue {
    fn from(value: f64) -> LogValue {
        LogValue::Float(value)
    }
}

impl From<i64> for LogValue {
    fn from(value: i64) -> LogValue {
        LogValue::Integer(value)
    }
}

impl<'a> From<&'a str> for LogValue {
    fn from(value: &'a str) -> LogValue {
        LogValue::String(String::from(value))
    }
}

impl From<String> for LogValue {
    fn from(value: String) -> LogValue {
        LogValue::String(value)
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::SystemTime;

    use super::Log;
    use super::LogValue;

    #[test]
    fn add_field() {
        let log = Log::new().log("key", "value");
        let entries: Vec<(&String, &LogValue)> = log.iter().collect();
        assert_eq!(entries, [
            (&String::from("key"), &LogValue::String(String::from("value")))
        ]);
    }

    #[test]
    fn defults_to_no_time() {
        match Log::new().timestamp() {
            None => (),
            _ => panic!("Time should not be set")
        }
    }

    #[test]
    fn set_default_timestamp() {
        let start = SystemTime::now();
        let mut log = Log::new();
        log.at_or_now();
        let time = log.timestamp().unwrap();
        let duration = time.duration_since(start).unwrap();
        if duration > Duration::from_millis(100) {
            panic!("Log timestamp too far from expected time");
        }
    }

    #[test]
    fn set_log_timestamp() {
        let time = SystemTime::now();
        let log = Log::new().at(time.clone());
        assert_eq!(&time, log.timestamp().unwrap());
    }
}
