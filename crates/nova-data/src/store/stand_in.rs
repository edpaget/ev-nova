//! Stand-ins: what a ship or stellar with no sprite draws instead.
//!
//! The new game has no sprites, so its loader works out at load what each
//! hull and stellar draws in their place
//! (`crate::new_game`'s `looks`) and keeps it here, on the store, beside
//! the records: [`GameData::with_stand_ins`] sets them, and
//! [`GameData::stand_in`] answers them. The view asks only where it has
//! no sprite to draw.
//!
//! The stock game's store holds none: every stock record answers `None`,
//! so stock drawing is the same with or without stand-ins.
//!
//! The starfield stands in too. The new game has no star sheet (`spïn`
//! 700), so its loader sets the dots each star may draw as instead
//! ([`StarDot`], [`GameData::with_star_dots`]), and
//! [`GameData::star_dots`] answers them. They aren't keyed by record:
//! the sky has one set. A stock store has none, so a stock starfield
//! draws its sheet or, when the sheet fails, nothing, as before.

use std::collections::BTreeMap;

pub use nova_galaxy_media::{Outline, Triangle as OutlineTriangle};

use super::GameData;
use crate::wire::id::{ShipId, StellarId};

/// Whose stand-in: a ship type's (its `shïp`) or a stellar's (its
/// `spöb`). The two number apart, so a ship and a stellar with the same
/// ID are different keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StandInOf {
    /// A ship type.
    Ship(ShipId),
    /// A stellar object, a hypergate among them.
    Stellar(StellarId),
}

/// What a ship or stellar draws in place of a sprite.
#[derive(Clone, Debug, PartialEq)]
pub enum StandIn {
    /// Its model (or a ship's size box) seen from above, nose up: a ship's
    /// in pixels, a stellar's in its model's own units, which the view
    /// scales to the stellar's reach.
    Outline(Outline),
    /// A filled disc in `color` (red, green and blue), as big as the view
    /// draws a stellar with no size.
    Disc {
        /// Its colour.
        color: [u8; 3],
    },
    /// A ring in `color` (red, green and blue): a hypergate.
    Ring {
        /// Its colour.
        color: [u8; 3],
    },
}

/// One kind of star the starfield may draw where it has no star sheet: a
/// solid square dot `side` pixels across (1 to 3), in the grey `grey`
/// (each of red, green and blue), at full brightness before the system's
/// murk dims it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StarDot {
    /// Its side, in pixels.
    pub side: u8,
    /// Its grey, at full brightness.
    pub grey: u8,
}

impl GameData {
    /// The store with `dots` as the starfield's stand-in, replacing any it
    /// had (see the module docs).
    #[must_use]
    pub fn with_star_dots(mut self, dots: Vec<StarDot>) -> Self {
        self.star_dots = dots;
        self
    }

    /// The dots the starfield draws as where it has no star sheet: the new
    /// game's; empty in a stock store (see the module docs).
    #[must_use]
    pub fn star_dots(&self) -> &[StarDot] {
        &self.star_dots
    }

    /// The store with `stand_ins`, replacing any it had.
    #[must_use]
    pub fn with_stand_ins(mut self, stand_ins: BTreeMap<StandInOf, StandIn>) -> Self {
        self.stand_ins = stand_ins;
        self
    }

    /// The store with `stand_in` as `of`'s, replacing only `of`'s and
    /// keeping the others: the one-record form of
    /// [`with_stand_ins`](Self::with_stand_ins), for a test that changes
    /// one look of a loaded store.
    #[must_use]
    pub fn with_stand_in(mut self, of: StandInOf, stand_in: StandIn) -> Self {
        self.stand_ins.insert(of, stand_in);
        self
    }

    /// What `of` draws in place of a sprite, if the store has a stand-in
    /// for it: only the new game's do (see the module docs).
    #[must_use]
    pub fn stand_in(&self, of: StandInOf) -> Option<&StandIn> {
        self.stand_ins.get(&of)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::store::fixture::store_of;

    fn disc(red: u8) -> StandIn {
        StandIn::Disc { color: [red, 0, 0] }
    }

    fn outline() -> StandIn {
        StandIn::Outline(Outline {
            triangles: vec![OutlineTriangle {
                corners: [[0.0, 1.0], [1.0, 0.0], [-1.0, 0.0]],
                color: [1, 2, 3],
            }],
        })
    }

    #[test]
    fn a_store_of_decoded_records_has_no_stand_ins() {
        let data = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        assert_eq!(data.stand_in(StandInOf::Ship(ShipId(128))), None);
        assert_eq!(data.stand_in(StandInOf::Stellar(StellarId(128))), None);
    }

    #[test]
    fn a_store_with_stand_ins_answers_each_by_kind_and_id() {
        let data = GameData::from_decoded(PathBuf::from("/game"), Vec::new()).with_stand_ins(
            BTreeMap::from([
                (StandInOf::Ship(ShipId(128)), outline()),
                (StandInOf::Stellar(StellarId(128)), disc(1)),
                (
                    StandInOf::Stellar(StellarId(130)),
                    StandIn::Ring { color: [9; 3] },
                ),
            ]),
        );
        assert_eq!(
            data.stand_in(StandInOf::Ship(ShipId(128))),
            Some(&outline())
        );
        assert_eq!(
            data.stand_in(StandInOf::Stellar(StellarId(128))),
            Some(&disc(1))
        );
        assert_eq!(
            data.stand_in(StandInOf::Stellar(StellarId(130))),
            Some(&StandIn::Ring { color: [9; 3] })
        );
        assert_eq!(data.stand_in(StandInOf::Ship(ShipId(129))), None);
        assert_eq!(data.stand_in(StandInOf::Stellar(StellarId(129))), None);
    }

    #[test]
    fn stand_ins_replace_those_set_before() {
        let data = GameData::default()
            .with_stand_ins(BTreeMap::from([(
                StandInOf::Stellar(StellarId(128)),
                disc(1),
            )]))
            .with_stand_ins(BTreeMap::from([(
                StandInOf::Stellar(StellarId(129)),
                disc(2),
            )]));
        assert_eq!(data.stand_in(StandInOf::Stellar(StellarId(128))), None);
        assert_eq!(
            data.stand_in(StandInOf::Stellar(StellarId(129))),
            Some(&disc(2))
        );
    }

    #[test]
    fn one_stand_in_is_added_or_replaced_and_the_others_kept() {
        let data = GameData::default()
            .with_stand_ins(BTreeMap::from([
                (StandInOf::Stellar(StellarId(128)), disc(1)),
                (StandInOf::Stellar(StellarId(129)), disc(2)),
            ]))
            .with_stand_in(StandInOf::Stellar(StellarId(128)), outline())
            .with_stand_in(StandInOf::Ship(ShipId(128)), disc(3));
        assert_eq!(
            data.stand_in(StandInOf::Stellar(StellarId(128))),
            Some(&outline())
        );
        assert_eq!(
            data.stand_in(StandInOf::Stellar(StellarId(129))),
            Some(&disc(2))
        );
        assert_eq!(data.stand_in(StandInOf::Ship(ShipId(128))), Some(&disc(3)));
    }

    #[test]
    fn a_ship_and_a_stellar_with_the_same_id_do_not_collide() {
        let data = GameData::default().with_stand_ins(BTreeMap::from([
            (StandInOf::Ship(ShipId(128)), outline()),
            (StandInOf::Stellar(StellarId(128)), disc(7)),
        ]));
        assert_eq!(
            data.stand_in(StandInOf::Ship(ShipId(128))),
            Some(&outline())
        );
        assert_eq!(
            data.stand_in(StandInOf::Stellar(StellarId(128))),
            Some(&disc(7))
        );
    }

    #[test]
    fn a_stock_store_has_no_star_dots() {
        let decoded = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&decoded, &empty, &loaded] {
            assert_eq!(data.star_dots(), &[]);
        }
    }

    #[test]
    fn star_dots_replace_those_set_before_and_answer_back() {
        let first = StarDot { side: 1, grey: 90 };
        let second = StarDot { side: 3, grey: 200 };
        let third = StarDot { side: 2, grey: 140 };
        let data = GameData::default()
            .with_star_dots(vec![first])
            .with_star_dots(vec![second, third]);
        assert_eq!(data.star_dots(), &[second, third]);
    }

    #[test]
    fn a_stock_store_has_no_stand_ins() {
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&empty, &loaded] {
            assert_eq!(data.stand_in(StandInOf::Ship(ShipId(128))), None);
            assert_eq!(data.stand_in(StandInOf::Stellar(StellarId(128))), None);
        }
    }
}
