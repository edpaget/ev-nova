//! The pilot catalog port: what a flight session starts from, in the
//! simulation's own terms.

use std::rc::Rc;

pub use nova_data::{GovtId, JunkId, OutfitId, ShipId, SoundId, StellarId, SystemId};

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

/// What the date is wrapped in wherever it is displayed: the first
/// `chär`'s `DatePrefix` and `DateSuffix`, verbatim.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DateAffixes {
    /// `DatePrefix`: put before the date.
    pub prefix: String,
    /// `DateSuffix`: put after the date (stock " NC").
    pub suffix: String,
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
    /// Its controlling government, `Govt`, or `None` when it is
    /// independent (-1).
    pub govt: Option<GovtId>,
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
    /// Its `TechLevel`: the [`outfitter`](crate::outfitter) sells outfits
    /// up to it.
    pub tech_level: i16,
    /// Its `SpecialTech` 1-8, in order: the outfitter also sells outfits of
    /// exactly these tech levels.
    pub special_tech: [i16; 8],
    /// Its `Govt`, or `None` when it is independent.
    pub govt: Option<GovtId>,
    /// Its `Flags2`, raw: the [`gate`](crate::gate) rules read its
    /// hypergate (0x1000) and wormhole (0x2000) bits.
    pub flags2: u16,
}

/// A stellar a hypergate or wormhole may lead to, raw from its `spöb`: the
/// [`gate`](crate::gate) rules decide what the values mean. Any stellar a
/// system lists may be one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GateSite {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// The system it is in: the lowest-ID `sÿst` whose `NavDefs` list it,
    /// as the original's `_FindSystemFromStellar` finds it when every
    /// system is active.
    pub system: SystemId,
    /// Its centre, `xPos` and `yPos`, in pixels from the system's centre.
    pub position: Vec2,
    /// Its `Flags2`, raw.
    pub flags2: u16,
    /// Its `HyperLink1-8`, in slot order, `None` for an unused one (-1).
    /// The Bible's other unused value, 0, is kept: no system lists a
    /// stellar 0, so it leads nowhere.
    pub links: [Option<StellarId>; 8],
    /// Its `CustSndID`, raw: the heading a ship comes out of it on, when
    /// it is 0 to 359.
    pub exit_angle: i16,
}

/// An outfit, raw from its `oütf`: the [`outfitter`](crate::outfitter)
/// rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitRecord {
    /// The `oütf`'s ID.
    pub id: OutfitId,
    /// Its name: the resource's name, or its `LCName` when the resource
    /// has none.
    pub name: String,
    /// Its `ShortName`, raw: a literal `\n` splits it in two lines.
    pub short_name: String,
    /// Its `DispWeight`: higher shows nearer the top.
    pub disp_weight: i16,
    /// Its `Mass`, in tons.
    pub mass: i16,
    /// Its `TechLevel`.
    pub tech_level: i16,
    /// Its `Max`: how many the player can own.
    pub max: i16,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Cost`.
    pub cost: i32,
    /// Its `ModType` and `ModVal` pairs 1-4, in order.
    pub mods: [(i16, i16); 4],
    /// Its `Contribute` bits.
    pub contribute: u64,
    /// Its `Require` bits.
    pub require: u64,
    /// Its `RequireGovt`, raw.
    pub require_govt: i16,
    /// Its `Availability` control-bit expression.
    pub availability: String,
}

/// A ship class, raw from its `shïp`: the [`shipyard`](crate::shipyard)
/// rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipRecord {
    /// The `shïp`'s ID.
    pub id: ShipId,
    /// Its name: the resource's name, or its `ShortName` on one line (each
    /// literal `\n` a space) when the resource has none.
    pub name: String,
    /// Its `ShortName`, raw: a literal `\n` splits it in two lines.
    pub short_name: String,
    /// Its `Long Name`.
    pub long_name: String,
    /// Its handling, reserve, cargo and mass fields.
    pub fields: ShipFields,
    /// Its `DefaultItems`, raw: each item's `oütf` ID with its count, in
    /// slot order, a negative count as none (as
    /// [`PilotCatalog::default_outfits`] gives them).
    pub defaults: Vec<(OutfitId, u16)>,
    /// Its `Cost`.
    pub cost: i32,
    /// Its `TechLevel`.
    pub tech_level: i16,
    /// Its `BuyRandom`: the percent chance a day it is for sale.
    pub buy_random: i16,
    /// Its `Require` bits.
    pub require: u64,
    /// Its `Availability` control-bit expression.
    pub availability: String,
    /// Its `Flags3`.
    pub flags3: u16,
    /// Its `DispWeight`: higher shows nearer the top.
    pub disp_weight: i16,
    /// Its `MaxGun`.
    pub max_gun: i16,
    /// Its `MaxTur`.
    pub max_tur: i16,
    /// Its `Length`, in metres.
    pub length: i16,
    /// Its `Crew`.
    pub crew: i16,
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
    /// Ship `id`'s default outfits, its `DefaultItems`, raw: each item's
    /// `oütf` ID with its count, in slot order, a negative count as none.
    /// None for a ship that cannot be read.
    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)>;
    /// Every `oütf` that can be read, by ascending ID.
    fn outfits(&self) -> Vec<OutfitRecord>;
    /// Every `shïp` that can be read, by ascending ID.
    fn ships(&self) -> Vec<ShipRecord>;
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
    /// The first `chär`'s `DatePrefix` and `DateSuffix`, or none (both
    /// empty) when there is no `chär` or it does not decode.
    fn date_affixes(&self) -> DateAffixes;
    /// Every stellar some system that can be read lists, that can be read
    /// itself, by ascending ID, with the system it is in: where a
    /// hypergate or wormhole may lead.
    fn gate_sites(&self) -> Vec<GateSite>;
}

/// A borrowed catalog is a catalog.
impl<T: PilotCatalog + ?Sized> PilotCatalog for &T {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        (**self).first_character()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        (**self).ship_fields(id)
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        (**self).default_outfits(id)
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        (**self).outfits()
    }

    fn ships(&self) -> Vec<ShipRecord> {
        (**self).ships()
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

    fn date_affixes(&self) -> DateAffixes {
        (**self).date_affixes()
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        (**self).gate_sites()
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

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        (**self).default_outfits(id)
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        (**self).outfits()
    }

    fn ships(&self) -> Vec<ShipRecord> {
        (**self).ships()
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

    fn date_affixes(&self) -> DateAffixes {
        (**self).date_affixes()
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        (**self).gate_sites()
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

        /// Ship 128 carries two of outfit 200.
        fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
            if id != ShipId(128) {
                return Vec::new();
            }
            vec![(OutfitId(200), 2)]
        }

        /// Outfit 200, a fuel scoop.
        fn outfits(&self) -> Vec<OutfitRecord> {
            vec![OutfitRecord {
                id: OutfitId(200),
                name: "Scoop".to_owned(),
                short_name: "Scoop".to_owned(),
                disp_weight: 0,
                mass: 1,
                tech_level: 1,
                max: 1,
                flags: 0,
                cost: 100,
                mods: [(18, 8), (0, 0), (0, 0), (0, 0)],
                contribute: 0,
                require: 0,
                require_govt: -1,
                availability: String::new(),
            }]
        }

        /// Ship 128, the Shuttle.
        fn ships(&self) -> Vec<ShipRecord> {
            vec![ShipRecord {
                name: "Shuttle".to_owned(),
                ..crate::testkit::ship(128, ShipFields::default())
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
                tech_level: 3,
                special_tech: [0; 8],
                govt: None,
                flags2: 0,
            }]
        }

        /// Hypergate 1400 in system 130, linked to 1405.
        fn gate_sites(&self) -> Vec<GateSite> {
            let mut links = [None; 8];
            links[0] = Some(StellarId(1405));
            vec![GateSite {
                id: StellarId(1400),
                system: SystemId(130),
                position: Vec2::new(-70.0, 250.0),
                flags2: 0x1200,
                links,
                exit_angle: 120,
            }]
        }

        /// System 130 at (5, -6), linked to 131.
        fn star_map(&self) -> Vec<StarSystem> {
            vec![StarSystem {
                id: SystemId(130),
                position: Vec2::new(5.0, -6.0),
                links: vec![SystemId(131)],
                govt: None,
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

        /// Dates read "Year ... NC".
        fn date_affixes(&self) -> DateAffixes {
            DateAffixes {
                prefix: "Year ".to_owned(),
                suffix: " NC".to_owned(),
            }
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
    /// outfits, systems 130 and 131, the star map, the goods and the
    /// outfits.
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
            format!("{:?}", catalog.outfits()),
            format!("{:?}", catalog.ships()),
            format!("{:?}", catalog.date_affixes()),
            format!("{:?}", catalog.gate_sites()),
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
        assert_eq!(direct[8], "[(OutfitId(200), 2)]");
        assert_eq!(direct[9], "[]");
        assert!(direct[10].contains("\"75\""), "{direct:?}");
        assert!(direct[11].contains("Opals"), "{direct:?}");
        assert!(direct[12].contains("food surplus"), "{direct:?}");
        assert!(direct[13].contains("Scoop"), "{direct:?}");
        assert!(direct[14].contains("Shuttle"), "{direct:?}");
        assert_eq!(
            direct[15],
            r#"DateAffixes { prefix: "Year ", suffix: " NC" }"#
        );
        assert!(direct[16].contains("StellarId(1405)"), "{direct:?}");
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
