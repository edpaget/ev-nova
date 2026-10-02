//! `csüm`: a single 4-byte resource with no documented layout.
//!
//! Neither the EV Nova Bible nor the ResForge Nova templates describe
//! `csüm`. The stock data holds one 4-byte resource that looks like a
//! checksum rather than gameplay content, so it is kept as raw bytes.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::raw::RawBytes;

/// The undocumented `csüm` resource, as raw bytes (hex in JSON).
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Checksum {
    /// Every byte of the resource.
    pub data: RawBytes,
}

impl Record for Checksum {
    const TYPE: ResType = ResType::new([b'c', b's', 0x9F, b'm']);
    const SIZE: Option<usize> = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_bytes;

    #[test]
    fn keeps_every_byte() {
        let decoded = decode_bytes::<Checksum>(&[0x91, 0x84, 0xEE, 0xB0]).expect("decodes");
        assert_eq!(decoded.record.data, RawBytes(vec![0x91, 0x84, 0xEE, 0xB0]));
        assert_eq!(decoded.consumed, 4);
    }
}
