use failure_derive::Fail;

#[derive(Fail, Debug)]
pub enum Icinga2CollectorError {
    #[fail(display = "CannotPerformHttpRequest: [{}]", message)]
    CannotPerformHttpRequest { message: String },
    #[fail(display = "UnexpectedEndOfHttpRequest")]
    UnexpectedEndOfHttpRequest,
}
