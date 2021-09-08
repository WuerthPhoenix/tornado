use std::future::Future;
use std::marker::{PhantomPinned, Unpin};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncWrite, ErrorKind, Result};

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

pub(crate) fn write_netstring<'a, A>(writer: &'a mut A, buf: &'a [u8]) -> WriteMessage<'a, A>
where
    A: AsyncWrite + Unpin + ?Sized,
{
    const MAX_NETSTRING_OVERHEAD: usize = 22; // usize::MAX.to_string().len() + [b':'].len() + [b','].len()

    let mut buffer = Vec::with_capacity(buf.len() + MAX_NETSTRING_OVERHEAD);
    buffer.extend_from_slice(buf.len().to_string().as_bytes());
    buffer.extend_from_slice(&[b':']);
    buffer.extend_from_slice(buf);
    buffer.extend_from_slice(&[b',']);

    WriteMessage {
        writer,
        buf: buffer,
        prog: 0,
        _pin: Default::default(),
    }
}

pin_project! {
    /// Creates a future which will read exactly enough bytes to fill `buf`,
    /// returning an error if EOF is hit sooner.
    ///
    /// On success the number of bytes is returned
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct WriteMessage<'a, A: ?Sized> {
        writer: &'a mut A,
        buf: Vec<u8>,
        prog: usize,
        // Make this future `!Unpin` for compatibility with async trait methods.
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<A> Future for WriteMessage<'_, A>
where
    A: AsyncWrite + Unpin + ?Sized,
{
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let me = self.project();

        loop {
            let n = ready_and_ok!(Pin::new(&mut *me.writer).poll_write(cx, &me.buf[*me.prog..]));
            *me.prog += n;

            if *me.prog == me.buf.len() {
                return Poll::Ready(Ok(*me.prog));
            }

            if n == 0 {
                return Poll::Ready(Err(ErrorKind::WriteZero.into()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio_test::io::Builder;

    use crate::NetstringWriter;

    #[test]
    fn should_write_netstring() {
        let msg = "Hello, World!";
        let expected = "13:Hello, World!,";

        let mut stream = Builder::new().write(expected.as_bytes()).build();

        tokio_test::block_on(stream.write_netstring(msg.as_bytes())).expect("Test passes");
    }

    #[test]
    fn should_write_netstring_in_two_steps() {
        let msg = "Hello, World!";
        let expected = "13:Hello, World!,";
        let cut_off = 8;

        let mut stream = Builder::new()
            .write(&expected.as_bytes()[..cut_off])
            .wait(Duration::from_millis(5))
            .write(&expected.as_bytes()[cut_off..])
            .build();

        tokio_test::block_on(stream.write_netstring(msg.as_bytes())).expect("Test passes");
    }
}
