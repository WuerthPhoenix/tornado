extern crate actix;
extern crate config;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate structopt;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;

pub mod cli;
pub mod standard;
pub mod tokio_actix;

fn main() {

    let conf = cli::Conf::new().unwrap();
    println!("Config:");
    println!("value_one: [{}]", conf.value_one);
    println!("value_two: [{}]", conf.value_two);


    cli::print_cli();

    // with tokio/actix
    tokio_actix::start_actix_stdin();

    // with std
    //standard::start_standard_stdin();
}
