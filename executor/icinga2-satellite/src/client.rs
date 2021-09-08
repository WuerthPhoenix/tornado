use crate::connection::Connection;
use crate::config::Icinga2ClientConfig;
use tornado_executor_common::ExecutorError;
use crate::message::Message;

pub struct ApiClient {
    pub server_api_url: String,
    pub connection: Connection,
}

impl ApiClient {
    pub async fn new(config: &Icinga2ClientConfig) -> Result<ApiClient, ExecutorError> {
        Ok(ApiClient {
            server_api_url: config.server_api_url.clone(),
            connection: config.connect().await?
        })
    }

    pub async fn send(&mut self, msg: Message) -> Result<(), ExecutorError> {
        self.connection.send(msg.into()).await
            .map_err(|err| ExecutorError::ActionExecutionError {
                message: err.to_string(),
                can_retry: true,
                code: None,
                data: Default::default()
            })
    }
}
