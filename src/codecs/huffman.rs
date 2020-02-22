//! Simple Huffman coding compression

use crate::{Codec, CodecError};
use bitvec::prelude::BitVec;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::iter::FromIterator;

struct HuffmanBuilder {
    counts: [usize; 256],
    len: usize,
}

impl HuffmanBuilder {
    pub fn new() -> Self {
        HuffmanBuilder {
            counts: [0; 256],
            len: 0,
        }
    }

    pub fn push(&mut self, byte: u8) {
        self.counts[byte as usize] += 1;
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn build<'a>(&mut self) -> HuffmanTree {
        let mut index = Vec::from_iter((0..255).map(|_| BitVec::new()));
        let mut heap: BinaryHeap<Reverse<HuffmanNode>> = BinaryHeap::from_iter(
            (0..255).map(|i| Reverse(HuffmanNode::Terminal(i, self.counts[i as usize]))),
        );

        // Go through the smallest items in the heap and continuously merge them until
        // there is only one item in the heap -- the root.
        while heap.len() >= 2 {
            error!("{}", heap.len());
            let left = heap.pop().unwrap().0; // should never panic
            let right = heap.pop().unwrap().0;

            for val in left.values() {
                index.get_mut(val as usize).unwrap().push(false);
            }

            for val in right.values() {
                index.get_mut(val as usize).unwrap().push(true);
                error!("{}: {:?}", val, index.get_mut(val as usize).unwrap());
            }

            let new_parent = HuffmanNode::Interior(Box::from(left), Box::from(right));
            heap.push(Reverse(new_parent));
        }
        error!("{} :: {:?}", heap.len(), heap.peek().unwrap().0);

        return HuffmanTree {
            root: heap.pop().unwrap().0, // should never panic
            index: index,
        };
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug)]
enum HuffmanNode {
    Interior(Box<HuffmanNode>, Box<HuffmanNode>), // 0 path, 1 path
    Terminal(u8, usize),                          // value, weight
}

impl HuffmanNode {
    fn weight(&self) -> usize {
        match self {
            HuffmanNode::Terminal(_, weight) => *weight,
            HuffmanNode::Interior(left, right) => left.weight() + right.weight(),
        }
    }

    fn values(&self) -> Vec<u8> {
        match self {
            HuffmanNode::Terminal(value, _) => vec![*value],
            HuffmanNode::Interior(left, right) => {
                let mut vals = left.values();
                vals.append(&mut right.values());
                vals
            }
        }
    }
}

struct HuffmanTree {
    root: HuffmanNode,
    index: Vec<BitVec>,
}

impl HuffmanTree {
    pub fn encode(&self, byte: u8) -> Option<BitVec> {
        let val = match self.index.get(byte as usize) {
            Some(v) => Some({
                // error!("{} -> {:?}", byte, v);
                v.clone()
            }),
            None => None,
        };
        val
    }
}

pub struct HuffmanEncoder {
    tree: Option<HuffmanTree>,
    builder: HuffmanBuilder,
    buffer_in: Vec<u8>,
    buffer_out: BitVec,
    build_tree_after: usize,
}

impl HuffmanEncoder {
    pub fn new(build_tree_after: usize) -> Self {
        HuffmanEncoder {
            tree: None,
            builder: HuffmanBuilder::new(),
            buffer_in: Vec::new(),
            buffer_out: BitVec::new(),
            build_tree_after: build_tree_after,
        }
    }

    fn process_buffer(&mut self, force_push: bool) -> Result<Vec<u8>, CodecError> {
        match &mut self.tree {
            Some(tree) => {
                while let Some(byte) = self.buffer_in.pop() {
                    self.buffer_out.append(&mut tree.encode(byte).unwrap()); // TODO: remove unwrap
                }
            }
            None => {
                if force_push || self.buffer_in.len() > self.build_tree_after {
                    self.tree = Some(self.builder.build());
                    self.process_buffer(force_push)?;
                }
            }
        }

        if self.buffer_out.len() % 8 == 0 || force_push {
            let out: Vec<u8> =
                Vec::from_iter(self.buffer_out.as_bitslice().chunks_exact(8).map(|chunk| {
                    let mut byte: u8 = 0;
                    for i in 0..8 {
                        byte <<= 1;
                        if *chunk.get(i).unwrap() {
                            byte += 1;
                        }
                    }
                    // error!("{}", byte);
                    byte
                }));
            self.buffer_out = BitVec::new();
            Ok(out)
        } else {
            Ok(Vec::new())
        }
    }
}

impl Codec for HuffmanEncoder {
    fn process(&mut self, mut buffer: Vec<u8>) -> Result<Vec<u8>, CodecError> {
        if self.tree.is_none() {
            for byte in &buffer {
                self.builder.push(*byte);
            }
        }
        self.buffer_in.append(&mut buffer);
        self.process_buffer(false)
    }

    fn flush(&mut self) -> Result<Vec<u8>, CodecError> {
        self.process_buffer(true)
    }
}
