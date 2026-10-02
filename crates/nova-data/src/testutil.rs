//! Test helpers: synthetic record buffers built by field offset.
//!
//! Offset tests write a value at a field's documented offset in an otherwise
//! zero buffer and check that the decoded field holds it. This is the
//! auditable link between each struct and the offsets in its doc comments.

use crate::decode::{Record, decode_bytes};

/// A deterministic, non-repeating byte pattern of `len` bytes.
pub fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + 3) as u8).collect()
}

/// A record buffer under construction.
pub struct Buf(pub Vec<u8>);

impl Buf {
    /// `len` zero bytes.
    pub fn zeroed(len: usize) -> Self {
        Self(vec![0; len])
    }

    /// Writes raw bytes at `offset`.
    pub fn bytes(mut self, offset: usize, bytes: &[u8]) -> Self {
        self.0[offset..offset + bytes.len()].copy_from_slice(bytes);
        self
    }

    /// Writes a big-endian `i16` at `offset`.
    pub fn i16(self, offset: usize, value: i16) -> Self {
        self.bytes(offset, &value.to_be_bytes())
    }

    /// Writes a big-endian `u16` at `offset`.
    pub fn u16(self, offset: usize, value: u16) -> Self {
        self.bytes(offset, &value.to_be_bytes())
    }

    /// Writes a big-endian `i32` at `offset`.
    pub fn i32(self, offset: usize, value: i32) -> Self {
        self.bytes(offset, &value.to_be_bytes())
    }

    /// Writes a big-endian `u64` at `offset`.
    pub fn u64(self, offset: usize, value: u64) -> Self {
        self.bytes(offset, &value.to_be_bytes())
    }

    /// Writes a big-endian `u32` at `offset`.
    pub fn u32(self, offset: usize, value: u32) -> Self {
        self.bytes(offset, &value.to_be_bytes())
    }

    /// Decodes the buffer, which must be used exactly.
    pub fn decode<T: Record>(&self) -> T {
        let decoded = decode_bytes::<T>(&self.0).expect("decodes");
        assert_eq!(decoded.consumed, self.0.len(), "layout uses every byte");
        decoded.record
    }
}

/// A fixed-size buffer for record `T`.
pub fn buf<T: Record>() -> Buf {
    Buf::zeroed(T::SIZE.expect("fixed-size record"))
}
