//! The player's ship in flight: where it is and how it flies, its
//! condition, tractor lock, ionization, fire command and secondary; what
//! its class and outfits give it ([`Fit`]), rewritten only by
//! `become_class` and `refit`; the latches each reset arms ([`Latches`]);
//! its death, the pod and self-destruct ([`Fate`]); and its warp and
//! double time ([`Warp`]). The pod's countdown, the jump under way and
//! the game's end (`pod`, the `mode` and `player_end`) stay on `Session`
//! as mode fields.

use std::collections::{BTreeMap, BTreeSet};

use crate::afterburner::MaxVel;
use crate::catalog::{OutfitId, RankId, WeaponId};
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::combat::impact::TractorLock;
use crate::combat::ion::Ion;
use crate::flight::ShipState;
use crate::handling::ShipFields;
use crate::stats::ShipStats;
use crate::warp;

use super::{ship_display, warp_stage};

/// The player's ship, in `Session`'s former field order, then its parts.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PlayerShip {
    pub(super) player: ShipState,
    /// Whether the ship is thrusting, as the last sounds told it.
    pub(super) thrusting: bool,
    /// The engine glow's base level (see [`glow`](crate::glow)), an
    /// `i16` as the original's, which an inertialess ship's dips to -1.
    pub(super) engine_glow: i16,
    /// The player's drawn state: its bank, display timers and sets (see
    /// the `ship_display` module). Unsaved.
    pub(super) ship_display: ship_display::PlayerDisplay,
    /// The player's speed along its heading, the original's +0x48, which
    /// an inertialess ship flies by (see the `inertialess` module).
    pub(super) player_speed: f32,
    /// Whether the player's afterburner is on this tick, `_abOn` (see the
    /// `afterburner` module). Unsaved.
    pub(super) afterburner_on: bool,
    /// The box the player's velocity is clamped to, `_playerMaxVel`, as
    /// the last tick left it (see the `afterburner` module). Unsaved, and
    /// none in a new session, as the original's static data is.
    pub(super) player_max_vel: MaxVel,
    /// How the player's ship is holding up.
    pub(super) condition: Condition,
    /// The player's repair inhibit, `_playerRepairInhibit`: armed as it is
    /// newly disabled, it holds back its armour's regeneration and its
    /// self-repair (see the `player_repair` module).
    pub(super) repair_inhibit: crate::self_repair::RepairInhibit,
    /// The player's tractor lock
    /// ([`Session::tractor`](super::Session::tractor)).
    pub(super) tractor: Option<TractorLock>,
    /// The player's ionization ([`Session::ion`](super::Session::ion)).
    pub(super) ion: Ion,
    /// The fire command the player holds; its secondary is filled in
    /// from the selection as it fires while the secondary fire key is held
    /// (see the `secondary_select` module).
    pub(super) trigger: Trigger,
    /// The secondary weapon the player has selected, if any: none to
    /// start with, set by the select key alone, and let go by the Weapon
    /// Safety key, by running out (see the `secondary_select` module) and
    /// as the module docs say. By the engine it may name a weapon the ship
    /// no longer carries ([`RuleKey::StaleSecondary`](crate::rulebook::RuleKey::StaleSecondary)).
    pub(super) secondary: Option<WeaponId>,
    /// The secondary fire key, a Weapon Select press waiting and the
    /// Weapon Safety key (see the `secondary_select` module). Unsaved.
    pub(super) weapon_keys: super::secondary_select::WeaponKeys,
    /// Whether the player's fuel capacity is cached, as `_ShipFuelCapacity`
    /// caches it (`*0x3b508c`, @0x2f55-0x2feb) at the flight tick's fuel
    /// regeneration and an Energy take, until a reset invalidates it
    /// (@0xc397): which capacity Use As My Ship draws its fuel below (see
    /// the `capture` module). Never saved.
    pub(super) fuel_cap_cached: bool,
    /// The player's desired heading, in whole degrees, the original's
    /// +0x68, which the double-time switch reads (see the `double_time`
    /// module); not saved, as a pilot's start zeroes it (@0x1c315).
    pub(super) aim: i16,
    /// What the ship's class and outfits give it.
    pub(super) fit: Fit,
    /// What each reset arms.
    pub(super) latches: Latches,
    /// The ship's death, the pod and self-destruct.
    pub(super) fate: Fate,
    /// The player's warp and double time.
    pub(super) warp: Warp,
    /// The player's cloak, its key and its latch (see the `player_cloak`
    /// module). Unsaved.
    pub(super) cloak: super::player_cloak::PlayerCloak,
}

/// What the player's ship's class and outfits give it, rewritten only by
/// `become_class` and `refit`, in `Session`'s former field order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Fit {
    /// The ship's fields, read when the session starts.
    pub(super) fields: ShipFields,
    /// The ship's default items, read when the session starts.
    pub(super) defaults: BTreeMap<OutfitId, u16>,
    /// The ship's stock weapons and their ammunition, as outfits
    /// ([`Arsenal::stock_outfits`](crate::combat::armament::Arsenal::stock_outfits)),
    /// read when the session starts.
    pub(super) stock: BTreeMap<OutfitId, u16>,
    /// How the ship performs, with the outfits it carries.
    pub(super) stats: ShipStats,
    /// The player's ship type's hull.
    pub(super) hull: HullSpec,
    /// The player's weapons.
    pub(super) armament: Armament,
}

/// What `reset_precalced` arms, in `Session`'s former field order.
// The fields keep the names they had on `Session`.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Latches {
    /// The active ranks as the last reset took them, which the blessing
    /// table is built from (see the `ranks` and `precalced` modules).
    /// Never saved, as the original's table lives in memory only.
    pub(super) blessing_ranks: BTreeSet<RankId>,
    /// Whether the resets leave the cargo past the holds untrimmed: set
    /// through an outfit sale by the other reading of
    /// [`RuleKey::OutfitSaleCargo`](crate::rulebook::RuleKey::OutfitSaleCargo)
    /// (see the `precalced` module). Never saved.
    pub(super) sale_keeps_cargo: bool,
    /// `_ScanPlayer`'s contraband latches, armed at each reset
    /// ([`Session::reset_precalced`](super::Session::reset_precalced), the
    /// session's stand-in for `_ResetPlayerPrecalcedValues`), and cleared
    /// by a conviction (see [`scan::Latches`](crate::scan::Latches)).
    /// Never saved, as the original's are globals.
    pub(super) scan_latches: crate::scan::Latches,
    /// The sabotage clock, `_sabotageCounter` with the cached
    /// `_playerSabotaged`, refreshed at each reset
    /// ([`Session::reset_precalced`](super::Session::reset_precalced); see
    /// the `sabotage` module). Never saved, as the original's are globals.
    pub(super) sabotage: crate::sabotage::SabotageClock,
    /// The `jünk` latches the tribbles and perishables step reads, set
    /// only at each reset
    /// ([`Session::reset_precalced`](super::Session::reset_precalced); see
    /// [`market::JunkLatches`](crate::market::JunkLatches)). Both clear
    /// until the first. Never saved, as the original's are globals.
    pub(super) junk_latches: crate::market::JunkLatches,
}

/// The player's ship's death, the pod and self-destruct, in `Session`'s
/// former field order.
// Each flag is its own part of the ship's fate, set and read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Fate {
    /// Whether the player's ship was destroyed on the latest advance
    /// pass ([`Session::game_work`](super::Session::game_work)), the pass
    /// the fight raised its `CombatEvent::Destroyed`; read by
    /// [`Session::player_fate`](super::Session::player_fate). It stays
    /// set until the next advance pass's head, so every interpolated view
    /// frame drawn after that pass sees
    /// [`PlayerFate::Exploded`](crate::player_fate::PlayerFate::Exploded):
    /// the original hides on its own frame boundary, so this crosses no
    /// boundary the player can see.
    pub(super) player_exploded_this_tick: bool,
    /// Whether +0xc908 is 0 outside the pod, as a beam of `ExitType` 5
    /// can leave it (see the `stray_words` module): a later such beam
    /// steps it to 1 without a draw. Cleared wherever the original writes
    /// -1 or 350: the eject, the rescue, a take-off or a gate's exit
    /// (`_ResetPlayer` from `_HandlePlayerDockRequest` @0x6709b). Never
    /// saved, as `_OpenPilotFile` writes -1 (@0x766b5).
    pub(super) pod_zeroed: bool,
    /// Whether the pod countdown has run out and the rescue is due (see
    /// the `rescue` module); never saved.
    pub(super) rescue_due: bool,
    /// The rescue's dësc, loaded and still to show (see the `rescue`
    /// module); never saved.
    pub(super) rescue_desc: Option<crate::flight_frame::LoadedDesc>,
    /// Whether the player holds the eject chord, Command and the eject
    /// key together (see the `eject` module); never saved.
    pub(super) eject_held: bool,
    /// The self-destruct countdown, `_destructCount` (see the
    /// `self_destruct` module); never saved.
    pub(super) self_destruct: crate::self_destruct::SelfDestruct,
    /// Whether the player holds the self-destruct chord, Command and `-`
    /// together (see the `self_destruct` module); never saved.
    pub(super) self_destruct_held: bool,
    /// Whether a pirate has docked with the player's disabled ship (its
    /// slot's +0xb9; see the `pirate_plunder` module): never saved.
    pub(super) player_boarded: bool,
}

/// The player's warp and double time, in `Session`'s former field order.
// The fields keep the names they had on `Session`.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Warp {
    /// Which warp-up sound is still playing, `snd ` 128 or 129, for the
    /// warp start's still-playing skip and the double-time switch (see
    /// [`warp::WarpUps`]); not saved, as the original's sounds are not.
    pub(super) warp_ups: warp::WarpUps,
    /// Whether the warp-out, `snd ` 130, is still playing, for a jump's
    /// natural end (see [`warp::WarpOut`]); not saved, as the original's
    /// sounds are not.
    pub(super) warp_out: warp::WarpOut,
    /// The player's warp stage: its +0x50, the warp end's flags and the
    /// screen's white (see the `warp_stage` module).
    pub(super) warp: warp_stage::PlayerWarp,
    /// The hypergate or wormhole the land key has just been pressed over,
    /// cleared: its entry awaits until the next tick.
    pub(super) gate: Option<crate::catalog::StellarId>,
    /// Whether double time is on, as the switch last said (see the
    /// `double_time` module); not saved, as the original's `_doubleTime`
    /// is a global, not a pilot field.
    pub(super) double_time: bool,
    /// Whether the double-time key is on, as its caller last said.
    pub(super) double_time_key: bool,
    /// Whether a new jump was accepted since the player part last took
    /// it, for the fighters' warning (see the `wait_for_me` module); not
    /// saved.
    pub(super) jump_accepted: bool,
}

impl PlayerShip {
    /// The ship a session starts or loads with, as `Session::load` builds
    /// it: `player` where it resumes, the ship's `fields`, `defaults` and
    /// `stock` read from the catalog, and everything else at its start,
    /// before the load's refit.
    pub(super) fn new(
        player: ShipState,
        fields: ShipFields,
        defaults: BTreeMap<OutfitId, u16>,
        stock: BTreeMap<OutfitId, u16>,
    ) -> Self {
        Self {
            player,
            thrusting: false,
            engine_glow: 0,
            ship_display: ship_display::PlayerDisplay::default(),
            player_speed: 0.0,
            afterburner_on: false,
            player_max_vel: MaxVel::START,
            condition: Condition::Intact,
            repair_inhibit: crate::self_repair::RepairInhibit::OFF,
            tractor: None,
            ion: Ion::default(),
            trigger: Trigger::default(),
            secondary: None,
            weapon_keys: super::secondary_select::WeaponKeys::default(),
            fuel_cap_cached: false,
            aim: 0,
            fit: Fit {
                fields,
                defaults,
                stock,
                // Refitted below, from the outfits the pilot owns.
                stats: ShipStats::default(),
                // Refitted below, from the ship and the outfits the pilot owns.
                hull: HullSpec::default(),
                armament: Armament::default(),
            },
            latches: Latches {
                blessing_ranks: BTreeSet::new(),
                sale_keeps_cargo: false,
                scan_latches: crate::scan::Latches {
                    junk: false,
                    outfits: false,
                },
                sabotage: crate::sabotage::SabotageClock::default(),
                junk_latches: crate::market::JunkLatches::default(),
            },
            fate: Fate {
                player_exploded_this_tick: false,
                pod_zeroed: false,
                rescue_due: false,
                rescue_desc: None,
                eject_held: false,
                self_destruct: crate::self_destruct::SelfDestruct::default(),
                self_destruct_held: false,
                player_boarded: false,
            },
            warp: Warp {
                warp_ups: warp::WarpUps::default(),
                warp_out: warp::WarpOut::default(),
                warp: warp_stage::PlayerWarp::default(),
                gate: None,
                double_time: false,
                double_time_key: false,
                jump_accepted: false,
            },
            cloak: super::player_cloak::PlayerCloak::default(),
        }
    }
}
