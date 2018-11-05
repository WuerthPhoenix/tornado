#![cfg(unix)]

use tokio_uds::*;

use tokio;
use tokio::io;
use tokio::runtime::Runtime;
use tokio_codec::{Framed, LinesCodec};

use futures::sync::mpsc;

#[cfg(test)]
mod test {

    use super::*;
    use futures::{Future, Stream};
    use std::{thread, time};
    use tempfile::Builder;
    use tokio_uds::UnixStream;

    #[test]
    fn echo() {
        println!("start test");

        let dir = Builder::new().prefix("tokio-uds-tests").tempdir().unwrap();

        let sock_path = dir.path().join("connect.sock");
        let mut rt = Runtime::new().unwrap();

        let server = UnixListener::bind(&sock_path).unwrap();

        let (tx, rx) = mpsc::unbounded();

        rt.spawn({
            server
                .incoming()
                .for_each(move |stream| {
                    println!("Socket created");

                    let tx_clone = tx.clone();

                    // Default constructor has no buffer size limits. To be used only with trusted sources.
                    let codec = LinesCodec::new();

                    let framed = Framed::new(stream, codec).for_each(move |line| {
                        println!(
                            "Server - Thread {:?} - Received line {}",
                            thread::current().name(),
                            line
                        );
                        tx_clone.unbounded_send(line).expect("should send a line");
                        Ok(())
                    });

                    tokio::spawn(framed.map_err(|e| panic!("err={:?}", e)));

                    Ok(())
                }).map_err(|e| panic!("err={:?}", e))
        });

        thread::sleep(time::Duration::from_millis(100));

        let client_socket = UnixStream::connect(&sock_path.clone());
        let client = rt.block_on(client_socket).unwrap();
        //let server = rt.block_on(rx).unwrap();

        println!("Write to the client");
        // Write to the client
        rt.block_on(io::write_all(client, b"hello1\nhello2\n")).unwrap();

        println!("Written");

        rt.spawn(rx.for_each(move |line| {
            //save receiver side tx to db
            println!("Client - Thread {:?} - Received line {}", thread::current().name(), line);
            Ok(())
        }));

        thread::sleep(time::Duration::from_millis(100));
    }
}
