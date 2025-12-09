use core::mem;

use alloc::vec;
use alloc::vec::Vec;

pub struct BitUnpacker<'inp> {
    bitlengths: Vec<u8>,
    input: &'inp [u8],
    // the size of the whole buffer after unpacking, it can be more than what we
    // have of data, the rest will be `0` values
    len: usize,
    addition: u8,
}

impl<'inp> BitUnpacker<'inp> {
    pub fn new_const_bits(bitlength: u8, input: &'inp [u8], size: usize) -> Self {
        Self {
            bitlengths: vec![bitlength; 1],
            input,
            len: size,
            addition: 0,
        }
    }

    pub fn new_var_bits(bitlengths: &[u8], input: &'inp [u8], size: usize) -> Self {
        Self {
            bitlengths: bitlengths.to_vec(),
            input,
            len: size,
            addition: 0,
        }
    }

    pub fn empty() -> Self {
        Self {
            bitlengths: vec![0],
            input: &[],
            len: 0,
            addition: 0,
        }
    }

    pub fn with_addition(mut self, addition: u8) -> Self {
        self.addition = addition;
        self
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> BitUnpackerIter<'_, '_> {
        BitUnpackerIter {
            parent: self,
            bits_stored: 0,
            bitlengths_index: 0,
            input_index: 0,
            value: 0,
            done: 0,
        }
    }
}

#[derive(Clone)]
pub(crate) struct BitUnpackerIter<'unpacker, 'inp> {
    parent: &'unpacker BitUnpacker<'inp>,
    bits_stored: u32,
    bitlengths_index: usize,
    input_index: usize,
    value: u32,
    done: usize,
}

impl<'unpacker, 'inp> BitUnpackerIter<'unpacker, 'inp> {
    const MASKS: [u32; 9] = [0, 1, 3, 7, 15, 31, 63, 127, 255];

    fn get_next_byte(&mut self) -> u8 {
        let bits = self.parent.bitlengths[self.bitlengths_index] as u32;
        let mask = Self::MASKS[bits as usize];

        while self.bits_stored < bits {
            let input_byte = if self.input_index < self.parent.input.len() {
                self.input_index += 1;
                self.parent.input[self.input_index - 1]
            } else {
                0
            };
            self.value = (self.value << 8) | (input_byte as u32);
            self.bits_stored += 8;
        }

        let item = (self.value >> (self.bits_stored - bits)) & mask;
        self.bits_stored -= bits;

        self.bitlengths_index += 1;
        if self.bitlengths_index == self.parent.bitlengths.len() {
            self.bitlengths_index = 0;
        }

        item as u8
    }
}

impl<'unpacker, 'inp> ExactSizeIterator for BitUnpackerIter<'unpacker, 'inp> {
    fn len(&self) -> usize {
        self.parent.len
    }
}

impl<'unpacker, 'inp> Iterator for BitUnpackerIter<'unpacker, 'inp> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done < self.len() {
            self.done += 1;
            Some(self.get_next_byte() + self.parent.addition)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.parent.len();
        (len, Some(len))
    }
}

pub struct BitPacker {
    accum: u32,
    bits_stored: u32,
    output: Vec<u8>,
}

impl BitPacker {
    pub fn new() -> Self {
        Self {
            accum: 0,
            bits_stored: 0,
            output: Vec::new(),
        }
    }

    pub fn push(&mut self, input: u32, bits: u8) {
        debug_assert!(bits <= 8);
        let mask: u32 = (1 << bits as u32) - 1;
        let value = (input) & mask;
        debug_assert!(
            input == value,
            "cannot pack value={} into {} bits",
            input,
            bits
        );
        self.accum = (self.accum << bits) | value;
        self.bits_stored += bits as u32;

        if self.bits_stored >= 8 {
            let value = self.accum >> (self.bits_stored - 8);
            self.output.push(value as u8);
            self.bits_stored -= 8;
            self.accum &= (1 << self.bits_stored) - 1;
        }
    }

    pub fn finish(&mut self) -> Vec<u8> {
        if self.bits_stored > 0 {
            let byte = (self.accum << (8 - self.bits_stored)) as u8;
            self.output.push(byte);
        }
        self.accum = 0;
        self.bits_stored = 0;

        mem::take(&mut self.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_bits() {
        // Test with multiple bitlengths
        let bitlengths = vec![3, 5, 2];
        // 3 bits: 0b101 (5)
        // 5 bits: 0b10010 (18)
        // 2 bits: 0b11 (3)
        // Repeat this pattern 2 times. Total items: 6.
        let values = vec![5, 18, 3, 5, 18, 3];

        let mut packer = BitPacker::new();
        for (i, val) in values.iter().enumerate() {
            packer.push(*val as u32, bitlengths[i % 3]);
        }
        let packed = packer.finish();

        let unpacker = BitUnpacker::new_var_bits(&bitlengths, &packed, values.len());
        let unpacked: Vec<u8> = unpacker.iter().collect();

        assert_eq!(unpacked, values);
    }

    #[test]
    fn test_constant_bits() {
        // Test with 1 bitlength (e.g. 3 bits)
        let bitlength = 3;
        let values = vec![0, 7, 2, 5, 1]; // 000, 111, 010, 101, 001

        let mut packer = BitPacker::new();
        for val in &values {
            packer.push(*val as u32, bitlength);
        }
        let packed = packer.finish();

        let unpacker = BitUnpacker::new_const_bits(bitlength, &packed, values.len());
        let unpacked: Vec<u8> = unpacker.iter().collect();

        assert_eq!(unpacked, values);
    }
}
