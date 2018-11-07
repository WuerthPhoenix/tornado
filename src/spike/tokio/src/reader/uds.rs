
use futures::{Future, Stream};
use futures::IntoFuture;
use futures::sync::mpsc;
use std::fs;
use std::io::Error;
use std::path::Path;
use std::thread;
use tokio_codec::{Framed, LinesCodec};
use tokio_uds::*;

pub fn start_uds_socket<P: AsRef<Path>>(path: P) -> impl Future<Item = (), Error = std::io::Error> {

    let listener = match UnixListener::bind(&path) {
        Ok(m) => m,
        Err(_) => {
            fs::remove_file(&path).unwrap();
            UnixListener::bind(&path).unwrap()
        }
    };

    listener
        .incoming()
        .for_each(move |stream| {
            info!("Socket created");

            //let tx_clone = tx.clone();

            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed = Framed::new(stream, codec).for_each(move |line| {
                info!(
                    "Server - Thread {:?} - Received line {}",
                    thread::current().name(),
                    line
                );
                //tx_clone.unbounded_send(line).expect("should send a line");
                Ok(())
            });

            tokio::spawn(framed.map_err(|e| panic!("err={:?}", e)));

            Ok(())
        })

}

