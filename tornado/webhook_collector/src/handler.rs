use actix_web::{error, http, HttpResponse};
use log::*;
use serde_derive::Deserialize;
use thiserror::Error;
use tornado_collector_common::Collector;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common_api::Event;

#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: String,
}

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("The request cannot be processed: {message}")]
    CollectorError { message: String },
    #[error("NotValidToken")]
    WrongTokenError,
}

impl error::ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            HandlerError::CollectorError { .. } => {
                HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR)
            }
            HandlerError::WrongTokenError => HttpResponse::new(http::StatusCode::UNAUTHORIZED),
        }
    }
}

pub struct Handler<F: Fn(Event)> {
    pub id: String,
    pub token: String,
    pub collector: JMESPathEventCollector,
    pub callback: F,
}

impl<F: Fn(Event)> Handler<F> {
    pub fn handle(&self, body: &str, received_token: &str) -> Result<String, HandlerError> {
        trace!("Endpoint [{}] called with token [{}]", self.id, received_token);
        debug!("Received call with body [{}]", body);

        if !(self.token.eq(received_token)) {
            error!("Endpoint [{}] - Token is not valid: [{}]", self.id, received_token);
            return Err(HandlerError::WrongTokenError);
        }

        let event = self
            .collector
            .to_event(body)
            .map_err(|err| HandlerError::CollectorError { message: format!("{}", err) })?;

        (self.callback)(event);

        Ok(self.id.to_string())
    }
}
