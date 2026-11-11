//! How a ship dies: its `wreck` ([`Wreck`]), re-exported through
//! [`fleet`](crate::fleet).
//!
//! A ship whose hull is gone doesn't vanish at once. It breaks up for a
//! while, throwing off small explosions, while its crew ejects in escape
//! pods. Then it goes up in a final explosion and is gone.

use schemars::JsonSchema;
use serde::Deserialize;

/// How a ship dies once its hull is gone. Each word is optional, and a
/// ship that names no `wreck` dies by every default: it breaks up for one
/// second, ends in a small explosion and ejects no pods. A variant
/// inherits its base's `wreck` word by word, the nearest given winning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Wreck {
    /// Seconds it breaks up for, before its final explosion. Left out,
    /// one second. The player's own ship breaks up for longer than this:
    /// the game holds on the player's death.
    #[serde(default)]
    pub break_up: Option<f64>,
    /// Its final explosion. Left out, `small`.
    #[serde(default)]
    pub explosion: Option<Explosion>,
    /// Escape pods its crew ejects while it breaks up. They leave from
    /// halfway through the break-up, at least a third of a second apart,
    /// so a break-up too short for its pods keeps the rest aboard. Left
    /// out, none.
    #[serde(default)]
    pub pods: Option<u32>,
}

impl Wreck {
    /// This wreck over `base`'s: each word this gives, else `base`'s.
    pub(crate) fn over(self, base: Wreck) -> Wreck {
        Wreck {
            break_up: self.break_up.or(base.break_up),
            explosion: self.explosion.or(base.explosion),
            pods: self.pods.or(base.pods),
        }
    }
}

/// A ship's final explosion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Explosion {
    /// A small explosion, for a light ship.
    Small,
    /// A large explosion, for a heavy one.
    Large,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::Ship;
    use crate::frontmatter::parse;

    fn read(yaml: &str) -> Result<Ship, String> {
        parse::<Ship>(&format!("---\nname: X\n{yaml}---\n")).map(|(ship, _)| ship)
    }

    #[test]
    fn a_ship_reads_its_whole_wreck() {
        let ship = read("wreck: {break_up: 2.5, explosion: large, pods: 3}\n").unwrap();
        assert_eq!(
            ship.wreck,
            Wreck {
                break_up: Some(2.5),
                explosion: Some(Explosion::Large),
                pods: Some(3),
            }
        );
        let small = read("wreck: {explosion: small}\n").unwrap();
        assert_eq!(small.wreck.explosion, Some(Explosion::Small));
    }

    #[test]
    fn a_ship_that_names_no_wreck_has_an_empty_one() {
        assert_eq!(read("").unwrap().wreck, Wreck::default());
        assert_eq!(
            Wreck::default(),
            Wreck {
                break_up: None,
                explosion: None,
                pods: None,
            }
        );
    }

    #[test]
    fn an_unknown_explosion_or_wreck_word_is_an_error() {
        assert!(read("wreck: {explosion: huge}\n").is_err());
        assert!(read("wreck: {debris: 4}\n").is_err());
        assert!(read("wreck: {pods: -1}\n").is_err());
    }

    #[test]
    fn a_wreck_over_its_base_takes_each_word_it_gives_else_the_base_s() {
        let base = Wreck {
            break_up: Some(2.0),
            explosion: Some(Explosion::Large),
            pods: Some(2),
        };
        let own = Wreck {
            pods: Some(5),
            ..Wreck::default()
        };
        assert_eq!(
            own.over(base),
            Wreck {
                pods: Some(5),
                ..base
            }
        );
        assert_eq!(Wreck::default().over(base), base);
        assert_eq!(base.over(Wreck::default()), base);
    }
}
