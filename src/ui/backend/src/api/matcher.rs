use crate::api::ApiHandler;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigManager};
use tornado_engine_matcher::error::MatcherError;

pub struct MatcherApiHandler {
    pub config_manager: Box<MatcherConfigManager>,
}

impl ApiHandler for MatcherApiHandler {
    fn read(&self) -> Result<MatcherConfig, MatcherError> {
        self.config_manager.read()
    }
}

impl MatcherApiHandler {
    pub fn new(config_manager: Box<MatcherConfigManager>) -> MatcherApiHandler {
        MatcherApiHandler { config_manager }
    }
}
