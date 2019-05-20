use actix_web::web::Query;
use actix_web::{error, http, HttpRequest, HttpResponse};
use failure::Fail;
use log::{debug, error};
use serde_derive::Deserialize;
use tornado_collector_common::Collector;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common_api::Event;

#[derive(Deserialize)]
pub struct TokenQuery {
    token: String,
}

#[derive(Fail, Debug)]
pub enum HandlerError {
    #[fail(display = "The request cannot be processed: {}", message)]
    CollectorError { message: String },
    #[fail(display = "NotValidToken")]
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
    pub fn handle(
        &self,
        (_req, body, query): (HttpRequest, String, Query<TokenQuery>),
    ) -> Result<String, HandlerError> {
        let received_token = &query.token;

        debug!("Endpoint [{}] called with token [{}]", self.id, received_token);

        if !(self.token.eq(received_token)) {
            error!("Endpoint [{}] - Token is not valid: [{}]", self.id, received_token);
            return Err(HandlerError::WrongTokenError);
        }

        let event = self
            .collector
            .to_event(&body)
            .map_err(|err| HandlerError::CollectorError { message: format!("{}", err) })?;

        (self.callback)(event);

        Ok(self.id.to_string())
    }
}
