//! The `ïntf` builder.

use super::Bytes;
use crate::records::interface::Interface;

/// Defines setters of `u32` colours at fixed offsets.
macro_rules! color_setters {
    ($($name:ident @ $offset:literal),* $(,)?) => {
        $(
            #[doc = concat!(
                "Sets [`Interface::", stringify!($name),
                "`] (offset ", stringify!($offset), ", u32)."
            )]
            #[must_use]
            pub fn $name(mut self, value: u32) -> Self {
                self.0.u32($offset, value);
                self
            }
        )*
    };
}

/// Defines setters of rectangles at fixed offsets, written top, left,
/// bottom, right, the order [`Rect`](crate::wire::geometry::Rect) reads.
macro_rules! area_setters {
    ($($name:ident @ $offset:literal),* $(,)?) => {
        $(
            #[doc = concat!(
                "Sets [`Interface::", stringify!($name),
                "`] (offset ", stringify!($offset),
                ", rect): `[top, left, bottom, right]`."
            )]
            #[must_use]
            pub fn $name(mut self, rect: [i16; 4]) -> Self {
                self.0.i16s($offset, 4, &rect);
                self
            }
        )*
    };
}

/// Builds an `ïntf` ([`Interface`]) record's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceBuilder(Bytes);

impl Default for InterfaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl InterfaceBuilder {
    /// A zeroed `ïntf`.
    #[must_use]
    pub fn new() -> Self {
        Self(Bytes::zeroed::<Interface>())
    }

    /// The `ïntf` `bytes`, to change some of its fields.
    ///
    /// # Panics
    ///
    /// If `bytes` isn't a whole `ïntf`.
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(Bytes::of::<Interface>(bytes))
    }

    /// The record's bytes.
    #[must_use]
    pub fn build(self) -> Vec<u8> {
        self.0.0
    }

    color_setters! {
        bright_text @ 0x00,
        dim_text @ 0x04,
        bright_radar @ 0x10,
        dim_radar @ 0x14,
        shield_color @ 0x20,
        armor_color @ 0x2C,
        fuel_full @ 0x38,
        fuel_partial @ 0x3C,
    }

    area_setters! {
        radar_area @ 0x08,
        shield_area @ 0x18,
        armor_area @ 0x24,
        fuel_area @ 0x30,
        nav_area @ 0x40,
        weap_area @ 0x48,
        targ_area @ 0x50,
        cargo_area @ 0x58,
    }

    i16_setters! { Interface:
        stat_font_size @ 0xA0,
        subtitle_size @ 0xA2,
        status_bkgnd @ 0xA4,
    }

    /// Sets [`Interface::status_font`] (offset 0x60, 64 bytes).
    #[must_use]
    pub fn status_font(mut self, font: &str) -> Self {
        self.0.text(0x60, 64, font);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Record;
    use crate::testutil::Buf;
    use crate::wire::geometry::Rect;

    fn interface(builder: InterfaceBuilder) -> Interface {
        Buf(builder.build()).decode()
    }

    #[test]
    fn a_new_intf_is_zeroed_and_full_size() {
        assert_eq!(
            InterfaceBuilder::new().build(),
            vec![0; Interface::SIZE.expect("fixed")]
        );
        assert_eq!(InterfaceBuilder::default(), InterfaceBuilder::new());
    }

    /// A setter's name, the setter, and the decoded field it sets.
    type ColorCase = (
        &'static str,
        fn(InterfaceBuilder, u32) -> InterfaceBuilder,
        fn(&Interface) -> u32,
    );

    #[test]
    fn every_colour_setter_writes_its_field() {
        let cases: [ColorCase; 8] = [
            ("bright_text", InterfaceBuilder::bright_text, |i| {
                i.bright_text
            }),
            ("dim_text", InterfaceBuilder::dim_text, |i| i.dim_text),
            ("bright_radar", InterfaceBuilder::bright_radar, |i| {
                i.bright_radar
            }),
            ("dim_radar", InterfaceBuilder::dim_radar, |i| i.dim_radar),
            ("shield_color", InterfaceBuilder::shield_color, |i| {
                i.shield_color
            }),
            ("armor_color", InterfaceBuilder::armor_color, |i| {
                i.armor_color
            }),
            ("fuel_full", InterfaceBuilder::fuel_full, |i| i.fuel_full),
            ("fuel_partial", InterfaceBuilder::fuel_partial, |i| {
                i.fuel_partial
            }),
        ];
        for (name, set, get) in cases {
            let decoded = interface(set(InterfaceBuilder::new(), 0x0012_3456));
            assert_eq!(get(&decoded), 0x0012_3456, "{name}");
        }
    }

    /// A setter's name, the setter, and the decoded area it sets.
    type AreaCase = (
        &'static str,
        fn(InterfaceBuilder, [i16; 4]) -> InterfaceBuilder,
        fn(&Interface) -> Rect,
    );

    #[test]
    fn every_area_setter_writes_its_rect_top_left_bottom_right() {
        let cases: [AreaCase; 8] = [
            ("radar_area", InterfaceBuilder::radar_area, |i| i.radar_area),
            ("shield_area", InterfaceBuilder::shield_area, |i| {
                i.shield_area
            }),
            ("armor_area", InterfaceBuilder::armor_area, |i| i.armor_area),
            ("fuel_area", InterfaceBuilder::fuel_area, |i| i.fuel_area),
            ("nav_area", InterfaceBuilder::nav_area, |i| i.nav_area),
            ("weap_area", InterfaceBuilder::weap_area, |i| i.weap_area),
            ("targ_area", InterfaceBuilder::targ_area, |i| i.targ_area),
            ("cargo_area", InterfaceBuilder::cargo_area, |i| i.cargo_area),
        ];
        for (name, set, get) in cases {
            let decoded = interface(set(InterfaceBuilder::new(), [1, 2, 3, 4]));
            assert_eq!(
                get(&decoded),
                Rect {
                    top: 1,
                    left: 2,
                    bottom: 3,
                    right: 4
                },
                "{name}"
            );
        }
    }

    /// A setter's name, the setter, and the decoded field it sets.
    type I16Case = (
        &'static str,
        fn(InterfaceBuilder, i16) -> InterfaceBuilder,
        fn(&Interface) -> i16,
    );

    #[test]
    fn every_i16_intf_setter_writes_its_field() {
        let cases: [I16Case; 3] = [
            ("stat_font_size", InterfaceBuilder::stat_font_size, |i| {
                i.stat_font_size
            }),
            ("subtitle_size", InterfaceBuilder::subtitle_size, |i| {
                i.subtitle_size
            }),
            ("status_bkgnd", InterfaceBuilder::status_bkgnd, |i| {
                i.status_bkgnd
            }),
        ];
        for (name, set, get) in cases {
            let decoded = interface(set(InterfaceBuilder::new(), 0x1234));
            assert_eq!(get(&decoded), 0x1234, "{name}");
        }
    }

    #[test]
    fn the_font_setter_writes_its_name() {
        let decoded = interface(InterfaceBuilder::new().status_font("Geneva"));
        assert_eq!(decoded.status_font.as_str(), "Geneva");
    }

    #[test]
    fn from_bytes_changes_only_the_fields_it_sets() {
        let base = InterfaceBuilder::new()
            .dim_text(5)
            .status_bkgnd(700)
            .build();
        let changed = InterfaceBuilder::from_bytes(base).status_bkgnd(701).build();
        assert_eq!(
            changed,
            InterfaceBuilder::new()
                .dim_text(5)
                .status_bkgnd(701)
                .build()
        );
    }

    #[test]
    #[should_panic(expected = "a record is")]
    fn from_bytes_wants_a_whole_intf() {
        let _ = InterfaceBuilder::from_bytes(vec![0; 3]);
    }
}
