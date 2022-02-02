use crate::actor::matcher::{
    EventMessageAndConfigWithReply, EventMessageWithReply, MatcherActor, ReconfigureMessage,
};
use crate::monitoring::metrics::{TornadoMeter, EVENT_SOURCE_LABEL_KEY, EVENT_TYPE_LABEL_KEY};
use actix::Addr;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tornado_engine_api::config::api::ConfigApiHandler;
use tornado_engine_api::error::ApiError;
use tornado_engine_api::event::api::{EventApiHandler, SendEventRequest};
use tornado_engine_matcher::config::operation::NodeFilter;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::model::ProcessedEvent;

pub mod runtime_config;

#[derive(Clone)]
pub struct MatcherApiHandler {
    matcher: Addr<MatcherActor>,
    meter: Arc<TornadoMeter>,
}

#[async_trait(?Send)]
impl EventApiHandler for MatcherApiHandler {
    async fn send_event_to_current_config(
        &self,
        config_filter: HashMap<String, NodeFilter>,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        let timer = SystemTime::now();
        let labels = [
            EVENT_SOURCE_LABEL_KEY.string("http"),
            EVENT_TYPE_LABEL_KEY.string(event.event.event_type.to_owned()),
        ];

        let request = self
            .matcher
            .send(EventMessageWithReply {
                event: event.to_event_with_metadata(),
                config_filter,
                process_type: event.process_type,
                include_metadata: true,
            })
            .await?;
        self.meter.events_received_counter.add(1, &labels);
        self.meter.http_requests_counter.add(1, &[]);
        self.meter
            .http_requests_duration_seconds
            .record(timer.elapsed().map(|t| t.as_secs_f64()).unwrap_or_default(), &[]);

        Ok(request?)
    }

    async fn send_event_to_config(
        &self,
        event: SendEventRequest,
        matcher_config: MatcherConfig,
    ) -> Result<ProcessedEvent, ApiError> {
        let timer = SystemTime::now();
        let labels = [
            EVENT_SOURCE_LABEL_KEY.string("http"),
            EVENT_TYPE_LABEL_KEY.string(event.event.event_type.to_owned()),
        ];

        let request = self
            .matcher
            .send(EventMessageAndConfigWithReply {
                event: event.to_event_with_metadata(),
                process_type: event.process_type,
                matcher_config,
                include_metadata: true,
            })
            .await?;

        self.meter.events_received_counter.add(1, &labels);
        self.meter.http_requests_counter.add(1, &[]);
        self.meter
            .http_requests_duration_seconds
            .record(timer.elapsed().map(|t| t.as_secs_f64()).unwrap_or_default(), &[]);

        Ok(request?)
    }
}

#[async_trait(?Send)]
impl ConfigApiHandler for MatcherApiHandler {
    async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError> {
        let request = self.matcher.send(ReconfigureMessage {}).await?;
        Ok(request
            .map_err(|err| ApiError::InternalServerError { cause: format!("{:?}", err) })?
            .as_ref()
            .clone())
    }
}

impl MatcherApiHandler {
    pub fn new(matcher: Addr<MatcherActor>, meter: Arc<TornadoMeter>) -> MatcherApiHandler {
        MatcherApiHandler { matcher, meter }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::actor::dispatcher::{ActixEventBus, DispatcherActor};
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tornado_common_api::{Event, Value, WithEventData};
    use tornado_engine_api::event::api::ProcessType;
    use tornado_engine_matcher::config::fs::{FsMatcherConfigManager, ROOT_NODE_NAME};
    use tornado_engine_matcher::config::rule::{Constraint, Operator, Rule};
    use tornado_engine_matcher::config::MatcherConfigReader;
    use tornado_engine_matcher::dispatcher::Dispatcher;
    use tornado_engine_matcher::model::{
        ProcessedFilterStatus, ProcessedNode, ProcessedRuleStatus,
    };

    #[actix_rt::test]
    async fn should_send_an_event_to_the_current_config_and_return_the_processed_event() {
        // Arrange
        let path = "./config/rules.d";
        let config_manager = Arc::new(FsMatcherConfigManager::new(path, ""));

        let event_bus = Arc::new(ActixEventBus { callback: |_| {} });

        let dispatcher_addr =
            DispatcherActor::start_new(1, Dispatcher::build(event_bus.clone()).unwrap());

        let matcher_addr = MatcherActor::start(
            dispatcher_addr.clone().recipient(),
            config_manager,
            47,
            Default::default(),
        )
        .await
        .unwrap();

        let api = MatcherApiHandler { matcher: matcher_addr, meter: Default::default() };

        let send_event_request = SendEventRequest {
            process_type: ProcessType::SkipActions,
            event: Event::new("test-type"),
            metadata: Value::Object(Default::default()),
        };

        let config_filter = HashMap::from([(ROOT_NODE_NAME.to_owned(), NodeFilter::AllChildren)]);

        // Act
        let res = api.send_event_to_current_config(config_filter, send_event_request).await;

        // Assert
        assert!(res.is_ok());
        assert_eq!(Some("test-type"), res.unwrap().event.event_type());
    }

    #[actix_rt::test]
    async fn should_reconfigure_the_matcher_and_send_new_config() {
        // Arrange
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_path = temp_dir.path().as_os_str().to_str().unwrap().to_owned();
        let config_manager = Arc::new(FsMatcherConfigManager::new(&temp_path, &temp_path));

        let event_bus = Arc::new(ActixEventBus { callback: |_| {} });

        let dispatcher_addr =
            DispatcherActor::start_new(1, Dispatcher::build(event_bus.clone()).unwrap());

        let matcher_addr = MatcherActor::start(
            dispatcher_addr.clone().recipient(),
            config_manager.clone(),
            47,
            Default::default(),
        )
        .await
        .unwrap();

        let api = MatcherApiHandler { matcher: matcher_addr, meter: Default::default() };

        // Act
        let res = config_manager.get_config().await;
        // Verify
        assert!(res.is_ok());
        match res.unwrap() {
            MatcherConfig::Ruleset { rules, .. } => assert!(rules.is_empty()),
            MatcherConfig::Filter { .. } => assert!(false),
        }

        // Add one rule after the tornado start
        std::fs::copy(
            "./config/rules.d/ruleset_01/001_all_emails.json",
            format!("{}/001_all_emails.json", temp_path),
        )
        .unwrap();

        // Act
        let res = api.reload_configuration().await;

        // Assert
        assert!(res.is_ok());
        match res.unwrap() {
            MatcherConfig::Ruleset { rules, .. } => assert_eq!(1, rules.len()),
            MatcherConfig::Filter { .. } => assert!(false),
        }
    }

    #[actix_rt::test]
    async fn should_send_an_event_to_the_draft_and_return_the_processed_event() {
        // Arrange
        let path = "./config/rules.d";
        let config_manager = Arc::new(FsMatcherConfigManager::new(path, ""));

        let event_bus = Arc::new(ActixEventBus { callback: |_| {} });

        let dispatcher_addr =
            DispatcherActor::start_new(1, Dispatcher::build(event_bus.clone()).unwrap());

        let matcher_addr = MatcherActor::start(
            dispatcher_addr.clone().recipient(),
            config_manager,
            47,
            Default::default(),
        )
        .await
        .unwrap();

        let api = MatcherApiHandler { matcher: matcher_addr, meter: Default::default() };

        let send_event_request = SendEventRequest {
            process_type: ProcessType::SkipActions,
            event: Event::new("test-type-custom"),
            metadata: Value::Object(Default::default()),
        };

        let config = MatcherConfig::Ruleset {
            name: "custom_ruleset".to_owned(),
            rules: vec![Rule {
                name: "rule_1".to_owned(),
                actions: vec![],
                active: true,
                description: "".to_owned(),
                do_continue: true,
                constraint: Constraint {
                    where_operator: Some(Operator::Equals {
                        first: Value::String("${event.type}".to_owned()),
                        second: Value::String("test-type-custom".to_owned()),
                    }),
                    with: HashMap::new(),
                },
            }],
        };

        // Act
        let res = api.send_event_to_config(send_event_request, config).await.unwrap();

        // Assert
        assert_eq!(Some("test-type-custom"), res.event.event_type());

        match res.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("custom_ruleset", &name);
                assert_eq!(1, rules.rules.len());
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules[0].status)
            }
            _ => assert!(false),
        }
    }

    #[actix_rt::test]
    async fn send_an_event_should_include_metadata() {
        // Arrange
        let path = "./config/rules.d";
        let config_manager = Arc::new(FsMatcherConfigManager::new(path, ""));

        let event_bus = Arc::new(ActixEventBus { callback: |_| {} });

        let dispatcher_addr =
            DispatcherActor::start_new(1, Dispatcher::build(event_bus.clone()).unwrap());

        let matcher_addr = MatcherActor::start(
            dispatcher_addr.clone().recipient(),
            config_manager,
            47,
            Default::default(),
        )
        .await
        .unwrap();

        let api = MatcherApiHandler { matcher: matcher_addr, meter: Default::default() };

        let metadata = HashMap::from([("tenant_id".to_owned(), Value::String("beta".to_owned()))]);

        let send_event_request = SendEventRequest {
            process_type: ProcessType::SkipActions,
            event: Event::new("test-type"),
            metadata: json!(metadata),
        };

        let config_filter = HashMap::from([(ROOT_NODE_NAME.to_owned(), NodeFilter::AllChildren)]);

        // Act
        let res =
            api.send_event_to_current_config(config_filter, send_event_request).await.unwrap();

        // Assert
        match res.result {
            ProcessedNode::Filter { nodes, .. } => {
                let tenant_beta_node_matched = nodes.iter().any(|n| match n {
                    ProcessedNode::Filter { name, filter, .. } => {
                        name.eq("tenant_id_beta") && filter.status == ProcessedFilterStatus::Matched
                    }
                    _ => false,
                });
                assert!(tenant_beta_node_matched);

                let tenant_alpha_node_matched = nodes.iter().any(|n| match n {
                    ProcessedNode::Filter { name, filter, .. } => {
                        name.eq("tenant_id_alpha")
                            && filter.status == ProcessedFilterStatus::Matched
                    }
                    _ => false,
                });
                assert!(!tenant_alpha_node_matched);
            }
            _ => assert!(false),
        };
    }
}
