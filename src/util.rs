use std::collections::VecDeque;
use std::iter::FromIterator;

fn bytes_to_bools(bytes: &[u8]) -> Vec<bool> {
    let mut vec = Vec::with_capacity(bytes.len());
    for byte in bytes {
        for i in 0..8 {
            // Internally, we use big endian when converting
            // bytes -- that is, we start with the most
            // significant bit.
            vec.push(byte & (0b10000000u8 >> i) != 0);
        }
    }
    vec
}

// will return padding of 0 if necessary
fn bools_to_bytes(bools: &[bool]) -> Vec<u8> {
    let len = bools.len();
    let mut vec = Vec::with_capacity(len / 8);
    let mut current: u8 = 0;
    for i in 0..len {
        if bools[i] {
            current |= 0b10000000u8 >> (i % 8);
        }
        if i % 8 == 7 || i == len - 1 {
            vec.push(current);
            current = 0;
        }
    }
    vec
}

pub struct BitQueue {
    q: VecDeque<bool>,
}

impl BitQueue {
    pub fn new() -> Self {
        BitQueue { q: VecDeque::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        BitQueue {
            q: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push_front(&mut self, item: bool) {
        self.q.push_front(item);
    }

    pub fn push_back(&mut self, item: bool) {
        self.q.push_back(item);
    }

    pub fn pop_front(&mut self) -> Option<bool> {
        self.q.pop_front()
    }

    pub fn pop_back(&mut self) -> Option<bool> {
        self.q.pop_back()
    }

    pub fn push_back_u8s(&mut self, bytes: &[u8]) {
        let bits = bytes_to_bools(bytes);
        for bit in bits.iter().rev() {
            self.push_back(*bit);
        }
    }

    pub fn clear(&mut self) {
        self.q.clear();
    }

    pub fn len(&self) -> usize {
        self.q.len()
    }

    pub fn to_u8s(&self) -> Vec<u8> {
        self.partial_to_u8s(self.len() / 8 + 1)
    }

    pub fn partial_to_u8s(&self, mut num_bytes: usize) -> Vec<u8> {
        if num_bytes > self.len() / 8 {
            num_bytes = self.len() / 8;
        }

        if num_bytes == 0 {
            return Vec::new();
        }

        bools_to_bytes(&self.as_vec().as_slice()[0..num_bytes * 8])
    }

    pub fn as_vec(&self) -> Vec<bool> {
        let mut vec: Vec<bool> = Vec::from_iter(self.q.iter().map(|x| *x));
        vec.reverse();
        vec
    }

    pub fn from_vec(vec: Vec<bool>) -> Self {
        Self {
            q: VecDeque::from(vec),
        }
    }

    /// Extends the back of the queue from an iterator
    pub fn extend<T: IntoIterator<Item = bool>>(&mut self, iter: T) {
        self.q.extend(iter)
    }

    /// Extends the back of the queue with another BitQueue,
    /// leaving the other BitQueue empty.
    pub fn append(&mut self, other: &mut Self) {
        self.q.append(&mut other.q);
    }

    /// Extends the back of the queue with another BitQueue,
    /// without changing the other BitQueue.
    pub fn copy_from(&mut self, other: &Self) {
        for item in other.as_vec() {
            self.push_back(item);
        }
    }
}

impl From<&[u8]> for BitQueue {
    fn from(bytes: &[u8]) -> Self {
        Self::from_vec(bytes_to_bools(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::{bools_to_bytes, bytes_to_bools, BitQueue};

    #[test]
    fn push_pop_front() {
        let mut bq = BitQueue::new();
        assert!(bq.pop_front() == None);
        bq.push_front(true);
        bq.push_front(true);
        bq.push_front(false);
        assert!(bq.pop_front() == Some(false));
        assert!(bq.pop_front() == Some(true));
        assert!(bq.pop_front() == Some(true));
        assert!(bq.pop_front() == None);
    }

    #[test]
    fn push_pop_back() {
        let mut bq = BitQueue::new();
        assert!(bq.pop_back() == None);
        bq.push_back(true);
        bq.push_back(true);
        bq.push_back(false);
        assert!(bq.pop_back() == Some(false));
        assert!(bq.pop_back() == Some(true));
        assert!(bq.pop_back() == Some(true));
        assert!(bq.pop_back() == None);
    }

    #[test]
    fn from_u8s() {
        let slice: &[u8] = &[0b11111111u8, 0b10101010u8, 0b00000000u8];
        let mut bq = BitQueue::from(slice);
        println!("bitqueue: {:?}", bq.as_vec());
        println!("internal: {:?}", bytes_to_bools(slice));
        assert!(bq.len() == 24);
        assert!(bq.pop_back() == Some(true));
        assert!(bq.pop_back() == Some(true));
        assert!(bq.pop_front() == Some(false));
        assert!(bq.pop_front() == Some(false));
        assert!(bq.len() == 20);
    }

    #[test]
    fn to_u8s() {
        let slice: &[u8] = &[0b11111111u8, 0b10101010u8, 0b00000000u8];
        let bq = BitQueue::from(slice);
        println!("bitqueue: {:?}", bq.to_u8s().as_slice());
        println!("internal: {:?}", slice);
        let vec_out = bq.to_u8s();
        for i in 0..slice.len() {
            assert_eq!(*vec_out.get(i).unwrap(), slice[i]);
        }
    }

    #[test]
    fn bools_to_u8s() {
        assert_eq!(
            bools_to_bytes(&[false, true, true, false, true, true, true, true]),
            &[0b01101111u8]
        );
        assert_eq!(bools_to_bytes(&[false, true, true, false]), &[0b01100000u8]);
        assert_eq!(
            bools_to_bytes(&[false, true, true, false, true, true, true, true, false]),
            &[0b01101111u8, 0b00000000]
        );
        assert_eq!(
            bools_to_bytes(&[
                true, true, true, true, true, true, true, true, true, false, true, false, true,
                false, true, false, false, false, false, false, false, false, false, false
            ]),
            &[255, 170, 0]
        );
    }

    #[test]
    fn u8s_to_bools() {
        assert_eq!(
            bytes_to_bools(&[0b01011010u8, 0b01001000u8]),
            vec![
                false, true, false, true, true, false, true, false, false, true, false, false,
                true, false, false, false
            ]
        );
        assert_eq!(
            bytes_to_bools(&[0]),
            vec![false, false, false, false, false, false, false, false]
        );
    }
}
