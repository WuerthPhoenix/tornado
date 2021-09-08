use std::sync::Arc;

use log::*;
use serde::Serialize;

use tornado_common_api::Action;
use tornado_common_api::Payload;
use tornado_executor_common::{ExecutorError, StatelessExecutor};

use crate::client::ApiClient;
use crate::config::Icinga2ClientConfig;
use crate::message::Message;
use tokio::sync::Mutex;

pub mod client;
pub mod config;
mod connection;
mod message;

pub const ICINGA2_ACTION_NAME_KEY: &str = "icinga2_action_name";
pub const ICINGA2_ACTION_PAYLOAD_KEY: &str = "icinga2_action_payload";

pub const ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE: &str = "IcingaObjectNotExisting";

/// An executor that logs received actions at the 'info' level
pub struct Icinga2Executor {
    pub api_client: Mutex<ApiClient>,
}

impl std::fmt::Display for Icinga2Executor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("Icinga2Executor")?;
        Ok(())
    }
}

impl Icinga2Executor {
    pub async fn new(config: Icinga2ClientConfig) -> Result<Icinga2Executor, ExecutorError> {
        Ok(Icinga2Executor { api_client: Mutex::new(ApiClient::new(&config).await?) })
    }

    async fn perform_request(&self, msg: Message) -> Result<(), ExecutorError>{
        self.api_client.lock().await.send(msg).await
    }

    fn parse_action(&self, action: &Arc<Action>) -> Result<Message, ExecutorError>{
        let json = match serde_json::to_string(&action.payload) {
            Ok(json) => json,
            Err(err) => return Err(ExecutorError::ActionExecutionError {
                message: format!("Error while trying to serialize Payload. {}", err),
                can_retry: false,
                code: None,
                data: Default::default()
            })
        };

        match serde_json::from_str(&json) {
            Ok(params) => Ok(Message::CheckResult(params)),
            Err(err) => Err(ExecutorError::JsonError {
                cause: err.to_string()
            }),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for Icinga2Executor {
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("Icinga2Executor - received action: \n[{:?}]", action);
        let action = self.parse_action(&action)?;

        self.perform_request(action).await
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Icinga2Action<'a> {
    pub name: &'a str,
    pub payload: Option<&'a Payload>,
}
