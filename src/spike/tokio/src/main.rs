extern crate tokio;
extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_network_common;
extern crate tornado_network_simple;

use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;

fn main() {

    println!("Hello from Tokio!");

    // Parse the address of whatever server we're talking to
    let addr = "127.0.0.1:6142".parse().unwrap();

    let client = TcpStream::connect(&addr).and_then(|stream| {
        println!("created stream");

        io::write_all(stream, "hello world\n").then(|result| {
            println!("wrote to stream; success={:?}", result.is_ok());
            Ok(())
        })
    }).map_err(|err| {
        // All tasks must have an `Error` type of `()`. This forces error
        // handling and helps avoid silencing failures.
        //
        // In our example, we are only going to log the error to STDOUT.
        println!("connection error = {:?}", err);
    });

    println!("About to create the stream and write to it...");
    tokio::run(client);
    println!("Stream has been created and written to.");

}
