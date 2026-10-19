//! The player's drawn state and the display of every ship, as the `EV
//! Nova` executable keeps them: the player's bank as `_HandlePlayer` steps
//! it, the writers of its fold, and `_HandleShipDisplay`'s frame for the
//! player and every NPC (see `nova_sim::combat::ship_frames` and
//! `nova_sim::traffic::ship_display`).
//!
//! **The bank** ([`Session::bank_player`], `_HandlePlayer`
//! @0x6cd59-0x6cf06, right after the thrust's glow step). The flag
//! (+0xc8f8) is first cleared. A ship of `shän` `Flags` 0x0001 moves its
//! bank (+0xc8e4) a frame toward ±16 turning that way (its turn keys or
//! its auto-turn's, @0x6c905-0x6ca9c), and 2 frames back toward 0 turning
//! neither way, 0 within 2 of it (16.0 @0xdd048, -16.0 @0xdd674, -2.0
//! @0xdd998). Its flag is 1 past 6 turning right or auto-turning, -1 below
//! -6 turning left or auto-turning, else 0 (6.0 @0xdd06c, -6.0 @0xdd964):
//! auto-turning (local -0x2cd) flags a bank either way. A banked ship of
//! 0x0002 raises its glow by 2 while it is 23 or less (@0x6cece-0x6cf06),
//! between the thrust's step and the afterburner's ([`step_glow`](crate::glow::step_glow)).
//!
//! Through a jump (+0x50 above 0, @0x6cf0d-0x6cf6c) the bank turns with the
//! jump's auto-turn, a ship of 0x0001 levels at once past a stage of 30
//! (30.0 @0xdd084), and the glow falls 1 more ([`ramp_jump_glow`](crate::glow::ramp_jump_glow)'s second
//! fall). The player's +0x4c, the block's other test, is never set here.
//!
//! **The fold's writers**, each the player's [`Fold`]:
//!
//! - a new pilot's (`_ResetPlayer` @0x1d55a-0x1d565): set 0, folding up;
//! - a load (`_LoadPilotData` @0x76348-0x76376) and a change of ship
//!   (`_HandlePlayer` @0x6ef1b-0x6ef3f): its last set, no direction, the
//!   timer 0;
//! - a take-off (`_HandlePlayerDockRequest` @0x66f83-0x66ff6): a ship of
//!   0x0002 and 0x0080 at its last set with no direction, any other at set
//!   0 folding up, the timer 0;
//! - an arrival (`_HandlePlayer` @0x6c538-0x6c57b): a ship of 0x0080 at its
//!   last set with no direction, any other at set 0 folding up, the timer
//!   0;
//! - each frame of the warp stage (@0x6b59c-0x6b5cb): a ship of 0x0002
//!   but not 0x0080 above set 0 unfolds; the Asteroid Miner does not;
//! - its fire's gate and stamp (`Armament::fire_player`), and its clock
//!   moving on each advance pass, as `TickCount` is real time.
//!
//! A loaded pilot's escorts take set 0, or, of 0x0080, the last set of the
//! player's class by the engine and their own by the other reading
//! ([`RuleKey::LoadedEscortFoldSet`], `_LoadPilotData` @0x763d4-0x763f0).
//! An escort entering a system keeps the display its ship had, its glow
//! cleared (`_RespawnEscort` @0x3d505), as its ship-table slot keeps it;
//! one whose ship was not in the system takes the load's.
//!
//! **The display** ([`Session::display_ships`]): after each pass's fight,
//! the player's frame and then each NPC's, so each ship's frame reads the
//! fold its flight and fire left, as `_HandleShipDisplay` runs after
//! `_HandlePlayer` and each `_HandleShip`; before the frame is drawn.
//! Each display ends with the ship's ion tint
//! (`nova_sim::combat::ion_tint`), its colour cleared once its charge is
//! gone ([`Session::player_ion_tint`]). No sound plays in any of it.

use super::Session;
use crate::Chance;
use crate::RuleKey;
use crate::combat::armament::{Fold, OutfitRounds};
use crate::combat::ion_tint::{self, IonTint};
use crate::combat::ship_frames::{
    BANKS, DisplayRules, Drawn, FOLDS, FrameSpec, Moment, ShipFrames, UNFOLDS_TO_FIRE,
};
use crate::flight::{Turn, shortest_turn};
use crate::rulebook::RuleSource;

/// The most the player banks either way (16.0 @0xdd048).
pub const PLAYER_BANK_MOST: f32 = 16.0;
/// How far the player's bank levels a frame turning neither way (-2.0
/// @0xdd998).
pub const PLAYER_BANK_LEVEL: f32 = 2.0;
/// The bank past which the player's flag is set (6.0 @0xdd06c).
pub const PLAYER_BANK_FLAG: f32 = 6.0;
/// The warp stage past which a banking ship levels at once (30.0
/// @0xdd084).
pub const JUMP_LEVEL_STAGE: f32 = 30.0;

/// The player's drawn state: its display state but its fold, which is its
/// armament's, and the sets its last display drew it on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct PlayerDisplay {
    /// Its display state.
    pub(super) frames: ShipFrames,
    /// The sets its last display drew it on.
    pub(super) drawn: Drawn,
    /// Its ion tint this frame, if it glows.
    pub(super) ion_tint: Option<IonTint>,
}

/// One frame of `_HandlePlayer`'s bank (see the module docs) of a ship of
/// class `spec`, its display `frames`, turning `way`, auto-turning when
/// `auto`.
pub(super) fn bank(frames: &mut ShipFrames, spec: &FrameSpec, way: Turn, auto: bool) {
    frames.flag = 0;
    if !spec.has(BANKS) {
        return;
    }
    let bank = &mut frames.bank;
    match way {
        Turn::Right => {
            if *bank < PLAYER_BANK_MOST {
                *bank += 1.0;
            }
        }
        Turn::Left => {
            if *bank > -PLAYER_BANK_MOST {
                *bank -= 1.0;
            }
        }
        Turn::None => {
            if *bank >= PLAYER_BANK_LEVEL {
                *bank -= PLAYER_BANK_LEVEL;
            } else if *bank <= -PLAYER_BANK_LEVEL {
                *bank += PLAYER_BANK_LEVEL;
            } else {
                *bank = 0.0;
            }
        }
    }
    frames.flag = if (way == Turn::Right || auto) && *bank > PLAYER_BANK_FLAG {
        1
    } else if (way == Turn::Left || auto) && *bank < -PLAYER_BANK_FLAG {
        -1
    } else {
        0
    };
}

/// Which way a heading turned from `before` to `after`: right clockwise,
/// left the other way, none not at all.
pub(super) fn way_turned(before: f32, after: f32) -> Turn {
    match shortest_turn(before, after) {
        off if off > 0.0 => Turn::Right,
        off if off < 0.0 => Turn::Left,
        _ => Turn::None,
    }
}

impl Session {
    /// Which way the player's ship turns this tick out of a jump under
    /// `controls`, toward `aim` while the auto-turn aims, as its turn
    /// block does ([`player_turn::turn`](crate::player_turn::turn)).
    pub(super) fn turn_way(&self, controls: crate::flight::Controls, aim: Option<i16>) -> Turn {
        let rate = self.flying_handling().turn_rate;
        let rule = self.flight_rules().player_turn;
        crate::player_turn::turn(self.player.heading, rate, aim, controls.turn, rule).1
    }

    /// One frame of the player's bank out of a jump, turning `way`,
    /// auto-turning when `auto` (see the module docs).
    pub(super) fn bank_player(&mut self, way: Turn, auto: bool) {
        let spec = self.hull.frames;
        bank(&mut self.ship_display.frames, &spec, way, auto);
    }

    /// One frame of the player's bank through a jump, its auto-turn turning
    /// it `way`: a banking ship levels at once past a stage of 30 (see the
    /// module docs).
    pub(super) fn bank_through_jump(&mut self, way: Turn) {
        self.bank_player(way, true);
        if self.hull.frames.has(BANKS) && self.warp_stage() > JUMP_LEVEL_STAGE {
            self.ship_display.frames.bank = 0.0;
        }
    }

    /// Whether the banking glow raises the player's glow this frame: a
    /// banked ship of 0x0002 (@0x6cece-0x6cef1).
    pub(super) fn banking_glow(&self) -> bool {
        self.ship_display.frames.flag != 0 && self.hull.frames.has(FOLDS)
    }

    /// `_ResetPlayer`'s fold: set 0, folding up.
    pub(super) fn reset_fold(&mut self) {
        *self.armament.fold_mut() = Fold {
            set: 0,
            dir: 1,
            ..self.armament.fold()
        };
    }

    /// A load's or a change of ship's fold: the last set, no direction,
    /// the timer 0.
    pub(super) fn last_set_fold(&mut self) {
        let last = self.hull.frames.last_set();
        *self.armament.fold_mut() = self.armament.fold().at(last);
        self.ship_display.frames.bank = 0.0;
    }

    /// The take-off's fold (see the module docs).
    pub(super) fn take_off_fold(&mut self) {
        let spec = self.hull.frames;
        self.land_fold(spec.has(FOLDS) && spec.has(UNFOLDS_TO_FIRE));
    }

    /// The arrival's fold (see the module docs).
    pub(super) fn arrival_fold(&mut self) {
        let unfolds = self.hull.frames.has(UNFOLDS_TO_FIRE);
        self.land_fold(unfolds);
    }

    /// A take-off's or an arrival's fold: the last set with no direction
    /// when `folded`, else set 0 folding up; the timer 0.
    fn land_fold(&mut self, folded: bool) {
        if folded {
            self.last_set_fold();
            return;
        }
        self.reset_fold();
        self.ship_display.frames.bank = 0.0;
    }

    /// The warp stage's unfold (@0x6b59c-0x6b5cb): a ship of 0x0002, not
    /// 0x0080, above set 0 unfolds.
    pub(super) fn jump_unfold(&mut self) {
        let spec = self.hull.frames;
        let fold = self.armament.fold_mut();
        if spec.has(FOLDS) && !spec.has(UNFOLDS_TO_FIRE) && fold.set > 0 {
            fold.dir = -1;
        }
    }

    /// The fold set a loaded pilot's escort of class `escort` takes (see
    /// the module docs).
    pub(super) fn loaded_escort_set(&self, escort: &FrameSpec) -> u16 {
        if !escort.has(UNFOLDS_TO_FIRE) {
            return 0;
        }
        match self.rules.source_for(RuleKey::LoadedEscortFoldSet) {
            RuleSource::Engine => self.hull.frames.last_set(),
            RuleSource::Bible => escort.last_set(),
        }
    }

    /// `_HandleShipDisplay` for the player and then every NPC, after the
    /// pass's fight (see the module docs), the NPCs' spawn rolls made on
    /// `chance`.
    pub(super) fn display_ships(&mut self, chance: &mut (impl Chance + ?Sized)) {
        let rules = self.display_rules();
        let spec = self.hull.frames;
        let rounds = OutfitRounds {
            owned: &mut self.pilot.outfits,
            sources: &self.ammo_outfits,
        };
        let moment = Moment {
            disabled: self.condition.disabled(),
            key_aboard: self
                .armament
                .key_ship_aboard(self.hull.key_carried, &rounds),
            rules,
        };
        let fold = self.armament.fold_mut();
        let frames = &mut self.ship_display.frames;
        let mut drawn = frames.step(&spec, fold, moment);
        drawn.lights = frames.light(&spec, &moment, |sides| chance.roll(sides));
        self.ship_display.drawn = drawn;
        let capacity = self.ion_capacity();
        self.ship_display.ion_tint = ion_tint::display(&mut self.ion, capacity, &mut *chance);
        let govts = &self.govts;
        self.traffic
            .display_ships(|govt| govts.derelict(govt), rules, chance);
    }

    /// The readings every ship's display runs by, from the session's
    /// rulebook.
    fn display_rules(&self) -> DisplayRules {
        DisplayRules {
            double_step: self.rules.source_for(RuleKey::ShipAnimationDoubleStep),
            alt_held: self.rules.source_for(RuleKey::DisabledAltHeld),
            lights_zeroed: self.rules.source_for(RuleKey::LightsOutZeroesBlink),
        }
    }

    /// The sets the player's ship was last drawn on: its hull's, and its
    /// alt layer's while it is drawn (see `nova_sim::combat::ship_frames`).
    #[must_use]
    pub fn player_drawn(&self) -> Drawn {
        self.ship_display.drawn
    }

    /// The player's ion tint as its last display left it, if it glows
    /// (see `nova_sim::combat::ion_tint`).
    #[must_use]
    pub fn player_ion_tint(&self) -> Option<IonTint> {
        self.ship_display.ion_tint
    }

    /// How a murk-colorized ionized ship's tint takes its colour
    /// ([`RuleKey::IonTintMurkColour`]), which the view reads.
    #[must_use]
    pub fn ion_tint_murk_colour(&self) -> RuleSource {
        self.rules.source_for(RuleKey::IonTintMurkColour)
    }

    /// Ship class `ship`'s own `Shield` as the original loads it (class
    /// +0x5c: a negative one 5 times its size,
    /// [`shield_points`](crate::reserves::shield_points)), none of its
    /// outfits': what a shield flash's level is a share of
    /// (`_HandleShipDisplay` @0x2c8aa); none for a class the session has
    /// no record of.
    #[must_use]
    pub fn class_shield(&self, ship: crate::catalog::ShipId) -> Option<f32> {
        // A `Shield` within an `i16` × 5: exact in an `f32`.
        #[allow(clippy::cast_precision_loss)]
        self.ship_record(ship)
            .map(|record| crate::reserves::shield_points(record.fields.shield) as f32)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{HullRecord, ShipDisplay, ShipExits, ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::ship_frames::ANIMATED;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::Rulebook;
    use crate::session::ReservePolicy;
    use crate::session::ship_change::ClassChange;
    use crate::session::tests::{OUT, bound_for_131};
    use crate::testkit::{FAST, FakePilotCatalog, catalog, hull, land_now, ship};

    /// The test catalog, the player's ship 128 of `shän` `Flags` `flags`
    /// and `sets` base sets, `AnimDelay` 5, 36 frames a turn; ship 129 of
    /// the Asteroid Miner's 0xc2 and six sets.
    fn displaying(flags: u16, sets: i16) -> FakePilotCatalog {
        let shän = |id, flags, sets| HullRecord {
            display: Some(ShipDisplay {
                flags,
                base_sets: sets,
                anim_delay: 5,
                ..ShipDisplay::default()
            }),
            exits: Some(ShipExits {
                frames: 36,
                ..ShipExits::default()
            }),
            ..hull(id)
        };
        FakePilotCatalog {
            hulls: vec![shän(128, flags, sets), shän(129, 0x00C2, 6)],
            ..catalog()
        }
    }

    /// The right turn key held.
    fn right() -> Controls {
        Controls {
            turn: Turn::Right,
            ..Controls::default()
        }
    }

    /// One step of the session under `controls`: the player's tick, then
    /// the fight and the display.
    fn step(session: &mut Session, controls: Controls) {
        session.tick(controls);
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    fn banking() -> FrameSpec {
        FrameSpec {
            flags: BANKS,
            base_sets: 3,
            ..FrameSpec::NONE
        }
    }

    /// The bank and flag after `n` frames turning `way` from `frames`.
    fn turned(frames: &mut ShipFrames, way: Turn, auto: bool, n: usize) -> (f32, i8) {
        for _ in 0..n {
            bank(frames, &banking(), way, auto);
        }
        (frames.bank, frames.flag)
    }

    #[test]
    fn the_player_banks_a_frame_a_tick_to_16_and_flags_past_6() {
        let mut frames = ShipFrames::default();
        assert_eq!(turned(&mut frames, Turn::Right, false, 6), (6.0, 0));
        assert_eq!(turned(&mut frames, Turn::Right, false, 1), (7.0, 1));
        assert_eq!(turned(&mut frames, Turn::Right, false, 20), (16.0, 1));
        assert_eq!(
            turned(&mut frames, Turn::None, false, 1),
            (14.0, 0),
            "let go: no flag"
        );
        assert_eq!(
            turned(&mut frames, Turn::None, false, 7),
            (0.0, 0),
            "2 a frame"
        );
        assert_eq!(turned(&mut frames, Turn::Left, false, 7), (-7.0, -1));
        frames.bank = 1.5;
        assert_eq!(
            turned(&mut frames, Turn::None, false, 1),
            (0.0, 0),
            "within 2: 0"
        );
        frames.bank = -1.5;
        assert_eq!(turned(&mut frames, Turn::None, false, 1), (0.0, 0));
        frames.bank = -3.0;
        assert_eq!(turned(&mut frames, Turn::None, false, 1), (-1.0, 0));
        frames.bank = -16.0;
        assert_eq!(
            turned(&mut frames, Turn::Left, false, 1),
            (-16.0, -1),
            "held at -16"
        );
        frames.bank = -5.0;
        assert_eq!(
            turned(&mut frames, Turn::Left, false, 1),
            (-6.0, 0),
            "-6: no flag"
        );
    }

    #[test]
    fn auto_turning_flags_a_bank_either_way() {
        let mut frames = ShipFrames {
            bank: 10.0,
            ..ShipFrames::default()
        };
        assert_eq!(turned(&mut frames, Turn::None, true, 1), (8.0, 1));
        frames.bank = -10.0;
        assert_eq!(turned(&mut frames, Turn::None, true, 1), (-8.0, -1));
        frames.bank = 10.0;
        assert_eq!(
            turned(&mut frames, Turn::Left, false, 1),
            (9.0, 0),
            "turning back"
        );
        assert_eq!(turned(&mut frames, Turn::Left, true, 1), (8.0, 1), "auto");
        frames.bank = 3.0;
        assert_eq!(
            turned(&mut frames, Turn::None, true, 1),
            (1.0, 0),
            "a small bank: none"
        );
    }

    #[test]
    fn a_ship_that_does_not_bank_clears_its_flag_and_keeps_its_bank() {
        let mut frames = ShipFrames {
            bank: 9.0,
            flag: 1,
            ..ShipFrames::default()
        };
        bank(&mut frames, &FrameSpec::NONE, Turn::Right, true);
        assert_eq!((frames.bank, frames.flag), (9.0, 0));
    }

    #[test]
    fn the_way_a_heading_turned() {
        assert_eq!(way_turned(10.0, 13.0), Turn::Right);
        assert_eq!(way_turned(1.0, 358.0), Turn::Left);
        assert_eq!(way_turned(359.0, 2.0), Turn::Right);
        assert_eq!(way_turned(40.0, 40.0), Turn::None);
    }

    #[test]
    fn holding_right_banks_the_player_onto_set_2_past_6_and_letting_go_levels_it() {
        let catalog = displaying(BANKS, 3);
        let mut session = Session::start(&catalog).expect("starts");
        let mut sets = Vec::new();
        for _ in 0..8 {
            step(&mut session, right());
            sets.push(session.player_drawn().set);
        }
        assert_eq!(sets, [0, 0, 0, 0, 0, 0, 2, 2], "past 6: set 2");
        step(&mut session, Controls::default());
        assert_eq!(session.player_drawn().set, 0, "let go: no flag");
        assert_eq!(session.ship_display.frames.bank, 6.0, "levelling 2 a frame");
        let plain = catalog_session(0, 3);
        assert_eq!(plain.ship_display.frames, ShipFrames::default());
    }

    fn catalog_session(flags: u16, sets: i16) -> Session {
        let mut session = Session::start(&displaying(flags, sets)).expect("starts");
        for _ in 0..8 {
            step(&mut session, right());
        }
        session
    }

    #[test]
    fn a_banked_folding_player_raises_its_glow_by_2() {
        let thrust_right = Controls {
            thrust: true,
            ..right()
        };
        let glow = |flags| {
            let mut session = Session::start(&displaying(flags, 3)).expect("starts");
            for _ in 0..7 {
                step(&mut session, thrust_right);
            }
            session.engine_glow()
        };
        assert_eq!(glow(BANKS), 7);
        assert_eq!(glow(BANKS | FOLDS), 9, "banked on the 7th: +2");
    }

    #[test]
    fn a_banking_ship_levels_at_once_past_a_warp_stage_of_30() {
        let catalog = displaying(BANKS, 3);
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 180.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        for _ in 0..400 {
            let stage = session.warp_stage();
            session.ship_display.frames.bank = 14.0;
            session.tick(Controls::default());
            if session.warp_stage() > JUMP_LEVEL_STAGE {
                assert_eq!(session.ship_display.frames.bank, 0.0, "past 30");
                return;
            }
            assert!(
                session.ship_display.frames.bank > 10.0,
                "{stage}: a frame or two off"
            );
        }
        panic!("never past 30");
    }

    #[test]
    fn a_new_pilot_folds_up_from_set_0_and_a_load_starts_folded() {
        let catalog = displaying(FOLDS, 6);
        let session = Session::start(&catalog).expect("starts");
        let fold = session.armament.fold();
        assert_eq!((fold.set, fold.dir), (0, 1), "_ResetPlayer's");
        let pilot = session.pilot.clone();
        let loaded = Session::fly(&catalog, pilot).expect("loads");
        let fold = loaded.armament.fold();
        assert_eq!((fold.set, fold.dir), (5, 0), "_LoadPilotData's");
    }

    #[test]
    fn a_change_of_ship_folds_to_the_new_hulls_last_set() {
        let mut catalog = displaying(0, 1);
        catalog.ship_records.push(ship(129, FAST));
        let mut session = Session::start(&catalog).expect("starts");
        session.armament.fold_mut().dir = -1;
        session.ship_display.frames.bank = 3.0;
        let record = session.ship_record(ShipId(129)).cloned().expect("ship 129");
        let change = ClassChange {
            outfits: session.pilot.outfits.clone(),
            reserves: ReservePolicy::Hold,
        };
        session.become_class(&record, change, |_| {});
        let fold = session.armament.fold();
        assert_eq!((fold.set, fold.dir), (5, 0));
        assert_eq!(session.ship_display.frames.bank, 0.0, "its timer");
    }

    #[test]
    fn a_take_off_and_an_arrival_fold_a_miner_and_unfold_any_other() {
        for (flags, folded) in [(0x00C2, true), (FOLDS, false)] {
            let catalog = displaying(flags, 6);
            let mut session = Session::start(&catalog).expect("starts");
            land_now(&mut session).expect("lands");
            session.armament.fold_mut().set = 3;
            session.take_off(&catalog, &mut NeverFires);
            let fold = session.armament.fold();
            let expected = if folded { (5, 0) } else { (0, 1) };
            assert_eq!((fold.set, fold.dir), expected, "{flags:#x} took off");
            session.armament.fold_mut().set = 3;
            session.arrival_fold();
            let fold = session.armament.fold();
            assert_eq!((fold.set, fold.dir), expected, "{flags:#x} arrived");
        }
        // The arrival tests 0x0080 alone.
        let catalog = displaying(0x0080, 6);
        let mut session = Session::start(&catalog).expect("starts");
        session.arrival_fold();
        assert_eq!(session.armament.fold().set, 5);
        session.take_off_fold();
        assert_eq!(
            session.armament.fold().set,
            0,
            "the take-off asks 0x0002 too"
        );
    }

    #[test]
    fn the_warp_stage_unfolds_a_folding_ship_but_not_a_miner() {
        for (flags, dir) in [(FOLDS, -1), (0x00C2, 0)] {
            let catalog = displaying(flags, 6);
            let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 180.0);
            session.armament.fold_mut().set = 5;
            session.armament.fold_mut().dir = 0;
            assert_eq!(session.begin_jump(), Ok(SystemId(131)));
            for _ in 0..200 {
                session.tick(Controls::default());
                if session.warp_stage() > 1.0 {
                    break;
                }
            }
            assert_eq!(session.armament.fold().dir, dir, "{flags:#x}");
        }
        // Unfolded already, nothing turns.
        let mut session = Session::start(&displaying(FOLDS, 6)).expect("starts");
        *session.armament.fold_mut() = crate::combat::armament::Fold::default();
        session.jump_unfold();
        assert_eq!(session.armament.fold().dir, 0);
    }

    #[test]
    fn a_loaded_escort_of_0x0080_folds_to_the_players_last_set_by_the_engine() {
        let catalog = displaying(FOLDS, 4);
        let escort = FrameSpec {
            flags: 0x00C2,
            base_sets: 6,
            ..FrameSpec::NONE
        };
        let plain = FrameSpec {
            flags: FOLDS,
            ..escort
        };
        for (reading, set) in [(RuleSource::Engine, 3), (RuleSource::Bible, 5)] {
            let rules = Rulebook::default().with_override(RuleKey::LoadedEscortFoldSet, reading);
            let session = Session::start(&catalog).expect("starts").with_rules(rules);
            assert_eq!(session.loaded_escort_set(&escort), set, "{reading:?}");
            assert_eq!(session.loaded_escort_set(&plain), 0, "{reading:?}");
        }
    }

    #[test]
    fn the_players_clock_moves_on_the_advance_pass_alone() {
        let catalog = displaying(0, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.armament.stamp_fire();
        session.tick(Controls::default());
        assert_eq!(session.armament.fold().since_fire, 2);
        session.tick_again(Controls::default(), Rules::default(), &mut NeverFires);
        assert_eq!(session.armament.fold().since_fire, 2, "not again");
    }

    #[test]
    fn the_display_draws_an_animated_player_and_steps_every_npc() {
        let catalog = displaying(ANIMATED, 3);
        let mut session = Session::start(&catalog).expect("starts");
        let mut sets = Vec::new();
        for _ in 0..7 {
            step(&mut session, Controls::default());
            sets.push(session.player_drawn().set);
        }
        assert_eq!(sets, [0, 0, 0, 0, 0, 1, 1], "past 5 it steps");
        assert_eq!(
            session.player_drawn().alt,
            crate::combat::ship_frames::Alt::None,
            "no alt sprite"
        );
    }

    #[test]
    fn an_escort_entering_takes_the_loads_fold_then_keeps_its_slots_display() {
        let mut catalog = displaying(FOLDS, 4);
        catalog.ship_records.push(ship(129, FAST));
        let mut session = Session::start(&catalog).expect("starts");
        session
            .pilot
            .escorts
            .push(crate::session::tests::trader_escort());
        session.populate(&catalog, &mut NeverFires);
        let id = session.fleet[0].expect("placed");
        let escort = session.npc(id).expect("in the system");
        assert_eq!(escort.armament.fold().set, 3, "the player's last set");
        assert_eq!(escort.display.glow, 0, "_RespawnEscort's");
        let npc = session.npc_mut(id).expect("in the system");
        npc.armament.fold_mut().set = 1;
        npc.armament.fold_mut().dir = -1;
        npc.display.frames.anim_set = 2;
        npc.display.glow = 20;
        let lights = crate::combat::ship_lights::ShipLights {
            intensity: 12.5,
            count: 1,
            timer: 3.0,
        };
        npc.display.frames.lights = lights;
        session.populate(&catalog, &mut NeverFires);
        let id = session.fleet[0].expect("placed");
        let escort = session.npc(id).expect("in the system");
        let fold = escort.armament.fold();
        assert_eq!((fold.set, fold.dir), (1, -1), "kept");
        assert_eq!(escort.display.frames.anim_set, 2);
        assert_eq!(escort.display.frames.lights, lights, "its lights too");
        assert_eq!(escort.display.glow, 0, "cleared again");
    }

    /// The test catalog, the player's ship 128 of `shän` `Flags` `flags`
    /// with a lights sprite blinking the stock triangle.
    fn lit(flags: u16) -> FakePilotCatalog {
        let mut catalog = displaying(flags, 1);
        let display = catalog.hulls[0].display.as_mut().expect("a shän");
        display.blink = crate::blink::Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        display.lights_image = true;
        catalog
    }

    /// The player's lights levels over `n` steps, disabled throughout
    /// when `disabled`.
    fn player_lights(session: &mut Session, n: usize, disabled: bool) -> Vec<Option<u8>> {
        if !disabled {
            session.condition = crate::combat::hull::Condition::Intact;
        }
        let rules = if disabled {
            crate::session::tests::DISABLING
        } else {
            Rules::default()
        };
        (0..n)
            .map(|_| {
                session.tick(Controls::default());
                session.tick_combat(rules, &mut NeverFires);
                session.player_drawn().lights
            })
            .collect()
    }

    #[test]
    fn the_players_lights_go_out_disabled_and_climb_back_from_nothing() {
        let catalog = lit(0x0040);
        let mut session = Session::start(&catalog).expect("starts");
        let before = player_lights(&mut session, 30, false);
        assert_eq!(before[29], Some(22), "rising from rest");
        assert_eq!(player_lights(&mut session, 5, true), [None; 5]);
        let after = player_lights(&mut session, 42, false);
        assert_eq!(after[0], None, "dark for a step");
        assert_eq!(after[1], Some(1));
        assert_eq!(after[41], Some(31));
        // By the other reading, hidden only.
        let rules =
            Rulebook::default().with_override(RuleKey::LightsOutZeroesBlink, RuleSource::Bible);
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        player_lights(&mut session, 30, false);
        player_lights(&mut session, 5, true);
        let after = player_lights(&mut session, 1, false);
        assert_eq!(after, [Some(27)], "as if never out: 0.75 x 36");
    }

    #[test]
    fn the_players_lights_carry_through_a_landing_and_a_jump() {
        let catalog = lit(0);
        let mut session = Session::start(&catalog).expect("starts");
        player_lights(&mut session, 20, false);
        let lights = session.ship_display.frames.lights;
        assert_ne!(lights, crate::combat::ship_lights::ShipLights::default());
        land_now(&mut session).expect("lands");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.ship_display.frames.lights, lights, "a landing");
        assert_eq!(
            crate::testkit::jump(&mut session, &catalog, 131),
            Some(SystemId(131))
        );
        assert_eq!(session.ship_display.frames.lights, lights, "a jump");
    }

    #[test]
    fn a_classs_own_shield_is_its_loaded_shield() {
        let mut catalog = displaying(0, 1);
        let mut record = ship(129, FAST);
        record.fields.shield = -40;
        catalog.ship_records.push(record);
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.class_shield(ShipId(129)), Some(200.0), "5 × 40");
        assert_eq!(session.class_shield(ShipId(999)), None);
    }
}
