use crate::error::ApiError;
use futures::Future;
use tornado_common_api::Event;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::model::ProcessedEvent;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
pub trait ApiHandler {
    fn get_config(&self) -> Box<dyn Future<Item = MatcherConfig, Error = ApiError>>;
    fn send_event(
        &self,
        event: SendEventRequest,
    ) -> Box<dyn Future<Item = ProcessedEvent, Error = ApiError>>;
}

pub struct SendEventRequest {
    pub event: Event,
    pub process_type: ProcessType,
}

pub enum ProcessType {
    Full,
    SkipActions,
}
