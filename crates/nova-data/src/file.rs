//! Decoding a whole resource file.

use nova_rsrc::{ResType, ResourceFile};

use crate::decode::Entry;
use crate::error::{DecodeError, DecodeWarning};
use crate::registry::{AnyRecord, decode_any};

/// Resource types [`decode_file`] skips: pictures and sprites (`PICT`,
/// `rlëD`, `cicn`, `ppat`), which [`crate::graphics`] decodes, and sounds
/// (`snd `), which [`crate::sound`] decodes.
pub const OUT_OF_SCOPE: &[ResType] = &[
    ResType::new(*b"PICT"),
    ResType::new([b'r', b'l', 0x91, b'D']),
    ResType::new(*b"cicn"),
    ResType::new(*b"ppat"),
    ResType::new(*b"snd "),
];

/// Everything found decoding one file.
#[derive(Clone, Debug, PartialEq)]
pub struct FileReport {
    /// Records that decoded, in map order.
    pub records: Vec<Entry<AnyRecord>>,
    /// Warnings about records that decoded.
    pub warnings: Vec<DecodeWarning>,
    /// Every resource that failed to decode, in map order.
    pub errors: Vec<DecodeError>,
    /// Types that are not decoded here (out of scope or unknown), with how
    /// many resources of each were skipped. Not errors.
    pub skipped: Vec<(ResType, usize)>,
}

/// Decodes every resource in `file`, in map order, collecting every failure
/// rather than stopping at the first.
#[must_use]
pub fn decode_file(file: &ResourceFile) -> FileReport {
    let mut report = FileReport {
        records: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        skipped: Vec::new(),
    };
    for ty in file.types() {
        let mut skipped = 0;
        for res in file.resources(ty) {
            match decode_any(&res) {
                Some(Ok((entry, warning))) => {
                    report.records.push(entry);
                    report.warnings.extend(warning);
                }
                Some(Err(err)) => report.errors.push(err),
                None => skipped += 1,
            }
        }
        if skipped > 0 {
            report.skipped.push((ty, skipped));
        }
    }
    report
}
