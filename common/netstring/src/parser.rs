use std::future::Future;
use std::marker::{PhantomPinned, Unpin};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, Error, ErrorKind, ReadBuf, Result};

macro_rules! ready {
    ($e:expr) => {
        match $e {
            Poll::Ready(t) => t,
            Poll::Pending => return Poll::Pending,
        }
    };
}

macro_rules! ready_and_ok {
    ($e:expr) => {
        match ready!($e) {
            Ok(val) => val,
            Err(err) => return Poll::Ready(Err(err)),
        }
    };
}

macro_rules! bytes_read {
    ($e:expr) => {
        match $e.filled().len() {
            0 => return Poll::Ready(Err(eof())),
            len => len,
        }
    };
}

// usize::MAX.to_string().len() + one byte separator
const MAX_LENGTH: usize = 21;

#[derive(Debug)]
enum State {
    Ready,
    ReadLength([u8; MAX_LENGTH], usize),
    ParseLength([u8; MAX_LENGTH], usize),
    VerifyLength(usize, u8),
    ParseSeparator(usize, u8),
    ReadMessage(usize),
    ParseTerminator,
}

pub(crate) fn read_netstring<'a, A>(reader: &'a mut A, buf: &'a mut [u8]) -> ReadMessage<'a, A>
where
    A: AsyncRead + Unpin + ?Sized,
{
    ReadMessage {
        reader,
        buf: ReadBuf::new(buf),
        state: State::Ready,
        _pin: PhantomPinned,
    }
}

pin_project! {
    /// Creates a future which will read exactly one message in the netstring format
    /// returning an error if EOF is hit sooner.
    ///
    /// On success the number of bytes is returned
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct ReadMessage<'a, A: ?Sized> {
        reader: &'a mut A,
        buf: ReadBuf<'a>,
        state: State,
        // Make this future `!Unpin` for compatibility with async trait methods.
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<A> Future for ReadMessage<'_, A>
where
    A: AsyncRead + Unpin + ?Sized,
{
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let me = self.project();

        loop {
            match me.state {
                //initialize the state machine
                State::Ready => {
                    *me.state = State::ReadLength([0; MAX_LENGTH], 0);
                }

                //read the length of the netstring, one byte at a time
                State::ReadLength(buf, prog) => {
                    let mut read_buf = ReadBuf::new(&mut buf[*prog..*prog + 1]);
                    ready_and_ok!(Pin::new(&mut *me.reader).poll_read(cx, &mut read_buf));
                    *prog += bytes_read!(read_buf);

                    if *prog == MAX_LENGTH || !read_buf.filled()[0].is_ascii_digit() {
                        *me.state = State::ParseLength(*buf, *prog);
                    }
                }

                //parse the length, the last byte in the buffer is the first non-ascii digit.
                State::ParseLength(buf, len) => {
                    match String::from_utf8_lossy(&buf[..*len - 1]).parse() {
                        Ok(msg_len) => *me.state = State::VerifyLength(msg_len, buf[*len - 1]),
                        Err(_) => return integer_overflow(),
                    }
                }

                //verify that the message fits into the buffer
                State::VerifyLength(msg_len, separator) => match me.buf.remaining() {
                    buf_size if buf_size <= *msg_len => return buffer_to_small(),
                    _ => *me.state = State::ParseSeparator(*msg_len, *separator),
                },

                //verify that length and message are separated by a ':'
                State::ParseSeparator(len, separator) => match *separator {
                    b':' => *me.state = State::ReadMessage(*len),
                    sep => return wrong_separator(sep),
                },

                //read the message from the stream
                State::ReadMessage(remaining) => match *remaining {
                    0 => *me.state = State::ParseTerminator,
                    _ => {
                        let read = {
                            let mut reader = (*me.buf).take(*remaining);
                            ready_and_ok!(Pin::new(&mut *me.reader).poll_read(cx, &mut reader));
                            bytes_read!(reader)
                        };

                        me.buf.advance(read);
                        *remaining -= read;
                    }
                },

                //verify that the message is terminated with a ','
                State::ParseTerminator => {
                    let mut byte_buf = [0; 1];
                    let mut read_buf = ReadBuf::new(&mut byte_buf);

                    ready_and_ok!(Pin::new(&mut *me.reader).poll_read(cx, &mut read_buf));
                    bytes_read!(read_buf);

                    return match byte_buf[0] {
                        b',' => Poll::Ready(Ok(me.buf.filled().len())),
                        term => wrong_terminator(term),
                    };
                }
            }
        }
    }
}

fn eof() -> Error {
    Error::new(ErrorKind::UnexpectedEof, "early eof")
}

fn integer_overflow() -> Poll<Result<usize>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        "ERROR: Integer overflow while parsing message length.".to_string(),
    )))
}

fn buffer_to_small() -> Poll<Result<usize>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::BrokenPipe,
        "ERROR: Output buffer to small for message".to_string(),
    )))
}

fn wrong_separator(separator: u8) -> Poll<Result<usize>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        format!(
            "ERROR: Expected separator ':' but found {} instead",
            separator
        ),
    )))
}

fn wrong_terminator(terminator: u8) -> Poll<Result<usize>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        format!(
            "ERROR: Expected terminator ',' but found {} instead",
            terminator
        ),
    )))
}

#[cfg(test)]
mod tests {
    use tokio::time::Duration;
    use tokio_test::io::Builder;

    use crate::NetstringReader;

    #[test]
    fn should_parse_netstring() {
        let msg = "13:Hello, World!,";
        let expected = "Hello, World!";
        let mut buf = [0; 13];

        let mut test = Builder::new().read(msg.as_bytes()).build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect("Test should pass");

        assert_eq!(expected.as_bytes(), buf);
    }

    #[test]
    fn should_parse_netstring_in_two_steps() {
        let msg = "13:Hello, World!,";
        let expected = "Hello, World!";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new()
            .read(&msg.as_bytes()[..split])
            .wait(Duration::from_micros(5))
            .read(&msg.as_bytes()[split..])
            .build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect("Test should pass");

        assert_eq!(expected.as_bytes(), buf);
    }

    #[test]
    fn should_fail_on_incomplete_message() {
        let msg = "13:Hello, World!,";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new().read(&msg.as_bytes()[..split]).build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect_err("Message not finished");
    }

    #[test]
    fn should_fail_on_incomplete_message_missing_terminator() {
        let msg = "13:Hello, World!";
        let split = 10;
        let mut buf = [0; 13];

        let mut test = Builder::new().read(&msg.as_bytes()[..split]).build();

        tokio_test::block_on(test.read_netstring(&mut buf)).expect_err("Message not finished");
    }
}
