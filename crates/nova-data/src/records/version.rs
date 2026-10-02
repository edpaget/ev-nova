//! `vers`: Mac OS version information, which carries no gameplay content.
//!
//! Not part of Nova's data model (the Bible does not describe it) and absent
//! from the stock data; plug-ins may carry one. Kept as raw bytes.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::raw::RawBytes;

/// A `vers` resource, as raw bytes (hex in JSON).
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Version {
    /// Every byte of the resource.
    pub data: RawBytes,
}

impl Record for Version {
    const TYPE: ResType = ResType::new(*b"vers");
    const SIZE: Option<usize> = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_bytes;

    #[test]
    fn keeps_every_byte() {
        let decoded = decode_bytes::<Version>(&[1, 2, 0x80, 0]).expect("decodes");
        assert_eq!(decoded.record.data, RawBytes(vec![1, 2, 0x80, 0]));
    }
}
