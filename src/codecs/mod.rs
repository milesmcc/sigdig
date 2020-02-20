use io::{Read, Write};
use std::convert::From;
use std::io;

pub const DEFAULT_BUFFER_SIZE: usize = 8192;

#[derive(Debug)]
pub enum CodecError {
    IOError(io::Error),
    ReadError(io::Error),
    WriteError(io::Error),
}

impl From<io::Error> for CodecError {
    fn from(err: io::Error) -> CodecError {
        CodecError::IOError(err)
    }
}

pub trait Codec {
    fn stream<R, W>(&mut self, reader: &mut R, writer: &mut W) -> Result<u64, CodecError>
    where
        R: Read,
        W: Write,
    {
        let mut total_bytes_written: u64 = 0;
        let mut buffer = [0 as u8; DEFAULT_BUFFER_SIZE];
        loop {
            let len = match reader.read(&mut buffer) {
                Ok(0) => return Ok(total_bytes_written),
                Ok(len) => len,
                Err(e) => return Err(CodecError::ReadError(e)),
            };
            writer.write_all(self.transform(Vec::from(&mut buffer[..len]))?.as_mut_slice())?;
            total_bytes_written += len as u64;
        }
    }

    fn transform(&mut self, buffer: Vec<u8>) -> Result<Vec<u8>, CodecError> {
        return Ok(buffer)
    }
}

pub mod pipe;
pub use pipe::Pipe;

pub mod not;
pub use not::Not;
