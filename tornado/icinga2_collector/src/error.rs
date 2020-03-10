use thiserror::Error;

#[derive(Error, Debug)]
pub enum Icinga2CollectorError {
    #[error("CannotPerformHttpRequest: [{message}]")]
    CannotPerformHttpRequest { message: String },
    #[error("UnexpectedEndOfHttpRequest")]
    UnexpectedEndOfHttpRequest,
}
