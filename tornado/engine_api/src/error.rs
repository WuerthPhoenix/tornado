use actix::MailboxError;
use actix_web::{http, HttpResponse, HttpResponseBuilder};
use std::collections::HashMap;
use std::fmt::Display;
use thiserror::Error;
use tornado_engine_api_dto::common::WebError;
use tornado_engine_matcher::error::MatcherError;

#[derive(Error, Debug, PartialEq)]
pub enum ApiError {
    #[error("MatcherError: [{cause}]")]
    MatcherError { cause: MatcherError },
    #[error("ActixMailboxError: [{cause}]")]
    ActixMailboxError { cause: String },
    #[error("JsonError: [{cause}]")]
    JsonError { cause: String },

    #[error("BadRequestError: [{cause}]")]
    BadRequestError { cause: String },
    #[error("InternalServerError: [{cause}]")]
    InternalServerError { cause: String },
    #[error("PayloadToLarge")]
    PayloadToLarge,

    #[error("MissingAuthTokenError")]
    MissingAuthTokenError,
    #[error("ParseAuthHeaderError: [{message}]")]
    ParseAuthHeaderError { message: String },
    #[error("InvalidAuthKeyError: [{message}]")]
    InvalidAuthKeyError { message: String },
    #[error("InvalidAuthorizedPath: [{message}]")]
    InvalidAuthorizedPath { message: String },
    #[error("InvalidTokenError: [{message}]")]
    InvalidTokenError { message: String },
    #[error("ExpiredTokenError: [{message}]")]
    ExpiredTokenError { message: String },
    #[error("UnauthenticatedError")]
    UnauthenticatedError,

    #[error("ForbiddenError [{message}]")]
    ForbiddenError { code: String, message: String, params: HashMap<String, String> },

    #[error("NodeNotFoundError [{message}]")]
    NodeNotFoundError { message: String },
}

impl From<MatcherError> for ApiError {
    fn from(err: MatcherError) -> Self {
        ApiError::MatcherError { cause: err }
    }
}

impl From<MailboxError> for ApiError {
    fn from(err: MailboxError) -> Self {
        ApiError::ActixMailboxError { cause: format!("{}", err) }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::JsonError { cause: format!("{}", err) }
    }
}

const VALIDATION_ERROR: &str = "VALIDATION_ERROR";

// Use default implementation for `error_response()` method.
impl actix_web::error::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::MatcherError { cause } => match cause {
                MatcherError::NotUniqueRuleNameError { name } => {
                    let mut params = HashMap::new();
                    params.insert("RULE_NAME".to_owned(), name.to_owned());
                    HttpResponseBuilder::new(http::StatusCode::BAD_REQUEST).json(WebError {
                        code: VALIDATION_ERROR.to_owned(),
                        message: Some(format!("{}", cause)),
                        params,
                    })
                }
                MatcherError::NotValidIdOrNameError { message } => {
                    HttpResponseBuilder::new(http::StatusCode::BAD_REQUEST).json(WebError {
                        code: VALIDATION_ERROR.to_owned(),
                        message: Some(message.to_owned()),
                        params: HashMap::new(),
                    })
                }
                _ => HttpResponse::BadRequest().finish(),
            },
            ApiError::ActixMailboxError { .. }
            | ApiError::JsonError { .. }
            | ApiError::InternalServerError { .. } => HttpResponse::InternalServerError().finish(),
            ApiError::BadRequestError { .. } => HttpResponse::BadRequest().finish(),
            ApiError::PayloadToLarge => HttpResponse::PayloadTooLarge().finish(),
            ApiError::NodeNotFoundError { .. } => HttpResponse::NotFound().finish(),
            ApiError::InvalidTokenError { .. }
            | ApiError::ExpiredTokenError { .. }
            | ApiError::MissingAuthTokenError { .. }
            | ApiError::ParseAuthHeaderError { .. }
            | ApiError::UnauthenticatedError
            | ApiError::InvalidAuthKeyError { .. }
            | ApiError::InvalidAuthorizedPath { .. } => HttpResponse::Unauthorized().finish(),
            ApiError::ForbiddenError { code, params, .. } => {
                let http_code = http::StatusCode::FORBIDDEN;
                HttpResponseBuilder::new(http_code).json(WebError {
                    code: code.to_owned(),
                    message: None,
                    params: params.clone(),
                })
            }
        }
    }
}
