extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate tornado_common_api;

use tornado_common_api::Event;

/// A Collector is an event data source.
/// It collects information from one or more unstructured sources (e.g. emails, log files, etc.)
/// and produces structured Events to be sent to the Tornado engine.
pub trait Collector<T> {
    /// Consumes an input an produces an Event
    fn to_event(&self, input: T) -> Result<Event, CollectorError>;
}

#[derive(Fail, Debug)]
pub enum CollectorError {
    #[fail(display = "EventCreationError: [{}]", message)]
    EventCreationError { message: String },
}
