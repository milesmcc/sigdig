//! Simple Huffman coding compression

use crate::{Codec, CodecError};
use bitvec::order::Msb0;
use bitvec::prelude::BitVec;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::iter::FromIterator;
use std::convert::TryInto;

// Type parameters for BitVecs
type BVb = BitVec<Msb0, u8>;
type BVs = BitVec<Msb0, u64>;

struct HuffmanBuilder {
    counts: [u64; 256],
    len: u64,
}

impl HuffmanBuilder {
    pub fn new() -> Self {
        HuffmanBuilder {
            counts: [0; 256],
            len: 0,
        }
    }

    pub fn serialize(&self) -> [u64; 256] {
        self.counts
    }

    pub fn deserialize(counts: [u64; 256]) -> Self {
        HuffmanBuilder {
            counts: counts,
            len: counts.iter().sum(),
        }
    }

    pub fn push(&mut self, byte: u8) {
        self.counts[byte as usize] += 1;
        self.len += 1;
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn build<'a>(&mut self) -> HuffmanTree {
        let mut index = Vec::from_iter((0..=255).map(|_| BVb::new()));
        let mut heap: BinaryHeap<Reverse<HuffmanNode>> = BinaryHeap::from_iter(
            (0..=255).map(|i| Reverse(HuffmanNode::Terminal(i, self.counts[i as usize]))),
        );

        for i in 0..256 {
            error!("{}: {}", i, self.counts[i]);
        }

        // Go through the smallest items in the heap and continuously merge them until
        // there is only one item in the heap -- the root.
        while heap.len() >= 2 {
            let left = heap.pop().unwrap().0; // should never panic
            let right = heap.pop().unwrap().0;

            for val in left.values() {
                index.get_mut(val as usize).unwrap().insert(0, false);
            }

            for val in right.values() {
                index.get_mut(val as usize).unwrap().insert(0, true);
            }

            let new_parent = HuffmanNode::Interior(Box::from(left), Box::from(right));
            heap.push(Reverse(new_parent));
        }

        return HuffmanTree {
            root: heap.pop().unwrap().0, // should never panic
            index: index,
        };
    }
}

#[derive(PartialEq, Eq, Ord, Debug)]
enum HuffmanNode {
    Interior(Box<HuffmanNode>, Box<HuffmanNode>), // 0 path, 1 path
    Terminal(u8, u64),                            // value, weight
}

impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.weight().cmp(&other.weight()))
    }
}

impl HuffmanNode {
    fn weight(&self) -> u64 {
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
    index: Vec<BVb>,
}

impl HuffmanTree {
    pub fn encode(&self, byte: u8) -> Option<BVb> {
        let val = match self.index.get(byte as usize) {
            Some(v) => Some({
                v.clone() // todo: is cloning necessary here?
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
    buffer_out: BVb,
    build_tree_after: u64,
}

impl HuffmanEncoder {
    pub fn new(build_tree_after: u64) -> Self {
        HuffmanEncoder {
            tree: None,
            builder: HuffmanBuilder::new(),
            buffer_in: Vec::new(),
            buffer_out: BVb::new(),
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
                if force_push || self.buffer_in.len() as u64 > self.build_tree_after {
                    self.tree = Some(self.builder.build());
                    let tree_bitvec: BVs = BitVec::from_slice(&self.builder.serialize());
                    self.buffer_out.extend(tree_bitvec);
                    self.process_buffer(false)?;
                }
            }
        }

        if self.buffer_out.len() % 64 == 0 || force_push {
            let out: Vec<u8> = self.buffer_out.clone().into_vec();
            self.buffer_out.clear();
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

pub struct HuffmanDecoder {
    tree: Option<HuffmanTree>,
    // there will never be a time we need to turn buffer_in into u8s, so best to keep at usize internal length
    buffer_in: BVs,
}

impl HuffmanDecoder {
    pub fn new() -> Self {
        HuffmanDecoder {
            tree: None,
            buffer_in: BVs::with_capacity(16),
        }
    }
}

impl Codec for HuffmanDecoder {
    fn process(&mut self, buffer: Vec<u8>) -> Result<Vec<u8>, CodecError> {
        self.buffer_in.append(&mut BVb::from_vec(buffer));
        let mut out: Vec<u8> = Vec::new();
        match &self.tree {
            Some(tree) => {
                let mut followed_path = BVb::new(); // the current path being followed; important for if buffer ends before we get to terminal tree node
                let mut current_position = &tree.root;
                while let Some(bit) = self.buffer_in.pop() { // navigate the tree
                    followed_path.push(bit);
                    match current_position {
                        HuffmanNode::Interior(left, right) => {
                            match bit {
                                false => current_position = left,
                                true => current_position = right,
                            }
                        },
                        HuffmanNode::Terminal(byte, _) => {
                            out.push(*byte);
                            current_position = &tree.root;
                            followed_path.clear();
                        },
                    }
                }
                self.buffer_in.append(&mut followed_path);
            }
            None => {
                // Check if it's possible to build the tree
                if self.buffer_in.len() >= 64 * 256 { // size of dictionary
                    // Get counts from incoming buffer
                    let counts_header = self.buffer_in[0..64*256].to_vec().into_vec();
                    self.buffer_in = self.buffer_in[64*256..self.buffer_in.len()].to_vec();
                    
                    // Build tree
                    let mut counts: [u64; 256] = [0; 256];
                    for i in 0..256 {
                        counts[i] = *counts_header.get(i).expect("huffman header missing counts");
                        // TODO: better workaround; see https://stackoverflow.com/questions/25428920/how-to-get-a-slice-as-an-array-in-rust
                    }
                    self.tree = Some(HuffmanBuilder::deserialize(counts).build());
                }
            }
        }
        Ok(out)
    }
}
