use crate::codecs::Codec;

pub struct Pipe {}

impl Pipe {
    pub fn new() -> Pipe {
        return Pipe {};
    }
}

impl Codec for Pipe {}
