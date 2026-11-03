//! The frame's gain in each ship's display, where the session hands it on
//! (`_HandleShipDisplay`'s `_gSpeedMult`, see `nova_sim_base::speed_mult`).
//!
//! Only `Session` methods read `Session::speed_mult` here; each sub-state
//! method takes the gain as a plain parameter:
//!
//! - `Session::player_display_step` hands it to the player's cloak fade
//!   (`PlayerShip::step_cloak_fade`) and frame
//!   (`PlayerShip::display_frames`, through `Moment::gain` to its sets and
//!   lights);
//! - `Session::ship_loop` to each NPC's display on both of its branches
//!   (`Space::step_displayed_npc`, then `Space::display_npc_frames` and
//!   `Traffic::display_npc`) and to the NPCs' fades
//!   (`Traffic::step_fades`);
//! - `Session::game_work`'s tail to the NPCs no loop display reached
//!   (`Space::display_undisplayed`), by the other reading of
//!   [`RuleKey::UndisplayedShips`](crate::RuleKey::UndisplayedShips);
//! - `Session::prepare_all_sprites` to the player's fade and the NPCs';
//! - a gate entry (`Session::come_out`) to the player's fade and
//!   `Space::step_fades_at_gate_entry`.
//!
//! A take-off zeroes the gain before its dock request's tail, so both of
//! that tail's `_PrepareAllSprites` run at 0 (`_PlayerLandOnStellar`
//! @0x6375e, returned @0x66f29 before @0x6764e and @0x67658): a moving
//! fade holds there and a drifting one, unscaled, still falls. Game
//! entry prepares the player's lights and every sprite at the session's
//! gain at construction, full, before `Session::enter_game` sets the
//! carried gain (filed as
//! rdm:phase/rendering/phase-13-prepare-all-sprites-steps-ship-frames).
//! The view takes the flash's and glow's gain from
//! [`PassReport::speed_mult`](crate::flight_frame::PassReport::speed_mult).
//!
//! These tests pin the wiring only: each expected value is the upstream
//! function at the same gain, on a copy of the state before the pass.

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::SpeedMult;
    use crate::catalog::{HullRecord, ShipDisplay, ShipExits, ShipId};
    use crate::chance::NeverFires;
    use crate::cloak::Cloak;
    use crate::combat::Rules;
    use crate::combat::hull::Condition;
    use crate::combat::ship_frames::{ANIMATED, FrameSpec, Moment, ShipFrames};
    use crate::flight::Controls;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::ship_display::tests::displaying;
    use crate::testkit::{FakePilotCatalog, hull, land_now, work};
    use crate::traffic::npc::NpcId;

    const HALF: SpeedMult = SpeedMult::new(0.5);

    /// A fading-in cloak at 10.
    const FADING_IN: Cloak = Cloak {
        fade: 10.0,
        direction: 1,
    };

    /// `cloak` after one display step at `gain`, not fast, `dying` or not.
    fn stepped(cloak: Cloak, dying: bool, gain: SpeedMult) -> Cloak {
        let mut stepped = cloak;
        stepped.display_step(false, dying, gain);
        stepped
    }

    /// The moment a shown ship of `session`'s with no key ship aboard
    /// displays in at `gain`.
    fn moment(session: &Session, gain: SpeedMult) -> Moment {
        Moment {
            disabled: false,
            key_aboard: false,
            rules: session.setup.display_rules(),
            gain,
        }
    }

    /// An animated class of three sets, `AnimDelay` 100, with the Shuttle's
    /// square-wave lights.
    fn animated_and_lit() -> FrameSpec {
        FrameSpec {
            flags: ANIMATED,
            base_sets: 3,
            anim_delay: 100,
            lights: true,
            blink: crate::blink::Blink {
                mode: 1,
                a: 4,
                b: 1,
                c: 2,
                d: 20,
            },
            ..FrameSpec::NONE
        }
    }

    #[test]
    fn a_bare_pass_steps_the_players_fade_at_the_sessions_gain() {
        let catalog = displaying(0, 1);
        let mut session = Session::start(&catalog).expect("starts");
        // Set up, so the pass runs no `_PrepareAllSprites` of its own.
        session.populate(&catalog, &mut NeverFires);
        session.ship.cloak.cloak = FADING_IN;
        session.at_gain(HALF);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.player_cloak(), stepped(FADING_IN, false, HALF));
    }

    #[test]
    fn a_bare_pass_steps_the_players_sets_and_lights_at_the_sessions_gain() {
        let catalog = displaying(0, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.ship.fit.hull.frames = animated_and_lit();
        session.ship.ship_display.frames.lights.timer = 3.0;
        let spec = session.ship.fit.hull.frames;
        let before = session.ship.ship_display.frames;
        let fold = session.ship.fit.armament.fold();
        let expected = frames_stepped(before, &spec, fold, moment(&session, HALF));
        let full = frames_stepped(before, &spec, fold, moment(&session, SpeedMult::FULL));
        session.at_gain(HALF);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.ship.ship_display.frames, expected);
        assert_ne!(expected, full, "the gain shows");
    }

    /// `frames` of class `spec` and fold `fold` after one step of their
    /// sets and lights in `moment`, every roll answered 0.
    fn frames_stepped(
        frames: ShipFrames,
        spec: &FrameSpec,
        mut fold: crate::combat::armament::Fold,
        moment: Moment,
    ) -> ShipFrames {
        let mut stepped = frames;
        stepped.step(spec, &mut fold, moment);
        stepped.light(spec, &moment, |_| 0);
        stepped
    }

    /// A session of `catalog` set up, with an NPC of [`animated_and_lit`]'s
    /// class added, its condition `condition`: the session and the NPC.
    fn with_animated_npc(catalog: &FakePilotCatalog, condition: Condition) -> (Session, NpcId) {
        let mut session = Session::start(catalog).expect("starts");
        // Set up, so a pass keeps the NPC the test adds.
        session.populate(catalog, &mut NeverFires);
        let mut ship = crate::traffic::fixture::npc(0, crate::stats::ShipStats::default());
        ship.hull.frames = animated_and_lit();
        ship.condition = condition;
        if condition != Condition::Intact {
            ship.reserves.armor.now = 0.0;
        }
        let id = session.space.traffic.add_npc(ship);
        session
            .space
            .npc_mut(id)
            .expect("there")
            .display
            .frames
            .lights
            .timer = 3.0;
        (session, id)
    }

    /// NPC `id`'s frames after a bare pass at half gain, and the frames a
    /// copy of them stepped once by its sets and lights has at half gain
    /// and at full gain.
    fn npc_frames_after_a_pass(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        id: NpcId,
    ) -> (ShipFrames, ShipFrames, ShipFrames) {
        let npc = session.space.npc(id).expect("there");
        let (spec, before, fold) = (npc.hull.frames, npc.display.frames, npc.armament.fold());
        let expected = frames_stepped(before, &spec, fold, moment(session, HALF));
        let full = frames_stepped(before, &spec, fold, moment(session, SpeedMult::FULL));
        session.at_gain(HALF);
        work(session, catalog, Controls::default());
        let frames = session.space.npc(id).expect("still there").display.frames;
        (frames, expected, full)
    }

    #[test]
    fn the_ship_loop_displays_an_npc_whose_escape_check_doesnt_run_at_the_sessions_gain() {
        let catalog = displaying(0, 1);
        let (mut session, id) = with_animated_npc(&catalog, Condition::Intact);
        let (frames, expected, full) = npc_frames_after_a_pass(&mut session, &catalog, id);
        assert_eq!(frames, expected);
        assert_ne!(expected, full, "the gain shows");
    }

    #[test]
    fn the_ship_loop_displays_a_dying_npc_on_its_escape_branch_at_the_sessions_gain() {
        let catalog = displaying(0, 1);
        let dying = Condition::Dying {
            ticks_left: 20,
            starting: false,
        };
        let (mut session, id) = with_animated_npc(&catalog, dying);
        let (frames, expected, full) = npc_frames_after_a_pass(&mut session, &catalog, id);
        assert_eq!(frames.anim_timer, expected.anim_timer);
        assert_eq!(frames.lights, expected.lights);
        assert_ne!(expected.anim_timer, full.anim_timer, "the gain shows");
    }

    #[test]
    fn the_ship_loop_steps_each_npcs_fade_at_the_sessions_gain() {
        let catalog = displaying(0, 1);
        let (mut session, id) = with_animated_npc(&catalog, Condition::Intact);
        session.space.npc_mut(id).expect("there").cloak = FADING_IN;
        session.at_gain(HALF);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(
            session.space.npc(id).expect("there").cloak,
            stepped(FADING_IN, false, HALF)
        );
    }

    #[test]
    fn by_the_other_reading_the_tail_displays_an_arrival_at_the_sessions_gain() {
        use crate::ai::reinforcements::{self, Clock, Event};
        use crate::session::reinforcements::tests::{FED, fight, flying, reinforced, timer};
        // The second pass's `_HandleReinforcements` (@0x44504) brings the
        // fleet in after the ship loop; by the other reading of
        // `UndisplayedShips` the tail steps each once.
        let display = ShipDisplay {
            flags: ANIMATED,
            base_sets: 3,
            anim_delay: 100,
            lights_image: true,
            blink: animated_and_lit().blink,
            ..ShipDisplay::default()
        };
        let mut catalog = reinforced();
        catalog.hulls.retain(|hull| hull.id != ShipId(128));
        catalog.hulls.push(HullRecord {
            display: Some(display),
            exits: Some(ShipExits {
                frames: 36,
                ..ShipExits::default()
            }),
            ..hull(128)
        });
        let rules = Rulebook::default().with_override(RuleKey::UndisplayedShips, RuleSource::Bible);
        let mut session = flying(&catalog).with_rules(rules);
        let warship = fight(&mut session, FED, 100.0);
        session.call_for_reinforcements(warship, false, None);
        session.at_gain(HALF);
        // The passes before the one the fleet arrives on, at the gain the
        // clock counts by (nova-sim-ai's `reinforcements::tick`).
        let fleet = session.space.traffic.reinforcement_fleet();
        // A call starts the timer with the countdown at 0 (its gate).
        let mut clock = Clock {
            timer: timer(&session),
            countdown: 0,
        };
        let arrives = |clock: Clock| {
            let mut next = clock;
            reinforcements::tick(&mut next, fleet, true, HALF) == Some(Event::Arrive)
        };
        let mut before_arrival = 0;
        while !arrives(clock) {
            reinforcements::tick(&mut clock, fleet, true, HALF);
            before_arrival += 1;
        }
        for _ in 0..before_arrival {
            work(&mut session, &catalog, Controls::default());
        }
        let before = session.npcs().len();
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        let fleet = &session.npcs()[before..];
        assert!(!fleet.is_empty(), "arrived");
        for npc in fleet {
            // A spawn's lights start at rest on a fresh berth's timer of 0.
            let mut expected = ShipFrames::default();
            expected.light(&npc.hull.frames, &moment(&session, HALF), |_| 0);
            assert_eq!(npc.display.frames.lights, expected.lights);
            let mut full = ShipFrames::default();
            full.light(&npc.hull.frames, &moment(&session, SpeedMult::FULL), |_| 0);
            assert_ne!(expected.lights, full.lights, "the gain shows");
        }
    }

    #[test]
    fn prepare_all_sprites_steps_the_players_fade_and_an_npcs_at_the_sessions_gain() {
        let catalog = displaying(0, 1);
        let (mut session, id) = with_animated_npc(&catalog, Condition::Intact);
        session.ship.cloak.cloak = FADING_IN;
        session.space.npc_mut(id).expect("there").cloak = FADING_IN;
        session.at_gain(HALF);
        session.prepare_all_sprites();
        let half = stepped(FADING_IN, false, HALF);
        assert_eq!(session.player_cloak(), half, "the player's");
        assert_eq!(session.space.npc(id).expect("there").cloak, half, "its");
    }

    #[test]
    fn a_gate_entry_steps_the_players_fade_and_an_old_ships_at_the_sessions_gain() {
        use crate::session::gate_escorts::tests::{
            exit_into_130, fade_in_hired, flying, gated_into_130, hired_cloak,
        };
        let catalog = gated_into_130();
        let mut session = flying(&catalog, Rulebook::default());
        let before = fade_in_hired(&mut session);
        session.ship.cloak.cloak = FADING_IN;
        session.at_gain(HALF);
        exit_into_130(&mut session, &catalog);
        // The entry's step, then the exit's two.
        let mut three = FADING_IN;
        for _ in 0..3 {
            three = stepped(three, false, HALF);
        }
        assert_eq!(session.player_cloak(), three, "the player's");
        assert_eq!(hired_cloak(&session, before), three, "the old escort's");
    }

    /// The fleet's two escorts after a landing and take-off of `source`'s
    /// rulebook reading of `RuleKey::SpeedMultRamp`, the first fading in
    /// and the second drifting with no direction at 10.
    fn taken_off(ramp: RuleSource) -> (Cloak, Cloak) {
        use crate::session::escorts::tests::{fleeted, flying};
        let catalog = crate::testkit::far_129_reachable(fleeted());
        let mut session = flying(&catalog, RuleSource::Engine);
        let rules = Rulebook::default().with_override(RuleKey::SpeedMultRamp, ramp);
        session = session.with_rules(rules);
        let drifting = Cloak {
            fade: 10.0,
            direction: 0,
        };
        let fleet: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .collect();
        session.space.npc_mut(fleet[0]).expect("there").cloak = FADING_IN;
        session.space.npc_mut(fleet[1]).expect("there").cloak = drifting;
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let cloaks: Vec<Cloak> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .map(|&id| session.space.npc(id).expect("there").cloak)
            .collect();
        (cloaks[0], cloaks[1])
    }

    #[test]
    fn a_take_off_runs_both_prepares_at_no_gain() {
        let drifting = Cloak {
            fade: 10.0,
            direction: 0,
        };
        let twice = |cloak, gain| stepped(stepped(cloak, false, gain), false, gain);
        let (fading, drifted) = taken_off(RuleSource::Engine);
        assert_eq!(fading, twice(FADING_IN, SpeedMult::ZERO), "held");
        assert_eq!(drifted, twice(drifting, SpeedMult::ZERO), "drifted");
        assert_ne!(fading, twice(FADING_IN, SpeedMult::FULL), "the gain shows");
        // By the other reading no take-off zeroes the gain.
        let (fading, drifted) = taken_off(RuleSource::Bible);
        assert_eq!(fading, twice(FADING_IN, SpeedMult::FULL));
        assert_eq!(drifted, twice(drifting, SpeedMult::FULL));
    }
}
