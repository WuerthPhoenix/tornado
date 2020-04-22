use actix::MailboxError;
use actix_web::HttpResponse;
use thiserror::Error;
use tornado_engine_matcher::error::MatcherError;

#[derive(Error, Debug, PartialEq)]
pub enum ApiError {
    #[error("MatcherError: [{cause}]")]
    MatcherError { cause: MatcherError },
    #[error("ActixMailboxError: [{cause}]")]
    ActixMailboxError { cause: String },
    #[error("JsonError: [{cause}]")]
    JsonError { cause: String },
    #[error("InternalServerError: [{cause}]")]
    InternalServerError { cause: String },

    #[error("MissingAuthTokenError")]
    MissingAuthTokenError,
    #[error("ParseAuthHeaderError: [{message}]")]
    ParseAuthHeaderError { message: String },
    #[error("InvalidTokenError: [{message}]")]
    InvalidTokenError { message: String },
    #[error("ExpiredTokenError: [{message}]")]
    ExpiredTokenError { message: String },
    #[error("UnauthenticatedError")]
    UnauthenticatedError,

    #[error("ForbiddenError [{message}]")]
    ForbiddenError { message: String },
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

// Use default implementation for `error_response()` method.
impl actix_web::error::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            ApiError::MatcherError { .. } => HttpResponse::BadRequest().finish(),
            ApiError::ActixMailboxError { .. } => HttpResponse::InternalServerError().finish(),
            ApiError::JsonError { .. } => HttpResponse::InternalServerError().finish(),
            ApiError::InternalServerError { .. } => HttpResponse::InternalServerError().finish(),
            ApiError::InvalidTokenError { .. }
            | ApiError::ExpiredTokenError { .. }
            | ApiError::MissingAuthTokenError { .. }
            | ApiError::ParseAuthHeaderError { .. }
            | ApiError::UnauthenticatedError => HttpResponse::Unauthorized().finish(),
            ApiError::ForbiddenError { .. } => HttpResponse::Forbidden().finish(),
        }
    }
}
