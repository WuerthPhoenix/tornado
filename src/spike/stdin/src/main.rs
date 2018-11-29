extern crate actix;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;

pub mod standard;
pub mod tokio_actix;

fn main() {

    // with tokio/actix
    tokio_actix::start_actix_stdin();

    // with std
    //standard::start_standard_stdin();
}
