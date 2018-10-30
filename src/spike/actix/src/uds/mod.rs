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
        //let (tx, rx) = oneshot::channel();

        let (tx, rx) = mpsc::unbounded();

        rt.spawn({
            server.incoming()
                .for_each(move |stream| {

                    println!("Socket created");

                    let tx_clone = tx.clone();

                    // Default constructor has no buffer size limits. To be used only with trusted sources.
                    let codec = LinesCodec::new();
                    let framed = stream.framed(codec).for_each(move |line| {
                        println!("Received line {}", line);
                        tx_clone.send(line).expect("should send a line");
                        thread::sleep_ms(2000);
                        Ok(())
                    });

                    tokio::spawn(framed.map_err(|e| panic!("err={:?}", e)));

                    //tx.send(sock.unwrap()).unwrap();

                    // let stream = sock.expect("Should return a valid socket");
                    // let (reader, writer) = stream.split();


/*
                    let mut framed_sock = Framed::new(stream, codec);

                    framed_sock.for_each(move |line| {
                        println!("Received line {}", line);
                        //tx.send(line).expect("should send a line");
                        thread::sleep_ms(2000);
                        Ok(())
                    }).poll().expect("should poll the framed_socket");
*/
                    thread::sleep_ms(2000);

                    Ok(())
                })
                .map_err(|e| panic!("err={:?}", e))
        });


        let client = rt.block_on(UnixStream::connect(&sock_path)).unwrap();
        //let server = rt.block_on(rx).unwrap();

        println!("Write to the client");
        // Write to the client
        rt.block_on(io::write_all(client, b"hello\nhello\n")).unwrap();

        println!("Written");
        // Read from the server
       // let (_, buf) = rt.block_on(io::read_to_end(server, vec![])).unwrap();

        //assert_eq!(buf, b"hello");

        thread::sleep_ms(2000);
    }
}