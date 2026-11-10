//! The `spïn` builder.

use super::Bytes;
use crate::records::spin::Spin;

/// Builds a `spïn` ([`Spin`]) record's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpinBuilder(Bytes);

impl Default for SpinBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SpinBuilder {
    /// A zeroed `spïn`.
    #[must_use]
    pub fn new() -> Self {
        Self(Bytes::zeroed::<Spin>())
    }

    /// The `spïn` `bytes`, to change some of its fields.
    ///
    /// # Panics
    ///
    /// If `bytes` isn't a whole `spïn`.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(Bytes::of::<Spin>(bytes))
    }

    /// The record's bytes.
    #[must_use]
    pub fn build(self) -> Vec<u8> {
        self.0.0
    }

    i16_setters! { Spin:
        sprites_id @ 0x00,
        masks_id @ 0x02,
        x_size @ 0x04,
        y_size @ 0x06,
        x_tiles @ 0x08,
        y_tiles @ 0x0A,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Record;
    use crate::testutil::Buf;
    use crate::wire::id::PictId;

    fn spin(builder: SpinBuilder) -> Spin {
        Buf(builder.build()).decode()
    }

    #[test]
    fn a_new_spin_is_zeroed_and_full_size() {
        assert_eq!(
            SpinBuilder::new().build(),
            vec![0; Spin::SIZE.expect("fixed")]
        );
        assert_eq!(SpinBuilder::default(), SpinBuilder::new());
    }

    /// A setter's name, the setter, and the decoded field it sets.
    type SpinCase = (
        &'static str,
        fn(SpinBuilder, i16) -> SpinBuilder,
        fn(&Spin) -> i16,
    );

    #[test]
    fn every_spin_setter_writes_its_field() {
        let cases: [SpinCase; 6] = [
            ("sprites_id", SpinBuilder::sprites_id, |s| s.sprites_id),
            ("masks_id", SpinBuilder::masks_id, |s| {
                s.masks_id.map_or(0, |id| id.0)
            }),
            ("x_size", SpinBuilder::x_size, |s| s.x_size),
            ("y_size", SpinBuilder::y_size, |s| s.y_size),
            ("x_tiles", SpinBuilder::x_tiles, |s| s.x_tiles),
            ("y_tiles", SpinBuilder::y_tiles, |s| s.y_tiles),
        ];
        for (name, set, get) in cases {
            let decoded = spin(set(SpinBuilder::new(), 0x1234));
            assert_eq!(get(&decoded), 0x1234, "{name}");
        }
        assert_eq!(
            spin(SpinBuilder::new().masks_id(-1)).masks_id,
            None::<PictId>
        );
    }

    #[test]
    fn from_bytes_changes_only_the_fields_it_sets() {
        let base = SpinBuilder::new().sprites_id(1000).x_tiles(6).build();
        let changed = SpinBuilder::from_bytes(base).x_tiles(7).build();
        assert_eq!(
            changed,
            SpinBuilder::new().sprites_id(1000).x_tiles(7).build()
        );
    }

    #[test]
    #[should_panic(expected = "a record is")]
    fn from_bytes_wants_a_whole_spin() {
        let _ = SpinBuilder::from_bytes(vec![0; 3]);
    }
}
