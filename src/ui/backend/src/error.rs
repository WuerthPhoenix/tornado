use actix::MailboxError;
use actix_web::HttpResponse;
use failure_derive::Fail;
use tornado_engine_matcher::error::MatcherError;

#[derive(Fail, Debug, PartialEq)]
pub enum ApiError {
    #[fail(display = "MatcherError: [{}]", cause)]
    MatcherError { cause: MatcherError },
    #[fail(display = "ActixMailboxError: [{}]", cause)]
    ActixMailboxError { cause: String },
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

// Use default implementation for `error_response()` method.
impl actix_web::error::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            ApiError::MatcherError { .. } => HttpResponse::BadRequest().finish(),
            ApiError::ActixMailboxError { .. } => {
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
