use crate::engine::{EventMessageWithReply, MatcherActor, ProcessType};
use actix::Addr;
use backend::api::handler::ApiHandler;
use backend::error::ApiError;
use futures::future::Future;
use tornado_common_api::Event;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigManager};
use tornado_engine_matcher::error::MatcherError;
use tornado_engine_matcher::model::ProcessedEvent;

pub struct MatcherApiHandler {
    matcher: Addr<MatcherActor>,
    config_manager: Box<MatcherConfigManager>,
}

impl ApiHandler for MatcherApiHandler {
    fn read(&self) -> Result<MatcherConfig, MatcherError> {
        self.config_manager.read()
    }

    fn send_event(&self, event: Event) -> Box<Future<Item = ProcessedEvent, Error = ApiError>> {
        let request = self
            .matcher
            .send(EventMessageWithReply { event, process_type: ProcessType::SkipActions });

        let response = request.map_err(ApiError::from).and_then(|res| Ok(res?));

        Box::new(response)
    }
}

impl MatcherApiHandler {
    pub fn new(
        config_manager: Box<MatcherConfigManager>,
        matcher: Addr<MatcherActor>,
    ) -> MatcherApiHandler {
        MatcherApiHandler { config_manager, matcher }
    }
}
