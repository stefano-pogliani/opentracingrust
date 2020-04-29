use std::error::Error;

use super::super::Log;
use super::super::Span;


/// Trait to make failing spans on error easier and nicer.
///
/// The most common use is for [`Result`] instances in combination with the `?` operator.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use std::num::ParseIntError;
///
/// use opentracingrust::Span;
/// use opentracingrust::tracers::NoopTracer;
/// use opentracingrust::utils::FailSpan;
/// 
/// fn work(mut span: &mut Span) -> Result<i32, ParseIntError> {
///     let ten: i32 = "10".parse().fail_span(&mut span)?;
///     let two: i32 = "2".parse().fail_span(&mut span)?;
///     Ok(ten * two)
/// }
///
/// fn main() {
///     let (tracer, _) = NoopTracer::new();
///     let mut span = tracer.span("test");
///     let result = work(&mut span).unwrap();
///     println!("{}", result);
/// }
/// ```
///
/// [`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html
pub trait FailSpan {
    type Error: Error + ?Sized;

    /// Access the current error information, if any.
    ///
    /// Returns [`None`] if there was no error.
    /// 
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    fn error(&self) -> Option<&Self::Error>;

    /// Tags a span as failed if there was an error.
    ///
    /// An `error` event should also be logged with the details
    /// following the [OpenTracing specification].
    ///
    /// Nothing is done if there was no error (`error()` returns [`None`]).
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    /// [OpenTracing specification]: https://github.com/opentracing/specification/blob/master/semantic_conventions.md#log-fields-table
    fn fail_span<S>(self, span: S) -> Self where S: AsMut<Span>;
}

impl<T, E> FailSpan for Result<T, E> where
    E: Error
{
    type Error = E;

    fn error(&self) -> Option<&E> {
        self.as_ref().err()
    }

    fn fail_span<S>(self, mut span: S) -> Result<T, E> where S: AsMut<Span> {
        // Skip if there was no error.
        let is_none = self.error().is_none();
        if is_none {
            return self;
        }

        // Scope error variable so we can return self.
        {
            let error = self.error().unwrap();
            let span = span.as_mut();
            span.tag("error", true);
            span.log(Log::new()
                .log("event", "error")
                .log("message", format!("{}", error))
                .log("error.kind", error.to_string())
                .log("error.object", format!("{:?}", error))
            );
        }
        self
    }
}



#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt;

    use super::super::super::TagValue;
    use super::super::super::tracers::NoopTracer;
    use super::FailSpan;

    #[derive(Debug)]
    struct SomeError {}
    impl Error for SomeError {
        fn description(&self) -> &str {
            "SomeError"
        }
        fn cause(&self) -> Option<&dyn Error> {
            None
        }
    }
    impl fmt::Display for SomeError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "SomeError")
        }
    }

    fn fail() -> Result<(), SomeError> {
        Err(SomeError{})
    }

    #[test]
    fn fail_spans() {
        let (tracer, receiver) = NoopTracer::new();
        let mut span = tracer.span("test");
        let result = fail().fail_span(&mut span);
        match result {
            Ok(_) => panic!("Should have see an error"),
            Err(_) => (),
        };
        span.finish().unwrap();
        let span = receiver.recv().unwrap();
        match span.tags().get("error").unwrap() {
            &TagValue::Boolean(_) => (),
            _ => panic!("Error tag not set")
        }
        let logs = span.logs();
        assert_eq!(1, logs.len());
        let mut logs: Vec<(String, String)> = logs[0].iter()
            .map(|(ref k, ref v)| ((*k).clone(), format!("{:?}", v)))
            .collect();
        logs.sort_by_key(|&(ref k, _)| k.clone());
        assert_eq!(logs, [
            (String::from("error.kind"), String::from(r#"String("SomeError")"#)),
            (String::from("error.object"), String::from(r#"String("SomeError")"#)),
            (String::from("event"), String::from(r#"String("error")"#)),
            (String::from("message"), String::from(r#"String("SomeError")"#)),
        ]);
    }
}
