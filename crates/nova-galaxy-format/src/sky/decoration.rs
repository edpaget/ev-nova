//! A system's sky decorations: images drawn as far scenery in its
//! layered sky, such as a distant hulk, a far galaxy or a giant on the
//! horizon ([`Decoration`], the `decorations` list of a system's `sky`
//! block).
//!
//! Decorations are scenery only: nothing collides with one, targets one
//! or reads one in play. Every field but the image is optional, and the
//! engine has a default for each. A system that leaves the list out
//! draws the engine's own default for its seed (`nova_render::sky` says
//! what that is), and an empty list draws none.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Deserialize;

use crate::id::{Color, Fraction, Id};
use crate::model::Position;

/// The most decorations one system's sky lists.
pub const MOST_DECORATIONS: usize = 6;

/// One image drawn as far scenery in a system's sky. Omit a field for
/// the engine's default.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Decoration {
    /// The image: an asset (`assets/<id>.md`) of `kind: texture` with one
    /// `.png`, alpha straight. Any shape; it is not held to a planet's
    /// 2:1.
    pub image: Id,
    /// Where it sits, `[x, y]` on a stellar position's axes: it is drawn
    /// where a stellar at that position would be when the ship is at the
    /// system's centre, and from there it moves `parallax` of the ship's
    /// move. Omit for a place picked by the system's seed.
    #[serde(default)]
    pub at: Option<Position>,
    /// How much of the ship's move it moves by, 0.0125 to 0.5: 0.5 is the
    /// nearest star layer's, 0.05 the farthest's, 0.025 the nebula
    /// backdrop's. Smaller reads as farther. Omit for 0.05.
    #[serde(default)]
    pub parallax: Option<DecorationParallax>,
    /// The drawn length of its longer side, 8 to 512 logical units; the
    /// other side follows the image's shape. It doesn't shrink with
    /// `parallax`. Omit for 128.
    #[serde(default)]
    pub size: Option<DecorationSize>,
    /// Its heading, -360 to 360 degrees clockwise as drawn. Omit for 0.
    #[serde(default)]
    pub turn: Option<DecorationTurn>,
    /// A slow turn about its centre, -20 to 20 degrees a second,
    /// clockwise as drawn. Omit for 0: it holds its heading.
    #[serde(default)]
    pub spin: Option<DecorationSpin>,
    /// A colour the image is multiplied by, taken at full brightness, so
    /// it never dims; black is no tint. Omit for the image's own colours.
    #[serde(default)]
    pub tint: Option<Color>,
    /// How bright it is, 0 to 1. Omit for 1.
    #[serde(default)]
    pub brightness: Option<Fraction>,
    /// How it is laid on the sky. Omit for `over`.
    #[serde(default)]
    pub blend: Blend,
}

/// How a decoration is laid on the sky behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Blend {
    /// Painted over the sky by its alpha, so a hulk hides the stars
    /// behind it.
    #[default]
    Over,
    /// Its light added to the sky, so a galaxy shines through with the
    /// stars.
    Glow,
}

/// A system's decorations, at most 6, in the order written.
#[derive(Debug, Default, Deserialize)]
#[serde(try_from = "Vec<Decoration>")]
pub struct Decorations(Vec<Decoration>);

impl TryFrom<Vec<Decoration>> for Decorations {
    type Error = String;
    fn try_from(list: Vec<Decoration>) -> Result<Self, String> {
        if list.len() <= MOST_DECORATIONS {
            Ok(Self(list))
        } else {
            Err(format!(
                "{} decorations: a sky lists at most {MOST_DECORATIONS}",
                list.len()
            ))
        }
    }
}

impl Decorations {
    /// The decorations, in the order written.
    #[must_use]
    pub fn as_slice(&self) -> &[Decoration] {
        &self.0
    }
}

impl JsonSchema for Decorations {
    fn schema_name() -> Cow<'static, str> {
        "Decorations".into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "array",
            "maxItems": MOST_DECORATIONS,
            "items": generator.subschema_for::<Decoration>(),
            "description": "A system's sky decorations, at most 6"
        })
    }
}

/// Declares a number held to a range when it is read, with its schema.
macro_rules! ranged {
    ($(#[$doc:meta])* $name:ident, $low:literal, $high:literal, $what:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Deserialize)]
        #[serde(try_from = "f64")]
        pub struct $name(f64);

        impl TryFrom<f64> for $name {
            type Error = String;
            fn try_from(n: f64) -> Result<Self, String> {
                if (Self::LEAST..=Self::MOST).contains(&n) {
                    Ok(Self(n))
                } else {
                    Err(format!("{n} is outside {} to {}", Self::LEAST, Self::MOST))
                }
            }
        }

        impl $name {
            /// The least it may be.
            pub const LEAST: f64 = $low;
            /// The most it may be.
            pub const MOST: f64 = $high;

            /// The value, within its range.
            #[must_use]
            pub fn get(self) -> f64 {
                self.0
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({
                    "type": "number", "minimum": $low, "maximum": $high,
                    "description": $what
                })
            }
        }
    };
}

ranged!(
    /// A decoration's parallax: the share of the ship's move it moves by,
    /// from 0.0125 to 0.5.
    DecorationParallax,
    0.0125,
    0.5,
    "The share of the ship's move a decoration moves by, from 0.0125 to 0.5"
);

ranged!(
    /// A decoration's size: the drawn length of its longer side, from 8
    /// to 512 logical units.
    DecorationSize,
    8.0,
    512.0,
    "The drawn length of a decoration's longer side, from 8 to 512 logical units"
);

ranged!(
    /// A decoration's turn: degrees clockwise as drawn, from -360 to 360.
    DecorationTurn,
    -360.0,
    360.0,
    "A decoration's heading in degrees clockwise, from -360 to 360"
);

ranged!(
    /// A decoration's spin: degrees a second clockwise as drawn, from -20
    /// to 20.
    DecorationSpin,
    -20.0,
    20.0,
    "A decoration's spin in degrees a second clockwise, from -20 to 20"
);

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::frontmatter::parse;
    use crate::model::System;
    use crate::sky::Sky;

    const SYSTEM: &str = "name: Fringe\nregion: rim\nmap: [0, 0]\n";

    fn read(yaml: &str) -> Result<System, String> {
        parse::<System>(&format!("---\n{SYSTEM}{yaml}---\n")).map(|(t, _)| t)
    }

    fn sky(yaml: &str) -> Sky {
        read(yaml).expect("a system").sky.expect("a sky")
    }

    /// A system whose sky lists one decoration with `fields` after its
    /// image.
    fn one(fields: &str) -> Result<System, String> {
        read(&format!(
            "sky:\n  decorations:\n    - image: far-hulk\n{fields}"
        ))
    }

    #[test]
    fn a_full_decoration_reads_each_field() {
        let sky = sky(
            "sky:\n  decorations:\n    - image: far-hulk\n      at: [120, -80]\n      \
             parallax: 0.2\n      size: 96\n      turn: -45\n      spin: 2.5\n      \
             tint: \"#ffd9b0\"\n      brightness: 0.75\n      blend: glow\n",
        );
        let list = sky.decorations.expect("a list");
        let [decoration] = list.as_slice() else {
            panic!("one decoration: {list:?}");
        };
        assert_eq!(decoration.image.as_str(), "far-hulk");
        assert_eq!(decoration.at, Some([120, -80]));
        assert_eq!(decoration.parallax.map(DecorationParallax::get), Some(0.2));
        assert_eq!(decoration.size.map(DecorationSize::get), Some(96.0));
        assert_eq!(decoration.turn.map(DecorationTurn::get), Some(-45.0));
        assert_eq!(decoration.spin.map(DecorationSpin::get), Some(2.5));
        assert_eq!(
            decoration.tint.as_ref().map(Color::rgb),
            Some([0xff, 0xd9, 0xb0])
        );
        assert_eq!(decoration.brightness.map(Fraction::get), Some(0.75));
        assert_eq!(decoration.blend, Blend::Glow);
    }

    #[test]
    fn only_the_image_is_required() {
        let sky = sky("sky:\n  decorations:\n    - image: far-galaxy\n");
        let list = sky.decorations.expect("a list");
        let [decoration] = list.as_slice() else {
            panic!("one decoration: {list:?}");
        };
        assert_eq!(decoration.image.as_str(), "far-galaxy");
        assert!(decoration.at.is_none() && decoration.parallax.is_none());
        assert!(decoration.size.is_none() && decoration.turn.is_none());
        assert!(decoration.tint.is_none() && decoration.brightness.is_none());
        assert!(decoration.spin.is_none());
        assert_eq!(decoration.blend, Blend::Over);
        let none = read("sky:\n  decorations:\n    - size: 64\n").unwrap_err();
        assert!(none.contains("image"), "{none}");
    }

    #[test]
    fn a_blend_is_over_or_glow() {
        assert!(one("      blend: over\n").is_ok());
        assert!(one("      blend: glow\n").is_ok());
        assert!(one("      blend: add\n").is_err());
    }

    #[test]
    fn each_range_takes_its_ends_and_refuses_just_past_them() {
        for (field, low, high) in [
            ("parallax", "0.0125", "0.5"),
            ("size", "8", "512"),
            ("turn", "-360", "360"),
            ("spin", "-20", "20"),
            ("brightness", "0", "1"),
        ] {
            for good in [low, high] {
                let read = one(&format!("      {field}: {good}\n"));
                assert!(read.is_ok(), "{field} {good}: {read:?}");
            }
        }
        for (field, bad) in [
            ("parallax", "0.0124"),
            ("parallax", "0.51"),
            ("parallax", ".nan"),
            ("size", "7.9"),
            ("size", "512.5"),
            ("turn", "-360.5"),
            ("turn", "361"),
            ("turn", ".inf"),
            ("spin", "-20.5"),
            ("spin", "21"),
            ("spin", ".nan"),
            ("brightness", "-0.1"),
            ("brightness", "1.1"),
            ("size", "big"),
        ] {
            let read = one(&format!("      {field}: {bad}\n"));
            assert!(read.is_err(), "{field} {bad}: {read:?}");
        }
        assert!(DecorationParallax::try_from(0.5 + 1e-9).is_err());
        assert!(DecorationParallax::try_from(0.0125 - 1e-9).is_err());
        assert!(DecorationSize::try_from(8.0 - 1e-9).is_err());
        assert!(DecorationSize::try_from(512.0 + 1e-9).is_err());
        assert!(DecorationTurn::try_from(-360.0 - 1e-9).is_err());
        assert!(DecorationTurn::try_from(360.0 + 1e-9).is_err());
        assert!(DecorationSpin::try_from(-20.0 - 1e-9).is_err());
        assert!(DecorationSpin::try_from(20.0 + 1e-9).is_err());
        assert_eq!((DecorationSpin::LEAST, DecorationSpin::MOST), (-20.0, 20.0));
        assert_eq!(
            DecorationParallax::try_from(0.0125).map(DecorationParallax::get),
            Ok(0.0125)
        );
        assert_eq!(
            (DecorationParallax::LEAST, DecorationParallax::MOST),
            (0.0125, 0.5)
        );
        assert_eq!((DecorationSize::LEAST, DecorationSize::MOST), (8.0, 512.0));
        assert_eq!(
            (DecorationTurn::LEAST, DecorationTurn::MOST),
            (-360.0, 360.0)
        );
        let why = DecorationSize::try_from(600.0).unwrap_err();
        assert!(why.contains("600") && why.contains("8 to 512"), "{why}");
    }

    /// A sky block listing `count` decorations.
    fn listing(count: usize) -> String {
        let mut yaml = String::from("sky:\n  decorations:\n");
        for _ in 0..count {
            yaml.push_str("    - image: far-hulk\n");
        }
        yaml
    }

    #[test]
    fn six_decorations_are_accepted_and_a_seventh_refused() {
        let six = sky(&listing(MOST_DECORATIONS));
        assert_eq!(six.decorations.expect("a list").as_slice().len(), 6);
        let seven = read(&listing(MOST_DECORATIONS + 1)).unwrap_err();
        assert!(seven.contains("at most 6"), "{seven}");
    }

    #[test]
    fn an_empty_list_is_a_list_and_a_missing_key_is_none() {
        let empty = sky("sky: {decorations: []}\n");
        assert_eq!(empty.decorations.map(|d| d.as_slice().len()), Some(0));
        assert!(sky("sky: {stars: 1}\n").decorations.is_none());
        assert!(sky("sky: {}\n").decorations.is_none());
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let why = one("      drift: 2\n").unwrap_err();
        assert!(why.contains("unknown field `drift`"), "{why}");
    }

    #[test]
    fn an_image_that_is_not_an_id_is_refused() {
        for bad in ["Far Hulk", "media/export/sky/hulk.png", "\"\""] {
            let read = read(&format!("sky:\n  decorations:\n    - image: {bad}\n"));
            assert!(read.is_err(), "{bad}: {read:?}");
        }
    }

    #[test]
    fn each_ranged_number_s_schema_is_named_and_bounded() {
        let mut generator = SchemaGenerator::default();
        let parallax = DecorationParallax::json_schema(&mut generator).to_value();
        assert_eq!(DecorationParallax::schema_name(), "DecorationParallax");
        assert_eq!(parallax["type"], "number");
        assert_eq!(
            (parallax["minimum"].as_f64(), parallax["maximum"].as_f64()),
            (Some(0.0125), Some(0.5))
        );
        let size = DecorationSize::json_schema(&mut generator).to_value();
        assert_eq!(DecorationSize::schema_name(), "DecorationSize");
        assert_eq!(
            (size["minimum"].as_f64(), size["maximum"].as_f64()),
            (Some(8.0), Some(512.0))
        );
        let turn = DecorationTurn::json_schema(&mut generator).to_value();
        assert_eq!(DecorationTurn::schema_name(), "DecorationTurn");
        assert_eq!(
            (turn["minimum"].as_f64(), turn["maximum"].as_f64()),
            (Some(-360.0), Some(360.0))
        );
        let spin = DecorationSpin::json_schema(&mut generator).to_value();
        assert_eq!(DecorationSpin::schema_name(), "DecorationSpin");
        assert_eq!(
            (spin["minimum"].as_f64(), spin["maximum"].as_f64()),
            (Some(-20.0), Some(20.0))
        );
    }

    #[test]
    fn the_list_s_schema_is_an_array_of_at_most_six_decorations() {
        assert_eq!(Decorations::schema_name(), "Decorations");
        let schema = Decorations::json_schema(&mut SchemaGenerator::default()).to_value();
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["maxItems"].as_u64(), Some(6));
        assert!(
            schema["items"]["$ref"]
                .as_str()
                .is_some_and(|r| r.ends_with("/Decoration")),
            "{schema}"
        );
    }
}
