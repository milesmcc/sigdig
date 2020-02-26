//! Simple Huffman coding compression

use crate::util::BitQueue;
use crate::{Codec, CodecError};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::iter::FromIterator;

struct HuffmanBuilder {
    counts: [u64; 256],
}

impl HuffmanBuilder {
    pub fn new() -> Self {
        HuffmanBuilder { counts: [0; 256] }
    }

    pub fn serialize(&self) -> [u64; 256] {
        self.counts
    }

    pub fn deserialize(counts: [u64; 256]) -> Self {
        HuffmanBuilder { counts: counts }
    }

    pub fn push(&mut self, byte: u8) {
        self.counts[byte as usize] += 1;
    }

    pub fn build<'a>(&mut self) -> HuffmanTree {
        let mut index = Vec::from_iter((0..=255).map(|_| BitQueue::with_capacity(32)));
        let mut heap: BinaryHeap<Reverse<HuffmanNode>> = BinaryHeap::from_iter(
            (0..=255).map(|i| Reverse(HuffmanNode::Terminal(i, self.counts[i as usize]))),
        );

        // Go through the smallest items in the heap and continuously merge them until
        // there is only one item in the heap -- the root.
        while heap.len() >= 2 {
            let left = heap.pop().unwrap().0; // should never panic
            let right = heap.pop().unwrap().0;

            for val in left.values() {
                index.get_mut(val as usize).unwrap().push_back(false);
            }

            for val in right.values() {
                index.get_mut(val as usize).unwrap().push_back(true);
            }

            let new_parent = HuffmanNode::Interior(Box::from(left), Box::from(right));
            heap.push(Reverse(new_parent));
        }

        // for i in 0..256 {
        //     error!("{}: {:?}", i, index.get(i));
        // }

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
    index: Vec<BitQueue>,
}

impl HuffmanTree {
    pub fn encode(&self, byte: u8) -> Option<&BitQueue> {
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
    buffer_in: VecDeque<u8>,
    buffer_out: BitQueue,
    build_tree_after: u64,
}

impl HuffmanEncoder {
    pub fn new(build_tree_after: u64) -> Self {
        HuffmanEncoder {
            tree: None,
            builder: HuffmanBuilder::new(),
            buffer_in: VecDeque::new(),
            buffer_out: BitQueue::with_capacity(32768),
            build_tree_after: build_tree_after,
        }
    }

    fn process_buffer(&mut self, force_push: bool) -> Result<Vec<u8>, CodecError> {
        // Process input buffer
        match &mut self.tree {
            Some(tree) => {
                while let Some(byte) = self.buffer_in.pop_front() {
                    self.buffer_out.copy_from(&tree.encode(byte).unwrap()); // TODO: remove unwrap
                }
            }
            None => {
                if force_push || self.buffer_in.len() as u64 > self.build_tree_after {
                    self.tree = Some(self.builder.build());
                    let mut byte_vec: Vec<u8> = Vec::new();
                    for component in self.builder.serialize().iter() {
                        byte_vec.extend(&component.to_be_bytes());
                    }
                    let tree_serialized = BitQueue::from(byte_vec.as_slice());
                    self.buffer_out.copy_from(&tree_serialized);
                    self.process_buffer(false)?;
                }
            }
        }

        // Process output buffer
        if self.buffer_out.len() % 8 == 0 || force_push {
            if force_push {
                error!("force pushing {} bits", self.buffer_out.len());
            }
            // Force align not strictly necessary here, because we haven't done
            // any slicing.
            let mut out: Vec<u8> = self.buffer_out.to_u8s();
            out.reverse();
            self.buffer_out.clear();
            Ok(out)
        } else {
            Ok(Vec::new())
        }
    }
}

impl Codec for HuffmanEncoder {
    fn process(&mut self, buffer: Vec<u8>) -> Result<Vec<u8>, CodecError> {
        if self.tree.is_none() {
            for byte in &buffer {
                self.builder.push(*byte);
            }
        }
        // error!("{:?}", buffer);
        self.buffer_in.extend(buffer.iter());
        self.process_buffer(false)
    }

    fn flush(&mut self) -> Result<Vec<u8>, CodecError> {
        self.process_buffer(true)
    }
}

pub struct HuffmanDecoder {
    tree: Option<HuffmanTree>,
    // there will never be a time we need to turn buffer_in into u8s, so best to keep at usize internal length
    buffer_in: BitQueue,
}

impl HuffmanDecoder {
    pub fn new() -> Self {
        HuffmanDecoder {
            tree: None,
            buffer_in: BitQueue::with_capacity(32),
        }
    }
}

impl Codec for HuffmanDecoder {
    fn process(&mut self, buffer: Vec<u8>) -> Result<Vec<u8>, CodecError> {
        self.buffer_in.push_back_u8s(buffer.as_slice());
        let mut out: Vec<u8> = Vec::new();
        match &self.tree {
            Some(tree) => {
                let mut followed_path = BitQueue::with_capacity(64); // the current path being followed; important for if buffer ends before we get to terminal tree node
                let mut current_position = &tree.root;
                'tree: loop {
                    // navigate the tree
                    match current_position {
                        HuffmanNode::Interior(left, right) => {
                            let bit = match self.buffer_in.pop_front() {
                                None => break 'tree,
                                Some(bit) => bit,
                            };
                            followed_path.push_back(bit);
                            match bit {
                                false => current_position = left,
                                true => current_position = right,
                            }
                        }
                        HuffmanNode::Terminal(byte, _) => {
                            out.push(*byte);
                            current_position = &tree.root;
                            followed_path.clear();
                        }
                    }
                }
                self.buffer_in.append(&mut followed_path);
            }
            None => {
                // Check if it's possible to build the tree
                if self.buffer_in.len() >= 64 * 256 {
                    // size of dictionary
                    // Get counts from incoming buffer
                    let counts_header = self.buffer_in.partial_to_u8s(8 * 256); // 4 because we're counting bytes
                    self.buffer_in = BitQueue::from_vec(Vec::from(
                        &self.buffer_in.as_vec().as_slice()[64 * 256..self.buffer_in.len()], // TODO: can be optimized...
                    ));
                    // Build tree
                    let mut counts: [u64; 256] = [0; 256];
                    for i in 0..256 {
                        let components = &counts_header.as_slice()[i * 8..i * 8 + 8];
                        let mut val: u64 = 0;
                        for j in 0..8 {
                            val |= (components[j] as u64) << (7 - j);
                        }
                        counts[i] = val;
                        // TODO: better workaround; see https://stackoverflow.com/questions/25428920/how-to-get-a-slice-as-an-array-in-rust
                    }
                    self.tree = Some(HuffmanBuilder::deserialize(counts).build());
                }
            }
        }
        Ok(out)
    }
}
