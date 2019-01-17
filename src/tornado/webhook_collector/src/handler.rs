use actix_web::{error, HttpRequest, HttpResponse, Query, http};
use serde_derive::Deserialize;
use failure::Fail;
use tornado_collector_common::{Collector};

#[derive(Deserialize)]
pub struct TokenQuery {
    token: String
}

#[derive(Fail, Debug)]
#[fail(display="NotValidToken")]
pub struct WrongTokenError {}

impl error::ResponseError for WrongTokenError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(http::StatusCode::UNAUTHORIZED)
    }
}

pub struct Handler<C>
    where C: Collector<&'static str>
{
    pub id: String,
    pub token: String,
    pub collector: C,
}

impl <C> Handler<C> where C: Collector<&'static str> {
    pub fn handle(&self, (_req, body, query): (HttpRequest, String, Query<TokenQuery>)) -> Result<String, WrongTokenError> {
        if !(self.token.eq(&query.token)) {
            return Err(WrongTokenError{});
        }
        Ok(format!("{}", self.id))
    }
}