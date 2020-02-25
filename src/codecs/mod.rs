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
    fn stream(&mut self, reader: &mut dyn Read, writer: &mut dyn Write) -> Result<u64, CodecError>
    {
        let mut total_bytes_written: u64 = 0;
        let mut buffer = [0 as u8; DEFAULT_BUFFER_SIZE];
        'read: loop {
            let len = match reader.read(&mut buffer) {
                Ok(0) => break 'read,
                Ok(len) => len,
                Err(e) => return Err(CodecError::ReadError(e)),
            };
            writer.write_all(self.process(Vec::from(&mut buffer[..len]))?.as_mut_slice())?;
            total_bytes_written += len as u64;
        }
        writer.write_all(self.flush()?.as_mut_slice())?;
        Ok(total_bytes_written)
    }

    fn process(&mut self, buffer: Vec<u8>) -> Result<Vec<u8>, CodecError> {
        return Ok(buffer);
    }

    fn flush(&mut self) -> Result<Vec<u8>, CodecError> {
        return Ok(Vec::new());
    }
}

pub mod pipe;
pub use pipe::Pipe;

pub mod not;
pub use not::Not;

pub mod huffman;
pub use huffman::{HuffmanEncoder, HuffmanDecoder};