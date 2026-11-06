//! `_PrepareAllSprites`' frames: before the first frame of each way into
//! flight, every ship's set, ion tint and lights step once, as each slot's
//! `_HandleShipDisplay` steps them (rdm rendering phase 13). All addresses
//! are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! # The original
//!
//! `_PrepareAllSprites` (@0x3a75a-0x3a806) sets the frozen flag
//! (@0x3a765-0x3a773) and calls `_HandleShipDisplay` for slots 0-63 in
//! order (@0x3a776-0x3a78e, the call @0x3a783), before the shots and the
//! stellars (`_HandleStellarSprites` @0x3a7c9). Each active ship of the
//! player's system (+0x74 slot 0's, +0xb8 set, @0x2b7d8-0x2b7ed) steps
//! its set (@0x2b7d8-0x2bc2e), its ion tint (@0x2beb9-0x2bf94) and its
//! lights (@0x2c2ea-0x2c5a6), then its cloak fade (@0x2d088-0x2d164); a
//! ship failing that test skips to @0x2ded6, past every step, and an
//! inactive slot is hidden and returns early (@0x2b555-0x2b60a). The
//! frozen flag gates only a destroyed slot's freeing (@0x2ded6-0x2dee7),
//! and no site holds a dying or destroyed ship. No `PlaySound` sits in
//! the set, tint or lights blocks (the function's one is the dying
//! block's, @0x2db5a), so no sound plays.
//!
//! Each display calls `_IsDisabled` once (@0x2b61f) and keeps it at
//! -0x152 (@0x2b624), which the layer block (@0x2bd24), the set step's
//! 0x0010 test (@0x2ba56) and the lights' 0x0040 lights-out (@0x2c585)
//! read; the tint reads none. `_IsDisabled` (@0x2ce6-0x2e0d) holds a
//! Derelict government's ship (@0x2d0f) and a mission ship held unboarded
//! (@0x2d24-0x2d69) disabled, never a defence ship by its armour
//! (@0x2d6f), and any other ship, the player's included, by its armour
//! (@0x2d79-0x2dcc). So each slot's steps here read the value its layer
//! reads ([`PreparedSlot::disabled`]).
//!
//! # Where it runs
//!
//! | Site | Here | Ships stepped | Gain |
//! |---|---|---|---|
//! | `_EnterGameFromMainScreen` @0x1a36c | `Session::enter_game` | the player | carried, by `RuleKey::SpeedMultRamp`'s engine reading |
//! | `_PlayGame` @0x45834 | the first step's [`Session::populate`], or `Session::enter_game` in a system already set up | every ship | the entry's |
//! | `_PlayerEnterHypergate` @0x63e91, `_PlayerEnterWormhole` @0x64743 | [`Session::prepare_at_gate_entry`] | the player; the old ships when the exit is in the system gone in | the frame's |
//! | `_HandlePlayerDockRequest` @0x6764e, @0x67658 | `populate` and `Session::end_dock_tail`, or a gate exit's two | every ship | 0 at a take-off, else the frame's |
//! | `_HandlePlayer` @0x6c59c (a jump's arrival) | [`Session::arrive`] | every ship | the frame's |
//!
//! The entry's prepare runs after the gain is entered, so at the carried
//! gain (0 after a game that ended landed) by the engine's reading of
//! `RuleKey::SpeedMultRamp`, at full gain by the other. `Session::load`
//! and `Session::start` step no display: they only put each slot in its
//! layer, shown or hidden, and clone its frames
//! (`Session::prepare_slots`), which `_PutSpriteInLayer` keeps for a slot
//! already there (@0x768c), so a session no entry prepares (the
//! developer's flight) still draws its ships from the first frame.
//!
//! **Left out**, filed as rdm:phase/rendering-2/phase-5-left-out-of-rendering-13:
//! a hidden ship's display (`Cost` -1, `RuleKey::HiddenShips`) skips from
//! @0x2b7f3 to @0x2cc6b, past the set, tint and lights, which every engine
//! display still steps; and a loaded pilot's escorts, which
//! `_LoadPilotData` spawns into slots 1 and up (@0x759af-0x75b91) before
//! the entry's prepare, have no NPC here until the first step's setup, so
//! they come in one step behind.

use super::Session;
use super::player_led::slot_of;
use crate::SpeedMult;
use crate::catalog::SystemId;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::ship_layer::{Slot, disabled};

/// One slot of `_PrepareAllSprites`' loop (@0x3a776-0x3a78e): its ship,
/// its slot, the `_IsDisabled` its display keeps (@0x2b61f), and whether
/// the player leads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PreparedSlot {
    /// The ship in the slot.
    pub(super) ship: ShipRef,
    /// Its slot.
    pub(super) slot: Slot,
    /// The `_IsDisabled` its display keeps (see the module docs).
    pub(super) disabled: bool,
    /// Whether the player leads it.
    pub(super) led: bool,
}

impl Session {
    /// Every slot `_PrepareAllSprites` displays, the player's first, then
    /// each NPC's in slot order, each with the `_IsDisabled` its display
    /// keeps (see the module docs).
    pub(super) fn prepared_slots(&self) -> Vec<PreparedSlot> {
        let mut slots = vec![PreparedSlot {
            ship: ShipRef::Player,
            slot: Slot::Player,
            // The player defends no stellar.
            disabled: disabled(self.player_condition(), false),
            led: false,
        }];
        let mut npcs: Vec<PreparedSlot> = self
            .npcs()
            .iter()
            .map(|npc| PreparedSlot {
                ship: ShipRef::Npc(npc.id),
                slot: slot_of(npc),
                disabled: disabled(npc.condition, npc.defends.is_some()) || self.held_disabled(npc),
                led: npc.escorts_player(),
            })
            .collect();
        npcs.sort_by_key(|prepared| prepared.slot);
        slots.extend(npcs);
        slots
    }

    /// Each of `slots`' set step, ion tint and lights, in order, at
    /// `gain`, read as its slot's `disabled`: the player's through
    /// `PlayerShip::display_frames`, each NPC's through
    /// `Space::display_npc_frames`, drawing on `chance`. They run each
    /// slot in turn, as an NPC's lights write its berth's blink timer,
    /// which the next ship in the berth reads.
    pub(super) fn display_slots(
        &mut self,
        slots: &[PreparedSlot],
        gain: SpeedMult,
        chance: &mut (impl Chance + ?Sized),
    ) {
        for prepared in slots {
            match prepared.ship {
                ShipRef::Player => self.ship.display_frames(
                    &mut self.pilot,
                    &self.statics,
                    &self.setup,
                    prepared.disabled,
                    gain,
                    &mut *chance,
                ),
                ShipRef::Npc(id) => self.space.display_npc_frames(
                    id,
                    prepared.disabled,
                    gain,
                    &self.statics,
                    &self.setup,
                    &mut *chance,
                ),
            }
        }
    }

    /// The gate entry's `_PrepareAllSprites` (@0x63e91, @0x64743), the
    /// exit in system `exit`, at the frame's gain: the player's slot
    /// always passes the display's system test (@0x2b7d8-0x2b7e0), the
    /// old ships only when `exit` is the system gone in, as it runs once
    /// the player's system is the exit's (@0x639fd, @0x642a7) and before
    /// they are deleted (@0x67096). Each passing ship's set, tint and
    /// lights step, drawing on `chance`, then its fade (see the module
    /// docs and the `slot_visibility` module). The rest of that display
    /// (layers, visibility) is redone by the exit's own two calls.
    pub(super) fn prepare_at_gate_entry(
        &mut self,
        exit: SystemId,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let here = self.pilot.system;
        let gain = self.speed_mult();
        let slots: Vec<PreparedSlot> = self
            .prepared_slots()
            .into_iter()
            .filter(|prepared| prepared.ship == ShipRef::Player || exit == here)
            .collect();
        self.display_slots(&slots, gain, &mut *chance);
        self.ship.step_cloak_fade(&self.pilot, &self.statics, gain);
        self.space.step_fades_at_gate_entry(exit, here, gain);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::SpeedMult;
    use crate::catalog::GovtId;
    use crate::catalog::{HullRecord, ShipDisplay, ShipExits, ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::armament::Fold;
    use crate::combat::hull::Condition;
    use crate::combat::ion::Ion;
    use crate::combat::ion_tint::{self, IonTint};
    use crate::combat::ship_frames::{FrameSpec, LIGHTS_OUT_WHILE_DISABLED, Moment, ShipFrames};
    use crate::combat::ship_lights::ShipLights;
    use crate::flight::Flyer;
    use crate::gate::GateKind;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::display_gain::tests::{
        FADING_IN, HALF, animated_and_lit, moment, stepped, with_animated_npc,
    };
    use crate::session::ship_display::tests::displaying;
    use crate::testkit::{
        FAST, FakePilotCatalog, begin_jump_now, far_129_reachable, fly_out, hull, land_now, ship,
    };
    use crate::traffic::npc::NpcId;

    /// A ship's display as it stands before a prepare.
    #[derive(Clone, Copy, Debug)]
    struct Before {
        spec: FrameSpec,
        frames: ShipFrames,
        fold: Fold,
        ion: Ion,
        capacity: f32,
    }

    impl Before {
        fn of(session: &Session, ship: ShipRef) -> Self {
            match ship {
                ShipRef::Player => Self {
                    spec: session.ship.fit.hull.frames,
                    frames: session.ship.ship_display.frames,
                    fold: session.ship.fit.armament.fold(),
                    ion: session.ship.ion,
                    capacity: session.ship.ion_capacity(),
                },
                ShipRef::Npc(id) => {
                    let npc = session.space.npc(id).expect("there");
                    Self {
                        spec: npc.hull.frames,
                        frames: npc.display.frames,
                        fold: npc.armament.fold(),
                        ion: npc.ion,
                        capacity: npc.stats.ion.capacity(Flyer::Npc),
                    }
                }
            }
        }

        /// Its frames after `steps` steps of its sets and lights at
        /// `gain`, read as `disabled`, every roll answered 0.
        fn stepped(
            &self,
            session: &Session,
            steps: usize,
            gain: SpeedMult,
            disabled: bool,
        ) -> ShipFrames {
            let moment = Moment {
                disabled,
                ..moment(session, gain)
            };
            let (mut frames, mut fold) = (self.frames, self.fold);
            for _ in 0..steps {
                frames.step(&self.spec, &mut fold, moment);
                frames.light(&self.spec, &moment, |_| 0);
            }
            frames
        }

        /// Its ion tint after one display on [`NeverFires`].
        fn tint(&self) -> Option<IonTint> {
            let mut ion = self.ion;
            ion_tint::display(&mut ion, self.capacity, &mut NeverFires)
        }
    }

    /// `ship`'s frames as they stand.
    fn frames_of(session: &Session, ship: ShipRef) -> ShipFrames {
        Before::of(session, ship).frames
    }

    /// `ship`'s ion tint as its last display left it.
    fn tint_of(session: &Session, ship: ShipRef) -> Option<IonTint> {
        match ship {
            ShipRef::Player => session.player_ion_tint(),
            ShipRef::Npc(id) => session.space.npc(id).expect("there").display.ion_tint,
        }
    }

    /// Gives the player [`animated_and_lit`]'s class, or `spec`, its
    /// lights timer at 3.
    fn light_player(session: &mut Session, spec: FrameSpec) {
        session.ship.fit.hull.frames = spec;
        session.ship.ship_display.frames.lights.timer = 3.0;
    }

    /// Charges `ship` to half its capacity of 100, in colour 0x80ff.
    fn ionise(session: &mut Session, ship: ShipRef) {
        let ion = Ion {
            charge: 50.0,
            colour: 0x0000_80ff,
            ..Ion::default()
        };
        match ship {
            ShipRef::Player => {
                session.ship.fit.stats.ion.class_capacity = 100.0;
                session.ship.ion = ion;
            }
            ShipRef::Npc(id) => {
                let npc = session.space.npc_mut(id).expect("there");
                npc.stats.ion.class_capacity = 100.0;
                npc.ion = ion;
            }
        }
    }

    /// A copy of NPC `of` added to `session`, its condition `condition`.
    fn twin(session: &mut Session, of: NpcId, condition: Condition) -> NpcId {
        let mut npc = session.space.npc(of).expect("there").clone();
        npc.condition = condition;
        npc.reserves.armor.now = if condition == Condition::Intact {
            npc.reserves.armor.max
        } else {
            0.0
        };
        session.space.traffic.add_npc(npc)
    }

    #[test]
    fn prepare_all_sprites_steps_every_ships_set_lights_and_tint_once_in_slot_order() {
        // `_PrepareAllSprites` runs `_HandleShipDisplay` for slots 0-63 in
        // order (@0x3a776-0x3a78e, @0x3a783): each ship's set
        // (@0x2b7d8-0x2bc2e), ion tint (@0x2beb9-0x2bf94) and lights
        // (@0x2c2ea-0x2c5a6) at the frame's gain.
        let catalog = displaying(0, 1);
        let (mut session, a) = with_animated_npc(&catalog, Condition::Intact);
        let b = twin(&mut session, a, Condition::Intact);
        light_player(&mut session, animated_and_lit());
        ionise(&mut session, ShipRef::Player);
        ionise(&mut session, ShipRef::Npc(a));
        session.at_gain(HALF);
        let ships = [ShipRef::Player, ShipRef::Npc(a), ShipRef::Npc(b)];
        let order: Vec<ShipRef> = session
            .prepared_slots()
            .iter()
            .map(|prepared| prepared.ship)
            .filter(|ship| ships.contains(ship))
            .collect();
        assert_eq!(order, ships, "in slot order");
        let before: Vec<Before> = ships
            .iter()
            .map(|&ship| Before::of(&session, ship))
            .collect();
        session.prepare_all_sprites(&mut NeverFires);
        for (&ship, before) in ships.iter().zip(&before) {
            let expected = before.stepped(&session, 1, HALF, false);
            assert_eq!(frames_of(&session, ship), expected, "{ship:?}");
            let full = before.stepped(&session, 1, SpeedMult::FULL, false);
            assert_ne!(expected, full, "{ship:?}: the gain shows");
            assert_eq!(tint_of(&session, ship), before.tint(), "{ship:?}");
        }
        assert!(tint_of(&session, ShipRef::Player).is_some(), "ionised");
        assert!(tint_of(&session, ShipRef::Npc(a)).is_some(), "ionised");
        assert_eq!(tint_of(&session, ShipRef::Npc(b)), None, "not ionised");
    }

    #[test]
    fn prepare_all_sprites_reads_a_disabled_ship_as_disabled() {
        // Each slot's display keeps its `_IsDisabled` (@0x2b61f), which
        // the lights' 0x0040 lights-out reads (@0x2c585): the armour's
        // (@0x2d79-0x2dcc) and a Derelict government's (@0x2d0f).
        let lights_out = FrameSpec {
            flags: animated_and_lit().flags | LIGHTS_OUT_WHILE_DISABLED,
            ..animated_and_lit()
        };
        let mut catalog = displaying(0, 1);
        catalog.govts = vec![crate::catalog::GovtRecord {
            flags: crate::govt::DERELICT,
            ..nova_sim_catalog::fixture::govt(131)
        }];
        let (mut session, out) = with_animated_npc(&catalog, Condition::Disabled);
        let npc = session.space.npc_mut(out).expect("there");
        npc.hull.frames = lights_out;
        // Lit, so a lights out shows.
        npc.display.frames.lights.intensity = 32.0;
        let lit = twin(&mut session, out, Condition::Intact);
        let derelict = twin(&mut session, out, Condition::Intact);
        session.space.npc_mut(derelict).expect("there").govt = Some(GovtId(131));
        light_player(&mut session, lights_out);
        session.ship.ship_display.frames.lights.intensity = 32.0;
        session.ship.condition = Condition::Disabled;
        session.at_gain(SpeedMult::FULL);
        let ships = [
            (ShipRef::Player, true),
            (ShipRef::Npc(out), true),
            (ShipRef::Npc(lit), false),
            (ShipRef::Npc(derelict), true),
        ];
        let before: Vec<Before> = ships
            .iter()
            .map(|&(ship, _)| Before::of(&session, ship))
            .collect();
        session.prepare_all_sprites(&mut NeverFires);
        for (&(ship, disabled), before) in ships.iter().zip(&before) {
            let expected = before.stepped(&session, 1, SpeedMult::FULL, disabled);
            assert_eq!(frames_of(&session, ship), expected, "{ship:?}");
            let other = before.stepped(&session, 1, SpeedMult::FULL, !disabled);
            assert_ne!(expected.lights, other.lights, "{ship:?}: the reading shows");
        }
    }

    /// [`displaying`] with the player's class animated and lit as
    /// [`animated_and_lit`]'s.
    fn lit_player() -> FakePilotCatalog {
        let mut catalog = displaying(animated_and_lit().flags, 3);
        let display = catalog.hulls[0].display.as_mut().expect("a shän");
        display.blink = animated_and_lit().blink;
        display.lights_image = true;
        catalog
    }

    /// A new pilot's session of [`lit_player`] by `ramp`'s reading of
    /// `RuleKey::SpeedMultRamp`, the player ionised, not yet entered.
    fn started(ramp: RuleSource) -> Session {
        let rules = Rulebook::default().with_override(RuleKey::SpeedMultRamp, ramp);
        let mut session = Session::start(&lit_player())
            .expect("starts")
            .with_rules(rules);
        ionise(&mut session, ShipRef::Player);
        session.ship.cloak.cloak = FADING_IN;
        session
    }

    #[test]
    fn entering_play_steps_the_players_frames_once_at_the_entrys_gain() {
        // `_EnterGameFromMainScreen`'s `_PrepareAllSprites` (@0x1a36c)
        // runs at the gain the last game left: nothing before it writes
        // `_gSpeedMult` (@0x19ca7-0x1a36c).
        let session = started(RuleSource::Engine);
        assert_eq!(
            session.ship.ship_display.frames,
            ShipFrames::default(),
            "no display at construction"
        );
        assert_eq!(session.player_ion_tint(), None, "nor a tint");
        for gain in [HALF, SpeedMult::ZERO] {
            let mut session = started(RuleSource::Engine);
            let before = Before::of(&session, ShipRef::Player);
            session.enter_game(gain, &mut NeverFires);
            let expected = before.stepped(&session, 1, gain, false);
            assert_eq!(session.ship.ship_display.frames, expected, "{gain:?}");
            let full = before.stepped(&session, 1, SpeedMult::FULL, false);
            assert_ne!(expected, full, "{gain:?}: the gain shows");
            assert_eq!(session.player_ion_tint(), before.tint(), "{gain:?}");
            assert!(session.player_ion_tint().is_some(), "{gain:?}");
        }
    }

    #[test]
    fn by_the_other_reading_entering_play_steps_the_players_frames_at_full_gain() {
        let mut session = started(RuleSource::Bible);
        let before = Before::of(&session, ShipRef::Player);
        session.enter_game(SpeedMult::ZERO, &mut NeverFires);
        assert_eq!(
            session.ship.ship_display.frames,
            before.stepped(&session, 1, SpeedMult::FULL, false)
        );
    }

    #[test]
    fn entering_play_steps_the_players_fade_at_the_entrys_gain() {
        // Slot 0's display steps its fade (@0x2d088-0x2d164) at @0x1a36c.
        for (ramp, gain) in [
            (RuleSource::Engine, HALF),
            (RuleSource::Bible, SpeedMult::FULL),
        ] {
            let mut session = started(ramp);
            session.enter_game(HALF, &mut NeverFires);
            assert_eq!(
                session.player_cloak(),
                stepped(FADING_IN, false, gain),
                "{ramp:?}"
            );
        }
        let mut session = started(RuleSource::Engine);
        session.enter_game(SpeedMult::ZERO, &mut NeverFires);
        assert_eq!(
            session.player_cloak(),
            stepped(FADING_IN, false, SpeedMult::ZERO),
            "held"
        );
        assert_ne!(
            session.player_cloak(),
            stepped(FADING_IN, false, SpeedMult::FULL),
            "not at the construction's full gain"
        );
    }

    #[test]
    fn entering_a_set_up_system_prepares_every_ship_twice() {
        // The entry's `_PrepareAllSprites` (@0x1a36c) and `_PlayGame`'s
        // (@0x45834), the system's ships already in place.
        let catalog = displaying(0, 1);
        let (mut session, id) = with_animated_npc(&catalog, Condition::Intact);
        light_player(&mut session, animated_and_lit());
        let ships = [ShipRef::Player, ShipRef::Npc(id)];
        let before: Vec<Before> = ships
            .iter()
            .map(|&ship| Before::of(&session, ship))
            .collect();
        session.enter_game(HALF, &mut NeverFires);
        for (&ship, before) in ships.iter().zip(&before) {
            let expected = before.stepped(&session, 2, HALF, false);
            assert_eq!(frames_of(&session, ship), expected, "{ship:?}");
            assert_ne!(
                expected,
                before.stepped(&session, 1, HALF, false),
                "{ship:?}: twice shows"
            );
        }
    }

    /// [`animated_and_lit`]'s class as a `shän`.
    fn lit_display() -> ShipDisplay {
        let spec = animated_and_lit();
        ShipDisplay {
            flags: spec.flags,
            base_sets: 3,
            anim_delay: 100,
            lights_image: true,
            blink: spec.blink,
            ..ShipDisplay::default()
        }
    }

    /// `catalog` with ship types `ids` of [`lit_display`]'s class.
    fn lit(mut catalog: FakePilotCatalog, ids: &[i16]) -> FakePilotCatalog {
        for &id in ids {
            let exits = Some(ShipExits {
                frames: 36,
                ..ShipExits::default()
            });
            match catalog.hulls.iter_mut().find(|hull| hull.id == ShipId(id)) {
                Some(hull) => {
                    hull.display = Some(lit_display());
                    hull.exits = hull.exits.or(exits);
                }
                None => catalog.hulls.push(HullRecord {
                    display: Some(lit_display()),
                    exits,
                    ..hull(id)
                }),
            }
        }
        catalog
    }

    /// Sets `ship`'s blink timer below 0, so its next lights step lights
    /// it whatever the gain, and its animation timer to 50.
    fn prime(session: &mut Session, ship: ShipRef) {
        let frames = match ship {
            ShipRef::Player => &mut session.ship.ship_display.frames,
            ShipRef::Npc(id) => &mut session.space.npc_mut(id).expect("there").display.frames,
        };
        frames.lights.timer = -1.0;
        frames.anim_timer = 50.0;
    }

    /// The parts of `frames` no fold writes: the animation timer and the
    /// lights.
    fn timers(frames: ShipFrames) -> (f32, ShipLights) {
        (frames.anim_timer, frames.lights)
    }

    /// The first escort of `session`'s fleet.
    fn first_escort(session: &Session) -> ShipRef {
        ShipRef::Npc(session.space.escorts.fleet[0].expect("placed"))
    }

    #[test]
    fn the_first_setup_prepares_every_ship_it_sets_up() {
        // `_PlayGame`'s `_PrepareAllSprites` (@0x45834) and the take-off's
        // first (@0x6764e), after the setup: an escort keeps its display
        // through `_RespawnEscort` and steps it once.
        let mut catalog = lit(displaying(0, 1), &[129]);
        catalog.ship_records.push(ship(129, FAST));
        let mut session = Session::start(&catalog).expect("starts");
        session
            .pilot
            .escorts
            .push(crate::session::tests::trader_escort());
        session.populate(&catalog, &mut NeverFires);
        let escort = first_escort(&session);
        prime(&mut session, escort);
        session.at_gain(HALF);
        let before = Before::of(&session, escort);
        session.populate(&catalog, &mut NeverFires);
        let escort = first_escort(&session);
        let expected = before.stepped(&session, 1, HALF, false);
        assert_eq!(timers(frames_of(&session, escort)), timers(expected));
        assert_ne!(timers(expected), timers(before.frames), "a step shows");
    }

    #[test]
    fn a_take_off_steps_every_ships_frames_twice_at_its_gain() {
        // `_HandlePlayerDockRequest`'s two `_PrepareAllSprites` (@0x6764e,
        // @0x67658), at the gain `_PlayerLandOnStellar` zeroed (@0x6375e)
        // by the engine's reading, at full gain by the other.
        use crate::session::escorts::tests::{fleeted, flying};
        let catalog = lit(far_129_reachable(fleeted()), &[128, 150, 151]);
        let mut taken = Vec::new();
        for (ramp, gain) in [
            (RuleSource::Engine, SpeedMult::ZERO),
            (RuleSource::Bible, SpeedMult::FULL),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::SpeedMultRamp, ramp);
            let mut session = flying(&catalog, RuleSource::Engine).with_rules(rules);
            land_now(&mut session).expect("lands");
            let ships = [ShipRef::Player, first_escort(&session)];
            for ship in ships {
                prime(&mut session, ship);
            }
            let before: Vec<Before> = ships
                .iter()
                .map(|&ship| Before::of(&session, ship))
                .collect();
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            let ships = [ShipRef::Player, first_escort(&session)];
            for (&ship, before) in ships.iter().zip(&before) {
                let expected = timers(before.stepped(&session, 2, gain, false));
                assert_eq!(
                    timers(frames_of(&session, ship)),
                    expected,
                    "{ramp:?} {ship:?}"
                );
                taken.push(expected);
            }
        }
        assert_ne!(taken[0], taken[2], "the gain shows");
    }

    #[test]
    fn an_arrival_steps_every_ships_frames_once_at_the_frames_gain() {
        // `_HandlePlayer`'s `_PrepareAllSprites` on arrival (@0x6c59c).
        use crate::session::escorts::tests::{fleeted, flying};
        let catalog = lit(fleeted(), &[128, 150, 151]);
        let mut session = flying(&catalog, RuleSource::Engine);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        let ships = [ShipRef::Player, first_escort(&session)];
        for ship in ships {
            prime(&mut session, ship);
        }
        ionise(&mut session, ShipRef::Player);
        session.at_gain(HALF);
        let before: Vec<Before> = ships
            .iter()
            .map(|&ship| Before::of(&session, ship))
            .collect();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        let ships = [ShipRef::Player, first_escort(&session)];
        for (&ship, before) in ships.iter().zip(&before) {
            let expected = timers(before.stepped(&session, 1, HALF, false));
            assert_eq!(timers(frames_of(&session, ship)), expected, "{ship:?}");
            let full = timers(before.stepped(&session, 1, SpeedMult::FULL, false));
            assert_ne!(expected, full, "{ship:?}: the gain shows");
        }
        assert_eq!(session.player_ion_tint(), before[0].tint());
        assert!(session.player_ion_tint().is_some(), "the player's tint");
    }

    #[test]
    fn a_gate_passage_steps_the_player_three_times_and_an_old_escort_by_its_exit() {
        // `_PlayerEnterHypergate`'s `_PrepareAllSprites` (@0x63e91), once
        // the player's system is the exit's (@0x639fd): slot 0 always
        // passes the display's system test (@0x2b7d8-0x2b7e0), the old
        // ships only with the exit in the system gone in. Then the exit's
        // two (@0x6764e, @0x67658).
        use crate::session::gate_escorts::tests::{exit_into_130, flying, gated_into_130};
        use crate::session::gate_follow::tests::exit_gate_raising;
        for (into_130, old_steps) in [(true, 3), (false, 2)] {
            let catalog = lit(gated_into_130(), &[128, 200]);
            let mut session = flying(&catalog, Rulebook::default());
            let ships = [ShipRef::Player, first_escort(&session)];
            for ship in ships {
                prime(&mut session, ship);
            }
            session.at_gain(HALF);
            let before: Vec<Before> = ships
                .iter()
                .map(|&ship| Before::of(&session, ship))
                .collect();
            if into_130 {
                exit_into_130(&mut session, &catalog);
            } else {
                exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
            }
            let ships = [ShipRef::Player, first_escort(&session)];
            for ((&ship, before), steps) in ships.iter().zip(&before).zip([3, old_steps]) {
                let expected = before.stepped(&session, steps, HALF, false);
                assert_eq!(
                    timers(frames_of(&session, ship)),
                    timers(expected),
                    "into 130: {into_130}, {ship:?}"
                );
            }
        }
    }

    #[test]
    fn a_wormhole_with_no_exit_steps_every_ship_twice_at_the_frames_gain() {
        // The tail's two `_PrepareAllSprites` (@0x6764e, @0x67658), at the
        // frame's gain: no landing zeroed it.
        use crate::session::tests::fleet_escort;
        use crate::session::wormhole_no_exit::tests::{fought, press_over, refuse};
        let catalog = lit(fought(crate::session::tests::blaster()), &[128, 129]);
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.escorts = vec![fleet_escort(129)];
        session.populate(&catalog, &mut NeverFires);
        press_over(&mut session);
        let ships = [ShipRef::Player, first_escort(&session)];
        for ship in ships {
            prime(&mut session, ship);
        }
        session.at_gain(HALF);
        let before: Vec<Before> = ships
            .iter()
            .map(|&ship| Before::of(&session, ship))
            .collect();
        refuse(&mut session, &catalog);
        let ships = [ShipRef::Player, first_escort(&session)];
        for (&ship, before) in ships.iter().zip(&before) {
            let expected = timers(before.stepped(&session, 2, HALF, false));
            assert_eq!(timers(frames_of(&session, ship)), expected, "{ship:?}");
            assert_ne!(
                expected,
                timers(before.stepped(&session, 2, SpeedMult::FULL, false)),
                "{ship:?}: the gain shows"
            );
        }
    }
}
