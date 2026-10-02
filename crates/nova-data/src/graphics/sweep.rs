//! A no-panic sweep over corrupted copies of a small valid resource.

use super::GraphicsError;

/// Decodes every proper prefix of `bytes` (each must fail) and every copy
/// with one byte set to `0x00`, `0x7F`, `0x80` or `0xFF` (each may succeed
/// or fail). Any panic, such as an arithmetic overflow in a debug build or
/// an out-of-range slice, fails the test.
pub(super) fn assert_never_panics<T>(bytes: &[u8], decode: fn(&[u8]) -> Result<T, GraphicsError>) {
    assert!(decode(bytes).is_ok(), "the unmodified resource decodes");
    for len in 0..bytes.len() {
        assert!(decode(&bytes[..len]).is_err(), "prefix of {len} bytes");
    }
    let mut copy = bytes.to_vec();
    for i in 0..bytes.len() {
        for value in [0x00, 0x7F, 0x80, 0xFF] {
            copy[i] = value;
            let _ = decode(&copy);
        }
        copy[i] = bytes[i];
    }
}
