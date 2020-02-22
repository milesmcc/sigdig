use crate::codecs::Codec;

pub struct Pipe {}

impl Pipe {
    pub fn new() -> Self {
        return Pipe {};
    }
}

impl Codec for Pipe {}
