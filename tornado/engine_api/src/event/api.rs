use crate::error::ApiError;
use async_trait::async_trait;
use tornado_common_api::Event;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::model::ProcessedEvent;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait]
pub trait EventApi: Send + Sync {
    async fn get_config(&self) -> Result<MatcherConfig, ApiError>;
    async fn send_event(&self, event: SendEventRequest) -> Result<ProcessedEvent, ApiError>;
}

pub struct SendEventRequest {
    pub event: Event,
    pub process_type: ProcessType,
}

pub enum ProcessType {
    Full,
    SkipActions,
}
