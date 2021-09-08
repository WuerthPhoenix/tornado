use tokio::io::{AsyncRead, AsyncWrite};

mod parser;
mod writer;

pub trait NetstringReader: AsyncRead {
    fn read_netstring<'a>(&'a mut self, buf: &'a mut [u8]) -> parser::ReadMessage<'a, Self>
        where
            Self: Unpin,
    {
        parser::read_netstring(self, buf)
    }
}

pub trait NetstringWriter: AsyncWrite {
    fn write_netstring<'a>(&'a mut self, buf: &'a [u8]) -> writer::WriteMessage<'a, Self>
        where
            Self: Unpin,
    {
        writer::write_netstring(self, buf)
    }
}

impl<R: AsyncRead + ?Sized> NetstringReader for R {}

impl<W: AsyncWrite + ?Sized> NetstringWriter for W {}
