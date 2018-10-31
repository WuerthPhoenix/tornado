#![cfg(unix)]

use tokio_uds::*;

use tokio;
use tokio::io;
use tokio_codec::{Framed, FramedRead, LinesCodec};
use tokio::runtime::Runtime;

use futures::{Future, Stream};
use futures::sync::{mpsc, oneshot};

#[cfg(test)]
mod test {

    use super::*;
    use std::thread;
    use std::sync::Arc;
    use tokio_uds::UnixStream;
    use tokio::io::AsyncRead;
    use tempfile::Builder;

    #[test]
    fn echo() {

        println!("start test");

        let dir = Builder::new().prefix("tokio-uds-tests").tempdir().unwrap();

        let sock_path = dir.path().join("connect.sock");
        let mut rt = Runtime::new().unwrap();

        let server = UnixListener::bind(&sock_path).unwrap();

        let (tx, rx) = mpsc::unbounded();

        rt.spawn({
            server.incoming()
                .for_each(move |stream| {

                    println!("Socket created");

                    let tx_clone = tx.clone();

                    // Default constructor has no buffer size limits. To be used only with trusted sources.
                    let codec = LinesCodec::new();

                    let mut framed = Framed::new(stream, codec).for_each(move |line| {
                        println!("Server - Thread {:?} - Received line {}", thread::current().name(), line);
                        tx_clone.send(line).expect("should send a line");
                        Ok(())
                    });

                    tokio::spawn(framed.map_err(|e| panic!("err={:?}", e)));

                    Ok(())
                })
                .map_err(|e| panic!("err={:?}", e))
        });

        thread::sleep_ms(100);

        let client_socket = UnixStream::connect(&sock_path.clone());
        let client = rt.block_on(client_socket).unwrap();
        //let server = rt.block_on(rx).unwrap();

        println!("Write to the client");
        // Write to the client
        rt.block_on(io::write_all(client, b"hello1\nhello2\n")).unwrap();

        println!("Written");

        rt.spawn(rx.for_each(move |line| {
            //save receiver side tx to db
            println!("Client - Thread {:?} - Received line {}", thread::current().name() , line);
            Ok(())
        }));

        thread::sleep_ms(100);
    }
}