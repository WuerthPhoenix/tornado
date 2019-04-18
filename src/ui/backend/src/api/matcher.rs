use crate::api::ApiHandler;
use tornado_engine_matcher::config::{MatcherConfigManager, MatcherConfig};
use tornado_engine_matcher::error::MatcherError;

pub struct MatcherApiHandler<T: MatcherConfigManager> {
    config_manager: T,
}

impl <T: MatcherConfigManager> ApiHandler for MatcherApiHandler<T> {
    fn read(&self) -> Result<MatcherConfig, MatcherError> {
        self.config_manager.read()
    }
}

impl <T: MatcherConfigManager> MatcherApiHandler<T> {

    pub fn new(config_manager: T) -> MatcherApiHandler<T> {
        MatcherApiHandler{
            config_manager
        }
    }

}