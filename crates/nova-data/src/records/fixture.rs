//! Byte builders for the records tests hand-build most: `shän` and `shïp`.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. Each builder starts from a zeroed record and writes
//! big-endian fields by name, so a test says which field it sets instead of
//! where the field lives. The setters are named after the decoded record's
//! fields ([`ShipAnim`], [`Ship`]), whose docs give each offset; this
//! module's tests decode every setter's bytes to check them. A setter is
//! added only when a test needs it.

use crate::decode::Record;
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;

/// Writes big-endian values into a zeroed record.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bytes(Vec<u8>);

impl Bytes {
    fn zeroed<T: Record>() -> Self {
        Self(vec![0; T::SIZE.expect("a fixed-size record")])
    }

    fn put(&mut self, at: usize, value: &[u8]) {
        self.0[at..at + value.len()].copy_from_slice(value);
    }

    fn i16(&mut self, at: usize, value: i16) {
        self.put(at, &value.to_be_bytes());
    }

    /// Writes `values` from slot 0: slots 0 to 3 at `low`, 4 to 7 at
    /// `high`.
    fn slots(&mut self, low: usize, high: usize, values: &[i16]) {
        assert!(
            values.len() <= 8,
            "a record has 8 slots, not {}",
            values.len()
        );
        for (slot, &value) in values.iter().enumerate() {
            let base = if slot < 4 { low } else { high };
            self.i16(base + 2 * (slot % 4), value);
        }
    }

    /// Writes `text`'s bytes at `at`, in a C-string field `len` long.
    fn text(&mut self, at: usize, len: usize, text: &str) {
        assert!(text.len() < len, "{text:?} doesn't fit in {len} bytes");
        self.put(at, text.as_bytes());
    }
}

/// Defines setters of `i16` fields at fixed offsets.
macro_rules! i16_setters {
    ($record:ident: $($name:ident @ $offset:literal),* $(,)?) => {
        $(
            #[doc = concat!(
                "Sets [`", stringify!($record), "::", stringify!($name),
                "`] (offset ", stringify!($offset), ")."
            )]
            #[must_use]
            pub fn $name(mut self, value: i16) -> Self {
                self.0.i16($offset, value);
                self
            }
        )*
    };
}

/// Builds a `shän` ([`ShipAnim`]) record's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipAnimBuilder(Bytes);

impl Default for ShipAnimBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ShipAnimBuilder {
    /// A zeroed `shän`.
    #[must_use]
    pub fn new() -> Self {
        Self(Bytes::zeroed::<ShipAnim>())
    }

    /// The record's bytes.
    #[must_use]
    pub fn build(self) -> Vec<u8> {
        self.0.0
    }

    i16_setters! { ShipAnim:
        base_image_id @ 0x00,
        base_mask_id @ 0x02,
        base_set_count @ 0x04,
        base_x_size @ 0x06,
        base_y_size @ 0x08,
        glow_image_id @ 0x16,
        light_image_id @ 0x1E,
        frames_per @ 0x34,
        blink_mode @ 0x36,
        blink_val_a @ 0x38,
        blink_val_b @ 0x3A,
        blink_val_c @ 0x3C,
        blink_val_d @ 0x3E,
    }
}

/// Builds a `shïp` ([`Ship`]) record's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipBuilder(Bytes);

impl Default for ShipBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ShipBuilder {
    /// A zeroed `shïp`.
    #[must_use]
    pub fn new() -> Self {
        Self(Bytes::zeroed::<Ship>())
    }

    /// The record's bytes.
    #[must_use]
    pub fn build(self) -> Vec<u8> {
        self.0.0
    }

    i16_setters! { Ship:
        holds @ 0x00,
        shield @ 0x02,
        accel @ 0x04,
        speed @ 0x06,
        maneuver @ 0x08,
        fuel @ 0x0A,
        free_mass @ 0x0C,
        armor @ 0x0E,
        tech_level @ 0x2E,
        death_delay @ 0x34,
        explode1 @ 0x38,
        explode2 @ 0x3A,
        mass @ 0x3E,
        inherent_ai @ 0x42,
        crew @ 0x44,
        fuel_regen @ 0x5E,
        buy_random @ 0x388,
        hire_random @ 0x38A,
    }

    /// Sets [`Ship::cost`] (offset 0x30, i32).
    #[must_use]
    pub fn cost(mut self, value: i32) -> Self {
        self.0.put(0x30, &value.to_be_bytes());
        self
    }

    /// Sets [`Ship::flags`] (offset 0x4A, u16).
    #[must_use]
    pub fn flags(mut self, value: u16) -> Self {
        self.0.put(0x4A, &value.to_be_bytes());
        self
    }

    /// Sets [`Ship::require`] (offset 0x380, u64).
    #[must_use]
    pub fn require(mut self, value: u64) -> Self {
        self.0.put(0x380, &value.to_be_bytes());
        self
    }

    /// Sets the weapon types from slot 0: [`Ship::weap_type1_4`] (offset
    /// 0x12) then [`Ship::weap_type5_8`] (offset 0x6CE). Slots past
    /// `types` keep what they held.
    #[must_use]
    pub fn weap_types(mut self, types: &[i16]) -> Self {
        self.0.slots(0x12, 0x6CE, types);
        self
    }

    /// Sets the weapon counts from slot 0: [`Ship::weap_count1_4`] (offset
    /// 0x1A) then [`Ship::weap_count5_8`] (offset 0x6D6).
    #[must_use]
    pub fn weap_counts(mut self, counts: &[i16]) -> Self {
        self.0.slots(0x1A, 0x6D6, counts);
        self
    }

    /// Sets the default items from slot 0: [`Ship::default_items1_4`]
    /// (offset 0x4E) then [`Ship::default_items5_8`] (offset 0x370).
    #[must_use]
    pub fn default_items(mut self, items: &[i16]) -> Self {
        self.0.slots(0x4E, 0x370, items);
        self
    }

    /// Sets the default items' counts from slot 0: [`Ship::item_count1_4`]
    /// (offset 0x56) then [`Ship::item_count5_8`] (offset 0x378).
    #[must_use]
    pub fn item_counts(mut self, counts: &[i16]) -> Self {
        self.0.slots(0x56, 0x378, counts);
        self
    }

    /// Sets [`Ship::short_name`] (offset 0x5CE, 64 bytes).
    #[must_use]
    pub fn short_name(mut self, name: &str) -> Self {
        self.0.text(0x5CE, 64, name);
        self
    }

    /// Sets [`Ship::comm_name`] (offset 0x60E, 32 bytes).
    #[must_use]
    pub fn comm_name(mut self, name: &str) -> Self {
        self.0.text(0x60E, 32, name);
        self
    }

    /// Sets [`Ship::subtitle`] (offset 0x6E6, 64 bytes).
    #[must_use]
    pub fn subtitle(mut self, text: &str) -> Self {
        self.0.text(0x6E6, 64, text);
        self
    }

    /// Sets [`Ship::long_name`] (offset 0x62E, 128 bytes).
    #[must_use]
    pub fn long_name(mut self, name: &str) -> Self {
        self.0.text(0x62E, 128, name);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::Buf;
    use crate::wire::id::{OutfitId, WeaponId};

    fn anim(builder: ShipAnimBuilder) -> ShipAnim {
        Buf(builder.build()).decode()
    }

    fn ship(builder: ShipBuilder) -> Ship {
        Buf(builder.build()).decode()
    }

    #[test]
    fn a_new_record_is_zeroed_and_full_size() {
        assert_eq!(
            ShipAnimBuilder::new().build(),
            vec![0; ShipAnim::SIZE.expect("fixed")]
        );
        assert_eq!(
            ShipBuilder::new().build(),
            vec![0; Ship::SIZE.expect("fixed")]
        );
        assert_eq!(ShipAnimBuilder::default(), ShipAnimBuilder::new());
        assert_eq!(ShipBuilder::default(), ShipBuilder::new());
    }

    /// A setter's name, the setter, and the decoded field it sets.
    type AnimCase = (
        &'static str,
        fn(ShipAnimBuilder, i16) -> ShipAnimBuilder,
        fn(&ShipAnim) -> i16,
    );

    #[test]
    fn every_shan_setter_writes_its_field() {
        let cases: [AnimCase; 13] = [
            ("base_image_id", ShipAnimBuilder::base_image_id, |a| {
                a.base_image_id
            }),
            ("base_mask_id", ShipAnimBuilder::base_mask_id, |a| {
                a.base_mask_id
            }),
            ("base_set_count", ShipAnimBuilder::base_set_count, |a| {
                a.base_set_count
            }),
            ("base_x_size", ShipAnimBuilder::base_x_size, |a| {
                a.base_x_size
            }),
            ("base_y_size", ShipAnimBuilder::base_y_size, |a| {
                a.base_y_size
            }),
            ("glow_image_id", ShipAnimBuilder::glow_image_id, |a| {
                a.glow_image_id
            }),
            ("light_image_id", ShipAnimBuilder::light_image_id, |a| {
                a.light_image_id
            }),
            ("frames_per", ShipAnimBuilder::frames_per, |a| a.frames_per),
            ("blink_mode", ShipAnimBuilder::blink_mode, |a| a.blink_mode),
            ("blink_val_a", ShipAnimBuilder::blink_val_a, |a| {
                a.blink_val_a
            }),
            ("blink_val_b", ShipAnimBuilder::blink_val_b, |a| {
                a.blink_val_b
            }),
            ("blink_val_c", ShipAnimBuilder::blink_val_c, |a| {
                a.blink_val_c
            }),
            ("blink_val_d", ShipAnimBuilder::blink_val_d, |a| {
                a.blink_val_d
            }),
        ];
        for (name, set, get) in cases {
            let decoded = anim(set(ShipAnimBuilder::new(), 0x1234));
            assert_eq!(get(&decoded), 0x1234, "{name}");
        }
    }

    #[test]
    fn shan_setters_write_only_their_own_field() {
        let built = ShipAnimBuilder::new().frames_per(36).build();
        let set: Vec<usize> = (0..built.len()).filter(|&at| built[at] != 0).collect();
        assert_eq!(set, [0x35]);
    }

    /// A setter's name, the setter, and the decoded field it sets.
    type ShipCase = (
        &'static str,
        fn(ShipBuilder, i16) -> ShipBuilder,
        fn(&Ship) -> i16,
    );

    #[test]
    fn every_i16_ship_setter_writes_its_field() {
        let cases: [ShipCase; 18] = [
            ("holds", ShipBuilder::holds, |s| s.holds),
            ("shield", ShipBuilder::shield, |s| s.shield),
            ("accel", ShipBuilder::accel, |s| s.accel),
            ("speed", ShipBuilder::speed, |s| s.speed),
            ("maneuver", ShipBuilder::maneuver, |s| s.maneuver),
            ("fuel", ShipBuilder::fuel, |s| s.fuel),
            ("free_mass", ShipBuilder::free_mass, |s| s.free_mass),
            ("armor", ShipBuilder::armor, |s| s.armor),
            ("tech_level", ShipBuilder::tech_level, |s| s.tech_level),
            ("death_delay", ShipBuilder::death_delay, |s| s.death_delay),
            ("explode1", ShipBuilder::explode1, |s| s.explode1),
            ("explode2", ShipBuilder::explode2, |s| s.explode2),
            ("mass", ShipBuilder::mass, |s| s.mass),
            ("inherent_ai", ShipBuilder::inherent_ai, |s| s.inherent_ai),
            ("crew", ShipBuilder::crew, |s| s.crew),
            ("fuel_regen", ShipBuilder::fuel_regen, |s| s.fuel_regen),
            ("buy_random", ShipBuilder::buy_random, |s| s.buy_random),
            ("hire_random", ShipBuilder::hire_random, |s| s.hire_random),
        ];
        for (name, set, get) in cases {
            assert_eq!(
                get(&ship(set(ShipBuilder::new(), 0x1234))),
                0x1234,
                "{name}"
            );
        }
    }

    #[test]
    fn the_wider_ship_setters_write_their_fields() {
        let decoded = ship(
            ShipBuilder::new()
                .cost(0x0123_4567)
                .flags(0x8001)
                .require(0x0102_0304_0506_0708),
        );
        assert_eq!(decoded.cost, 0x0123_4567);
        assert_eq!(decoded.flags.bits(), 0x8001);
        assert_eq!(decoded.require.bits(), 0x0102_0304_0506_0708);
    }

    #[test]
    fn slot_setters_fill_both_halves_from_slot_zero() {
        let decoded = ship(
            ShipBuilder::new()
                .weap_types(&[128, 129, 130, 131, 132, 133, 134, 135])
                .weap_counts(&[1, 2, 3, 4, 5, 6, 7, 8])
                .default_items(&[140, 141, 142, 143, 144, 145, 146, 147])
                .item_counts(&[11, 12, 13, 14, 15, 16, 17, 18]),
        );
        assert_eq!(
            decoded.weap_type1_4,
            [128, 129, 130, 131].map(|id| Some(WeaponId(id)))
        );
        assert_eq!(
            decoded.weap_type5_8,
            [132, 133, 134, 135].map(|id| Some(WeaponId(id)))
        );
        assert_eq!(decoded.weap_count1_4, [1, 2, 3, 4]);
        assert_eq!(decoded.weap_count5_8, [5, 6, 7, 8]);
        assert_eq!(
            decoded.default_items1_4,
            [140, 141, 142, 143].map(|id| Some(OutfitId(id)))
        );
        assert_eq!(
            decoded.default_items5_8,
            [144, 145, 146, 147].map(|id| Some(OutfitId(id)))
        );
        assert_eq!(decoded.item_count1_4, [11, 12, 13, 14]);
        assert_eq!(decoded.item_count5_8, [15, 16, 17, 18]);
    }

    #[test]
    fn a_short_slot_list_leaves_the_later_slots_alone() {
        let decoded = ship(ShipBuilder::new().weap_types(&[-1; 8]).weap_types(&[128]));
        assert_eq!(
            decoded.weap_type1_4,
            [Some(WeaponId(128)), None, None, None]
        );
        let built = ShipBuilder::new()
            .weap_types(&[-1; 8])
            .weap_types(&[128])
            .build();
        assert_eq!(&built[0x14..0x1A], &[0xFF; 6]);
        assert_eq!(&built[0x6CE..0x6D6], &[0xFF; 8]);
    }

    #[test]
    #[should_panic(expected = "8 slots")]
    fn more_than_eight_slots_is_a_mistake() {
        let _ = ShipBuilder::new().weap_types(&[1; 9]);
    }

    #[test]
    fn name_setters_write_their_strings() {
        let decoded = ship(
            ShipBuilder::new()
                .short_name("Shuttle")
                .comm_name("Shuttle Kestrel")
                .long_name("A long shuttle name")
                .subtitle("Light courier"),
        );
        assert_eq!(decoded.short_name.as_str(), "Shuttle");
        assert_eq!(decoded.comm_name.as_str(), "Shuttle Kestrel");
        assert_eq!(decoded.long_name.as_str(), "A long shuttle name");
        assert_eq!(decoded.subtitle.as_str(), "Light courier");
    }

    #[test]
    #[should_panic(expected = "doesn't fit")]
    fn a_name_too_long_for_its_field_is_a_mistake() {
        let _ = ShipBuilder::new().comm_name(&"x".repeat(32));
    }
}
