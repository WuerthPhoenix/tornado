extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_network_common;
extern crate tornado_network_simple;

extern crate actix;
extern crate bytes;
extern crate futures;
#[macro_use] extern crate log;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

pub mod matcher;
pub mod uds;

#[cfg(test)]
extern crate tempfile;

use actix::prelude::*;
use futures::Stream;
use matcher::MatcherActor;
use uds::{UdsConnectMessage, UdsServerActor};
use tokio_uds::*;

fn main() {
    // start system, this is required step
    System::run(|| {
        // start new actor
        let matcher_actor = MatcherActor{ }.start();

        let sock_path = "/tmp/something";
        let listener = UnixListener::bind(&sock_path).unwrap();

        UdsServerActor::create(|ctx| {
            ctx.add_message_stream(listener.incoming()
                .map_err(|e| panic!("err={:?}", e))
                .map(|stream| {
                    let addr = st.peer_addr().unwrap();
                    UdsConnectMessage(stream)
                }));
            UdsServerActor{ matcher_addr: matcher_actor.clone() }
        });

    });

}