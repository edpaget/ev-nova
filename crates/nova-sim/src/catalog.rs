//! The pilot catalog port: what a flight session starts from, in the
//! simulation's own terms.

use std::rc::Rc;

pub use nova_data::{GovtId, JunkId, ShipId, SoundId, StellarId, SystemId};

use crate::fuel::OutfitMod;
use crate::geometry::Vec2;
use crate::handling::ShipFields;

/// A new pilot's start, from the first `chär`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharacterStart {
    /// The starting `shïp`, if it names one.
    pub ship: Option<ShipId>,
    /// The starting `sÿst`s, in order; any may be unused.
    pub systems: [Option<SystemId>; 4],
    /// The starting date, raw.
    pub start: StartDate,
    /// The starting credits, raw: the [`pilot`](crate::pilot) rules
    /// decide what a negative amount means.
    pub cash: i32,
    /// The starting legal records, `Govt1-4` with `Status1-4`: each
    /// government and the record with it, or `None` for an unused slot.
    pub legal: [Option<(GovtId, i16)>; 4],
}

/// A new pilot's starting date, raw from the `chär`: the
/// [`date`](crate::date) rules decide what the values mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartDate {
    /// The day of the month.
    pub day: i16,
    /// The month, 1 (January) to 12.
    pub month: i16,
    /// The year.
    pub year: i16,
}

/// A star system on the map, raw from its `sÿst`: the
/// [`hyperspace`](crate::hyperspace) rules decide which links count.
#[derive(Clone, Debug, PartialEq)]
pub struct StarSystem {
    /// The `sÿst`'s ID.
    pub id: SystemId,
    /// Its map position, `xPos` and `yPos`.
    pub position: Vec2,
    /// Its hyperlinks, `Con1-Con16`, in record order: they may repeat,
    /// point at itself or at a system that does not exist.
    pub links: Vec<SystemId>,
}

/// A stellar the player might land on, raw from its `spöb`; the
/// [`landing`](crate::landing) rules decide what the values mean.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LandingSite {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its centre, `xPos` and `yPos`, in pixels from the system's centre.
    pub position: Vec2,
    /// Its sprite's frame width and height in pixels, or `None` when it
    /// has no sprite that can be read.
    pub frame_size: Option<(u32, u32)>,
    /// Its `Flags`.
    pub flags: u32,
    /// Its `MinStatus`: the legal record below which landing is refused.
    pub min_status: i16,
    /// The sound it plays when the player lands, from its `CustSndID`:
    /// stellar landing sounds are `snd ` 10000 and up, and any other value
    /// (-1 for none, 0, or the angle hypergates and wormholes keep there)
    /// is none.
    pub landing_sound: Option<SoundId>,
}

/// The standard commodities, raw from their string lists: `STR#` 4000
/// "All Cargo" names them and `STR#` 4004 "Base Prices" prices them, the
/// nth string for commodity n (from 0); the [`market`](crate::market)
/// rules decide which are traded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommodityStrings {
    /// Every string of `STR#` 4000, in order; none when it is missing.
    pub names: Vec<String>,
    /// Every string of `STR#` 4004, in order; none when it is missing.
    pub base_prices: Vec<String>,
}

/// A special commodity, raw from its `jünk`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JunkRecord {
    /// The `jünk`'s ID.
    pub id: JunkId,
    /// Its name: the resource's name, or its `LCName` when the resource
    /// has none.
    pub name: String,
    /// Its `BasePrice`.
    pub base_price: i16,
    /// Its `SoldAt` stellars, the unused (-1) slots left out.
    pub sold_at: Vec<StellarId>,
    /// Its `BoughtAt` stellars, the unused (-1) slots left out.
    pub bought_at: Vec<StellarId>,
    /// Its `BuyOn` control-bit expression.
    pub buy_on: String,
    /// Its `SellOn` control-bit expression.
    pub sell_on: String,
}

/// An `öops` resource's ID: a planetary event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisasterId(pub i16);

/// A planetary event that moves one commodity's price, raw from its
/// `öops`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisasterRecord {
    /// The `öops`'s ID.
    pub id: DisasterId,
    /// Its resource name, which the commodity exchange shows while it is
    /// active.
    pub name: String,
    /// Its `Stellar`, raw: a `spöb` ID, -1 for any stellar or -2 for none
    /// (news only).
    pub stellar: i16,
    /// Its `Commodity`: 0 food, 1 industrial, and so on.
    pub commodity: i16,
    /// Its `PriceDelta`.
    pub price_delta: i16,
    /// Its `Duration`, in days.
    pub duration: i16,
    /// Its `Freq`: the percent chance each day that it starts.
    pub freq: i16,
    /// Its `ActivateOn` control-bit expression.
    pub activate_on: String,
}

/// Why a flight session could not start. Each message is ready to display.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StartError {
    /// There is no `chär` at all.
    #[error("no chär to start from")]
    NoCharacter,
    /// The first `chär` does not decode; the decoder's message.
    #[error("{0}")]
    Character(String),
    /// The first `chär` names no ship.
    #[error("the first chär has no ship")]
    NoShip,
    /// The starting ship cannot be read; why.
    #[error("{1}")]
    Ship(ShipId, String),
    /// None of the first `chär`'s starting systems exists.
    #[error("none of the first chär's starting systems ({}) exists", slots(.0))]
    NoStartingSystem([Option<SystemId>; 4]),
    /// The system a saved pilot is in no longer exists.
    #[error("the pilot's system, sÿst {}, does not exist", .0.0)]
    NoSystem(SystemId),
}

/// The starting system slots as text: each ID, or "none".
fn slots(systems: &[Option<SystemId>; 4]) -> String {
    systems
        .iter()
        .map(|slot| slot.map_or_else(|| "none".to_owned(), |id| id.0.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The game data a flight session starts from.
pub trait PilotCatalog {
    /// The first `chär` by ascending ID: its ship and starting systems.
    fn first_character(&self) -> Result<CharacterStart, StartError>;
    /// Ship `id`'s handling and reserve fields, or why they cannot be read.
    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String>;
    /// Every mod of ship `id`'s default outfits, its `DefaultItems`, raw:
    /// each readable `oütf`'s `ModType`s and `ModVal`s, with how many the
    /// ship carries. None for a ship that cannot be read.
    fn default_outfits(&self, id: ShipId) -> Vec<OutfitMod>;
    /// Whether system `id` exists and can be read.
    fn system_exists(&self, id: SystemId) -> bool;
    /// The stellars of system `id` that can be read, in its `nav_def`
    /// order; none for a system that cannot be read.
    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite>;
    /// Every system that can be read, by ascending ID, with its map
    /// position and hyperlinks.
    fn star_map(&self) -> Vec<StarSystem>;
    /// The standard commodities' names and base prices.
    fn commodity_strings(&self) -> CommodityStrings;
    /// Every `jünk` that can be read, by ascending ID.
    fn junk(&self) -> Vec<JunkRecord>;
    /// Every `öops` that can be read, by ascending ID.
    fn disasters(&self) -> Vec<DisasterRecord>;
}

/// A borrowed catalog is a catalog.
impl<T: PilotCatalog + ?Sized> PilotCatalog for &T {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        (**self).first_character()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        (**self).ship_fields(id)
    }

    fn default_outfits(&self, id: ShipId) -> Vec<OutfitMod> {
        (**self).default_outfits(id)
    }

    fn system_exists(&self, id: SystemId) -> bool {
        (**self).system_exists(id)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        (**self).landing_sites(system)
    }

    fn star_map(&self) -> Vec<StarSystem> {
        (**self).star_map()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        (**self).commodity_strings()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        (**self).junk()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        (**self).disasters()
    }
}

/// A shared catalog is a catalog, so the sim and the views can read the
/// same game data.
impl<T: PilotCatalog + ?Sized> PilotCatalog for Rc<T> {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        (**self).first_character()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        (**self).ship_fields(id)
    }

    fn default_outfits(&self, id: ShipId) -> Vec<OutfitMod> {
        (**self).default_outfits(id)
    }

    fn system_exists(&self, id: SystemId) -> bool {
        (**self).system_exists(id)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        (**self).landing_sites(system)
    }

    fn star_map(&self) -> Vec<StarSystem> {
        (**self).star_map()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        (**self).commodity_strings()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        (**self).junk()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        (**self).disasters()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ship 128 is average; system 130 alone exists.
    struct One;

    impl PilotCatalog for One {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
                start: StartDate {
                    day: 23,
                    month: 6,
                    year: 1177,
                },
                ..CharacterStart::default()
            })
        }

        fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
            (id == ShipId(128))
                .then_some(ShipFields {
                    speed: 300,
                    accel: 300,
                    maneuver: 10,
                    ..ShipFields::default()
                })
                .ok_or_else(|| format!("no shïp {}", id.0))
        }

        /// Ship 128 carries two of an outfit whose mod is 18, 8.
        fn default_outfits(&self, id: ShipId) -> Vec<OutfitMod> {
            if id != ShipId(128) {
                return Vec::new();
            }
            vec![OutfitMod {
                mod_type: 18,
                mod_val: 8,
                count: 2,
            }]
        }

        fn system_exists(&self, id: SystemId) -> bool {
            id == SystemId(130)
        }

        /// System 130 holds stellar 128 at (1, 2); no other system has
        /// any.
        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            if system != SystemId(130) {
                return Vec::new();
            }
            vec![LandingSite {
                id: StellarId(128),
                position: Vec2::new(1.0, 2.0),
                frame_size: Some((10, 20)),
                flags: 0x01,
                min_status: 0,
                landing_sound: None,
            }]
        }

        /// System 130 at (5, -6), linked to 131.
        fn star_map(&self) -> Vec<StarSystem> {
            vec![StarSystem {
                id: SystemId(130),
                position: Vec2::new(5.0, -6.0),
                links: vec![SystemId(131)],
            }]
        }

        /// Food at 75.
        fn commodity_strings(&self) -> CommodityStrings {
            CommodityStrings {
                names: vec!["Food".to_owned()],
                base_prices: vec!["75".to_owned()],
            }
        }

        /// Opals, sold at stellar 128.
        fn junk(&self) -> Vec<JunkRecord> {
            vec![JunkRecord {
                id: JunkId(146),
                name: "Opals".to_owned(),
                base_price: 1200,
                sold_at: vec![StellarId(128)],
                bought_at: Vec::new(),
                buy_on: String::new(),
                sell_on: String::new(),
            }]
        }

        /// A food surplus at stellar 128.
        fn disasters(&self) -> Vec<DisasterRecord> {
            vec![DisasterRecord {
                id: DisasterId(128),
                name: "An enormous food surplus".to_owned(),
                stellar: 128,
                ..DisasterRecord::default()
            }]
        }
    }

    /// Everything `catalog` says about ships 128 and 129 and their
    /// outfits, systems 130 and 131 and the star map.
    fn reads(catalog: impl PilotCatalog) -> Vec<String> {
        vec![
            format!("{:?}", catalog.first_character()),
            format!("{:?}", catalog.ship_fields(ShipId(128))),
            format!("{:?}", catalog.ship_fields(ShipId(129))),
            format!("{}", catalog.system_exists(SystemId(130))),
            format!("{}", catalog.system_exists(SystemId(131))),
            format!("{:?}", catalog.landing_sites(SystemId(130))),
            format!("{:?}", catalog.landing_sites(SystemId(131))),
            format!("{:?}", catalog.star_map()),
            format!("{:?}", catalog.default_outfits(ShipId(128))),
            format!("{:?}", catalog.default_outfits(ShipId(129))),
            format!("{:?}", catalog.commodity_strings()),
            format!("{:?}", catalog.junk()),
            format!("{:?}", catalog.disasters()),
        ]
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        let direct = reads(One);
        assert!(direct[1].contains("speed: 300"), "{direct:?}");
        assert_eq!(direct[2], r#"Err("no shïp 129")"#);
        assert_eq!(direct[3..5], ["true", "false"]);
        assert!(direct[5].contains("StellarId(128)"), "{direct:?}");
        assert_eq!(direct[6], "[]");
        assert!(direct[0].contains("year: 1177"), "{direct:?}");
        assert!(direct[7].contains("SystemId(131)"), "{direct:?}");
        assert!(direct[8].contains("count: 2"), "{direct:?}");
        assert_eq!(direct[9], "[]");
        assert!(direct[10].contains("\"75\""), "{direct:?}");
        assert!(direct[11].contains("Opals"), "{direct:?}");
        assert!(direct[12].contains("food surplus"), "{direct:?}");
        assert_eq!(reads(&One), direct);
        assert_eq!(reads(Rc::new(One)), direct);
    }

    #[test]
    fn each_error_reads_as_a_sentence() {
        assert_eq!(StartError::NoCharacter.to_string(), "no chär to start from");
        assert_eq!(
            StartError::Character("chär 128: too short".to_owned()).to_string(),
            "chär 128: too short"
        );
        assert_eq!(StartError::NoShip.to_string(), "the first chär has no ship");
        assert_eq!(
            StartError::Ship(ShipId(128), "no shïp 128".to_owned()).to_string(),
            "no shïp 128"
        );
        assert_eq!(
            StartError::NoStartingSystem([None, Some(SystemId(999)), None, Some(SystemId(5))])
                .to_string(),
            "none of the first chär's starting systems (none, 999, none, 5) exists"
        );
        assert_eq!(
            StartError::NoSystem(SystemId(130)).to_string(),
            "the pilot's system, sÿst 130, does not exist"
        );
    }
}
