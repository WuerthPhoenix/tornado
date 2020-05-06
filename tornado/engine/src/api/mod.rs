use crate::engine::{
    EventMessageWithReply, GetCurrentConfigMessage, MatcherActor, ReconfigureMessage,
};
use actix::Addr;
use async_trait::async_trait;
use tornado_engine_api::config::api::ConfigApiHandler;
use tornado_engine_api::error::ApiError;
use tornado_engine_api::event::api::{EventApi, SendEventRequest};
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::model::ProcessedEvent;

#[derive(Clone)]
pub struct MatcherApiHandler {
    matcher: Addr<MatcherActor>,
}

#[async_trait]
impl EventApi for MatcherApiHandler {
    async fn get_config(&self) -> Result<MatcherConfig, ApiError> {
        let request = self.matcher.send(GetCurrentConfigMessage {}).await?;
        Ok(request.as_ref().clone())
    }

    async fn send_event(&self, event: SendEventRequest) -> Result<ProcessedEvent, ApiError> {
        let request = self
            .matcher
            .send(EventMessageWithReply { event: event.event, process_type: event.process_type })
            .await?;

        Ok(request?)
    }
}

#[async_trait]
impl ConfigApiHandler for MatcherApiHandler {
    async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError> {
        let request = self.matcher.send(ReconfigureMessage {}).await?;
        Ok(request?.as_ref().clone())
    }
}

impl MatcherApiHandler {
    pub fn new(matcher: Addr<MatcherActor>) -> MatcherApiHandler {
        MatcherApiHandler { matcher }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::dispatcher::{ActixEventBus, DispatcherActor};
    use actix::{Arbiter, SyncArbiter, System};
    use std::sync::Arc;
    use tornado_common_api::Event;
    use tornado_engine_api::event::api::ProcessType;
    use tornado_engine_matcher::config::fs::FsMatcherConfigManager;
    use tornado_engine_matcher::config::MatcherConfigReader;
    use tornado_engine_matcher::dispatcher::Dispatcher;

    #[test]
    fn should_send_an_event_to_the_matcher_and_return_the_processed_event() {
        // Arrange
        let path = "./config/rules.d";
        let config_manager = Arc::new(FsMatcherConfigManager::new(path, ""));

        System::run(move || {
            let event_bus = Arc::new(ActixEventBus { callback: |_| {} });

            let dispatcher_addr = SyncArbiter::start(1, move || {
                let dispatcher = Dispatcher::build(event_bus.clone()).unwrap();
                DispatcherActor { dispatcher }
            });

            let matcher_addr =
                MatcherActor::start(dispatcher_addr.clone(), config_manager).unwrap();

            let api = MatcherApiHandler { matcher: matcher_addr };

            let send_event_request = SendEventRequest {
                process_type: ProcessType::SkipActions,
                event: Event::new("test-type"),
            };

            // Act
            Arbiter::spawn(async move {
                let res = api.send_event(send_event_request).await;
                // Verify
                assert!(res.is_ok());
                assert_eq!(Some("test-type"), res.unwrap().event.event_type.get_text());
                System::current().stop();
            });
        })
        .unwrap();
    }

    #[test]
    fn should_reconfigure_the_matcher_and_send_new_config() {
        // Arrange
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_path = temp_dir.path().as_os_str().to_str().unwrap().to_owned();
        let config_manager = Arc::new(FsMatcherConfigManager::new(&temp_path, &temp_path));

        System::run(move || {
            let event_bus = Arc::new(ActixEventBus { callback: |_| {} });

            let dispatcher_addr = SyncArbiter::start(1, move || {
                let dispatcher = Dispatcher::build(event_bus.clone()).unwrap();
                DispatcherActor { dispatcher }
            });

            let matcher_addr =
                MatcherActor::start(dispatcher_addr.clone(), config_manager.clone()).unwrap();

            let api = MatcherApiHandler { matcher: matcher_addr };

            // Act
            let res = config_manager.get_config();
            // Verify
            assert!(res.is_ok());
            match res.unwrap() {
                MatcherConfig::Ruleset { rules, .. } => assert!(rules.is_empty()),
                MatcherConfig::Filter { .. } => assert!(false),
            }

            // Add one rule after the tornado start
            std::fs::copy(
                "./config/rules.d/001_all_emails.json",
                format!("{}/001_all_emails.json", temp_path),
            )
            .unwrap();

            Arbiter::spawn(async move {
                // Act
                let res = api.reload_configuration().await;
                // Verify
                assert!(res.is_ok());
                match res.unwrap() {
                    MatcherConfig::Ruleset { rules, .. } => assert_eq!(1, rules.len()),
                    MatcherConfig::Filter { .. } => assert!(false),
                }

                System::current().stop();
            });
        })
        .unwrap();
    }
}
