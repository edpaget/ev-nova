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
//! | 5 | @0x1551c-0x15668, `E` and `H` only | the new class's `WCount`s and `AmmoLoad`s are added, signed, to the weapon and rounds arrays, each of its `DefaultItems` slots of count above none to the outfits alone (`+=`, @0x15609), and the arrays folded into the outfits again (@0x15622): each outfit whose first mod names a weapon held to what the array has left, the rest put on the lowest-ID such outfit, so a default weapon or ammunition outfit is pulled back to the array's count; then every outfit owned, old and new, is clamped to its limit by `_HasMaxOfItem` (@0x4512, called @0x15648: `if 0 < lim < count { count = lim }`). `C` clamps nothing |
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
//! The block never calls `_DestroyPartialFleetCargo`, but each
//! `_SystemInfoToShipStats` ends in `_ResetPlayerPrecalcedValues`
//! (@0xcae9), whose trim (@0xc7d8-0xc887) cuts the commodities to the
//! fleet's holds, and so does the set-string wrapper's tail once the
//! expression ends (@0x99a85, @0x99dea): the cargo is kept past the new
//! hold only until then. The block leaves the escorts and the fighters
//! out alone, checks no mass, moves no cash, draws no `_Rand`, and does
//! not branch on `_playerIsLanded`, so landed and in flight are alike.
//! It never calls `_EvalMissionBitSetString`, so no `OnPurchase`,
//! `OnCapture` or `OnRetire` runs, and it names and paints nothing.
//!
//! The shipyard by contrast (`_DoShipyardDialog` ~0x5ebc8-0x5f0ac) trades
//! in, runs `OnRetire`, names the ship from its dialog
//! (`_CullNameString`, 0x40), clears the fighters out, strips the outfits
//! whose persistent byte (+0x378, which the loader sets from 0x0004 alone,
//! @0x78cad) is clear, adds the defaults (`+=`), with no `_HasMaxOfItem`
//! clamp, keeps the cargo that fits (`_DestroyPartialFleetCargo(0)`),
//! runs `OnPurchase` (@0x5f00d), and only then fills the reserves
//! (@0x5f051-0x5f07b), to the class and outfits `OnPurchase` leaves. Capture (`_DoShipCapture`) strips by +0x378 too.
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
//! cargo left as it was (the reset at the expression's end trims it, as
//! [`ShipChangeRules::cargo`] says; see the `precalced` module), and the
//! armour raised a point at a time, never past its most, while the
//! session's [`DisableRule`] has the ship disabled
//! ([`Session::with_disable_rule`]; [`NovaDisable`] by default). Its
//! position, velocity and heading, condition, cash, name, paint, escorts
//! and fighters out are left as they were, no hook runs, and a save is
//! due.
//!
//! A shield or armour kept above its most lasts until damage takes it or
//! a refit holds it. A later `G` or `D`, in the same expression or
//! after, or a boarding grant, keeps it by the engine reading of
//! [`RuleKey::RefitReserves`](crate::RuleKey::RefitReserves), as the
//! original's `_SystemInfoToShipStats` clamps no reserve, and holds it by
//! the other. The outfitter and flying the pilot again from a save
//! ([`Session::fly`]; the save itself keeps the surplus) hold it either
//! way, as the original's outfitter does when it closes (@0x5da4b-0x5dad4)
//! and its `_LoadPilotData` fills the shield and armour (@0x754dc).
//!
//! - `Cxxx` keeps every outfit and adds no default.
//! - `Exxx` keeps every outfit and adds the new class's default items; by
//!   the engine every outfit is then held to its limit
//!   ([`ShipChangeRules::max`]; see
//!   [`ship_change`](crate::ship_change) for the limits).
//! - `Hxxx` keeps only the persistent outfits, as
//!   [`ShipChangeRules::persistence`] says, adds the default items, and
//!   holds them as `E` does.
//!
//! The ship's stock weapons and their `AmmoLoad` are owned as the
//! outfits that hold them, so `C` keeps the old ones as any outfit and
//! gives no new ones, and `E` and `H` add the new class's to the weapon
//! and rounds arrays
//! ([`Arsenal::stock_added`](crate::combat::armament::Arsenal::stock_added))
//! and fold them into the outfits after the default items, as the
//! original's fold does (steps 2 and 5; see
//! [`ship_change`](crate::ship_change) for the fold). Each lands on the
//! lowest-ID outfit whose **first** mod names it, and a weapon or a load
//! no such outfit holds is lost. A default item that is a weapon or
//! ammunition outfit is in neither array, so by the engine the fold pulls
//! it back to the array's count: it is lost unless the array has a
//! surplus left after the holders before it. By the other reading of
//! [`RuleKey::ShipChangeDefaultWeapons`] it is added to the arrays too,
//! and kept ([`ShipChangeRules::default_weapons`]). A negative
//! `WCount` or `AmmoLoad` is added signed, so it takes that many
//! launchers or rounds away from the outfits that hold them, never below
//! none; by the other reading of [`RuleKey::ShipChangeNegativeStock`] it
//! adds nothing, as a purchase reads it
//! ([`ShipChangeRules::negative_stock`]). They add rather than top up: the loop
//! (@0x1551c-0x155de) does `weapons[i] += WCount[i]` and
//! `rounds[a] += AmmoLoad[i]` for each weapon index i, `a` being the raw
//! `AmmoType` of `wëap` i when that is at most 0xff (unsigned
//! `cmpw $0xff` @0x15573) and i itself otherwise, never reading the
//! guidance. So owning one of a stock weapon and adding a class that
//! stocks two leaves three, where a purchase or a capture tops up to two;
//! two launchers sharing an ammunition add both their loads, where a
//! purchase takes the larger; and a fighter bay, whose raw `AmmoType` is
//! its fighter's `shïp` ID, loads `wëap` 128 + that ID when the ID is 128
//! to 255, which no `oütf` holds, so its fighters are lost (the Fed
//! Carrier's Viper and Anaconda bays, among others). A bay of fighter 256
//! or above loads itself, as the shipyard does. By the other reading of
//! [`RuleKey::ShipChangeBayAmmo`] a bay loads its own fighters always
//! ([`ShipChangeRules::bay_ammo`]).
//!
//! The clamp's limits are the new class's (`MaxGun`, `MaxTur`), the
//! weapons' `MaxAmmo` and the session's
//! [`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax) reading. Read from
//! `_HasMaxOfItem` (@0x4512-0x4894) and `_ShipStatsToSystemInfo`
//! (@0xcaee-0xcce7): the fold called @0x15622 reads the weapon array
//! (`player` +0xc8 + i x 0xc8) and the rounds (+0xd0) into locals and
//! writes only the outfit counts, so at the clamp the weapon array still
//! holds the carried launchers and the `+= WCount` of every stock weapon,
//! held by an outfit or not, and only `_SystemInfoToShipStats` @0x1566a
//! builds it again. The ammunition cap counts a stock launcher no outfit
//! holds, a launcher the clamp holds keeps its count for the cap, and a
//! default launcher the fold took away counts for nothing.
//!
//! Each change of class decides its own carry and applies it through
//! [`Session::become_class`], the one writer of the class, its fields,
//! default items and stock, and the outfits owned. Use As My Ship
//! ([`Session::assign`]) carries by [`carried_outfits`] too, keeping by
//! 0x0004 alone, holding nothing to its `Max`, and topping its stock
//! weapons up rather than adding them; its hooks and its take-over
//! reserves run between the outfits and the class, and it always holds
//! the reserves. [`Session::buy_ship`] is the third caller, with the
//! shipyard's carry
//! ([`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry): by the
//! engine every persistent outfit, by the other reading `Max` and the free
//! mass with the sale of the rest) and the reserves held.
//!
//! **`Txxx`** (@0x15b25-0x15be9) reads `STR#` `xxx`,
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

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use super::hire::Shared;
use super::{ReservePolicy, Session, SessionEvent};
use crate::catalog::{CommCatalog, GovtId, OutfitId, ShipId, ShipRecord};
use crate::chance::Chance;
use crate::combat::armament;
use crate::combat::exits::ExitCursors;
use crate::combat::hull::{DisableRule, NovaDisable};
use crate::navigation::NavMode;
use crate::outfitter::{Hardpoints, OutfitFlags};
use crate::pilot;
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::ship_change::{Fold, Limits, OutfitCarry, SET_OP_PERSISTENT, carried_outfits};
use crate::strings::PatchedStrings;

/// What the armour gains a step while the ship is disabled after a
/// change (the constant @0xdd068).
const ARMOR_BUMP: f32 = 1.0;

/// A `C`, `E` or `H` run while docked, as the status bar sees it: the
/// original's `_LoadIntfResource` call at the operator (@0x156c7, after
/// the armour bump @0x15677-0x156c5). It asks for the `ïntf` of
/// `government` (as [`Session::interface_government`] reads it), and
/// when that is not the one loaded (`_currentIntf`, @0x73663-0x73672) it
/// loads it and redraws the whole bar (`_RedrawStatus(1)`
/// @0x739b6-0x739bd), drawing `nav` and `reserves` as they stood at the
/// operator. Noted by `Session::change_ship`, taken by
/// [`Session::take_interface_loads`], with how many mission texts had been
/// queued by then, so that a view can order the redraws against the texts
/// a script queues around them ([`TextBar`](super::TextBar)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterfaceLoad {
    /// The government whose `Interface` the operator's load asks for.
    pub government: Option<GovtId>,
    /// The nav mode and selection at the operator.
    pub nav: NavMode,
    /// The shield, armour and fuel at the operator, after its armour
    /// bump.
    pub reserves: Reserves,
    /// How many mission texts had been queued at the operator
    /// ([`Session::texts_queued`]): it ran after each text whose
    /// [`TextBar::texts`](super::TextBar::texts) is at most this, and
    /// before the others.
    pub texts: u64,
}

/// The disputed rules of the ship-change set operators that the session
/// follows, projected from its rule set (see [`Session::with_rules`]):
/// the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipChangeRules {
    /// Which outfits `H` keeps ([`RuleKey::ShipChangePersistence`]).
    pub persistence: RuleSource,
    /// Whether `E` and `H` hold the outfits to their limits
    /// ([`RuleKey::ShipChangeMax`]).
    pub max: RuleSource,
    /// Whether the cargo `C`, `E` and `H` leave past the new hold is
    /// trimmed as the set expression ends, or kept until a reset outside
    /// a set expression ([`RuleKey::ShipChangeCargo`]).
    pub cargo: RuleSource,
    /// Whether `C`, `E` and `H` keep a shield or armour above the new
    /// most ([`RuleKey::ShipChangeReserves`]).
    pub reserves: RuleSource,
    /// Where `E` and `H` load a fighter bay's `AmmoLoad`
    /// ([`RuleKey::ShipChangeBayAmmo`]).
    pub bay_ammo: RuleSource,
    /// Whether `E` and `H` keep a default item that is a weapon or
    /// ammunition outfit ([`RuleKey::ShipChangeDefaultWeapons`]).
    pub default_weapons: RuleSource,
    /// Whether a negative `WeapCount` or `AmmoLoad` takes launchers or
    /// rounds away on `E` and `H` ([`RuleKey::ShipChangeNegativeStock`]).
    pub negative_stock: RuleSource,
}

impl ShipChangeRules {
    /// The rules `rulebook` chooses: its
    /// [`RuleKey::ShipChangePersistence`], [`RuleKey::ShipChangeMax`],
    /// [`RuleKey::ShipChangeCargo`], [`RuleKey::ShipChangeReserves`],
    /// [`RuleKey::ShipChangeBayAmmo`],
    /// [`RuleKey::ShipChangeDefaultWeapons`] and
    /// [`RuleKey::ShipChangeNegativeStock`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            persistence: rulebook.source_for(RuleKey::ShipChangePersistence),
            max: rulebook.source_for(RuleKey::ShipChangeMax),
            cargo: rulebook.source_for(RuleKey::ShipChangeCargo),
            reserves: rulebook.source_for(RuleKey::ShipChangeReserves),
            bay_ammo: rulebook.source_for(RuleKey::ShipChangeBayAmmo),
            default_weapons: rulebook.source_for(RuleKey::ShipChangeDefaultWeapons),
            negative_stock: rulebook.source_for(RuleKey::ShipChangeNegativeStock),
        }
    }

    /// The flags `H` keeps an outfit by.
    pub(super) fn persistent(self) -> u16 {
        match self.persistence {
            RuleSource::Engine => SET_OP_PERSISTENT,
            RuleSource::Bible => OutfitFlags::MISSION_PERSISTENT,
        }
    }

    /// Whether `E` and `H` hold the outfits to their limits.
    pub(super) fn clamps(self) -> bool {
        self.max == RuleSource::Engine
    }
}

/// A change of the player's ship class, decided by the caller (see
/// [`Session::become_class`]).
pub(super) struct ClassChange {
    /// What the pilot owns afterwards: the caller's carry.
    pub(super) outfits: BTreeMap<OutfitId, u16>,
    /// How the refit treats a shield or armour above the new most.
    pub(super) reserves: ReservePolicy,
}

impl ClassChange {
    /// A purchase's change: the shipyard's carry, the reserves held, as the
    /// shipyard has always refitted. The purchase fills them only after
    /// `OnPurchase` (`_DoShipyardDialog` @0x5f051-0x5f07b, after
    /// @0x5f00d); until then, holding keeps the old ship's within the new
    /// most. The original leaves them unclamped while `OnPurchase` runs
    /// (`_SystemInfoToShipStats` clamps no reserve), which only a
    /// set-operator observer could see; that is not modelled.
    pub(super) fn bought(outfits: BTreeMap<OutfitId, u16>) -> Self {
        Self {
            outfits,
            reserves: ReservePolicy::Hold,
        }
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
    /// The rules the ship-change set operators follow, projected from
    /// the session's rule set ([`Session::with_rules`]).
    pub(super) fn ship_change_rules(&self) -> ShipChangeRules {
        ShipChangeRules::from_rulebook(&self.setup.rules)
    }

    /// This session with `rule` saying when the player's ship is
    /// disabled after a change of ship (see the module docs):
    /// [`NovaDisable`] by default. The fight's own rule is given each
    /// tick ([`Session::tick_combat`]).
    #[must_use]
    pub fn with_disable_rule(mut self, rule: Rc<dyn DisableRule>) -> Self {
        self.setup.disable_rule = Shared(rule);
        self
    }

    /// The rule that says when the player's ship is disabled after a
    /// change of ship.
    #[must_use]
    pub fn disable_rule(&self) -> &dyn DisableRule {
        &*self.setup.disable_rule.0
    }

    /// This session with `T` reading its names from `strings`' string
    /// lists (see the module docs): none by default. The lists a plug-in's
    /// `'STR '` patches are built once, patched, as `_LoadStrings` builds
    /// them ([`PatchedStrings`]). The texts it fills are `l33t` when
    /// `strings` says the data turns that on ([`CommCatalog::leet_mode`];
    /// see [`text_tags`](crate::text_tags)).
    #[must_use]
    pub fn with_strings(mut self, strings: Rc<dyn CommCatalog>) -> Self {
        self.setup.leet = strings.leet_mode();
        self.setup.strings = Strings(Rc::new(PatchedStrings::new(strings)));
        self
    }

    /// The string lists `T` reads its names from.
    #[must_use]
    pub fn strings(&self) -> &dyn CommCatalog {
        &*self.setup.strings.0
    }

    /// The disable rule a session starts with.
    pub(super) fn nova_disable() -> Shared<dyn DisableRule> {
        Shared(Rc::new(NovaDisable))
    }

    /// Makes the player's ship class `ship`, as `change` says: the one
    /// writer of a change of class, deciding nothing itself. In order, it
    /// writes `change.outfits` as the outfits owned, runs `before_class`,
    /// takes the class, its fields, default items and stock weapons from
    /// `ship`, refits by `change.reserves`, makes a save due and tells
    /// [`SessionEvent::ShipChanged`]. What the change carries, and what
    /// runs between, is the caller's. The exit cursors go on as the refit
    /// carries them, or start at exit 0 by the other reading of
    /// [`RuleKey::ReusedSlotExits`]; the fold goes to the new hull's last
    /// set by either reading.
    pub(super) fn become_class(
        &mut self,
        ship: &ShipRecord,
        change: ClassChange,
        before_class: impl FnOnce(&mut Self),
    ) {
        self.pilot.outfits = change.outfits;
        before_class(self);
        self.pilot.ship = ship.id;
        self.fields = ship.fields;
        self.defaults = pilot::tally(ship.defaults.iter().copied());
        self.stock = armament::fitted(
            &self
                .statics
                .arsenal
                .stock_fits(ship.id, &self.statics.outfits),
        );
        self.refit(false, change.reserves);
        // The original's ship change writes no exit cursor (+0xc8fe-+0xc905),
        // so the turn of exits goes on, as the refit carried it; by the
        // other reading a new ship starts each `ExitType` at exit 0.
        if self.setup.rules.source_for(RuleKey::ReusedSlotExits) == RuleSource::Bible {
            *self.armament.exit_cursors_mut() = ExitCursors::default();
        }
        // The fold does not carry: the original's ship change rewrites it
        // as the new hull's `BaseSets` − 1 with no direction and its timer
        // 0 (`_HandlePlayer` @0x6ef1b-0x6ef3f), 0 for a hull of one set,
        // by either reading (see the `ship_display` module).
        self.last_set_fold();
        self.save_due = true;
        self.emit(SessionEvent::ShipChanged);
    }

    /// Changes the player's ship to class `ship`, its outfits carried as
    /// `carry` says and held to their limits when `clamp` (see the module
    /// docs); nothing changes when the session has no record of the
    /// class.
    pub(super) fn change_ship(&mut self, ship: ShipId, carry: OutfitCarry, clamp: bool) {
        let Some(record) = self.statics.ship_record(ship).cloned() else {
            return;
        };
        let rules = self.ship_change_rules();
        let defaults = pilot::tally(record.defaults.iter().copied());
        // `E` and `H` add the stock weapons and their `AmmoLoad` to the
        // weapon and rounds arrays and fold them into the outfits after
        // the default items (step 5).
        let stock =
            (self.statics.arsenal).stock_added(record.id, rules.bay_ammo, rules.negative_stock);
        let fold = Fold {
            stock: &stock,
            default_weapons: rules.default_weapons,
        };
        let folds = (carry != OutfitCarry::Keep).then_some(&fold);
        let max_ammo = |weapon| {
            self.statics
                .arsenal
                .weapon(weapon)
                .map_or(0, |spec| spec.max_ammo)
        };
        let limits = Limits {
            hardpoints: Hardpoints {
                guns: record.max_gun,
                turrets: record.max_tur,
            },
            max_ammo: &max_ammo,
            raised_max: self.setup.rules.source_for(RuleKey::RaisedMax),
        };
        let held = clamp.then_some(&limits);
        let change = ClassChange {
            outfits: carried_outfits(
                &self.pilot.outfits,
                carry,
                &defaults,
                folds,
                held,
                &self.statics.outfits,
            ),
            reserves: ReservePolicy::reading(rules.reserves),
        };
        self.become_class(&record, change, |_| {});
        self.bump_armor();
        // `_LoadIntfResource` (@0x156c7), as the docked bar sees it.
        if self.landed.is_some() {
            self.interface_loads.push(InterfaceLoad {
                government: self.interface_government(),
                nav: self.nav_mode(),
                reserves: self.reserves(),
                texts: self.texts_queued,
            });
        }
    }

    /// The `C`, `E` and `H` run while docked since the last run of docked
    /// scripts began, in order, each as the status bar's
    /// `_LoadIntfResource` saw it at the operator ([`InterfaceLoad`]);
    /// taking them empties the list. Each of these begins a run, clearing
    /// what the one before left untaken: an outfit order
    /// ([`Session::outfit_order`]), a ship purchase
    /// ([`Session::buy_ship`]), a mission accept, refusal or abort
    /// ([`Session::accept_mission`], [`Session::refuse_mission`],
    /// [`Session::abort_mission`]), an offer asked for at a venue
    /// ([`Session::next_offer`]), a settling ([`Session::settle_script`],
    /// a landing's mission checks among what it settles) and a jettison
    /// ([`Session::jettison_cargo`], its failures' `OnFailure`). An order
    /// or a purchase that is refused before it begins clears nothing.
    pub fn take_interface_loads(&mut self) -> Vec<InterfaceLoad> {
        std::mem::take(&mut self.interface_loads)
    }

    /// Raises the player's armour [`ARMOR_BUMP`] at a time, never past
    /// its most, while the session's disable rule has its ship disabled,
    /// as the original's loop does (@0x15677-0x156c5). It ends at the
    /// most, so a rule that never lets the ship fly still ends.
    fn bump_armor(&mut self) {
        let rule = Rc::clone(&self.setup.disable_rule.0);
        let armor = &mut self.pilot.reserves.armor;
        while armor.now < armor.max && rule.disabled(*armor, &self.hull) {
            armor.now = (armor.now + ARMOR_BUMP).min(armor.max);
        }
    }

    /// `T`: renames the ship from `STR#` `list`, drawing on `chance` (see
    /// the module docs).
    pub(super) fn rename_ship(&mut self, list: i16, chance: &mut dyn Chance) {
        let Some(pick) = self.pick_string(list, chance) else {
            return;
        };
        let previous = self.pilot.ship_name.as_deref().unwrap_or_default();
        self.pilot.ship_name = Some(pick.replace('*', previous));
    }

    /// One of `STR#` `list`'s strings, from the session's string lists,
    /// picked evenly on `chance` as `_GetRandomIndString` (@0x72ef7) picks
    /// it: a roll of as many sides as the list holds. `None`, with no roll,
    /// for a missing or empty list; an empty string picked is given as
    /// it is. The session's one port of that routine: the bar's news, a
    /// fleet's arrival quote and [`pick_string`](Self::pick_string) all
    /// draw through it.
    pub(super) fn random_string(&self, list: i16, chance: &mut dyn Chance) -> Option<String> {
        let strings = self.setup.strings.0.string_list(list);
        if strings.is_empty() {
            return None;
        }
        let sides = u16::try_from(strings.len()).unwrap_or(u16::MAX);
        strings.into_iter().nth(usize::from(chance.roll(sides)))
    }

    /// String `index` (1-based) of `STR#` `list`, from the session's
    /// string lists, if it has one: what the bar's news and the rank tags
    /// read through.
    pub(super) fn nth_string(&self, list: i16, index: usize) -> Option<String> {
        let mut strings = self.setup.strings.0.string_list(list);
        let at = index.checked_sub(1)?;
        (at < strings.len()).then(|| strings.swap_remove(at))
    }

    /// [`random_string`](Self::random_string), with an empty pick read as
    /// none, as the `T` rename, the `Q` leave message, and the bar's commercial
    /// and generic story read it.
    pub(super) fn pick_string(&self, list: i16, chance: &mut dyn Chance) -> Option<String> {
        self.random_string(list, chance)
            .filter(|pick| !pick.is_empty())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{
        HullRecord, OutfitId, OutfitRecord, ShipRecord, StockWeapon, WeaponId, WeaponRecord,
    };
    use crate::combat::hull::{HullSpec, TOUGH};
    use crate::control::{Bit, Script, SetExpr, SetOp};
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::market::Good;
    use crate::outfit_effects::Rgb15;
    use crate::outfitter::outfit_mods;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::{Gauge, Reserves};
    use crate::save;
    use crate::session::control::SetOpObserver;
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
    fn the_ship_change_rules_follow_their_seven_rulebook_keys() {
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
                bay_ammo: RuleSource::Engine,
                default_weapons: RuleSource::Engine,
                negative_stock: RuleSource::Engine,
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
        assert_eq!(
            bible(RuleKey::ShipChangeBayAmmo),
            ShipChangeRules {
                bay_ammo: RuleSource::Bible,
                ..ShipChangeRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::ShipChangeDefaultWeapons),
            ShipChangeRules {
                default_weapons: RuleSource::Bible,
                ..ShipChangeRules::default()
            }
        );
        assert_eq!(
            bible(RuleKey::ShipChangeNegativeStock),
            ShipChangeRules {
                negative_stock: RuleSource::Bible,
                ..ShipChangeRules::default()
            }
        );
        assert_eq!(bible(RuleKey::CrimeGains), ShipChangeRules::default());
    }

    // Becoming a class.

    /// `session`'s record of ship `id`.
    fn record(session: &Session, id: i16) -> ShipRecord {
        session
            .statics
            .ship_record(ShipId(id))
            .cloned()
            .expect("a record")
    }

    #[test]
    fn become_class_runs_its_step_with_the_new_outfits_and_the_old_class() {
        let catalog = changing_armed();
        let mut session = session(&catalog);
        let before = session.clone();
        let outfits = owned(&[(LICENCE, 1), (OutfitId(405), 1)]);
        let change = ClassChange {
            outfits: outfits.clone(),
            reserves: ReservePolicy::Hold,
        };
        let mut seen = None;
        session.become_class(&record(&session, 129), change, |s| {
            seen = Some((
                s.ship(),
                s.pilot.outfits.clone(),
                s.fields,
                s.defaults.clone(),
                s.stock.clone(),
                s.stats(),
                s.save_due,
            ));
        });
        assert_eq!(
            seen,
            Some((
                ShipId(128),
                outfits,
                before.fields,
                before.defaults.clone(),
                before.stock.clone(),
                before.stats(),
                false,
            )),
            "the outfits written, nothing else yet"
        );
    }

    #[test]
    fn become_class_flies_the_record_with_the_outfits_given() {
        let catalog = changing_armed();
        let mut session = session(&catalog);
        let outfits = owned(&[(LICENCE, 1), (SHIELD, 1), (OutfitId(405), 1)]);
        let change = ClassChange {
            outfits: outfits.clone(),
            reserves: ReservePolicy::Hold,
        };
        session.become_class(&record(&session, 129), change, |_| {});
        assert_eq!(session.ship(), ShipId(129));
        assert_eq!(session.pilot.outfits, outfits);
        assert_eq!(session.fields, BIG);
        assert_eq!(session.defaults, owned(&[(SHIELD, 1), (LIMITED, 2)]));
        assert_eq!(session.stock, owned(&[(OutfitId(405), 1)]));
        assert_eq!(
            session.stats(),
            ShipStats::new(BIG, &outfit_mods(&outfits, &session.statics.outfits))
        );
        assert_eq!(session.hull(), session.statics.arsenal.hull(ShipId(129)));
        assert_eq!(session.armament.mounts().len(), 1, "405 mounts weapon 200");
        assert!(session.take_save_due());
    }

    #[test]
    fn become_class_tells_the_ship_changed() {
        let catalog = changing();
        let mut session = session(&catalog);
        let change = ClassChange {
            outfits: BTreeMap::new(),
            reserves: ReservePolicy::Hold,
        };
        session.become_class(&record(&session, 129), change, |s| {
            assert_eq!(s.take_events(), [], "not before the class is taken");
        });
        assert_eq!(session.take_events(), [SessionEvent::ShipChanged]);
    }

    #[test]
    fn c_e_and_h_tell_the_ship_changed_and_a_class_with_no_record_tells_nothing() {
        let catalog = changing();
        for op in ["C129", "E129", "H129", "C128"] {
            let mut session = session(&catalog);
            run(&mut session, op);
            assert_eq!(session.take_events(), [SessionEvent::ShipChanged], "{op}");
        }
        let mut session = session(&catalog);
        run(&mut session, "C200");
        assert_eq!(session.take_events(), []);
    }

    #[test]
    fn become_class_keeps_a_surplus_shield_and_armour_only_when_told() {
        let catalog = changing();
        for (reserves, kept) in [
            (ReservePolicy::KeepSurplus, (130.0, 45.0)),
            (ReservePolicy::Hold, (10.0, 20.0)),
        ] {
            let mut session = session(&catalog);
            let change = ClassChange {
                outfits: BTreeMap::new(),
                reserves,
            };
            session.become_class(&record(&session, 130), change, |_| {});
            let now = session.reserves();
            assert_eq!((now.shield.now, now.armor.now), kept, "{reserves:?}");
            assert_eq!(now.fuel, Gauge::full(100.0), "{reserves:?}: always held");
        }
    }

    #[test]
    fn a_purchases_class_change_holds_the_reserves() {
        let outfits = owned(&[(LICENCE, 1)]);
        let change = ClassChange::bought(outfits.clone());
        assert_eq!(change.outfits, outfits);
        assert_eq!(change.reserves, ReservePolicy::Hold);
        let catalog = changing();
        let mut session = session(&catalog);
        session.become_class(
            &record(&session, 130),
            ClassChange::bought(BTreeMap::new()),
            |_| {},
        );
        let now = session.reserves();
        assert_eq!((now.shield.now, now.armor.now), (10.0, 20.0), "held");
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
            ShipStats::new(BIG, &outfit_mods(&kept, &session.statics.outfits))
        );
        assert_eq!(
            session.stats().shield,
            400.0,
            "the class's and the outfits'"
        );
        assert_eq!(session.hull(), session.statics.arsenal.hull(ShipId(129)));
        assert_eq!(
            session.armament,
            session
                .statics
                .arsenal
                .player(&kept, &session.statics.outfits)
        );
        assert!(
            session.armament.mounts().is_empty(),
            "no stock weapon: no outfit holds the new class's"
        );
        assert!(session.take_save_due());
        assert_eq!(session.take_script_notes(), []);
    }

    /// [`changing`], with outfit 405 holding weapon 200, ship 129's stock
    /// weapon.
    fn changing_armed() -> FakePilotCatalog {
        let mut catalog = changing();
        catalog
            .outfits
            .push(outfit(405, &[(armament::MOD_WEAPON, 200)]));
        catalog
    }

    #[test]
    fn e_and_h_add_the_new_classs_stock_weapons_as_outfits_and_c_none() {
        let catalog = changing_armed();
        let held = OutfitId(405);
        for (op, added) in [("C129", 0), ("E129", 1), ("H129", 1)] {
            let mut session = session(&catalog);
            run(&mut session, op);
            assert_eq!(session.pilot.owned(held), added, "{op}");
            assert_eq!(session.stock, owned(&[(held, 1)]), "{op}: its standard");
            assert_eq!(session.armament.mounts().len(), usize::from(added), "{op}");
        }
    }

    const STOCK_GUN: OutfitId = OutfitId(405);
    const LAUNCHER: OutfitId = OutfitId(406);
    const ROCKETS: OutfitId = OutfitId(407);
    const BAY: OutfitId = OutfitId(408);
    const FIGHTERS: OutfitId = OutfitId(409);
    const KEPT_LAUNCHER: OutfitId = OutfitId(410);

    /// [`changing_armed`], with ship 132 stocking two of weapon 200, one
    /// each of launchers 201 (7 rounds) and 202 (12 rounds), both firing
    /// weapon 138's rockets, and one of bay 203 (4 fighters of ship 144),
    /// and ship 133 stocking one of weapon 200, of `MaxGun` 1. Outfits:
    /// 406 holds 201, 407 the rockets (`Max` 50), 408 the bay, 409 its
    /// fighters, 410 holds 202 and is persistent (0x0004).
    fn stocked() -> FakePilotCatalog {
        let mut catalog = changing_armed();
        let slot = |weapon, count, ammo| StockWeapon {
            weapon: WeaponId(weapon),
            count,
            ammo,
        };
        let launcher = |id| WeaponRecord {
            ammo_type: 10,
            ..weapon(id)
        };
        catalog.weapons.extend([
            launcher(201),
            launcher(202),
            WeaponRecord {
                guidance: 99,
                ammo_type: 144,
                ..weapon(203)
            },
        ]);
        catalog.hulls.extend([
            HullRecord {
                weapons: vec![
                    slot(200, 2, 0),
                    slot(201, 1, 7),
                    slot(202, 1, 12),
                    slot(203, 1, 4),
                ],
                ..hull(132)
            },
            HullRecord {
                weapons: vec![slot(200, 1, 0)],
                ..hull(133)
            },
        ]);
        catalog
            .ships
            .extend([(ShipId(132), Ok(FAST)), (ShipId(133), Ok(FAST))]);
        catalog.ship_records.extend([
            ship(132, FAST),
            ShipRecord {
                max_gun: 1,
                ..ship(133, FAST)
            },
        ]);
        catalog.outfits.extend([
            outfit(406, &[(armament::MOD_WEAPON, 201)]),
            OutfitRecord {
                max: 50,
                ..outfit(407, &[(armament::MOD_AMMO, 138)])
            },
            outfit(408, &[(armament::MOD_WEAPON, 203)]),
            outfit(409, &[(armament::MOD_AMMO, 203)]),
            OutfitRecord {
                flags: OutfitFlags::PERSISTENT,
                ..outfit(410, &[(armament::MOD_WEAPON, 202)])
            },
        ]);
        catalog
    }

    /// The [`session`] of `catalog`, its old class's stock weapons owned
    /// too: one of 405 (weapon 200), 406 (201) and 410 (202), and five
    /// rockets.
    fn armed_session(catalog: &FakePilotCatalog) -> Session {
        let mut session = session(catalog);
        session.pilot.outfits.extend([
            (STOCK_GUN, 1),
            (LAUNCHER, 1),
            (KEPT_LAUNCHER, 1),
            (ROCKETS, 5),
        ]);
        session.refit(false, ReservePolicy::Hold);
        session
    }

    #[test]
    fn c_keeps_the_old_stock_weapon_outfits_and_their_rounds_and_adds_none() {
        let catalog = stocked();
        let mut session = armed_session(&catalog);
        let before = session.pilot.outfits.clone();
        run(&mut session, "C132");
        assert_eq!(session.ship(), ShipId(132));
        assert_eq!(session.pilot.outfits, before);
        assert_eq!(
            session.armament,
            session
                .statics
                .arsenal
                .player(&before, &session.statics.outfits)
        );
        let mounted: Vec<(WeaponId, u32)> = session
            .armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id, mount.count))
            .collect();
        assert_eq!(
            mounted,
            [(WeaponId(200), 1), (WeaponId(201), 1), (WeaponId(202), 1)],
            "the old ones still mounted"
        );
    }

    #[test]
    fn e_adds_the_new_stock_weapons_on_top_of_those_owned_and_sums_shared_loads() {
        let catalog = stocked();
        let mut session = armed_session(&catalog);
        run(&mut session, "E132");
        let own = |id| session.pilot.owned(id);
        assert_eq!(own(STOCK_GUN), 3, "1 owned + 2, not topped up to 2");
        assert_eq!(own(LAUNCHER), 2);
        assert_eq!(own(KEPT_LAUNCHER), 2);
        assert_eq!(own(ROCKETS), 24, "5 + 7 + 12, not the larger load");
        assert_eq!(own(BAY), 1);
        assert_eq!(own(FIGHTERS), 0, "by the engine the fighters are lost");
        assert_eq!(
            session.armament,
            session
                .statics
                .arsenal
                .player(&session.pilot.outfits, &session.statics.outfits)
        );
    }

    #[test]
    fn h_drops_the_old_stock_weapons_unless_persistent_and_adds_the_new_on_top() {
        let catalog = stocked();
        let mut session = armed_session(&catalog);
        run(&mut session, "H132");
        assert_eq!(
            session.pilot.outfits,
            owned(&[
                (LICENCE, 1),
                (ABILITY, 1),
                (BOTH, 1),
                (STOCK_GUN, 2),
                (LAUNCHER, 1),
                (ROCKETS, 19),
                (BAY, 1),
                (KEPT_LAUNCHER, 2),
            ]),
            "405, 406 and the rockets dropped, 410 kept, the new stock added"
        );
    }

    #[test]
    fn e_and_h_load_a_bay_as_ship_change_bay_ammo_says() {
        let catalog = stocked();
        for op in ["E132", "H132"] {
            let mut engine = armed_session(&catalog);
            run(&mut engine, op);
            assert_eq!(engine.pilot.owned(FIGHTERS), 0, "{op}: to weapon 272");
            let mut other = armed_session(&catalog).with_rules(
                Rulebook::default().with_override(RuleKey::ShipChangeBayAmmo, RuleSource::Bible),
            );
            run(&mut other, op);
            assert_eq!(other.pilot.owned(FIGHTERS), 4, "{op}: the bay's own");
            assert_eq!(other.pilot.owned(BAY), 1, "{op}");
        }
    }

    #[test]
    fn e_caps_ammunition_by_the_new_stock_launchers_held_or_not() {
        // Ship 134 stocks two of weapon 204, which no outfit holds, and one
        // of 205, which 412 holds, each of `MaxAmmo` 3; 411 and 413 are
        // their ammunition.
        let mut catalog = stocked();
        let capped = |id| WeaponRecord {
            max_ammo: 3,
            ..weapon(id)
        };
        catalog.weapons.extend([capped(204), capped(205)]);
        catalog.hulls.push(HullRecord {
            weapons: vec![
                StockWeapon {
                    weapon: WeaponId(204),
                    count: 2,
                    ammo: 0,
                },
                StockWeapon {
                    weapon: WeaponId(205),
                    count: 1,
                    ammo: 0,
                },
            ],
            ..hull(134)
        });
        catalog.ships.push((ShipId(134), Ok(FAST)));
        catalog.ship_records.push(ship(134, FAST));
        let ammo = |id, weapon| OutfitRecord {
            max: 50,
            ..outfit(id, &[(armament::MOD_AMMO, weapon)])
        };
        catalog.outfits.extend([
            ammo(411, 204),
            outfit(412, &[(armament::MOD_WEAPON, 205)]),
            ammo(413, 205),
        ]);
        let mut session = session(&catalog);
        session
            .pilot
            .outfits
            .extend([(OutfitId(411), 10), (OutfitId(413), 10)]);
        run(&mut session, "E134");
        let own = |id| session.pilot.owned(OutfitId(id));
        assert_eq!(own(411), 6, "3 x the two launchers no outfit holds");
        assert_eq!(own(412), 1, "the held launcher added");
        assert_eq!(own(413), 3, "3 x the one held launcher, counted once");
    }

    #[test]
    fn e_caps_ammunition_by_a_stock_launcher_named_only_in_a_later_mod() {
        // Ship 139 stocks two of weapon 206, of `MaxAmmo` 3. Outfit 415
        // names 206 only in its second mod, so it neither holds 206 in the
        // weapon array nor takes the surplus; 416 is 206's ammunition.
        let mut catalog = stocked();
        catalog.weapons.push(WeaponRecord {
            max_ammo: 3,
            ..weapon(206)
        });
        catalog.hulls.push(HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(206),
                count: 2,
                ammo: 0,
            }],
            ..hull(139)
        });
        catalog.ships.push((ShipId(139), Ok(FAST)));
        catalog.ship_records.push(ship(139, FAST));
        catalog.outfits.extend([
            outfit(415, &[(MORE_SHIELD, 1), (armament::MOD_WEAPON, 206)]),
            OutfitRecord {
                max: 50,
                ..outfit(416, &[(armament::MOD_AMMO, 206)])
            },
        ]);
        let mut session = session(&catalog);
        session
            .pilot
            .outfits
            .extend([(OutfitId(415), 1), (OutfitId(416), 10)]);
        run(&mut session, "E139");
        let own = |id| session.pilot.owned(OutfitId(id));
        assert_eq!(own(415), 1, "a later mod naming 206 takes no surplus");
        assert_eq!(own(416), 6, "3 x the two stock launchers, not three");
    }

    /// A second outfit holding weapon 200, of higher ID than 405.
    const SPARE_GUN: OutfitId = OutfitId(414);

    /// [`stocked`], with outfit 414 holding weapon 200 too, ship 135
    /// stocking nothing and carrying one of 414 and three rockets by
    /// default, ship 136 stocking two of weapon 200 and carrying one of
    /// 405 by default, ship 137 stocking launcher 201 with a load of -1,
    /// and ship 138 stocking 201 with a load of -1 and 202 with 3.
    fn defaulted() -> FakePilotCatalog {
        let mut catalog = stocked();
        catalog
            .outfits
            .push(outfit(414, &[(armament::MOD_WEAPON, 200)]));
        let slot = |weapon, count, ammo| StockWeapon {
            weapon: WeaponId(weapon),
            count,
            ammo,
        };
        let stocking = |id, weapons| HullRecord {
            weapons,
            ..hull(id)
        };
        catalog.hulls.extend([
            hull(135),
            stocking(136, vec![slot(200, 2, 0)]),
            stocking(137, vec![slot(201, 1, -1)]),
            stocking(138, vec![slot(201, 1, -1), slot(202, 1, 3)]),
        ]);
        let carrying = |id, defaults: Vec<(OutfitId, u16)>| ShipRecord {
            defaults,
            ..ship(id, FAST)
        };
        let spare = vec![(SPARE_GUN, 1), (ROCKETS, 3)];
        let gun = vec![(STOCK_GUN, 1)];
        catalog
            .defaults
            .extend([(ShipId(135), spare.clone()), (ShipId(136), gun.clone())]);
        catalog.ship_records.extend([
            carrying(135, spare),
            carrying(136, gun),
            ship(137, FAST),
            ship(138, FAST),
        ]);
        catalog
            .ships
            .extend([135, 136, 137, 138].map(|id| (ShipId(id), Ok(FAST))));
        catalog
    }

    /// `session` with its rules reading `key` the other way.
    fn other(session: Session, key: RuleKey) -> Session {
        session.with_rules(Rulebook::default().with_override(key, RuleSource::Bible))
    }

    #[test]
    fn by_the_engine_e_folds_a_default_weapon_or_ammunition_outfit_away() {
        let catalog = defaulted();
        let mut engine = armed_session(&catalog);
        run(&mut engine, "E135");
        let own = |session: &Session, id| session.pilot.owned(id);
        assert_eq!(
            own(&engine, STOCK_GUN),
            1,
            "the holder owned keeps its count"
        );
        assert_eq!(own(&engine, SPARE_GUN), 0, "the default gun is folded away");
        assert_eq!(own(&engine, ROCKETS), 5, "the default rockets too");
        let mut kept = other(armed_session(&catalog), RuleKey::ShipChangeDefaultWeapons);
        run(&mut kept, "E135");
        assert_eq!(own(&kept, STOCK_GUN), 1);
        assert_eq!(own(&kept, SPARE_GUN), 1, "by the other reading it is added");
        assert_eq!(own(&kept, ROCKETS), 8);
    }

    #[test]
    fn by_the_engine_e_leaves_the_stock_count_of_a_weapon_also_a_default() {
        let catalog = defaulted();
        let mut engine = session(&catalog);
        run(&mut engine, "E136");
        assert_eq!(
            engine.pilot.owned(STOCK_GUN),
            2,
            "two stock and one default: 2, not 3"
        );
        let mut kept = other(session(&catalog), RuleKey::ShipChangeDefaultWeapons);
        run(&mut kept, "E136");
        assert_eq!(kept.pilot.owned(STOCK_GUN), 3, "by the other reading 3");
    }

    #[test]
    fn by_the_engine_h_folds_a_default_weapon_or_ammunition_outfit_away() {
        let catalog = defaulted();
        let mut engine = armed_session(&catalog);
        run(&mut engine, "H135");
        assert_eq!(engine.pilot.owned(SPARE_GUN), 0);
        assert_eq!(
            engine.pilot.owned(ROCKETS),
            0,
            "the rockets owned dropped, the default lost"
        );
        let mut kept = other(armed_session(&catalog), RuleKey::ShipChangeDefaultWeapons);
        run(&mut kept, "H135");
        assert_eq!(kept.pilot.owned(SPARE_GUN), 1);
        assert_eq!(kept.pilot.owned(ROCKETS), 3);
    }

    #[test]
    fn by_the_engine_a_negative_load_takes_rounds_away_never_below_none() {
        let catalog = defaulted();
        let rockets_after = |session: Session, op: &str, owned: u16| {
            let mut session = session;
            session.pilot.outfits.insert(ROCKETS, owned);
            session.pilot.outfits.retain(|_, count| *count > 0);
            run(&mut session, op);
            session.pilot.owned(ROCKETS)
        };
        assert_eq!(rockets_after(armed_session(&catalog), "E137", 5), 4);
        assert_eq!(
            rockets_after(armed_session(&catalog), "E137", 0),
            0,
            "never below none"
        );
        assert_eq!(
            rockets_after(armed_session(&catalog), "E138", 5),
            7,
            "-1 + 3 nets +2"
        );
        assert_eq!(
            rockets_after(armed_session(&catalog), "H137", 5),
            0,
            "H dropped them"
        );
        let by_purchase = || other(armed_session(&catalog), RuleKey::ShipChangeNegativeStock);
        assert_eq!(rockets_after(by_purchase(), "E137", 5), 5, "adds nothing");
        assert_eq!(rockets_after(by_purchase(), "E138", 5), 8);
    }

    #[test]
    fn e_holds_the_guns_to_the_new_classs_gun_slots() {
        let mut catalog = stocked();
        for record in &mut catalog.outfits {
            if record.id == STOCK_GUN {
                record.flags = OutfitFlags::GUN;
            }
        }
        let mut session = armed_session(&catalog);
        run(&mut session, "E133");
        assert_eq!(
            session.pilot.owned(STOCK_GUN),
            1,
            "1 owned + 1 added, held to ship 133's MaxGun of 1, not ship 128's 2"
        );
        let mut session = armed_session(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::ShipChangeMax, RuleSource::Bible),
        );
        run(&mut session, "E133");
        assert_eq!(session.pilot.owned(STOCK_GUN), 2, "nothing held");
    }

    #[test]
    fn e_holds_an_outfit_to_its_max_as_raised_max_reads_it() {
        // Outfit 414 names 404 (`Max` 2) in two `ModType` 27 mods: by the
        // engine one of it raises the `Max` to 4, by the other reading to 2.
        let raiser = OutfitId(414);
        let mut catalog = changing();
        catalog.outfits.push(outfit(
            414,
            &[
                (crate::outfitter::RAISES_MAX, 404),
                (crate::outfitter::RAISES_MAX, 404),
            ],
        ));
        let owning = |mut changed: Session| {
            changed.pilot.outfits.extend([(LIMITED, 6), (raiser, 1)]);
            changed
        };
        let mut engine = owning(session(&catalog));
        run(&mut engine, "E130");
        assert_eq!(engine.pilot.owned(LIMITED), 4, "each mod counts");
        let mut other =
            owning(session(&catalog).with_rules(
                Rulebook::default().with_override(RuleKey::RaisedMax, RuleSource::Bible),
            ));
        run(&mut other, "E130");
        assert_eq!(other.pilot.owned(LIMITED), 2, "each raiser counts once");
    }

    #[test]
    fn e_holds_the_turrets_to_the_new_classs_turret_slots() {
        // Outfit 414 is a turret; ships 135 and 136 have `MaxTur` 2 and 3,
        // ship 128's being 1 and each one's `MaxGun` 2.
        let turret = OutfitId(414);
        let mut catalog = changing();
        catalog.outfits.push(OutfitRecord {
            flags: OutfitFlags::TURRET,
            ..outfit(414, &[])
        });
        for (id, max_tur) in [(135, 2), (136, 3)] {
            catalog.ships.push((ShipId(id), Ok(FAST)));
            catalog.ship_records.push(ShipRecord {
                max_tur,
                ..ship(id, FAST)
            });
        }
        for (op, held) in [("E135", 2), ("E136", 3)] {
            let mut session = session(&catalog);
            session.pilot.outfits.insert(turret, 5);
            run(&mut session, op);
            assert_eq!(session.pilot.owned(turret), held, "{op}: its own MaxTur");
        }
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
        let mut session = self::session(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::ShipChangeReserves, RuleSource::Bible),
        );
        run(&mut session, "C130");
        assert_eq!(
            session.reserves(),
            Reserves::full(110.0, 20.0, 100.0),
            "by the other reading each is held to its new most"
        );
    }

    #[test]
    fn e_and_h_to_a_smaller_class_keep_the_surplus_by_the_engine_and_hold_it_by_the_other() {
        let catalog = changing();
        for op in ["E130", "H130"] {
            let mut engine = session(&catalog);
            run(&mut engine, op);
            let kept = engine.reserves();
            assert!(kept.shield.max < 130.0 && kept.armor.max < 45.0, "{op}");
            assert_eq!(kept.shield.now, 130.0, "{op}: the shield's surplus kept");
            assert_eq!(kept.armor.now, 45.0, "{op}: the armour's surplus kept");
            assert_eq!(kept.fuel, Gauge::full(kept.fuel.max), "{op}: fuel held");
            let mut bible = session(&catalog).with_rules(
                Rulebook::default().with_override(RuleKey::ShipChangeReserves, RuleSource::Bible),
            );
            run(&mut bible, op);
            assert_eq!(
                bible.reserves(),
                Reserves::full(kept.shield.max, kept.armor.max, kept.fuel.max),
                "{op}: each held to its new most"
            );
        }
    }

    #[test]
    fn a_larger_class_is_alike_by_either_reading_of_the_reserves() {
        let catalog = changing();
        for op in ["C129", "E129", "H129"] {
            let mut engine = session(&catalog);
            run(&mut engine, op);
            let mut bible = session(&catalog).with_rules(
                Rulebook::default().with_override(RuleKey::ShipChangeReserves, RuleSource::Bible),
            );
            run(&mut bible, op);
            assert_eq!(bible.reserves(), engine.reserves(), "{op}");
            assert_eq!(engine.reserves().shield.now, 130.0, "{op}: not refilled");
        }
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

    /// Counts each `S` it sees run.
    #[derive(Debug, Default)]
    struct Probe(RefCell<Vec<i16>>);

    impl SetOpObserver for Probe {
        fn saw(&self, op: &SetOp, _session: &Session) {
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
            let mut session = session(&catalog).with_set_op_observer(probe.clone());
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
            ShipStats::new(
                BIG,
                &outfit_mods(&session.pilot.outfits, &session.statics.outfits)
            )
        );
        let mut session = self::session(&catalog);
        session.pilot.outfits.insert(LIMITED, 5);
        run(&mut session, "E130");
        assert_eq!(session.pilot.owned(LIMITED), 2, "an old one over its Max");
        let bible = Rulebook::default().with_override(RuleKey::ShipChangeMax, RuleSource::Bible);
        let mut session = self::session(&catalog).with_rules(bible);
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
        let mut session = self::session(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::ShipChangePersistence, RuleSource::Bible),
        );
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
        let mut session = self::session(&kept).with_rules(
            Rulebook::default().with_override(RuleKey::ShipChangeMax, RuleSource::Bible),
        );
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

    /// Commodity 0 x3 and commodity 2 x9: 12 tons.
    fn twelve_tons() -> BTreeMap<Good, u32> {
        [(Good::Commodity(0), 3), (Good::Commodity(2), 9)]
            .into_iter()
            .collect()
    }

    fn cargo(session: &Session) -> BTreeMap<Good, u32> {
        session.pilot().cargo().collect()
    }

    #[test]
    fn by_the_engine_c_e_and_h_trim_the_cargo_past_the_new_hold_as_the_expression_ends() {
        let catalog = changing();
        for op in ["C130", "E130", "H130"] {
            let mut session = session(&catalog);
            session.pilot.cargo = twelve_tons();
            let mut inside = Vec::new();
            session.run_ops(
                &SetExpr::parse(op).expect("parses"),
                &mut Scripted::default(),
                &mut |session, _| inside.push(cargo(session)),
            );
            assert_eq!(inside, [twelve_tons()], "{op}: kept inside the expression");
            assert_eq!(session.capacity(), 5);
            assert_eq!(
                cargo(&session),
                [(Good::Commodity(0), 1), (Good::Commodity(2), 3)].into(),
                "{op}: 5/12 of each, truncated, as the expression ends"
            );
            assert!(session.take_save_due(), "{op}");
        }
    }

    #[test]
    fn the_trim_counts_the_trader_escorts_and_the_mission_cargo_and_keeps_the_junk() {
        let catalog = changing();
        let mut session = session(&catalog);
        session.pilot.escorts.push(Escort {
            ship: ShipId(131),
            reserves: Reserves::default(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        });
        session.pilot.cargo = [
            (Good::Commodity(0), 20),
            (Good::Commodity(2), 9),
            (Good::Junk(crate::catalog::JunkId(150)), 4),
        ]
        .into_iter()
        .collect();
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 3));
        run(&mut session, "C130");
        assert_eq!(session.capacity(), 25, "5 and the escort's 20");
        assert_eq!(
            cargo(&session),
            [
                (Good::Commodity(0), 13),
                (Good::Commodity(2), 6),
                (Good::Junk(crate::catalog::JunkId(150)), 4)
            ]
            .into(),
            "25/36 of each commodity"
        );
        let held = session.pilot.missions.get(0).expect("still held");
        assert_eq!((held.cargo_tons, held.aboard), (3, true));
    }

    #[test]
    fn by_the_other_reading_the_cargo_past_the_hold_waits_for_a_reset_outside_a_set_expression() {
        let catalog = changing();
        for op in ["C130", "E130", "H130"] {
            let mut session = session(&catalog).with_rules(
                Rulebook::default().with_override(RuleKey::ShipChangeCargo, RuleSource::Bible),
            );
            session.pilot.cargo = twelve_tons();
            run(&mut session, op);
            assert_eq!(session.capacity(), 5, "{op}");
            assert_eq!(cargo(&session), twelve_tons(), "{op}: every ton kept");
            run(&mut session, "b1");
            assert_eq!(
                cargo(&session),
                twelve_tons(),
                "{op}: no set expression trims"
            );
            crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
            assert_eq!(
                cargo(&session),
                [(Good::Commodity(0), 1), (Good::Commodity(2), 3)].into(),
                "{op}: the arrival's reset trims (@0x6c51f)"
            );
        }
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
            let mut session = session(&catalog).with_rules(
                Rulebook::default().with_override(RuleKey::RefitReserves, RuleSource::Engine),
            );
            run(&mut session, op);
            let kept = session.reserves();
            assert_eq!((kept.shield.now, kept.armor.now), (130.0, 45.0), "{op}");
            let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
            assert_eq!(pilot.reserves(), kept, "{op}: the save keeps the surplus");
            // `Session::fly` refits under `ReservePolicy::Hold` before any
            // rules apply, so a reload holds the surplus whatever the
            // `refit_reserves` reading.
            let reloaded = Session::fly(&catalog, pilot).expect("flies");
            assert_eq!(reloaded.reserves(), held, "{op}: flying it holds it");
        }
    }

    /// The reserves `C130` leaves the [`session`]: the shield and armour
    /// above the new class's most, by the engine reading of
    /// `ship_change_reserves`.
    fn kept_surplus(shield_max: f32) -> Reserves {
        Reserves {
            shield: Gauge {
                now: 130.0,
                max: shield_max,
            },
            armor: Gauge {
                now: 45.0,
                max: 20.0,
            },
            fuel: Gauge::full(100.0),
        }
    }

    #[test]
    fn by_the_engine_a_later_g_or_d_keeps_a_kept_surplus() {
        let catalog = changing();
        for op in ["C130 G400", "C130 D400"] {
            let mut session = session(&catalog);
            run(&mut session, op);
            assert_eq!(
                session.reserves(),
                kept_surplus(110.0),
                "{op}: the licence changes no most, and nothing is held"
            );
        }
        let mut session = session(&catalog);
        run(&mut session, "C130 G403");
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge {
                    now: 180.0,
                    max: 160.0
                },
                ..kept_surplus(160.0)
            },
            "a shield gained over the surplus and kept; the armour kept"
        );
    }

    #[test]
    fn by_the_other_reading_a_later_g_or_d_holds_a_kept_surplus_to_the_most() {
        let catalog = changing();
        let holding = |catalog| {
            session(catalog).with_rules(
                Rulebook::default().with_override(RuleKey::RefitReserves, RuleSource::Bible),
            )
        };
        for op in ["C130 G400", "C130 D400"] {
            let mut session = holding(&catalog);
            run(&mut session, op);
            assert_eq!(
                session.reserves(),
                Reserves::full(110.0, 20.0, 100.0),
                "{op}: the licence changes no most"
            );
        }
        let mut session = holding(&catalog);
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
        assert_eq!(
            *strings.asked.borrow(),
            [4001, 4002, 4003, 25040],
            "the given lists are read: the patched ones once as they are given"
        );
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
    fn a_t_past_32767_renames_nothing_and_the_rest_runs() {
        let mut session = named(lists());
        assert_eq!(rename(&mut session, "T40000 b7", &[0]), Vec::<u16>::new());
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"));
        assert!(session.pilot().control_bit(Bit::new(7).expect("a bit")));
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
