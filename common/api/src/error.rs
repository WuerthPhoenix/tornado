use thiserror::Error;

#[derive(Error, Clone, Debug, PartialEq)]
pub enum CommonError {
    #[error("BadDataError: [{message}]")]
    BadDataError { message: String },
}