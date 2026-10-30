//! The `_HandleStellarSprites` passes the flight runs outside a play frame
//! (play-feel-7 phase 3): each `_PrepareAllSprites` runs one (@0x3a7c9),
//! and game entry one more of its own (@0x1a359). All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! The code is the play frame's (@0x444d2): the frozen flag `*0x3b523c` is
//! not read anywhere in it, the gates' held-open tests read `_dockTarget`,
//! `_dockPermission` and the ship table as each site leaves them, and it
//! plays no sound. Only the gain and the callers' state differ.
//!
//! [`Session::stellar_pass`] steps every gate shown by a pass's gain and
//! raises the pass ([`StellarPass`]), which the view replays on its scene
//! before the next frame is drawn. The streams:
//!
//! | Way in | Passes |
//! |---|---|
//! | Entering play ([`Session::enter_game`]) | `Ticked` x 3 (@0x1a359, @0x1a36c, @0x45834) |
//! | A take-off | `Still` x 2 (@0x6764e, @0x67658) |
//! | A hypergate's or wormhole's exit | `Entry` (@0x63e91, @0x64743), `Ticked` x 2 (@0x6764e, @0x67658) |
//! | A wormhole with no exit | `Ticked` x 2 (@0x6764e, @0x67658) |
//! | A jump's arrival | `Ticked` (@0x6c59c) |
//!
//! **The gain.** `_PlayerLandOnStellar` zeroes `_gSpeedMult` on every path
//! (@0x6375e-0x63769), and only `_PlayGame`'s loop raises it (@0x45b28), so
//! a take-off's two run at gain 0 ([`StellarPass::Still`]). They still test
//! the threshold, so a timer already there, or no delay, still steps.
//!
//! **What holds a gate** ([`GateHolds`]). A gate entry's pass follows a
//! rebuild onto the exit system's stellars (@0x63e8c, @0x6473e), with the
//! old ships and the dock request still standing, which reach only a gate
//! of the system gone in: the session has already dropped the request
//! ([`Session::land`]'s press), so [`GateHolds::Entered`] stands in for it
//! on this pass alone, and an exit in another system holds nothing
//! ([`GateHolds::Nobody`]), the new ships not being there yet. Every other
//! pass reads the ships and the request as they stand; `_dockPermission`
//! is 0 by the tail's two (@0x67064).
//!
//! **Game entry.** Every way into play but the developer's flight runs
//! `_EnterGameFromMainScreen`, so the passes run from
//! [`Session::enter_game`], not from the load or the new pilot's begin.
//! Its third pass is `_PlayGame`'s (@0x45834), after the setup, whose
//! ships can hold a gate. On a first entry the engine sets them up in the
//! first step (`Space::traffic_due`), so the pass is raised for the view
//! at entry and the gates take its step after that setup
//! (`Session::traffic_think`, [`Session::take_entry_gate_pass`]). The
//! warm-up frame before it is drawn under the entry's black and shows no
//! ship either, so nothing the player sees crosses.
//!
//! **Left out**, each recorded in
//! rdm:phase/play-feel-7/phase-3-prepare-all-sprites-steps-stellar-animation:
//!
//! - the slot plumbing (@0x2e744-0x2e7e5, @0x2ee35-0x2ef1a), which the view
//!   draws every frame;
//! - the fractional gain (@0x2e810-0x2e825): only 0 and 1 are modelled, the
//!   engine's tick being 1.0;
//! - `_gSpeedMult` left 0.0 by an earlier game that ended landed,
//!   rdm:phase/play-feel-7-fidelity/phase-3-speed-mult-ramp-after-a-landing-or-dialog;
//! - the stellar step of the two frozen `_DoPlayGameWork(1)` passes
//!   (@0x1a38a, @0x4580c),
//!   rdm:phase/play-feel-7-cross-roadmap/phase-1-stellar-step-of-the-frozen-entry-passes.

use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::gate_sprite::StellarPass;

use super::Session;

/// What may hold a gate open through a pass (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GateHolds {
    /// The ships and the player's dock request, as they stand.
    AsTheyStand,
    /// The ships as they stand, and the player's request, already dropped,
    /// at the gate it entered.
    Entered(StellarId),
    /// Nothing.
    Nobody,
}

impl Session {
    /// Runs `pass` on every gate shown, held as `holds` says, its random
    /// picks drawn on `chance`, and raises it for the view.
    pub(super) fn stellar_pass(
        &mut self,
        pass: StellarPass,
        holds: GateHolds,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.step_gates_by(pass.gain(), holds, chance);
        self.out.stellar_passes.push(pass);
    }

    /// The stellar passes raised since they were last taken, in order;
    /// taking them empties the list.
    pub(crate) fn take_stellar_passes(&mut self) -> Vec<StellarPass> {
        std::mem::take(&mut self.out.stellar_passes)
    }

    /// Game entry's three passes (see the module docs): the first two now,
    /// and the third now only with the ships in place. Otherwise it is
    /// raised now, for the view, and the gates' step is owed to the first
    /// step's setup: the warm-up frame before it shows neither the ships
    /// nor the gate's change, under the entry's black.
    pub(super) fn enter_game_passes(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.stellar_pass(StellarPass::Ticked, GateHolds::AsTheyStand, chance);
        self.stellar_pass(StellarPass::Ticked, GateHolds::AsTheyStand, chance);
        if self.space.traffic_due {
            self.out.stellar_passes.push(StellarPass::Ticked);
            self.space.entry_gate_pass_owed = true;
        } else {
            self.stellar_pass(StellarPass::Ticked, GateHolds::AsTheyStand, chance);
        }
    }

    /// Takes the gates' step of game entry's third pass, if it is owed
    /// ([`Session::enter_game_passes`]), once the system's ships are set
    /// up: the view has its pass already.
    pub(super) fn take_entry_gate_pass(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if std::mem::take(&mut self.space.entry_gate_pass_owed) {
            self.step_gates_by(StellarPass::Ticked.gain(), GateHolds::AsTheyStand, chance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionEvent;
    use crate::ai::NovaAi;
    use crate::catalog::{
        DudeId, DudeRecord, FleetId, FleetRecord, GovtId, LandingSite, ShipId, SiteAnimation,
        SystemId, SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::clearance::{CLEARED, DockRequest};
    use crate::combat::Rules;
    use crate::control::Test;
    use crate::flight::{Controls, ShipState};
    use crate::gate::{GateRefusal, HYPERGATE};
    use crate::gate_sprite::{GateAnimation, GateSprite};
    use crate::geometry::Vec2;
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::session::SavedOpening;
    use crate::session::tests::{at_gate, at_wormhole, gate_site, gated, wormholes};
    use crate::testkit::{
        FAST, FakePilotCatalog, Scripted, catalog, jump, land_now, planet, ship, work_with,
    };
    use crate::traffic::npc::{Mode, Npc};

    const PASSES_THROUGH_A_GATE: [StellarPass; 3] =
        [StellarPass::Entry, StellarPass::Ticked, StellarPass::Ticked];
    const ENTERING_PLAY: [StellarPass; 3] = [StellarPass::Ticked; 3];

    /// Hypergate `id` of 6 frames stepping every `delay` ticks.
    fn gate(id: i16, delay: i16) -> LandingSite {
        LandingSite {
            flags2: HYPERGATE,
            animation: SiteAnimation {
                delay,
                type_frames: 6,
                ..SiteAnimation::default()
            },
            ..planet(id, 100.0, 200.0)
        }
    }

    /// [`gated`], 131 showing hypergate 310, its exit, of `AnimDelay` 3,
    /// and hypergate 311 of `AnimDelay` 5.
    fn gate_exits() -> FakePilotCatalog {
        let mut catalog = gated();
        catalog.sites[1].1.extend([gate(310, 3), gate(311, 5)]);
        catalog
    }

    /// [`catalog`] with system 130 showing planet 128 and `gates`.
    fn showing(gates: impl IntoIterator<Item = LandingSite>) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![planet(128, 0.0, 0.0)];
        catalog.sites[0].1.extend(gates);
        catalog
    }

    fn sprite(session: &Session, id: i16) -> GateSprite {
        session.space.gate_sprites[&StellarId(id)]
    }

    fn animation(session: &Session, id: i16) -> GateAnimation {
        GateAnimation::of(session.chart.gate_site(StellarId(id)).expect("shown"))
    }

    /// `from` after `ticks` ticks of `anim`, released.
    fn stepped(mut from: GateSprite, anim: &GateAnimation, ticks: usize) -> GateSprite {
        for _ in 0..ticks {
            from.step(anim, false, false, &mut NeverFires);
        }
        from
    }

    /// `from` after the take-off's reset.
    fn reset(mut from: GateSprite) -> GateSprite {
        from.take_off_reset();
        from
    }

    fn tick(session: &mut Session, catalog: &FakePilotCatalog) {
        work_with(
            session,
            catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// Adds a ship coming out of `gate`.
    fn emerging_from(session: &mut Session, gate: i16) {
        let fields = crate::stats::ShipStats::new(FAST, &[]);
        session.space.traffic.add_npc(Npc {
            state: ShipState {
                position: Vec2::new(100.0, 200.0),
                velocity: Vec2::ZERO,
                heading: 90.0,
            },
            mode: Mode::Emerging { ticks_left: 1000 },
            emerges_from: Some(StellarId(gate)),
            ..crate::testkit::npc(0, fields)
        });
    }

    #[test]
    fn a_hypergate_exit_steps_each_gate_of_the_exit_system_once_for_each_of_its_three_passes() {
        let catalog = gate_exits();
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
        assert_eq!(session.take_stellar_passes(), PASSES_THROUGH_A_GATE);
        let second = animation(&session, 311);
        assert_eq!(
            sprite(&session, 311),
            stepped(GateSprite::after_reset(), &second, 3),
            "never shown: its timer 3, its frames the reset's"
        );
        let exit = animation(&session, 310);
        assert_eq!(
            sprite(&session, 310),
            stepped(GateSprite::exit_gate(&exit), &exit, 2),
            "opened after the entry's pass"
        );
    }

    #[test]
    fn a_wormhole_exit_steps_the_gates_shown_three_times_and_opens_none() {
        let mut catalog = wormholes();
        catalog.sites[1].1.push(gate(311, 5));
        let mut session = at_wormhole(&catalog);
        session
            .enter_wormhole(&catalog, &mut Scripted::rolling(&[0]))
            .expect("enters");
        assert_eq!(session.take_stellar_passes(), PASSES_THROUGH_A_GATE);
        let anim = animation(&session, 311);
        assert_eq!(
            sprite(&session, 311),
            stepped(GateSprite::after_reset(), &anim, 3)
        );
    }

    #[test]
    fn a_refused_wormhole_steps_the_gates_shown_twice_with_its_request_let_go() {
        let mut catalog = wormholes();
        catalog.gates.retain(|site| site.id == StellarId(400));
        catalog.sites[0].1.extend([gate(330, 5), gate(331, 1)]);
        let mut session = at_wormhole(&catalog);
        session.take_stellar_passes();
        // The dock request stands, as `_dockPermission` doesn't by
        // @0x6764e (@0x67064).
        session.port.dock = Some(DockRequest {
            stellar: StellarId(331),
            permission: CLEARED,
        });
        let before = sprite(&session, 330);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NoExit)
        );
        assert_eq!(session.take_stellar_passes(), [StellarPass::Ticked; 2]);
        let anim = animation(&session, 330);
        assert_eq!(sprite(&session, 330), stepped(reset(before), &anim, 2));
        assert_eq!(session.gate_frame(StellarId(331)), Some(0), "not held");
    }

    #[test]
    fn a_jumps_arrival_steps_the_gates_shown_once() {
        let catalog = gate_exits();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(session.take_stellar_passes(), [StellarPass::Ticked]);
        let anim = animation(&session, 311);
        assert_eq!(sprite(&session, 311), stepped(GateSprite::new(), &anim, 1));
    }

    #[test]
    fn a_take_off_tests_each_threshold_and_adds_to_no_timer() {
        // Delay 3, bias 2: held on its open frame (3) a gate waits 6 ticks,
        // so a timer of 4 is past the 3 it waits released.
        let bias = |id| {
            let mut site = gate(id, 3);
            site.animation.frame0_bias = 2;
            site
        };
        let catalog = showing([bias(300), bias(301)]);
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let anim = animation(&session, 300);
        let held_for = |ticks| {
            let mut sprite = GateSprite::new();
            for _ in 0..ticks {
                sprite.step(&anim, false, true, &mut NeverFires);
            }
            sprite
        };
        let (past, below) = (held_for(13), held_for(11));
        assert!(past.is_open(&anim) && below.is_open(&anim));
        session.space.gate_sprites.insert(StellarId(300), past);
        session.space.gate_sprites.insert(StellarId(301), below);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.take_stellar_passes(), [StellarPass::Still; 2]);
        assert_eq!(
            sprite(&session, 300),
            GateSprite::after_reset(),
            "its timer zeroed"
        );
        assert_eq!(sprite(&session, 301), reset(below), "its timer kept");
    }

    /// [`showing`] gate `delay`, the only gate: 300.
    fn one_gate() -> FakePilotCatalog {
        showing([gate(300, 5)])
    }

    #[test]
    fn a_new_pilots_gates_take_three_passes_on_entering_play_the_third_in_the_first_step() {
        let catalog = one_gate();
        let pilot = Pilot::new(&catalog, "").expect("starts");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.begin(&catalog, &mut NeverFires);
        assert_eq!(session.take_stellar_passes(), [], "no pass yet");
        let (start, anim) = (sprite(&session, 300), animation(&session, 300));
        session.enter_game(&mut NeverFires);
        assert_eq!(session.take_stellar_passes(), ENTERING_PLAY);
        assert_eq!(sprite(&session, 300), stepped(start, &anim, 2));
        tick(&mut session, &catalog);
        assert_eq!(session.take_stellar_passes(), [], "raised at entry");
        assert_eq!(
            sprite(&session, 300),
            stepped(start, &anim, 4),
            "the third, beside the step's own"
        );
    }

    #[test]
    fn a_saved_pilots_gates_take_the_same_passes_on_entering_play() {
        let catalog = one_gate();
        let pilot = Pilot::new(&catalog, "").expect("starts");
        let mut session = Session::load(&catalog, pilot, &Rulebook::default()).expect("loads");
        session.open_saved(SavedOpening::AtLaunch, &mut NeverFires);
        assert_eq!(session.take_stellar_passes(), [], "no pass yet");
        let (start, anim) = (sprite(&session, 300), animation(&session, 300));
        session.enter_game(&mut NeverFires);
        assert_eq!(session.take_stellar_passes(), ENTERING_PLAY);
        assert_eq!(sprite(&session, 300), stepped(start, &anim, 2));
        tick(&mut session, &catalog);
        assert_eq!(sprite(&session, 300), stepped(start, &anim, 4));
    }

    #[test]
    fn a_re_entry_steps_the_gates_three_times_at_once_with_the_ships_in_place() {
        let catalog = one_gate();
        let mut session = Session::start(&catalog).expect("starts");
        session.enter_game(&mut NeverFires);
        tick(&mut session, &catalog);
        session.take_stellar_passes();
        let (before, anim) = (sprite(&session, 300), animation(&session, 300));
        session.enter_game(&mut NeverFires);
        assert_eq!(session.take_stellar_passes(), ENTERING_PLAY);
        assert_eq!(sprite(&session, 300), stepped(before, &anim, 3));
        tick(&mut session, &catalog);
        assert_eq!(
            sprite(&session, 300),
            stepped(before, &anim, 4),
            "the step's own alone"
        );
    }

    #[test]
    fn a_session_never_entered_steps_its_gates_by_the_first_steps_own_alone() {
        let catalog = one_gate();
        let mut session = Session::start(&catalog).expect("starts");
        let (start, anim) = (sprite(&session, 300), animation(&session, 300));
        assert!(session.space.traffic_due);
        tick(&mut session, &catalog);
        assert_eq!(sprite(&session, 300), stepped(start, &anim, 1));
        assert_eq!(session.take_stellar_passes(), []);
    }

    /// [`gated`] with hypergate 300, its links leading to 301 in the same
    /// system.
    fn same_system_exit() -> FakePilotCatalog {
        let entered = LandingSite {
            flags2: 0x1200,
            ..gate(300, 2)
        };
        let mut catalog = gated();
        catalog.sites[0].1 = vec![
            LandingSite {
                position: Vec2::ZERO,
                animation: SiteAnimation {
                    frame0_bias: 3,
                    ..entered.animation
                },
                ..entered
            },
            gate(301, 5),
        ];
        catalog.gates = vec![
            gate_site(300, (0.0, 0.0), 0x1200, &[301], 120),
            gate_site(301, (100.0, 200.0), HYPERGATE, &[300], 90),
        ];
        catalog.gate_listers = vec![(SystemId(130), StellarId(301))];
        catalog
    }

    #[test]
    fn an_exit_in_the_system_gone_in_holds_the_entered_gate_through_the_entry_pass_alone() {
        let catalog = same_system_exit();
        let mut session = at_gate(&catalog);
        assert_eq!(session.port.dock, None, "the press dropped it");
        // On its open frame (3) held it waits 6 ticks, released 2: a timer
        // of 1 shows which the entry's pass was. Then, off its open frame,
        // held would open a frame where released stays on 0.
        let anim = animation(&session, 300);
        let mut sprite_held = GateSprite::new();
        for _ in 0..7 {
            sprite_held.step(&anim, false, true, &mut NeverFires);
        }
        session
            .space
            .gate_sprites
            .insert(StellarId(300), sprite_held);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(130)), &catalog, &mut NeverFires),
            Ok(SystemId(130))
        );
        assert_eq!(session.take_stellar_passes(), PASSES_THROUGH_A_GATE);
        let mut expected = sprite_held;
        expected.step(&anim, false, true, &mut NeverFires);
        assert_eq!(
            sprite(&session, 300),
            stepped(reset(expected), &anim, 2),
            "held once, then released twice"
        );
    }

    /// A deterministic stream of draws, for a setup whose draws no script
    /// could follow through a gate's exit.
    struct Stream(u64);

    impl crate::chance::Chance for Stream {
        fn fires(&mut self, percent: u8) -> bool {
            self.below(100) < u32::from(percent)
        }

        fn below(&mut self, n: u32) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            // The high half, below 2^32; its remainder below `n`.
            u32::try_from((self.0 >> 32) % u64::from(n)).expect("below n")
        }
    }

    /// [`gated`] with 131 showing hypergates 310 and 311 alone, 311 of
    /// `AnimDelay` 2 and `Frame0Bias` 3, one ship on average, and `flët`
    /// 131 linked to its government: a fleet comes in as it is set up, out
    /// of a gate if the pick finds one.
    fn fleet_at_the_exit() -> FakePilotCatalog {
        let mut slow = gate(311, 2);
        slow.animation.frame0_bias = 3;
        let mut catalog = gated();
        catalog.sites[1].1 = vec![gate(310, 3), slow];
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        catalog.traffic = vec![(
            SystemId(131),
            SystemTraffic {
                dude_types,
                avg_ships: 1,
                persons: Default::default(),
            },
        )];
        catalog.dudes = vec![(
            DudeId(128),
            DudeRecord {
                ai_type: 1,
                govt: Some(GovtId(140)),
                ships: vec![(ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        )];
        catalog.ship_records = vec![ship(129, FAST)];
        catalog.star_map[1].govt = Some(GovtId(141));
        catalog.fleets = vec![FleetRecord {
            id: FleetId(131),
            lead: Some(ShipId(129)),
            escorts: Vec::new(),
            govt: Some(GovtId(150)),
            link_syst: 10_000 + (141 - 128),
            appear_on: Test::default(),
            quote: None,
            flags: 0,
        }];
        catalog
    }

    #[test]
    fn an_exit_in_another_system_takes_no_hold_from_a_ship_coming_out_there() {
        let catalog = fleet_at_the_exit();
        let anim = GateAnimation {
            delay: 2,
            frame0_bias: 3,
            frames: 6,
            ..GateAnimation::default()
        };
        // On its open frame: see the test of the system gone in.
        let mut left = GateSprite::new();
        for _ in 0..7 {
            left.step(&anim, false, true, &mut NeverFires);
        }
        let mut expected = left;
        expected.step(&anim, false, false, &mut NeverFires);
        expected.take_off_reset();
        expected.step(&anim, false, true, &mut NeverFires);
        expected.step(&anim, false, true, &mut NeverFires);
        // Over many draws, as the ship's own come from the setup, before
        // the entry's pass, or from the exit's arrivals pass, after it.
        let came_out: Vec<GateSprite> = (0..6_000)
            .filter_map(|seed| {
                let mut session = at_gate(&catalog);
                session.space.gate_sprites.insert(StellarId(311), left);
                session
                    .enter_hypergate(Some(SystemId(131)), &catalog, &mut Stream(seed))
                    .expect("enters");
                let waiting = session
                    .npcs()
                    .iter()
                    .any(|npc| npc.holds_gate_open(StellarId(311)));
                waiting.then(|| sprite(&session, 311))
            })
            .collect();
        assert!(!came_out.is_empty(), "a ship came out");
        assert!(
            came_out.iter().all(|&after| after == expected),
            "held by the tail's two alone: {came_out:?}"
        );
    }

    #[test]
    fn each_hold_says_what_holds_a_gate_through_a_pass() {
        let catalog = showing([gate(300, 1), gate(301, 1)]);
        let frames = |holds, ship: bool, dock: bool| {
            let mut session = Session::start(&catalog).expect("starts");
            if ship {
                emerging_from(&mut session, 300);
            }
            if dock {
                session.port.dock = Some(DockRequest {
                    stellar: StellarId(301),
                    permission: CLEARED,
                });
            }
            session.stellar_pass(StellarPass::Ticked, holds, &mut NeverFires);
            (
                session.gate_frame(StellarId(300)),
                session.gate_frame(StellarId(301)),
            )
        };
        let as_they_stand = GateHolds::AsTheyStand;
        assert_eq!(frames(as_they_stand, false, false), (Some(0), Some(0)));
        assert_eq!(frames(as_they_stand, true, false), (Some(1), Some(0)));
        assert_eq!(frames(as_they_stand, false, true), (Some(0), Some(1)));
        let entered = GateHolds::Entered(StellarId(301));
        assert_eq!(frames(entered, false, false), (Some(0), Some(1)));
        assert_eq!(frames(entered, true, false), (Some(1), Some(1)));
        assert_eq!(frames(GateHolds::Nobody, true, true), (Some(0), Some(0)));
    }

    #[test]
    fn a_pass_leaves_a_gate_not_shown_alone() {
        let catalog = one_gate();
        let mut session = Session::start(&catalog).expect("starts");
        let left = sprite(&session, 300);
        session.chart.sites.retain(|site| site.id != StellarId(300));
        session.stellar_pass(StellarPass::Ticked, GateHolds::AsTheyStand, &mut NeverFires);
        assert_eq!(sprite(&session, 300), left);
    }

    #[test]
    fn a_pass_at_gain_0_is_raised_and_adds_nothing() {
        let catalog = one_gate();
        let mut session = Session::start(&catalog).expect("starts");
        let left = sprite(&session, 300);
        session.stellar_pass(StellarPass::Still, GateHolds::AsTheyStand, &mut NeverFires);
        assert_eq!(sprite(&session, 300), left);
        assert_eq!(session.take_stellar_passes(), [StellarPass::Still]);
        assert_eq!(session.take_stellar_passes(), [], "taken");
    }

    #[test]
    fn a_pass_raises_no_session_event() {
        let catalog = one_gate();
        let mut session = Session::start(&catalog).expect("starts");
        session.take_events();
        session.enter_game(&mut NeverFires);
        let events: Vec<SessionEvent> = session.take_events();
        assert_eq!(events, []);
    }
}
