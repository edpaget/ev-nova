//! The ship-change set operators on the session: `C`, `E` and `H` change
//! the player's ship outside the shipyard, and `T` renames it.
//!
//! **The original.** `_EvalSetExp` (@0x150fc) dispatches on the letter
//! through the table at 0xdd2fc (@0x15cc2). `C`, `E` and `H` all jump to
//! one inline block (@0x15493-0x156da), which tests the letter again
//! where they differ; there is no separate change-ship routine. In order:
//!
//! | Step | Where | What |
//! |---|---|---|
//! | 1 | @0x15493-0x154a2 | an ID outside 128 to 895 does nothing; the `shïp`'s existence is not checked, its slot indexed blindly |
//! | 2 | `_ShipStatsToSystemInfo` @0xcaee, called @0x154b3 | the ship's weapons and their ammunition are folded into the outfit counts: each onto the first outfit of `ModType` 1 (or 3, for ammunition) whose `ModVal` names it; one that no outfit backs is lost |
//! | 3 | @0x154c1-0x154ed, `H` only | every outfit whose `Flags & 0x24` is none is zeroed: `H` keeps an outfit flagged 0x0004 *or* 0x0020 |
//! | 4 | `_SystemInfoToShipStats` @0x154ef, then @0x154fc | the outfits are written back, and the class is set |
//! | 5 | @0x1551c-0x15668, `E` and `H` only | the new class's `WCount`s and `AmmoLoad`s are added to the weapons, each of its `DefaultItems` slots of count above none to the outfits (`+=`, @0x15609), folded again (@0x15622), and then every outfit owned, old and new, is clamped to its limit by `_HasMaxOfItem` (@0x4512, called @0x15648: `if 0 < lim < count { count = lim }`). `C` clamps nothing |
//! | 6 | `_SystemInfoToShipStats` @0x1566a | the stats are worked out again |
//! | 7 | @0x15677-0x156c5 | while `_IsDisabled` holds for the player and its armour is below `_ShipArmorCapacity`, the armour gains 1.0 (the constant @0xdd068) |
//! | 8 | @0x156c7-0x156d5 | `_LoadIntfResource`, `_RestoreBuffer` |
//!
//! The block refills no reserve and clamps none (step 0 of the phase's
//! reading): flight regenerates the shield and armour only while they
//! are below their most (`_HandlePlayer` @0x6cf99 and @0x6d01d) and
//! clamps the fuel alone each frame (@0x6d9e1), the outfitter clamps all
//! three as it closes (`_DoOutfitDialog` @0x5da4b-0x5dad4), and landing
//! (`_PlayerLandOnStellar` @0x63421) and loading a pilot
//! (`_LoadPilotData` @0x754dc) fill the shield and armour. So a shield or
//! armour above the new class's most lasts, in flight, until damage
//! takes it: [`RuleKey::ShipChangeReserves`].
//! The block never calls `_DestroyPartialFleetCargo`, so the cargo is
//! kept even past the new hold; it leaves the escorts and the fighters
//! out alone, checks no mass, moves no cash, draws no `_Rand`, and does
//! not branch on `_playerIsLanded`, so landed and in flight are alike.
//! It never calls `_EvalMissionBitSetString`, so no `OnPurchase`,
//! `OnCapture` or `OnRetire` runs, and it names and paints nothing.
//!
//! The shipyard by contrast (`_DoShipyardDialog` ~0x5ebc8-0x5f0ac) trades
//! in, runs `OnRetire` then `OnPurchase`, names the ship from its dialog
//! (`_CullNameString`, 0x40), clears the fighters out, strips the outfits
//! whose persistent byte (+0x378, which the loader sets from 0x0004 alone,
//! @0x78cad) is clear, adds the defaults (`+=`), keeps the cargo that fits
//! (`_DestroyPartialFleetCargo(0)`) and fills the reserves, with no
//! `_HasMaxOfItem` clamp. Capture (`_DoShipCapture`) strips by +0x378 too.
//!
//! **Here.** [`Session::change_ship`] is that block, for the set
//! operators alone: with no record of the class nothing changes (the
//! original's blind indexing of a missing `shïp` is not reproduced);
//! otherwise the outfits carry over as
//! [`carried_outfits`] says, the
//! session flies the new class (its fields and default items from its
//! record, the hull, weapons and stats from them and the outfits), each
//! reserve held to its new most unless
//! [`ShipChangeRules::reserves`] keeps a shield or armour above it, the
//! cargo kept or trimmed as [`ShipChangeRules::cargo`] says, and the
//! armour raised a point at a time, never past its most, while the
//! session's [`DisableRule`] has the ship disabled
//! ([`Session::with_disable_rule`]; [`NovaDisable`] by default). Its
//! position, velocity and heading, condition, cash, name, paint, escorts
//! and fighters out are left as they were, no hook runs, and a save is
//! due.
//!
//! A shield or armour kept above its most lasts until damage takes it or
//! the ship is next refitted, which holds every reserve to its most: a
//! later `G` or `D`, in the same expression or after, or flying the pilot
//! again from a save ([`Session::fly`]; the save itself keeps the
//! surplus). The reload agrees with the original,
//! whose `_LoadPilotData` fills the shield and armour (@0x754dc), so no
//! surplus outlasts a load there either; the refit does not (below).
//!
//! - `Cxxx` ([`ChangeShipOp`]) keeps every outfit and adds no default.
//! - `Exxx` ([`ChangeShipWithDefaultsOp`]) keeps every outfit and adds the
//!   new class's default items; by the engine every outfit is then held
//!   to its `Max` ([`ShipChangeRules::max`]).
//! - `Hxxx` ([`ReplaceShipOp`]) keeps only the persistent outfits, as
//!   [`ShipChangeRules::persistence`] says, adds the default items, and
//!   holds them as `E` does.
//!
//! Not yet as the original: the ship's stock weapons are mounted beside
//! its outfits, not owned as outfits, so `C` mounts the new class's stock
//! weapons and drops the old class's, where the original keeps the old
//! ones an outfit backs and gives no new ones, and `E` and `H`'s
//! `WCount` and `AmmoLoad` are only approximated by that mount; and the
//! limit held to is `Max` alone, without `ModType` 27, `MaxAmmo` times
//! the launchers, or the gun and turret slots. A later `G` or `D` holds a
//! kept surplus to the most, where the original's, which works the stats
//! out with the same `_SystemInfoToShipStats` that clamps no reserve in
//! the block above, leaves it to damage.
//! [`Session::buy_ship`] and [`Session::assign`] keep their own carry,
//! which folding onto [`carried_outfits`]
//! waits for: a purchase's carry is a trade (`Max` and the free mass,
//! the sale of the rest, the trade-in), and Use As My Ship keeps its own
//! reserves and hook order.
//!
//! **`Txxx`** ([`RenameShipOp`], @0x15b25-0x15be9) reads `STR#` `xxx`,
//! the raw ID, with no range check, from the session's string lists
//! ([`Session::with_strings`]; none by default, so `T` does nothing). It
//! picks one of its strings evenly, a roll of as many sides as the list
//! holds (`_GetRandomIndString` @0x72ef7: `Rand(count) + 1` into
//! `GetIndString`). A missing or empty list, or an empty pick (the draw
//! spent), changes nothing (@0x15b47). Otherwise the pick becomes the
//! ship's name, with every `*` in it replaced by the whole name before
//! (@0x15b7a-0x15b97): the original's `_shipName`, the one name the
//! new-pilot dialog and the shipyard set too. The original's 8-bit length
//! counter garbles a name past 255 characters, reachable only by a `*`
//! in a long name; it is not reproduced, and the name has no limit.

use std::fmt;
use std::rc::Rc;

use super::Session;
use super::hire::Shared;
use crate::catalog::{CommCatalog, ShipId};
use crate::chance::Chance;
use crate::combat::hull::{DisableRule, NovaDisable};
use crate::control::{SetOp, SetOpHandler};
use crate::outfitter::OutfitFlags;
use crate::pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::ship_change::{OutfitCarry, SET_OP_PERSISTENT, carried_outfits};
use crate::shipyard;

/// What the armour gains a step while the ship is disabled after a
/// change (the constant @0xdd068).
const ARMOR_BUMP: f32 = 1.0;

/// The disputed rules of the ship-change set operators that the session
/// follows (see [`Session::with_ship_change_rules`]): the engine's by
/// default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipChangeRules {
    /// Which outfits `H` keeps ([`RuleKey::ShipChangePersistence`]).
    pub persistence: RuleSource,
    /// Whether `E` and `H` hold the outfits to their `Max`
    /// ([`RuleKey::ShipChangeMax`]).
    pub max: RuleSource,
    /// Whether `C`, `E` and `H` keep the cargo past the new hold
    /// ([`RuleKey::ShipChangeCargo`]).
    pub cargo: RuleSource,
    /// Whether `C`, `E` and `H` keep a shield or armour above the new
    /// most ([`RuleKey::ShipChangeReserves`]).
    pub reserves: RuleSource,
}

impl ShipChangeRules {
    /// The rules `rulebook` chooses: its
    /// [`RuleKey::ShipChangePersistence`], [`RuleKey::ShipChangeMax`],
    /// [`RuleKey::ShipChangeCargo`] and [`RuleKey::ShipChangeReserves`]
    /// entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            persistence: rulebook.source_for(RuleKey::ShipChangePersistence),
            max: rulebook.source_for(RuleKey::ShipChangeMax),
            cargo: rulebook.source_for(RuleKey::ShipChangeCargo),
            reserves: rulebook.source_for(RuleKey::ShipChangeReserves),
        }
    }

    /// The flags `H` keeps an outfit by.
    fn persistent(self) -> u16 {
        match self.persistence {
            RuleSource::Engine => SET_OP_PERSISTENT,
            RuleSource::Bible => OutfitFlags::MISSION_PERSISTENT,
        }
    }

    /// Whether `E` and `H` hold the outfits to their `Max`.
    fn clamps(self) -> bool {
        self.max == RuleSource::Engine
    }
}

/// The game's string lists, as `T` reads them, shared.
#[derive(Clone)]
pub(super) struct Strings(pub(super) Rc<dyn CommCatalog>);

impl Strings {
    /// None at all.
    pub(super) fn none() -> Self {
        Self(Rc::new(NoStrings))
    }
}

impl fmt::Debug for Strings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Strings")
    }
}

impl PartialEq for Strings {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// No string lists.
struct NoStrings;

impl CommCatalog for NoStrings {
    fn string_list(&self, _id: i16) -> Vec<String> {
        Vec::new()
    }
}

impl Session {
    /// This session with the ship-change set operators following `rules`
    /// where the Bible and the engine disagree: the engine's by default.
    #[must_use]
    pub fn with_ship_change_rules(mut self, rules: ShipChangeRules) -> Self {
        self.ship_change_rules = rules;
        self
    }

    /// The rules the ship-change set operators follow.
    #[must_use]
    pub fn ship_change_rules(&self) -> ShipChangeRules {
        self.ship_change_rules
    }

    /// This session with `rule` saying when the player's ship is
    /// disabled after a change of ship (see the module docs):
    /// [`NovaDisable`] by default. The fight's own rule is given each
    /// tick ([`Session::tick_combat`]).
    #[must_use]
    pub fn with_disable_rule(mut self, rule: Rc<dyn DisableRule>) -> Self {
        self.disable_rule = Shared(rule);
        self
    }

    /// The rule that says when the player's ship is disabled after a
    /// change of ship.
    #[must_use]
    pub fn disable_rule(&self) -> &dyn DisableRule {
        &*self.disable_rule.0
    }

    /// This session with `T` reading its names from `strings`' string
    /// lists (see the module docs): none by default.
    #[must_use]
    pub fn with_strings(mut self, strings: Rc<dyn CommCatalog>) -> Self {
        self.strings = Strings(strings);
        self
    }

    /// The string lists `T` reads its names from.
    #[must_use]
    pub fn strings(&self) -> &dyn CommCatalog {
        &*self.strings.0
    }

    /// The disable rule a session starts with.
    pub(super) fn nova_disable() -> Shared<dyn DisableRule> {
        Shared(Rc::new(NovaDisable))
    }

    /// Changes the player's ship to class `ship`, its outfits carried as
    /// `carry` says and held to their `Max` when `clamp` (see the module
    /// docs); nothing changes when the session has no record of the
    /// class.
    fn change_ship(&mut self, ship: ShipId, carry: OutfitCarry, clamp: bool) {
        let Some(record) = self.ship_record(ship).cloned() else {
            return;
        };
        let before = self.pilot.reserves;
        let defaults = pilot::tally(record.defaults.iter().copied());
        self.pilot.outfits =
            carried_outfits(&self.pilot.outfits, carry, &defaults, clamp, &self.outfits);
        self.pilot.ship = record.id;
        self.fields = record.fields;
        self.defaults = defaults;
        self.refit(false);
        if self.ship_change_rules.reserves == RuleSource::Engine {
            let reserves = &mut self.pilot.reserves;
            reserves.shield.now = before.shield.now;
            reserves.armor.now = before.armor.now;
        }
        if self.ship_change_rules.cargo == RuleSource::Bible {
            shipyard::keep_cargo(&mut self.pilot, self.stats.capacity);
        }
        self.bump_armor();
        self.save_due = true;
    }

    /// Raises the player's armour [`ARMOR_BUMP`] at a time, never past
    /// its most, while the session's disable rule has its ship disabled,
    /// as the original's loop does (@0x15677-0x156c5). It ends at the
    /// most, so a rule that never lets the ship fly still ends.
    fn bump_armor(&mut self) {
        let rule = Rc::clone(&self.disable_rule.0);
        let armor = &mut self.pilot.reserves.armor;
        while armor.now < armor.max && rule.disabled(*armor, &self.hull) {
            armor.now = (armor.now + ARMOR_BUMP).min(armor.max);
        }
    }

    /// `T`: renames the ship from `STR#` `list`, drawing on `chance` (see
    /// the module docs).
    fn rename_ship(&mut self, list: i16, chance: &mut dyn Chance) {
        let names = self.strings.0.string_list(list);
        if names.is_empty() {
            return;
        }
        let sides = u16::try_from(names.len()).unwrap_or(u16::MAX);
        let Some(pick) = names.get(usize::from(chance.roll(sides))) else {
            return;
        };
        if pick.is_empty() {
            return;
        }
        let previous = self.pilot.ship_name.as_deref().unwrap_or_default();
        self.pilot.ship_name = Some(pick.replace('*', previous));
    }
}

/// `Cxxx`: changes the ship, keeping every outfit (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct ChangeShipOp;

impl SetOpHandler<Session> for ChangeShipOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::ChangeShip(ship) = op {
            session.change_ship(*ship, OutfitCarry::Keep, false);
        }
    }
}

/// `Exxx`: changes the ship, keeping every outfit and adding the new
/// class's default items (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct ChangeShipWithDefaultsOp;

impl SetOpHandler<Session> for ChangeShipWithDefaultsOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::ChangeShipWithDefaults(ship) = op {
            let clamp = session.ship_change_rules.clamps();
            session.change_ship(*ship, OutfitCarry::KeepWithDefaults, clamp);
        }
    }
}

/// `Hxxx`: changes the ship, keeping the persistent outfits and adding
/// the new class's default items (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct ReplaceShipOp;

impl SetOpHandler<Session> for ReplaceShipOp {
    fn apply(&self, op: &SetOp, session: &mut Session, _chance: &mut dyn Chance) {
        if let SetOp::ReplaceShip(ship) = op {
            let rules = session.ship_change_rules;
            let carry = OutfitCarry::Persistent {
                mask: rules.persistent(),
            };
            session.change_ship(*ship, carry, rules.clamps());
        }
    }
}

/// `Txxx`: renames the ship from a string list (see the module docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct RenameShipOp;

impl SetOpHandler<Session> for RenameShipOp {
    fn apply(&self, op: &SetOp, session: &mut Session, chance: &mut dyn Chance) {
        if let SetOp::RenameShip(list) = op {
            session.rename_ship(list.0, chance);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{HullRecord, OutfitId, OutfitRecord, ShipRecord, StockWeapon, WeaponId};
    use crate::combat::hull::{HullSpec, TOUGH};
    use crate::control::{Bit, Script, SetExpr, SetOpKind};
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::market::Good;
    use crate::outfit_effects::Rgb15;
    use crate::outfitter::outfit_mods;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::{Gauge, Reserves};
    use crate::save;
    use crate::stats::{MORE_SHIELD, ShipStats};
    use crate::testkit::{
        FAST, FakePilotCatalog, Scripted, catalog, hull, land_now, outfit, ship, weapon,
    };

    /// Ship 129: shield 300, armour 100, fuel 400, a hold of 50 and 60
    /// tons free.
    const BIG: ShipFields = ShipFields {
        shield: 300,
        armor: 100,
        fuel: 400,
        holds: 50,
        free_mass: 60,
        ..FAST
    };
    /// Ship 130: shield 10, armour 20, fuel 100 and a hold of 5.
    const SMALL: ShipFields = ShipFields {
        shield: 10,
        armor: 20,
        fuel: 100,
        holds: 5,
        ..FAST
    };
    /// Ship 131: armour 1000.
    const TANKY: ShipFields = ShipFields {
        armor: 1000,
        ..FAST
    };

    const LICENCE: OutfitId = OutfitId(400);
    const ABILITY: OutfitId = OutfitId(401);
    const BOTH: OutfitId = OutfitId(402);
    const SHIELD: OutfitId = OutfitId(403);
    const LIMITED: OutfitId = OutfitId(404);

    /// The pilot flies ship 128 (FAST: shield 30, armour 45, fuel 300, a
    /// hold of 20). Outfits: 400 a licence (0x000c), 401 an ability
    /// (0x0020), 402 both (0x0024), 403 a plain shield of +50, 404 a
    /// plain outfit of `Max` 2. Ship 129 ([`BIG`]) mounts weapon 200 and
    /// carries a shield and two of 404 by default; 130 is [`SMALL`]; 131
    /// is [`TANKY`]. Ship 128 runs `S1` on retiring, and 129 and 130 run
    /// `S2` on purchase and `S3` on capture.
    fn changing() -> FakePilotCatalog {
        let flagged = |id, flags| OutfitRecord {
            flags,
            ..outfit(id, &[])
        };
        let hooked = |id, fields| ShipRecord {
            on_purchase: Script::parse("S2"),
            on_capture: Script::parse("S3"),
            ..ship(id, fields)
        };
        let defaults = vec![(SHIELD, 1), (LIMITED, 2)];
        FakePilotCatalog {
            ships: vec![
                (ShipId(128), Ok(FAST)),
                (ShipId(129), Ok(BIG)),
                (ShipId(130), Ok(SMALL)),
                (ShipId(131), Ok(TANKY)),
            ],
            defaults: vec![(ShipId(129), defaults.clone())],
            ship_records: vec![
                ShipRecord {
                    on_retire: Script::parse("S1"),
                    ..ship(128, FAST)
                },
                ShipRecord {
                    defaults,
                    ..hooked(129, BIG)
                },
                hooked(130, SMALL),
                ship(131, TANKY),
            ],
            outfits: vec![
                flagged(400, 0x000c),
                flagged(401, 0x0020),
                flagged(402, 0x0024),
                outfit(403, &[(MORE_SHIELD, 50)]),
                OutfitRecord {
                    max: 2,
                    ..outfit(404, &[])
                },
            ],
            hulls: vec![
                hull(128),
                HullRecord {
                    weapons: vec![StockWeapon {
                        weapon: WeaponId(200),
                        count: 1,
                        ammo: 0,
                    }],
                    ..hull(129)
                },
            ],
            weapons: vec![weapon(200)],
            ..catalog()
        }
    }

    fn owned(pairs: &[(OutfitId, u16)]) -> BTreeMap<OutfitId, u16> {
        pairs.iter().copied().collect()
    }

    /// The pilot of [`changing`], in flight, owning one licence, ability
    /// and both-flagged outfit, two shields and one of 404, with the
    /// reserves full for that.
    fn session(catalog: &FakePilotCatalog) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.outfits = owned(&[
            (LICENCE, 1),
            (ABILITY, 1),
            (BOTH, 1),
            (SHIELD, 2),
            (LIMITED, 1),
        ]);
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.pilot.reserves = session.stats().full();
        session.take_save_due();
        session
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    // The rules.

    #[test]
    fn the_ship_change_rules_are_the_engines_until_others_are_given() {
        let session = session(&changing());
        assert_eq!(session.ship_change_rules(), ShipChangeRules::default());
        let rules = ShipChangeRules {
            cargo: RuleSource::Bible,
            ..ShipChangeRules::default()
        };
        assert_eq!(
            session.with_ship_change_rules(rules).ship_change_rules(),
            rules
        );
    }

    #[test]
    fn the_ship_change_rules_follow_their_four_rulebook_keys() {
        assert_eq!(
            ShipChangeRules::from_rulebook(&Rulebook::default()),
            ShipChangeRules::default()
        );
        assert_eq!(
            ShipChangeRules::default(),
            ShipChangeRules {
                persistence: RuleSource::Engine,
                max: RuleSource::Engine,
                cargo: RuleSource::Engine,
                reserves: RuleSource::Engine,
            }
        );
        let bible = |key| {
            ShipChangeRules::from_rulebook(
                &Rulebook::default().with_override(key, RuleSource::Bible),
            )
        };
        assert_eq!(
            bible(RuleKey::ShipChangePersistence),
            ShipChangeRules {
                persistence: RuleSource::Bible,
                ..ShipChangeRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::ShipChangeMax),
            ShipChangeRules {
                max: RuleSource::Bible,
                ..ShipChangeRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::ShipChangeCargo),
            ShipChangeRules {
                cargo: RuleSource::Bible,
                ..ShipChangeRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::ShipChangeReserves),
            ShipChangeRules {
                reserves: RuleSource::Bible,
                ..ShipChangeRules::default()
            }
        );
        assert_eq!(bible(RuleKey::CrimeGains), ShipChangeRules::default());
    }

    // C.

    #[test]
    fn c_flies_the_new_class_keeping_every_outfit_and_adding_no_default() {
        let catalog = changing();
        let mut session = session(&catalog);
        let kept = session.pilot.outfits.clone();
        run(&mut session, "C129");
        assert_eq!(session.ship(), ShipId(129));
        assert_eq!(session.fields, BIG);
        assert_eq!(session.defaults, owned(&[(SHIELD, 1), (LIMITED, 2)]));
        assert_eq!(session.pilot.outfits, kept, "every outfit, no default");
        assert_eq!(
            session.stats(),
            ShipStats::new(BIG, &outfit_mods(&kept, &session.outfits))
        );
        assert_eq!(
            session.stats().shield,
            400.0,
            "the class's and the outfits'"
        );
        assert_eq!(session.hull(), session.arsenal.hull(ShipId(129)));
        assert_eq!(
            session.armament,
            session.arsenal.player(ShipId(129), &kept, &session.outfits)
        );
        assert_ne!(
            session.armament,
            session.arsenal.player(ShipId(128), &kept, &session.outfits),
            "the new class's stock weapon mounted"
        );
        assert!(session.take_save_due());
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn c_leaves_everything_but_the_ship_as_it_was() {
        let catalog = changing();
        let mut session = session(&catalog);
        session.pilot.cash = 777;
        session.pilot.paint = Some(Rgb15 { r: 1, g: 2, b: 3 });
        session.pilot.ship_name = Some("Kestrel".to_owned());
        session.pilot.cargo.insert(Good::Commodity(2), 4);
        session.pilot.escorts.push(Escort {
            ship: ShipId(130),
            reserves: Reserves::full(10.0, 20.0, 100.0),
            order: None,
            carried: true,
            wage: None,
            person: None,
        });
        session.player = ShipState {
            position: Vec2::new(5.0, -3.0),
            velocity: Vec2::new(0.25, 0.0),
            heading: 180.0,
        };
        let before = session.clone();
        run(&mut session, "C129");
        let (now, was) = (session.pilot(), before.pilot());
        assert_eq!(now.cash(), 777);
        assert_eq!(now.paint(), was.paint());
        assert_eq!(now.ship_name(), Some("Kestrel"));
        assert_eq!(now.cargo, was.cargo);
        assert_eq!(now.escorts(), was.escorts(), "the fighter out too");
        assert_eq!(session.player(), before.player());
        assert_eq!(session.player_condition(), before.player_condition());
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn c_refills_no_reserve_and_a_smaller_class_keeps_the_shield_and_armour_by_the_engine() {
        let catalog = changing();
        let mut session = session(&catalog);
        assert_eq!(session.reserves(), Reserves::full(130.0, 45.0, 300.0));
        run(&mut session, "C129");
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge {
                    now: 130.0,
                    max: 400.0
                },
                armor: Gauge {
                    now: 45.0,
                    max: 100.0
                },
                fuel: Gauge {
                    now: 300.0,
                    max: 400.0
                },
            },
            "a larger class is not refilled"
        );
        let mut session = self::session(&catalog);
        run(&mut session, "C130");
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge {
                    now: 130.0,
                    max: 110.0
                },
                armor: Gauge {
                    now: 45.0,
                    max: 20.0
                },
                fuel: Gauge::full(100.0),
            },
            "by the engine the shield and armour keep their surplus; the fuel is held"
        );
        let mut session = self::session(&catalog).with_ship_change_rules(ShipChangeRules {
            reserves: RuleSource::Bible,
            ..ShipChangeRules::default()
        });
        run(&mut session, "C130");
        assert_eq!(
            session.reserves(),
            Reserves::full(110.0, 20.0, 100.0),
            "by the other reading each is held to its new most"
        );
    }

    #[test]
    fn a_change_is_the_same_landed_and_in_flight() {
        let catalog = changing();
        for op in ["C130", "E129", "H129"] {
            let mut flying = session(&catalog);
            let mut landed = session(&catalog);
            land_now(&mut landed).expect("lands");
            run(&mut flying, op);
            run(&mut landed, op);
            let (flying, landed) = (flying.pilot(), landed.pilot());
            assert_eq!(landed.ship(), flying.ship(), "{op}");
            assert_eq!(landed.outfits, flying.outfits, "{op}");
            assert_eq!(landed.reserves(), flying.reserves(), "{op}");
        }
    }

    // Hooks.

    /// Counts each `S` it applies.
    #[derive(Debug, Default)]
    struct Probe(RefCell<Vec<i16>>);

    impl SetOpHandler<Session> for Probe {
        fn apply(&self, op: &SetOp, _session: &mut Session, _chance: &mut dyn Chance) {
            if let SetOp::StartMission(mission) = op {
                self.0.borrow_mut().push(mission.0);
            }
        }
    }

    #[test]
    fn no_change_of_ship_runs_a_hook() {
        let catalog = changing();
        for op in ["C129", "E129", "H129", "C130", "E130", "H130"] {
            let probe = Rc::new(Probe::default());
            let registry = crate::nova_set_ops().with(SetOpKind::StartMission, probe.clone());
            let mut session = session(&catalog).with_set_ops(Rc::new(registry));
            run(&mut session, op);
            assert_ne!(session.ship(), ShipId(128), "{op}: changed");
            assert!(probe.0.borrow().is_empty(), "{op}");
        }
    }

    // E and H.

    #[test]
    fn e_adds_the_new_classs_defaults_and_by_the_engine_holds_each_outfit_to_its_max() {
        let catalog = changing();
        let mut session = session(&catalog);
        run(&mut session, "E129");
        assert_eq!(
            session.pilot.outfits,
            owned(&[
                (LICENCE, 1),
                (ABILITY, 1),
                (BOTH, 1),
                (SHIELD, 3),
                (LIMITED, 2)
            ]),
            "one of 404 owned and two added, held to 2"
        );
        assert_eq!(
            session.stats(),
            ShipStats::new(BIG, &outfit_mods(&session.pilot.outfits, &session.outfits))
        );
        let mut session = self::session(&catalog);
        session.pilot.outfits.insert(LIMITED, 5);
        run(&mut session, "E130");
        assert_eq!(session.pilot.owned(LIMITED), 2, "an old one over its Max");
        let bible = ShipChangeRules {
            max: RuleSource::Bible,
            ..ShipChangeRules::default()
        };
        let mut session = self::session(&catalog).with_ship_change_rules(bible);
        run(&mut session, "E129");
        assert_eq!(session.pilot.owned(LIMITED), 3, "nothing held");
        assert_eq!(session.pilot.owned(SHIELD), 3);
    }

    #[test]
    fn h_keeps_the_persistent_outfits_as_its_rule_says_and_adds_the_defaults() {
        let catalog = changing();
        let mut session = session(&catalog);
        run(&mut session, "H129");
        assert_eq!(
            session.pilot.outfits,
            owned(&[
                (LICENCE, 1),
                (ABILITY, 1),
                (BOTH, 1),
                (SHIELD, 1),
                (LIMITED, 2)
            ]),
            "0x0004 or 0x0020 kept, the plain ones dropped, the defaults added"
        );
        let mut session = self::session(&catalog).with_ship_change_rules(ShipChangeRules {
            persistence: RuleSource::Bible,
            ..ShipChangeRules::default()
        });
        run(&mut session, "H130");
        assert_eq!(
            session.pilot.outfits,
            owned(&[(ABILITY, 1), (BOTH, 1)]),
            "by the other reading the licence (0x0004 only) goes too"
        );
    }

    #[test]
    fn h_holds_the_outfits_to_their_max_as_e_does() {
        // 404 kept by H, as persistent through a mission's change.
        let mut kept = changing();
        for record in &mut kept.outfits {
            if record.id == LIMITED {
                record.flags = OutfitFlags::MISSION_PERSISTENT;
            }
        }
        let mut session = session(&kept);
        run(&mut session, "H129");
        assert_eq!(session.pilot.owned(LIMITED), 2, "1 kept + 2 added, held");
        let mut session = self::session(&kept).with_ship_change_rules(ShipChangeRules {
            max: RuleSource::Bible,
            ..ShipChangeRules::default()
        });
        run(&mut session, "H129");
        assert_eq!(session.pilot.owned(LIMITED), 3);
    }

    #[test]
    fn a_class_with_no_record_changes_nothing() {
        let catalog = changing();
        for op in ["C200", "E200", "H200"] {
            let mut session = session(&catalog);
            let before = session.clone();
            run(&mut session, op);
            assert!(!session.take_save_due(), "{op}");
            assert_eq!(session, before, "{op}");
        }
    }

    // Cargo.

    #[test]
    fn by_the_engine_the_cargo_is_kept_past_the_new_hold_and_otherwise_trimmed() {
        let catalog = changing();
        let loaded = |rules| {
            let mut session = session(&catalog).with_ship_change_rules(rules);
            session.pilot.cargo = [(Good::Commodity(0), 3), (Good::Commodity(2), 9)]
                .into_iter()
                .collect();
            run(&mut session, "C130");
            session
        };
        let session = loaded(ShipChangeRules::default());
        assert_eq!(session.capacity(), 5);
        assert_eq!(
            session.pilot().cargo().collect::<Vec<_>>(),
            [(Good::Commodity(0), 3), (Good::Commodity(2), 9)],
            "every ton kept"
        );
        let session = loaded(ShipChangeRules {
            cargo: RuleSource::Bible,
            ..ShipChangeRules::default()
        });
        assert_eq!(
            session.pilot().cargo().collect::<Vec<_>>(),
            [(Good::Commodity(0), 3), (Good::Commodity(2), 2)],
            "trimmed in Good order"
        );
    }

    // The armour bump.

    #[test]
    fn a_change_that_leaves_the_ship_disabled_raises_its_armour_just_past_the_threshold() {
        let catalog = changing();
        let mut session = session(&catalog);
        run(&mut session, "C131");
        assert_eq!(
            session.reserves().armor,
            Gauge {
                now: 334.0,
                max: 1000.0
            },
            "45 raised a point at a time to a third"
        );
        assert!(!NovaDisable.disabled(session.reserves().armor, &session.hull()));
        let mut tough = changing();
        tough.hulls.push(HullRecord {
            flags: TOUGH,
            ..hull(131)
        });
        let mut session = self::session(&tough);
        run(&mut session, "C131");
        assert_eq!(session.reserves().armor.now, 100.0, "a tough hull: a tenth");
        let mut session = self::session(&catalog);
        run(&mut session, "C129");
        assert_eq!(session.reserves().armor.now, 45.0, "intact: untouched");
    }

    /// Disables a ship below `below` armour, recording each armour it is
    /// asked about.
    #[derive(Debug)]
    struct Below {
        below: f32,
        asked: RefCell<Vec<f32>>,
    }

    impl DisableRule for Below {
        fn disabled(&self, armor: Gauge, _hull: &HullSpec) -> bool {
            self.asked.borrow_mut().push(armor.now);
            armor.now < self.below
        }
    }

    #[test]
    fn the_bump_asks_the_sessions_disable_rule_and_never_passes_the_most() {
        let catalog = changing();
        let rule = Rc::new(Below {
            below: 47.5,
            asked: RefCell::default(),
        });
        let mut session = session(&catalog).with_disable_rule(rule.clone());
        assert!(
            !session
                .disable_rule()
                .disabled(Gauge::full(50.0), &HullSpec::default())
        );
        rule.asked.borrow_mut().clear();
        run(&mut session, "C131");
        assert_eq!(session.reserves().armor.now, 48.0);
        assert_eq!(*rule.asked.borrow(), [45.0, 46.0, 47.0, 48.0]);
        let always = Rc::new(Below {
            below: f32::INFINITY,
            asked: RefCell::default(),
        });
        let mut session = self::session(&catalog).with_disable_rule(always);
        run(&mut session, "C129");
        assert_eq!(
            session.reserves().armor,
            Gauge::full(100.0),
            "never past it"
        );
    }

    // Saving.

    #[test]
    fn the_new_ship_survives_a_save_and_reload() {
        let catalog = changing();
        for op in ["C129", "E129", "H129"] {
            let mut session = session(&catalog);
            session.pilot.cargo.insert(Good::Commodity(1), 6);
            session.pilot.ship_name = Some("Kestrel".to_owned());
            run(&mut session, op);
            let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
            let reloaded = Session::fly(&catalog, pilot).expect("flies");
            assert_eq!(reloaded.ship(), ShipId(129), "{op}");
            assert_eq!(reloaded.pilot.outfits, session.pilot.outfits, "{op}");
            assert_eq!(reloaded.defaults, session.defaults, "{op}");
            assert_eq!(reloaded.stats(), session.stats(), "{op}");
            assert_eq!(reloaded.reserves(), session.reserves(), "{op}");
            assert_eq!(reloaded.pilot.cargo, session.pilot.cargo, "{op}");
            assert_eq!(reloaded.pilot().ship_name(), Some("Kestrel"), "{op}");
            assert_eq!(reloaded.armament, session.armament, "{op}");
        }
    }

    #[test]
    fn a_reload_holds_a_kept_surplus_to_the_new_most() {
        let catalog = changing();
        for (op, held) in [
            ("C130", Reserves::full(110.0, 20.0, 100.0)),
            ("H130", Reserves::full(10.0, 20.0, 100.0)),
        ] {
            let mut session = session(&catalog);
            run(&mut session, op);
            let kept = session.reserves();
            assert_eq!((kept.shield.now, kept.armor.now), (130.0, 45.0), "{op}");
            let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
            assert_eq!(pilot.reserves(), kept, "{op}: the save keeps the surplus");
            let reloaded = Session::fly(&catalog, pilot).expect("flies");
            assert_eq!(reloaded.reserves(), held, "{op}: flying it holds it");
        }
    }

    #[test]
    fn a_later_refit_in_the_same_expression_holds_a_kept_surplus_to_the_most() {
        let catalog = changing();
        for op in ["C130 G400", "C130 D400"] {
            let mut session = session(&catalog);
            run(&mut session, op);
            assert_eq!(
                session.reserves(),
                Reserves::full(110.0, 20.0, 100.0),
                "{op}: the licence changes no most"
            );
        }
        let mut session = session(&catalog);
        run(&mut session, "C130 G403");
        assert_eq!(
            session.reserves(),
            Reserves::full(160.0, 20.0, 100.0),
            "a shield gained and held to its most; the armour held"
        );
    }

    // T.

    #[test]
    fn the_string_lists_print_as_such_and_are_equal_only_to_themselves() {
        let none = Strings::none();
        assert_eq!(format!("{none:?}"), "Strings");
        assert_eq!(none, none.clone());
        assert_ne!(none, Strings::none(), "another list");
    }

    /// String lists by ID, recording each list asked for.
    #[derive(Debug, Default)]
    struct Lists {
        lists: Vec<(i16, Vec<&'static str>)>,
        asked: RefCell<Vec<i16>>,
    }

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.asked.borrow_mut().push(id);
            self.lists
                .iter()
                .find(|(list, _)| *list == id)
                .map_or_else(Vec::new, |(_, names)| {
                    names.iter().map(|&name| name.to_owned()).collect()
                })
        }
    }

    /// 25040 holds "* II *" and "Second"; 25041 is empty; 25042 holds an
    /// empty string.
    fn lists() -> Rc<Lists> {
        Rc::new(Lists {
            lists: vec![
                (25040, vec!["* II *", "Second"]),
                (25041, vec![]),
                (25042, vec![""]),
            ],
            asked: RefCell::default(),
        })
    }

    fn named(strings: Rc<Lists>) -> Session {
        let mut session = session(&changing()).with_strings(strings);
        session.pilot.ship_name = Some("Kestrel".to_owned());
        session
    }

    fn rename(session: &mut Session, text: &str, rolls: &[u16]) -> Vec<u16> {
        let mut chance = Scripted::rolling(rolls);
        session.run_set(&SetExpr::parse(text).expect("parses"), &mut chance);
        chance.sides_asked
    }

    #[test]
    fn t_picks_a_name_evenly_from_its_list_replacing_each_star_with_the_old_name() {
        let strings = lists();
        let mut session = named(strings.clone());
        assert_eq!(rename(&mut session, "T25040", &[0]), [2]);
        assert_eq!(session.pilot().ship_name(), Some("Kestrel II Kestrel"));
        assert_eq!(*strings.asked.borrow(), [25040], "the given lists are read");
        assert!(session.take_save_due());
        let mut session = named(lists());
        assert_eq!(rename(&mut session, "T25040", &[1]), [2]);
        assert_eq!(session.pilot().ship_name(), Some("Second"));
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn t_with_no_names_or_an_empty_pick_changes_nothing() {
        for (op, sides) in [("T25041", vec![]), ("T999", vec![]), ("T25042", vec![1])] {
            let mut session = named(lists());
            assert_eq!(rename(&mut session, op, &[0]), sides, "{op}: the draws");
            assert_eq!(session.pilot().ship_name(), Some("Kestrel"), "{op}");
            assert!(!session.take_save_due(), "{op}");
        }
        let mut session = session(&changing());
        let before = session.clone();
        assert!(
            rename(&mut session, "T25040", &[0]).is_empty(),
            "no strings given"
        );
        assert_eq!(session, before);
    }

    #[test]
    fn a_renamed_ship_survives_a_save() {
        let mut session = named(lists());
        rename(&mut session, "T25040 T25040", &[0, 0]);
        let name = "Kestrel II Kestrel II Kestrel II Kestrel";
        assert_eq!(session.pilot().ship_name(), Some(name), "twice");
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.ship_name(), Some(name));
    }

    #[test]
    fn an_unnamed_ship_gives_an_empty_name_to_each_star() {
        let mut session = named(lists());
        session.pilot.ship_name = None;
        rename(&mut session, "T25040", &[0]);
        assert_eq!(session.pilot().ship_name(), Some(" II "));
    }

    #[test]
    fn a_bit_written_beside_a_change_is_written_too() {
        let catalog = changing();
        let mut session = session(&catalog);
        run(&mut session, "b7 C129 b8");
        assert!(session.control_bit(Bit::new(7).expect("a bit")));
        assert!(session.control_bit(Bit::new(8).expect("a bit")));
        assert_eq!(session.ship(), ShipId(129));
        assert_eq!(session.take_script_notes(), [], "handled");
    }
}
