//! How a system's sky looks behind the play: its star density, the
//! colour its stars lean toward, a distant nebula backdrop, and the
//! images that decorate it ([`Sky`], a system's optional `sky` block).
//!
//! Every field is optional, and the engine has a default for each, so a
//! system with no `sky` block draws the engine's star field and a faint
//! backdrop whose colour and amount come from the system's own seed. The
//! backdrop is scenery only: it is not the `nebula` hazard
//! ([`crate::model::Hazard::Nebula`]), which is fog that cuts sensors,
//! and neither sets nor implies the other.
//!
//! A system's decorations ([`decoration`]: distant hulks, far galaxies
//! and the like, each an image asset with a place, a depth and a size)
//! are scenery only too. A missing `decorations` key draws the engine's
//! default for the system's seed, and an empty list draws none.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Deserialize;

use crate::id::{Color, Fraction};

pub mod decoration;

use decoration::Decorations;

/// How a system's sky looks: its stars, its distant backdrop and its
/// decorations. Omit a field for the engine's default.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sky {
    /// How many background stars, times the engine's density, 0 to 2: 0
    /// is an empty star field, 2 twice as busy. Omit for 1.
    #[serde(default)]
    pub stars: Option<StarDensity>,
    /// A colour the stars lean toward, 35% of the way from their usual
    /// white, blue and yellow temperatures. It is taken at full
    /// brightness, so a dark colour doesn't dim the sky, and black leaves
    /// the stars as they are. Omit for the engine's star colours.
    #[serde(default)]
    pub tint: Option<Color>,
    /// The distant nebula behind every star: faint clouds on a far
    /// layer that barely moves. Omit for a backdrop whose colour and
    /// amount come from the system's seed.
    #[serde(default)]
    pub backdrop: Option<Backdrop>,
    /// Images drawn as far scenery, behind every planet and ship: distant
    /// hulks, far galaxies and the like, at most 6. Omit for the engine's
    /// default, picked by the system's seed; an empty list draws none.
    #[serde(default)]
    pub decorations: Option<Decorations>,
}

/// A system's distant nebula backdrop. Each field left out comes from the
/// system's seed, the other one staying as written.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Backdrop {
    /// The colour of the clouds, taken at full brightness: the backdrop's
    /// own faintness sets its light, and black draws none. Omit for one
    /// of the engine's nebula hues, picked by the system's seed.
    #[serde(default)]
    pub color: Option<Color>,
    /// How much of the sky holds cloud, 0 to 1: the chance that each
    /// patch of the backdrop holds a cloud. 0 turns the backdrop off.
    /// Omit for 0.25 to 0.7, picked by the system's seed.
    #[serde(default)]
    pub amount: Option<Fraction>,
}

/// A star density: how many background stars, times the engine's, from
/// 0 to 2.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Deserialize)]
#[serde(try_from = "f64")]
pub struct StarDensity(f64);

impl TryFrom<f64> for StarDensity {
    type Error = String;
    fn try_from(n: f64) -> Result<Self, String> {
        if (0.0..=2.0).contains(&n) {
            Ok(Self(n))
        } else {
            Err(format!("{n} is outside 0 to 2"))
        }
    }
}

impl StarDensity {
    /// The value, 0 to 2.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl JsonSchema for StarDensity {
    fn schema_name() -> Cow<'static, str> {
        "StarDensity".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "number", "minimum": 0, "maximum": 2,
            "description": "How many background stars, times the engine's, from 0 to 2"
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::frontmatter::parse;
    use crate::model::System;

    const SYSTEM: &str = "name: Fringe\nregion: rim\nmap: [0, 0]\n";

    fn read(yaml: &str) -> Result<System, String> {
        parse::<System>(&format!("---\n{SYSTEM}{yaml}---\n")).map(|(t, _)| t)
    }

    fn sky(yaml: &str) -> Sky {
        read(yaml).expect("a system").sky.expect("a sky")
    }

    #[test]
    fn a_system_with_no_sky_reads_none() {
        assert!(read("").unwrap().sky.is_none());
    }

    #[test]
    fn a_full_sky_reads_each_field() {
        let sky = sky("sky:\n  stars: 1.5\n  tint: \"#ffd9b0\"\n  backdrop:\n    \
             color: \"#2c3a5e\"\n    amount: 0.4\n");
        assert_eq!(sky.stars.map(StarDensity::get), Some(1.5));
        assert_eq!(sky.tint.map(|c| c.rgb()), Some([0xff, 0xd9, 0xb0]));
        let backdrop = sky.backdrop.expect("a backdrop");
        assert_eq!(backdrop.color.map(|c| c.rgb()), Some([0x2c, 0x3a, 0x5e]));
        assert_eq!(backdrop.amount.map(Fraction::get), Some(0.4));
    }

    #[test]
    fn a_partial_sky_leaves_the_rest_none() {
        let stars = sky("sky: {stars: 0.5}\n");
        assert_eq!(stars.stars.map(StarDensity::get), Some(0.5));
        assert!(stars.tint.is_none() && stars.backdrop.is_none());
        let empty = sky("sky: {}\n");
        assert!(empty.stars.is_none() && empty.tint.is_none() && empty.backdrop.is_none());
        let amount = sky("sky: {backdrop: {amount: 0}}\n")
            .backdrop
            .expect("a backdrop");
        assert_eq!(amount.amount.map(Fraction::get), Some(0.0));
        assert!(amount.color.is_none());
        let color = sky("sky: {backdrop: {color: \"#102030\"}}\n")
            .backdrop
            .expect("a backdrop");
        assert!(color.amount.is_none());
        assert_eq!(color.color.map(|c| c.rgb()), Some([0x10, 0x20, 0x30]));
    }

    #[test]
    fn stars_run_from_0_to_2() {
        for good in ["0", "2", "1"] {
            let read = read(&format!("sky: {{stars: {good}}}\n"));
            assert!(read.is_ok(), "{good}: {read:?}");
        }
        for bad in ["-0.1", "2.1", "busy", ".nan"] {
            let read = read(&format!("sky: {{stars: {bad}}}\n"));
            assert!(read.is_err(), "{bad}: {read:?}");
        }
        assert_eq!(StarDensity::try_from(2.0).map(StarDensity::get), Ok(2.0));
        assert!(StarDensity::try_from(2.0 + 1e-9).is_err());
        assert!(StarDensity::try_from(-1e-9).is_err());
    }

    #[test]
    fn a_backdrop_amount_runs_from_0_to_1() {
        for bad in ["-0.1", "1.1"] {
            let read = read(&format!("sky: {{backdrop: {{amount: {bad}}}}}\n"));
            assert!(read.is_err(), "{bad}: {read:?}");
        }
        assert!(read("sky: {backdrop: {amount: 1}}\n").is_ok());
    }

    #[test]
    fn a_tint_or_a_backdrop_colour_that_is_not_rrggbb_is_refused() {
        for bad in ["\"ffd9b0\"", "\"#ffd9b\"", "red"] {
            let tint = read(&format!("sky: {{tint: {bad}}}\n"));
            assert!(tint.is_err(), "{bad}: {tint:?}");
            let color = read(&format!("sky: {{backdrop: {{color: {bad}}}}}\n"));
            assert!(color.is_err(), "{bad}: {color:?}");
        }
    }

    #[test]
    fn an_unknown_field_in_sky_or_backdrop_is_refused() {
        let sky = read("sky: {dust: 1}\n").unwrap_err();
        assert!(sky.contains("unknown field `dust`"), "{sky}");
        let backdrop = read("sky: {backdrop: {haze: 1}}\n").unwrap_err();
        assert!(backdrop.contains("unknown field `haze`"), "{backdrop}");
    }

    #[test]
    fn a_star_density_schema_is_named_and_a_number_from_0_to_2() {
        assert_eq!(StarDensity::schema_name(), "StarDensity");
        let schema = StarDensity::json_schema(&mut SchemaGenerator::default()).to_value();
        assert_eq!(schema["type"], "number");
        assert_eq!(
            (schema["minimum"].as_u64(), schema["maximum"].as_u64()),
            (Some(0), Some(2))
        );
    }
}
