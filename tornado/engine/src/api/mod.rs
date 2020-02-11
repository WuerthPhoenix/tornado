use crate::engine::{EventMessageWithReply, MatcherActor};
use actix::Addr;
use async_trait::async_trait;
use std::sync::Arc;
use tornado_engine_api::api::handler::{ApiHandler, SendEventRequest};
use tornado_engine_api::error::ApiError;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigManager};
use tornado_engine_matcher::model::ProcessedEvent;

#[derive(Clone)]
pub struct MatcherApiHandler {
    matcher: Addr<MatcherActor>,
    config_manager: Arc<dyn MatcherConfigManager>,
}

#[async_trait]
impl ApiHandler for MatcherApiHandler {
    async fn get_config(&self) -> Result<MatcherConfig, ApiError> {
        self.config_manager.read().map_err(ApiError::from)
    }

    async fn send_event(&self, event: SendEventRequest) -> Result<ProcessedEvent, ApiError> {
        let request = self
            .matcher
            .send(EventMessageWithReply { event: event.event, process_type: event.process_type })
            .await?;

        Ok(request?)
    }
}

impl MatcherApiHandler {
    pub fn new(
        config_manager: Arc<dyn MatcherConfigManager>,
        matcher: Addr<MatcherActor>,
    ) -> MatcherApiHandler {
        MatcherApiHandler { config_manager, matcher }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::dispatcher::{ActixEventBus, DispatcherActor};
    use actix::{Arbiter, SyncArbiter, System};
    use std::sync::Arc;
    use tornado_common_api::Event;
    use tornado_engine_api::api::handler::ProcessType;
    use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
    use tornado_engine_matcher::dispatcher::Dispatcher;
    use tornado_engine_matcher::matcher::Matcher;

    #[test]
    fn should_send_an_event_to_the_matcher_and_return_the_processed_event() {
        // Arrange
        let path = "./config/rules.d";
        let config = FsMatcherConfigManager::new(path);
        let matcher = Arc::new(config.read().and_then(|config| Matcher::build(&config)).unwrap());

        System::run(move || {
            let event_bus = Arc::new(ActixEventBus { callback: |_| {} });

            let dispatcher_addr = SyncArbiter::start(1, move || {
                let dispatcher = Dispatcher::build(event_bus.clone()).unwrap();
                DispatcherActor { dispatcher }
            });

            let matcher_addr = SyncArbiter::start(1, move || MatcherActor {
                matcher: matcher.clone(),
                dispatcher_addr: dispatcher_addr.clone(),
            });

            let api = MatcherApiHandler { matcher: matcher_addr, config_manager: Box::new(config) };

            let send_event_request = SendEventRequest {
                process_type: ProcessType::SkipActions,
                event: Event::new("test-type"),
            };

            // Act
            Arbiter::spawn({
                api.send_event(send_event_request).then(|res| {
                    // Verify
                    assert!(res.is_ok());
                    assert_eq!(Some("test-type"), res.unwrap().event.event_type.get_text());
                    System::current().stop();
                    Ok(())
                })
            });
        })
        .unwrap();
    }
}
