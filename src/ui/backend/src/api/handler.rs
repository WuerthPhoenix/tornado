use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::error::MatcherError;

pub trait ApiHandler {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
}
