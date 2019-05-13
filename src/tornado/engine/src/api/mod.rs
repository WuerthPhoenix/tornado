use backend::api::handler::ApiHandler;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigManager};
use tornado_engine_matcher::error::MatcherError;
use actix::Addr;
use crate::engine::MatcherActor;
use tornado_engine_matcher::model::{ProcessedEvent, ProcessedNode, ProcessedRules};
use tornado_common_api::Event;
use std::collections::HashMap;

pub struct MatcherApiHandler {
    matcher: Addr<MatcherActor>,
    config_manager: Box<MatcherConfigManager>,
}

impl ApiHandler for MatcherApiHandler {
    fn read(&self) -> Result<MatcherConfig, MatcherError> {
        self.config_manager.read()
    }

    fn send_event(&self, event: Event) -> Result<ProcessedEvent, MatcherError> {
        Ok(ProcessedEvent{
            event: event.into(),
            result: ProcessedNode::Rules {rules: ProcessedRules{
                rules: HashMap::new(),
                extracted_vars: HashMap::new()
            }}
        })
    }
}

impl MatcherApiHandler {
    pub fn new(config_manager: Box<MatcherConfigManager>, matcher: Addr<MatcherActor>) -> MatcherApiHandler {
        MatcherApiHandler { config_manager, matcher }
    }
}
