use dto::event::{ProcessedEventDto, EventDto};
use serde_json::Error;
use tornado_engine_matcher::model::{ProcessedEvent, InternalEvent};

pub fn processed_event_to_dto(processed_event: ProcessedEvent) -> Result<ProcessedEventDto, Error> {
    Ok(ProcessedEventDto{
        event:
    })
}

pub fn internal_event_to_dto(internal_event: InternalEvent) -> Result<EventDto, Error> {
    Ok(EventDto{
        event_type: internal_event.event_type.
    })
}