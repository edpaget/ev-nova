//! What a launcher fires and what a bay launches: an outfit's
//! `guidance`, `ammunition` and `fighters`, re-exported through
//! [`fleet`](crate::fleet).
//!
//! A launcher (an outfit of mount `launcher`) can steer its shots at their
//! target and spend rounds of an ammunition outfit, each hardpoint
//! carrying it coming with a load. A bay (an outfit of mount `bay`) can
//! launch fighters of a hull, each bay hardpoint carrying it coming with a
//! load of them; the fighters come with the hull and are never sold.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::id::Id;

/// How a launcher's shots fly. Read only on a launcher. Left out,
/// `unguided`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Guidance {
    /// Straight ahead, as fired.
    #[default]
    Unguided,
    /// Steering at the firer's target.
    Homing,
}

/// The ammunition a launcher fires. Read only on a launcher.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ammunition {
    /// The outfit whose rounds it fires (`outfits/<id>.md`): sold where
    /// its own `sold_at` says, and granted by a mission as any outfit is.
    pub outfit: Id,
    /// How many rounds each hardpoint carrying the launcher comes with as
    /// built.
    pub load: u32,
}

/// The fighters a bay launches. Read only on a bay.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fighters {
    /// The hull it launches (`ships/<id>.md`), which flies as a fighter
    /// and docks back aboard. It must not carry a bay with `fighters` of
    /// its own.
    pub hull: Id,
    /// How many each bay hardpoint carrying it comes with as built. They
    /// come with the hull: no outfitter sells them.
    pub load: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::Outfit;
    use crate::frontmatter::parse;

    fn read(mount: &str, yaml: &str) -> Result<Outfit, String> {
        parse::<Outfit>(&format!(
            "---\nname: X\nmount: {mount}\ndamage: 5\nrange: 300\nreload: 1\nprice: 10\n{yaml}---\n"
        ))
        .map(|(outfit, _)| outfit)
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    #[test]
    fn an_outfit_reads_its_guidance_ammunition_and_fighters() {
        let launcher = read(
            "launcher",
            "guidance: homing\nammunition: {outfit: seekers, load: 6}\n",
        )
        .unwrap();
        assert_eq!(launcher.guidance, Guidance::Homing);
        assert_eq!(
            launcher.ammunition,
            Some(Ammunition {
                outfit: id("seekers"),
                load: 6,
            })
        );
        let bay = read("bay", "fighters: {hull: drone, load: 2}\n").unwrap();
        assert_eq!(
            bay.fighters,
            Some(Fighters {
                hull: id("drone"),
                load: 2,
            })
        );
        let unguided = read("launcher", "guidance: unguided\n").unwrap();
        assert_eq!(unguided.guidance, Guidance::Unguided);
    }

    #[test]
    fn left_out_an_outfit_is_unguided_with_no_ammunition_and_no_fighters() {
        let outfit = read("launcher", "").unwrap();
        assert_eq!(outfit.guidance, Guidance::Unguided);
        assert_eq!(outfit.ammunition, None);
        assert_eq!(outfit.fighters, None);
        assert_eq!(Guidance::default(), Guidance::Unguided);
    }

    #[test]
    fn an_unknown_guidance_an_unknown_key_or_a_missing_load_is_an_error() {
        for yaml in [
            "guidance: beam\n",
            "ammunition: {outfit: seekers, load: 6, max: 9}\n",
            "ammunition: {outfit: seekers}\n",
            "ammunition: {load: 6}\n",
            "fighters: {hull: drone, load: 2, crew: 1}\n",
            "fighters: {hull: drone}\n",
            "fighters: {hull: drone, load: -1}\n",
        ] {
            assert!(read("launcher", yaml).is_err(), "{yaml}");
        }
    }
}
