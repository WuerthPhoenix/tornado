use actix::prelude::*;
use log::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;
use tornado_common_api::Action;
use tornado_executor_common::Executor;

pub mod director;
pub mod icinga2;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ActionMessage {
    pub action: Action,
}

pub struct ExecutorActor<E: Executor + Display + Unpin> {
    pub executor: E,
}

impl<E: Executor + Display + Unpin + 'static> Actor for ExecutorActor<E> {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("ExecutorActor started.");
    }
}

impl<E: Executor + Display + Unpin + 'static> Handler<ActionMessage> for ExecutorActor<E> {
    type Result = ();

    fn handle(&mut self, msg: ActionMessage, _: &mut SyncContext<Self>) {
        trace!("ExecutorActor - received new action [{:?}]", &msg.action);
        match self.executor.execute(msg.action) {
            Ok(_) => debug!("ExecutorActor - {} - Action executed successfully", &self.executor),
            Err(e) => {
                error!("ExecutorActor - {} - Failed to execute action: {}", &self.executor, e)
            }
        };
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LazyExecutorActorInitMessage<E: Executor + Display, F: Fn() -> E>
where
    F: Send + Sync,
{
    pub init: F,
}

pub struct LazyExecutorActor<E: Executor + Display + Unpin> {
    pub executor: Option<E>,
}

impl<E: Executor + Display + Unpin + 'static> Actor for LazyExecutorActor<E> {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("ExecutorActor started.");
    }
}

impl<E: Executor + Display + Unpin + 'static> Handler<ActionMessage> for LazyExecutorActor<E> {
    type Result = ();

    fn handle(&mut self, msg: ActionMessage, _: &mut SyncContext<Self>) {
        trace!("LazyExecutorActor - received new action [{:?}]", &msg.action);

        if let Some(executor) = &mut self.executor {
            match executor.execute(msg.action) {
                Ok(_) => debug!("LazyExecutorActor - {} - Action executed successfully", &executor),
                Err(e) => {
                    error!("LazyExecutorActor - {} - Failed to execute action: {}", &executor, e)
                }
            };
        } else {
            error!("LazyExecutorActor received a message when it was not yet initialized!");
        }
    }
}

impl<E: Executor + Display + Unpin + 'static, F: Fn() -> E>
    Handler<LazyExecutorActorInitMessage<E, F>> for LazyExecutorActor<E>
where
    F: Send + Sync,
{
    type Result = ();

    fn handle(&mut self, msg: LazyExecutorActorInitMessage<E, F>, _: &mut SyncContext<Self>) {
        trace!("LazyExecutorActor - received init message");
        self.executor = Some((msg.init)());
    }
}

#[derive(Error, Debug)]
pub enum ApiClientActorError {
    #[error("ServerNotAvailableError: cannot connect to [{message}]")]
    ServerNotAvailableError { message: String },
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ApiClientConfig {
    /// The complete URL of the API Server
    pub server_api_url: String,

    /// Username used to connect to the APIs
    pub username: String,

    /// Password used to connect to the APIs
    pub password: String,

    /// If true, the client will not verify the SSL certificate
    pub disable_ssl_verification: bool,
}

pub struct ApiClientActor {
    server_api_url: String,
    http_auth_header: String,
    client: Client,
}

impl Actor for ApiClientActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("ApiClientActor started.");
    }
}

impl ApiClientActor {
    pub fn start_new(config: ApiClientConfig) -> Addr<Self> {
        ApiClientActor::create(move |_ctx: &mut Context<ApiClientActor>| {
            let auth = format!("{}:{}", config.username, config.password);
            let http_auth_header = format!("Basic {}", base64::encode(&auth));

            let mut client_builder = Client::builder().use_native_tls();
            if config.disable_ssl_verification {
                client_builder = client_builder.danger_accept_invalid_certs(true)
            }

            let client = client_builder.build().expect("Error while building the ApiClientActor");

            ApiClientActor { server_api_url: config.server_api_url, http_auth_header, client }
        })
    }
}
