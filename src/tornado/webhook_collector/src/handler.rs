use actix_web::{error, http, HttpRequest, HttpResponse, Query};
use failure::Fail;
use log::{debug, error, info};
use serde_derive::Deserialize;
use tornado_collector_common::Collector;
use tornado_collector_jmespath::JMESPathEventCollector;

#[derive(Deserialize)]
pub struct TokenQuery {
    token: String,
}

#[derive(Fail, Debug)]
#[fail(display = "NotValidToken")]
pub struct WrongTokenError {}

impl error::ResponseError for WrongTokenError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(http::StatusCode::UNAUTHORIZED)
    }
}

pub struct Handler {
    pub id: String,
    pub token: String,
    pub collector: JMESPathEventCollector,
}

impl Handler {
    pub fn handle(
        &self,
        (_req, body, query): (HttpRequest, String, Query<TokenQuery>),
    ) -> Result<String, WrongTokenError> {
        let received_token = &query.token;

        debug!("Endpoint [{}] called with token [{}]", self.id, received_token);

        if !(self.token.eq(received_token)) {
            error!("Endpoint [{}] - Token is not valid: [{}]", self.id, received_token);
            return Err(WrongTokenError {});
        }

        info!("collector result = {:#?}", self.collector.to_event(&body));

        Ok(self.id.to_string())
    }
}
