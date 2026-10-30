//! A synthetic MacBinary container around a fork, for tests.
//!
//! [`MacBinaryBuilder`] wraps resource-fork bytes (usually
//! [`ForkBuilder`](super::ForkBuilder) output) in a MacBinary I, II or III
//! file: the 128-byte header, an optional secondary header, the data fork
//! and the resource fork, each padded to a multiple of 128 bytes.
//! [`BuiltMacBinary`] holds the bytes and a [`MacBinaryLayout`], and its
//! helpers poke header fields; [`BuiltMacBinary::recompute_crc`] keeps a
//! corrupted II or III header's CRC valid.

/// Length of the MacBinary header.
pub const MACBINARY_HEADER_LEN: usize = 128;
/// Header field: the file name's length (a byte, 1 to 63).
pub const MACBINARY_NAME_LEN_FIELD: usize = 1;
/// Header field: the file name (63 bytes, Mac Roman).
pub const MACBINARY_NAME_FIELD: usize = 2;
/// Header field: the file type (four bytes).
pub const MACBINARY_FILE_TYPE_FIELD: usize = 65;
/// Header field: the data fork's length (big-endian `u32`).
pub const MACBINARY_DATA_LEN_FIELD: usize = 83;
/// Header field: the resource fork's length (big-endian `u32`).
pub const MACBINARY_RESOURCE_LEN_FIELD: usize = 87;
/// Header field: the MacBinary III signature, `mBIN`.
pub const MACBINARY_SIGNATURE_FIELD: usize = 102;
/// Header field: the secondary header's length (big-endian `u16`).
pub const MACBINARY_SECONDARY_HEADER_FIELD: usize = 120;
/// Header field: the version that wrote the file (129 for II, 130 for III).
pub const MACBINARY_VERSION_FIELD: usize = 122;
/// Header field: the minimum version needed to read the file (129).
pub const MACBINARY_MIN_VERSION_FIELD: usize = 123;
/// Header field: CRC-16/XMODEM of the first 124 bytes (big-endian `u16`).
pub const MACBINARY_CRC_FIELD: usize = 124;
/// The MacBinary III signature.
pub const MACBINARY_SIGNATURE: &[u8; 4] = b"mBIN";

/// Which MacBinary version a [`MacBinaryBuilder`] writes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum MacBinaryVersion {
    /// MacBinary I: no CRC, no signature, version bytes zero.
    I,
    /// MacBinary II: a CRC, version bytes 129 and 129.
    #[default]
    II,
    /// MacBinary III: a CRC, the `mBIN` signature, version bytes 130 and 129.
    III,
}

/// Builds a MacBinary file around a resource fork.
#[derive(Clone, Debug)]
pub struct MacBinaryBuilder {
    resource_fork: Vec<u8>,
    data_fork: Vec<u8>,
    name: Vec<u8>,
    version: MacBinaryVersion,
    secondary_header: u16,
}

impl MacBinaryBuilder {
    /// A MacBinary II file named `Plug-in` with `resource_fork` and an
    /// empty data fork.
    #[must_use]
    pub fn new(resource_fork: Vec<u8>) -> Self {
        Self {
            resource_fork,
            data_fork: Vec::new(),
            name: b"Plug-in".to_vec(),
            version: MacBinaryVersion::default(),
            secondary_header: 0,
        }
    }

    /// Chooses the version written.
    #[must_use]
    pub fn version(mut self, version: MacBinaryVersion) -> Self {
        self.version = version;
        self
    }

    /// Sets the data fork.
    #[must_use]
    pub fn data_fork(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.data_fork = bytes.into();
        self
    }

    /// Sets the file name (at most 63 bytes).
    #[must_use]
    pub fn name(mut self, name: &[u8]) -> Self {
        self.name = name.to_vec();
        self
    }

    /// Declares a secondary header of `len` bytes, written as zeros rounded
    /// up to a multiple of 128.
    #[must_use]
    pub fn secondary_header(mut self, len: u16) -> Self {
        self.secondary_header = len;
        self
    }

    /// Writes the file.
    #[must_use]
    pub fn build(&self) -> BuiltMacBinary {
        let mut bytes = vec![0; MACBINARY_HEADER_LEN];
        bytes[MACBINARY_NAME_LEN_FIELD] = u8::try_from(self.name.len()).expect("name fits");
        super::put(&mut bytes, MACBINARY_NAME_FIELD, &self.name);
        super::put(&mut bytes, MACBINARY_FILE_TYPE_FIELD, b"NplgNovA");
        super::put(
            &mut bytes,
            MACBINARY_DATA_LEN_FIELD,
            &super::u32_of(self.data_fork.len()).to_be_bytes(),
        );
        super::put(
            &mut bytes,
            MACBINARY_RESOURCE_LEN_FIELD,
            &super::u32_of(self.resource_fork.len()).to_be_bytes(),
        );
        super::put(
            &mut bytes,
            MACBINARY_SECONDARY_HEADER_FIELD,
            &self.secondary_header.to_be_bytes(),
        );
        let (version, signed) = match self.version {
            MacBinaryVersion::I => (None, false),
            MacBinaryVersion::II => (Some(129), false),
            MacBinaryVersion::III => (Some(130), true),
        };
        if signed {
            super::put(&mut bytes, MACBINARY_SIGNATURE_FIELD, MACBINARY_SIGNATURE);
        }

        bytes.resize(padded(bytes.len() + usize::from(self.secondary_header)), 0);
        let data_offset = bytes.len();
        bytes.extend(&self.data_fork);
        bytes.resize(padded(bytes.len()), 0);
        let resource_offset = bytes.len();
        bytes.extend(&self.resource_fork);
        let resource_end = bytes.len();
        bytes.resize(padded(bytes.len()), 0);

        let mut built = BuiltMacBinary {
            bytes,
            layout: MacBinaryLayout {
                data_offset,
                resource_offset,
                resource_end,
            },
        };
        if let Some(version) = version {
            built
                .put_u8(MACBINARY_VERSION_FIELD, version)
                .put_u8(MACBINARY_MIN_VERSION_FIELD, 129)
                .recompute_crc();
        }
        built
    }
}

/// Where a built MacBinary file's forks landed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MacBinaryLayout {
    /// Start of the data fork.
    pub data_offset: usize,
    /// Start of the resource fork.
    pub resource_offset: usize,
    /// End of the resource fork, before its padding.
    pub resource_end: usize,
}

/// A built MacBinary file and the positions of its forks.
#[derive(Clone, Debug)]
pub struct BuiltMacBinary {
    /// The whole file.
    pub bytes: Vec<u8>,
    /// Where each fork was written.
    pub layout: MacBinaryLayout,
}

impl BuiltMacBinary {
    /// Overwrites the byte at `pos`.
    pub fn put_u8(&mut self, pos: usize, value: u8) -> &mut Self {
        self.bytes[pos] = value;
        self
    }

    /// Overwrites a big-endian `u16` at `pos`.
    pub fn put_u16(&mut self, pos: usize, value: u16) -> &mut Self {
        self.put(pos, &value.to_be_bytes())
    }

    /// Overwrites a big-endian `u32` at `pos`.
    pub fn put_u32(&mut self, pos: usize, value: u32) -> &mut Self {
        self.put(pos, &value.to_be_bytes())
    }

    /// Overwrites `value.len()` bytes at `pos`.
    pub fn put(&mut self, pos: usize, value: &[u8]) -> &mut Self {
        super::put(&mut self.bytes, pos, value);
        self
    }

    /// Cuts the file down to `len` bytes.
    pub fn truncate(&mut self, len: usize) -> &mut Self {
        self.bytes.truncate(len);
        self
    }

    /// Rewrites the header CRC to match the header's first 124 bytes, so a
    /// corruption test changes only the field it means to.
    pub fn recompute_crc(&mut self) -> &mut Self {
        let crc = crate::macbin::crc16_xmodem(&self.bytes[..MACBINARY_CRC_FIELD]);
        self.put_u16(MACBINARY_CRC_FIELD, crc)
    }
}

/// `len` rounded up to a multiple of 128.
fn padded(len: usize) -> usize {
    len.next_multiple_of(MACBINARY_HEADER_LEN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResType;
    use crate::fixture::ForkBuilder;

    fn fork() -> Vec<u8> {
        ForkBuilder::new()
            .resource(ResType(*b"PICT"), 128, Some(b"Pic"), b"pixels")
            .build()
            .bytes
    }

    fn u16_at(bytes: &[u8], pos: usize) -> u16 {
        u16::from_be_bytes([bytes[pos], bytes[pos + 1]])
    }

    fn u32_at(bytes: &[u8], pos: usize) -> u32 {
        u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
    }

    #[test]
    fn every_version_parses_with_macbinary() {
        for (version, expected) in [
            (MacBinaryVersion::I, macbinary::Version::I),
            (MacBinaryVersion::II, macbinary::Version::II),
            (MacBinaryVersion::III, macbinary::Version::III),
        ] {
            let built = MacBinaryBuilder::new(fork())
                .version(version)
                .data_fork(b"hello".to_vec())
                .build();
            let parsed = macbinary::parse(&built.bytes).expect("macbinary parses");
            assert_eq!(parsed.version(), expected, "{version:?}");
            assert_eq!(parsed.data_fork(), b"hello", "{version:?}");
            assert_eq!(parsed.resource_fork_raw(), fork(), "{version:?}");
            assert_eq!(parsed.filename_bytes(), b"Plug-in", "{version:?}");
        }
    }

    #[test]
    fn ii_and_iii_differ_only_in_signature_version_and_crc() {
        let ii = MacBinaryBuilder::new(fork()).build().bytes;
        let iii = MacBinaryBuilder::new(fork())
            .version(MacBinaryVersion::III)
            .build()
            .bytes;
        let differ: Vec<usize> = (0..ii.len()).filter(|&i| ii[i] != iii[i]).collect();
        let allowed = [
            MACBINARY_SIGNATURE_FIELD..MACBINARY_SIGNATURE_FIELD + 4,
            MACBINARY_VERSION_FIELD..MACBINARY_VERSION_FIELD + 1,
            MACBINARY_CRC_FIELD..MACBINARY_CRC_FIELD + 2,
        ];
        assert!(!differ.is_empty());
        assert!(
            differ.iter().all(|i| allowed.iter().any(|r| r.contains(i))),
            "{differ:?}"
        );
        assert_eq!(&iii[MACBINARY_SIGNATURE_FIELD..][..4], MACBINARY_SIGNATURE);
        assert_eq!(iii[MACBINARY_VERSION_FIELD], 130);
        assert_eq!(ii[MACBINARY_VERSION_FIELD], 129);
        assert_eq!(ii[MACBINARY_MIN_VERSION_FIELD], 129);
    }

    #[test]
    fn version_i_has_no_crc_and_no_version_bytes() {
        let i = MacBinaryBuilder::new(fork())
            .version(MacBinaryVersion::I)
            .build()
            .bytes;
        assert!(i[101..=125].iter().all(|&b| b == 0));
    }

    #[test]
    fn header_fields_and_layout_place_padded_forks() {
        let resource = fork();
        let built = MacBinaryBuilder::new(resource.clone())
            .data_fork(vec![7; 5])
            .name(b"Override")
            .secondary_header(200)
            .build();
        let bytes = &built.bytes;
        assert_eq!(bytes[0], 0);
        assert_eq!(bytes[MACBINARY_NAME_LEN_FIELD], 8);
        assert_eq!(&bytes[MACBINARY_NAME_FIELD..][..8], b"Override");
        assert_eq!(u32_at(bytes, MACBINARY_DATA_LEN_FIELD), 5);
        assert_eq!(
            u32_at(bytes, MACBINARY_RESOURCE_LEN_FIELD) as usize,
            resource.len()
        );
        assert_eq!(u16_at(bytes, MACBINARY_SECONDARY_HEADER_FIELD), 200);
        assert_eq!(
            built.layout,
            MacBinaryLayout {
                data_offset: 128 + 256,
                resource_offset: 128 + 256 + 128,
                resource_end: 128 + 256 + 128 + resource.len(),
            }
        );
        assert_eq!(&bytes[384..389], &[7; 5]);
        assert_eq!(&bytes[512..512 + resource.len()], resource.as_slice());
        assert_eq!(bytes.len() % 128, 0, "the resource fork is padded");
        assert!(bytes[built.layout.resource_end..].iter().all(|&b| b == 0));
    }

    #[test]
    fn helpers_poke_fields_and_recompute_the_crc() {
        let mut built = MacBinaryBuilder::new(fork()).build();
        built
            .put_u8(MACBINARY_NAME_FIELD, b'X')
            .put_u16(MACBINARY_SECONDARY_HEADER_FIELD, 0x1234)
            .put_u32(MACBINARY_DATA_LEN_FIELD, 0x0102_0304);
        assert_eq!(built.bytes[MACBINARY_NAME_FIELD], b'X');
        assert_eq!(
            u16_at(&built.bytes, MACBINARY_SECONDARY_HEADER_FIELD),
            0x1234
        );
        assert_eq!(u32_at(&built.bytes, MACBINARY_DATA_LEN_FIELD), 0x0102_0304);
        assert!(macbinary::detect(&built.bytes).is_none(), "CRC now stale");

        built.put_u32(MACBINARY_DATA_LEN_FIELD, 0);
        built.put_u16(MACBINARY_SECONDARY_HEADER_FIELD, 0);
        built.recompute_crc();
        assert_eq!(
            macbinary::detect(&built.bytes),
            Some(macbinary::Version::II)
        );
        built.truncate(100);
        assert_eq!(built.bytes.len(), 100);
    }
}
