use crate::codecs::{Codec, CodecError};

pub struct Not {}

impl Not {
    pub fn new() -> Not {
        return Not {};
    }
}

impl Codec for Not {
    fn transform(&mut self, buffer: Vec<u8>) -> Result<Vec<u8>, CodecError> {
        let mut bytes = Vec::with_capacity(buffer.len());
        for byte in buffer {
            bytes.push(!byte);
        }
        Ok(bytes)
    }
}
