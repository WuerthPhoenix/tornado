use crate::api::ApiHandler;
use std::sync::Arc;
use tornado_engine_matcher::matcher::Matcher;

pub struct MatcherApiHandler {
    matcher: Arc<Matcher>,
}

impl ApiHandler for MatcherApiHandler {}
