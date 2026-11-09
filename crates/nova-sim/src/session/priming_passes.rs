//! Game entry's work after the refresh's snapshot and the gain
//! ([`Session::enter_game`]): the system's ships, the stellar passes and
//! the two priming passes of `_DoPlayGameWork(1)` the original runs
//! before its first frame (rdm combat-and-flight phase 2). All addresses
//! are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! # The original
//!
//! `_EnterGameFromMainScreen` runs `_HandleStellarSprites` (@0x1a359) and
//! `_PrepareAllSprites` (@0x1a36c), then sets `_ignoreKeys` (*0x3b5178)
//! and `_gameFrozen` (*0x3b523c), runs `_DoPlayGameWork(1)` once
//! (@0x1a38a) and clears both (@0x1a371-0x1a397). `_PlayGame` then runs
//! its refresh (`_PropagateMissionBitEffects` @0x457e9,
//! `_HandleStellarSystemVisibility` @0x457ee), the same frozen pass
//! (@0x457f3-0x45819) and its own `_PrepareAllSprites` (@0x45834). The
//! system's ships are in place at all of them: `_SetupShipsInSystem` runs
//! at the load and the new pilot (`_DoMainScreen` @0x1afeb,
//! `_MainScreenChoice` @0x1a8c5, `_DoNewPilot` @0x1937a), never in
//! `_EnterGameFromMainScreen`.
//!
//! **The frozen flag is gone at once.** `_DoPlayGameWork` doesn't read
//! it, nor does `_QuickKeyCheckHandler` (@0x43e01), and its first step,
//! `_HandlePlayer` (@0x43e19), clears it at its head unless
//! `_cheatsActive` (@0x68424-0x68433), on a path every entry reaches. So
//! without cheats every gate on it (`_HandleShipDisplay` @0x2ded6,
//! `_HandleShot` @0x3568c, `_HandleAsteroids` @0x36300, `_HandleExplods`
//! @0x2f629 and @0x2f7eb, `_AIDispatch` @0x8fbbf and the rest) reads 0,
//! and each pass is a whole advance pass: a **priming pass**.
//!
//! **`_ignoreKeys`** holds through the pass: `_QuickKeyCheck` (@0xa0f5),
//! `_QuickKeyCheckPlusEscorts` (@0xa158) and `_KeyCheck` (@0xa1a4) read
//! no key, and `_HandlePlayer` skips its screen-white block
//! (@0x683b5-0x68421).
//!
//! **What isn't the pass's.** None of `_PlayGame`'s loop runs around it:
//! no `_HandleTimeAdjustment` (so the pass runs at the gain the last game
//! left), no frame counter move (@0x45f36), no mission sound (@0x462fa),
//! no double-time latch (@0x463b3), no `_HandleMouse`, no draw. Nor does
//! any real time pass, as the pass is all the original does between the
//! two reads of `TickCount` around it.
//!
//! **After the first pass** (@0x1a38f-0x1a3b5): `_statusRedraw[2]` = 1,
//! `_escortMenuOnscreen` = 0, `_mBarHidden` = 1 and `_gPause` = 0.
//! **After the second** (@0x4581c-0x45831): `_hyperGamma` = −1.0,
//! `_cursorAlpha` = 0 and `_playerKilledByHector` = 0.
//!
//! # Here
//!
//! [`Session::enter_play`], in the original's order:
//!
//! 1. the system's ships set up, if they are still due
//!    (`Space::traffic_due`) and the player is in flight
//!    (`Session::populate_as`, with no display of its own), before every
//!    pass, so a fleet the setup brings out of a gate holds it at the
//!    first stellar pass (@0x1a359);
//! 2. that pass; then `_PrepareAllSprites` (@0x1a36c) and its pass
//!    ([`Session::prepare_all_sprites`], [`Session::stellar_pass`]);
//! 3. priming pass 1 ([`Session::priming_pass`]);
//! 4. `_PlayGame`'s refresh (`Session::refresh_galaxy`): pass 1's
//!    `_MissionHandler` (@0x44214) can meet a goal or fail a deadline whose
//!    script sets a bit it reads, which the flight refreshes nowhere else
//!    before a landing or an arrival;
//! 5. priming pass 2;
//! 6. `_hyperGamma` = −1.0 (`PlayerShip::reset_white`): a white of 0 or
//!    more survives the passes, whose screen-white block is skipped;
//! 7. `_PlayGame`'s `_PrepareAllSprites` and its pass (@0x45834).
//!
//! So an entry runs exactly two `_PrepareAllSprites`, both with the ships
//! in place, three stellar passes and two sprite ticks, one in each
//! priming pass: it steps every stellar shown and every gate five times. [`Session::fly`], the developer's
//! flight, enters no game: its ships are still set up in its first step.
//!
//! **A priming pass** is [`Session::advance_pass`] with none of the
//! frame's head (`Session::pass_head`: the warp-ups and warp-out, an
//! escort's voice clip and the last shot's clock, all real time; the
//! double-time latch, the take-off grace's count and the mission sound,
//! all `_PlayGame`'s loop; the frame counter's move), on the frame counter
//! as it stands; under no controls, with every held key let go
//! (`Session::ignore_keys`) and the screen's white left as it is; at the
//! gain entry set, in double time or not as the session stands; with the
//! hail quote rolls and the line's countdown, which are `_HandleShip`'s
//! (@0x33f0f) and `_HandleComm`'s (@0x4421e), but not the quote clock's
//! tick, which stands for `TickCount`
//! ([`PassCatalogs::priming`](super::game_work::PassCatalogs::priming)); and
//! with no double time's second pass. The NPCs then answer what the
//! session left them and the attack quotes left are said, as after a
//! frame's pass. What the passes raise (sounds, lines, quotes, fight
//! events) stays for the view: the warm-up frame drains the sounds and
//! lines, and the first step's report carries the fight events and
//! quotes. Each priming pass runs the sprite tick after its ship loop, as
//! each of a frame's passes does (`_HandleStellarSprites` @0x444d2,
//! `Session::stellar_sprite_tick`, which reads no `_gameFrozen`): at the
//! gain entry set, gates held as they stand, the ships its own arrivals
//! (`_EnterMoreShips` @0x44237) brought among them, and the dock request
//! too, which `ignore_keys` leaves.
//!
//! A player landed at entry, which no way into play leaves since a pilot
//! opens in space over its stellar, has no ships set up (the take-off sets
//! them up) and runs no priming pass: the original's passes are flight's.
//!
//! **Not modelled**: `_gPause` = 0, as flight has no pause; the
//! status-panel redraw, which the view draws every frame; the escort
//! menu's shut, the view's hand-over (`FlightView::shut_escort_menu`);
//! the menu bar, the app's; and `_playerKilledByHector`, the Captain
//! Hector nag, a non-goal (see the `player_downed` module).
//!
//! `_cursorAlpha` = 0 is built by the view at entry:
//! `nova_view::FlightView::enter_game` calls `HoverCursor::fade_out`.
//!
//! **Left out.** The cheat-mode freeze (`_cheatsActive` keeps
//! `_gameFrozen` through each pass and the loop, @0x6842c, @0x46437)
//! can't be reached: the engine has no cheat mode in flight. A primary
//! button held through the entry clicks in space in priming pass 1, as
//! the click block reads `_Button` live (@0x6a000) and not `_ignoreKeys`:
//! the view hands flight no press held across entry. Filed as one
//! follow-up on the combat-and-flight-2 roadmap.

use super::Session;
use super::StepCollaborators;
use super::game_work::AdvanceRun;
use super::stellar_passes::{GateHolds, PassKind};
use crate::catalog::CommCatalog;
use crate::flight::Controls;

impl Session {
    /// Game entry's work after the refresh's snapshot and the gain, handed
    /// `collab` (see the module docs).
    pub(super) fn enter_play(&mut self, collab: &mut StepCollaborators<'_>) {
        let catalog = collab.catalog;
        let flying = self.mode.landed().is_none();
        // `_SetupShipsInSystem`, at the load or the new pilot.
        if flying && self.space.traffic_due {
            self.populate_as(&catalog, &mut *collab.chance, None);
        }
        // `_EnterGameFromMainScreen` @0x1a359, then `_PrepareAllSprites`
        // @0x1a36c.
        self.stellar_pass(
            PassKind::Ticked,
            GateHolds::AsTheyStand,
            &mut *collab.chance,
        );
        self.prepare_all_sprites(&mut *collab.chance);
        self.stellar_pass(
            PassKind::Ticked,
            GateHolds::AsTheyStand,
            &mut *collab.chance,
        );
        if flying {
            // @0x1a38a.
            self.priming_pass(collab);
            // `_PlayGame` @0x457e9, @0x457ee.
            self.refresh_galaxy(&catalog);
            // @0x4580c.
            self.priming_pass(collab);
        }
        // @0x4581c.
        self.ship.reset_white();
        // `_PlayGame` @0x45834.
        self.prepare_all_sprites(&mut *collab.chance);
        self.stellar_pass(
            PassKind::Ticked,
            GateHolds::AsTheyStand,
            &mut *collab.chance,
        );
    }

    /// One priming pass, `_DoPlayGameWork(1)` under `_ignoreKeys` and
    /// `_gameFrozen`, handed `collab` (see the module docs).
    pub(super) fn priming_pass(&mut self, collab: &mut StepCollaborators<'_>) {
        let catalog = collab.catalog;
        let quotes: &dyn CommCatalog = catalog;
        self.ignore_keys();
        // No head: the frame counter as it stands.
        self.ship.warp.gate = None;
        let frame = self.space.frame;
        self.space.pass_frame = frame;
        // The pass's sprite tick runs after its ship loop, as a frame's
        // does (`_HandleStellarSprites` @0x444d2).
        let run = AdvanceRun {
            quotes: Some(quotes),
            priming: true,
        };
        let mut source = &mut *collab.chance;
        let rescued = self.advance_pass(
            frame,
            Controls::default(),
            &catalog,
            run,
            collab.behaviour,
            collab.rules,
            &mut source,
        );
        if rescued {
            return;
        }
        self.answer_strikes(collab.behaviour);
        self.say_attack_quotes_left(catalog, &mut *collab.chance);
    }

    /// Lets go of every key the session holds, as `_ignoreKeys` has every
    /// key read none (see the module docs). L's latch, `_dockTryFlag`,
    /// isn't a key, and the double-time key is read only by `_PlayGame`'s
    /// loop: both stay.
    fn ignore_keys(&mut self) {
        self.hold_fire(false, false);
        self.hold_eject(false);
        self.hold_self_destruct(false);
        self.hold_cloak(false);
        self.hold_weapon_safety(false);
        self.set_land_key(false);
        self.hold_jettison(None);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // gammas and fades of whole steps, exact
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::SpeedMult;
    use crate::ai::{Behaviour, Decision, Goal, NovaAi, Reaction, Surroundings};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::armament::Trigger;
    use crate::combat::{Rules, ShipRef, Strike};
    use crate::pilot::Pilot;
    use crate::session::tests::trafficked;
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::npc::Npc;

    /// Nova's AI, counting its decisions.
    #[derive(Debug, Default)]
    struct Counting {
        ai: NovaAi,
        decisions: Cell<usize>,
    }

    impl Behaviour for Counting {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.ai.decide(npc, around, chance)
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            self.decisions.set(self.decisions.get() + 1);
            self.ai.decision(npc, around, chance)
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            self.ai.trigger(npc, around)
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            self.ai.target(npc, around)
        }

        fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
            self.ai.react(npc, strike, around)
        }
    }

    /// `session` entering play at `gain`, reading `catalog`, its NPCs
    /// deciding as `behaviour` says, on `chance`.
    fn enter(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        gain: SpeedMult,
        behaviour: &dyn Behaviour,
        chance: &mut dyn Chance,
    ) {
        let mut collab = StepCollaborators {
            catalog,
            behaviour,
            rules: Rules::default(),
            chance,
        };
        session.enter_game(gain, &mut collab);
    }

    /// Two NPCs of [`trafficked`] in 130, a new pilot's session begun.
    fn begun() -> (FakePilotCatalog, Session) {
        let catalog = trafficked(130, 2, 1);
        let pilot = Pilot::new(&catalog, "").expect("starts");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.begin(&catalog, &mut NeverFires);
        (catalog, session)
    }

    #[test]
    fn entry_sets_the_ships_up_before_its_passes() {
        let (catalog, mut session) = begun();
        assert!(session.space.traffic_due, "due before entry");
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &NovaAi::default(),
            &mut NeverFires,
        );
        assert!(!session.space.traffic_due, "set up at entry");
        assert_eq!(session.npcs().len(), 2);
    }

    #[test]
    fn each_priming_pass_thinks_for_every_ship_even_at_gain_0() {
        // `_AIDispatch` (@0x4429d) runs whatever the gain, so at 0 the
        // unscaled thinks still run, once a pass.
        for gain in [SpeedMult::FULL, SpeedMult::ZERO] {
            let (catalog, mut session) = begun();
            let behaviour = Counting::default();
            enter(&mut session, &catalog, gain, &behaviour, &mut NeverFires);
            assert_eq!(behaviour.decisions.get(), 2 * 2, "{gain:?}");
        }
    }

    #[test]
    fn at_full_gain_entry_leaves_the_ships_where_two_whole_passes_leave_them() {
        let (catalog, mut session) = begun();
        // The player drifting under a box that holds it, so each pass's
        // move shows (a run's first box clamps it to rest).
        session.ship.player.velocity = crate::geometry::Vec2::new(1.5, -0.5);
        let top = session.top_speed();
        session
            .ship
            .player_max_vel
            .bleed(top, session.handling().accel);
        let before = session.ship.player;
        let mut twin = session.clone();
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &NovaAi::default(),
            &mut NeverFires,
        );
        // By hand: the setup, then two advance passes under no controls.
        twin.populate(&catalog, &mut NeverFires);
        let set_up: Vec<_> = twin.npcs().iter().map(|npc| npc.state).collect();
        for _ in 0..2 {
            crate::testkit::work_with(
                &mut twin,
                &catalog,
                Controls::default(),
                &NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
        }
        let entered: Vec<_> = session.npcs().iter().map(|npc| npc.state).collect();
        let by_hand: Vec<_> = twin.npcs().iter().map(|npc| npc.state).collect();
        assert_eq!(entered, by_hand);
        assert_ne!(entered, set_up, "they flew");
        // The player too: `_HandlePlayer` moves it in each pass.
        assert_eq!(session.ship.player, twin.ship.player);
        assert_ne!(session.ship.player.position, before.position, "it drifted");
    }

    #[test]
    fn a_new_pilots_entry_shows_the_land_hint_from_its_first_priming_pass() {
        // `_HandlePlayer`'s hint block (@0x7093d) reads no `_ignoreKeys`,
        // and the passes run on the frame counter as it stands, a new
        // pilot's 0, a multiple of 60: the first pass shows the hint,
        // before the first frame, and the second, at -2, shows none.
        use crate::message::SimMessage;
        use crate::sound::SimSound;
        use crate::tutorial::TutorialHint;
        let (catalog, mut session) = begun();
        session.take_messages();
        session.take_sounds();
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &NovaAi::default(),
            &mut NeverFires,
        );
        let hints: Vec<_> = session
            .take_messages()
            .into_iter()
            .filter(|message| matches!(message, SimMessage::Tutorial(_)))
            .collect();
        assert!(
            matches!(
                hints.as_slice(),
                [SimMessage::Tutorial(TutorialHint::Land { .. })]
            ),
            "{hints:?}"
        );
        let beeps = session
            .take_sounds()
            .into_iter()
            .filter(|&sound| sound == SimSound::TutorialHint)
            .count();
        assert_eq!(beeps, 1);
        assert_eq!(session.tutorial().flag(), -2);
    }

    #[test]
    fn a_new_pilots_entry_off_a_hint_frame_shows_no_hint() {
        // The frame counter doesn't move between the passes, so on a frame
        // that is no multiple of 60 neither pass checks.
        let (catalog, mut session) = begun();
        session.space.frame = 1;
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &NovaAi::default(),
            &mut NeverFires,
        );
        assert_eq!(session.tutorial(), crate::tutorial::Tutorial::NEW_PILOT);
    }

    #[test]
    fn entry_writes_the_pilots_whole_class_order_table_as_its_passes_leave_it() {
        // Each pass's bookkeeping writes all four entries (@0x6e43f, see
        // the `class_table` module): a new pilot's -1s, never commanded,
        // read 0 once the ships set up at entry let it run.
        use crate::fleet::OrderWord;
        let (catalog, mut session) = begun();
        assert_eq!(session.pilot.class_orders, [OrderWord::NONE; 4]);
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &NovaAi::default(),
            &mut NeverFires,
        );
        assert_eq!(session.pilot.class_orders, [OrderWord(0); 4]);
    }

    #[test]
    fn entry_moves_the_frame_counter_not_at_all_and_the_first_step_once() {
        let (catalog, mut session) = begun();
        let frame = session.space.frame;
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &NovaAi::default(),
            &mut NeverFires,
        );
        assert_eq!(session.space.frame, frame);
        crate::testkit::work(&mut session, &catalog, Controls::default());
        assert_eq!(session.space.frame, crate::market::next_frame(frame));
    }

    #[test]
    fn entry_displays_every_ship_and_steps_the_fades_as_two_prepares_do() {
        use crate::cloak::Cloak;
        let (catalog, mut session) = begun();
        // A drifting fade falls at each display, unscaled.
        let drifting = Cloak {
            fade: 10.0,
            direction: 0,
        };
        session.ship.cloak.cloak = drifting;
        enter(
            &mut session,
            &catalog,
            SpeedMult::ZERO,
            &NovaAi::default(),
            &mut NeverFires,
        );
        let drawn = session.draw_order();
        for npc in session.npcs() {
            assert!(drawn.contains(&ShipRef::Npc(npc.id)), "{:?} drawn", npc.id);
        }
        // A drifting fade falls a step at each display of the player's
        // slot (@0x2d088-0x2d164), unscaled: the two prepares' and the two
        // passes' own (@0x43e29), and no more.
        let mut stepped = drifting;
        for _ in 0..4 {
            stepped.display_step(false, false, SpeedMult::ZERO);
        }
        assert_eq!(stepped.fade, 6.0);
        assert_eq!(session.player_cloak(), stepped);
    }

    #[test]
    fn a_white_of_0_or_more_before_entry_is_out_after_it() {
        for gamma in [0.0, 40.0] {
            let (catalog, mut session) = begun();
            session.ship.light_white(gamma);
            enter(
                &mut session,
                &catalog,
                SpeedMult::FULL,
                &NovaAi::default(),
                &mut NeverFires,
            );
            assert_eq!(session.hyper_gamma(), -1.0, "{gamma}");
        }
    }

    #[test]
    fn a_priming_pass_leaves_the_white_alone() {
        // `_HandlePlayer` skips its white block under `_ignoreKeys`
        // (@0x683b5-0x68421): out of a jump a frame's pass puts a white
        // of 0 or more out, a priming pass doesn't.
        let (catalog, mut session) = begun();
        session.ship.light_white(40.0);
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &NovaAi::default(),
            rules: Rules::default(),
            chance: &mut NeverFires,
        };
        session.priming_pass(&mut collab);
        assert_eq!(session.hyper_gamma(), 40.0);
    }

    #[test]
    fn a_priming_pass_reads_no_key_held() {
        let (catalog, mut session) = begun();
        session.hold_fire(true, true);
        session.hold_cloak(true);
        session.hold_eject(true);
        session.hold_self_destruct(true);
        session.hold_weapon_safety(true);
        session.set_land_key(true);
        session.hold_jettison(Some(true));
        session.set_dock_try(true);
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &NovaAi::default(),
            rules: Rules::default(),
            chance: &mut NeverFires,
        };
        session.priming_pass(&mut collab);
        assert_eq!(session.ship.trigger, Trigger::default());
        assert!(!session.ship.weapon_keys.fire_secondary);
        assert!(!session.ship.cloak.held);
        assert!(!session.ship.fate.eject_held);
        assert!(!session.ship.fate.self_destruct_held);
        assert!(!session.ship.weapon_keys.safety_held);
        assert!(!session.port.land_key);
        assert_eq!(session.ship.fate.jettison_held, None);
        assert!(session.port.dock_try, "a latch, not a key");
    }

    #[test]
    fn a_priming_pass_steps_every_stellar_shown_and_every_gate_once() {
        // `_HandleStellarSprites` (@0x444d2) runs after each priming pass's
        // ship loop, at the gain as it stands, gates held as they stand.
        use crate::catalog::{LandingSite, SiteAnimation, StellarId};
        use crate::clearance::{CLEARED, DockRequest};
        use crate::gate::HYPERGATE;
        use crate::gate_sprite::GateAnimation;
        use crate::stellar_frames::StellarAnimation;
        use crate::testkit::planet;
        let animated = LandingSite {
            animation: SiteAnimation {
                delay: 1,
                type_frames: 5,
                type_sheet_frames: 5,
                ..SiteAnimation::default()
            },
            ..planet(140, -100.0, 200.0)
        };
        let gate = |id| LandingSite {
            flags2: HYPERGATE,
            animation: SiteAnimation {
                delay: 1,
                type_frames: 6,
                ..SiteAnimation::default()
            },
            ..planet(id, 100.0, 200.0)
        };
        let mut catalog = crate::testkit::catalog();
        catalog.sites[0].1 = vec![animated, gate(300), gate(301)];
        let mut session = Session::start(&catalog).expect("starts");
        // A dock request at 301: `ignore_keys` lets go of the key, not it.
        session.port.dock = Some(DockRequest {
            stellar: StellarId(301),
            permission: CLEARED,
        });
        let oracle = session.clone();
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &NovaAi::default(),
            rules: Rules::default(),
            chance: &mut NeverFires,
        };
        session.priming_pass(&mut collab);
        let id = StellarId(140);
        let five = std::num::NonZeroU16::new(5).expect("non-zero");
        let mut frames = oracle.space.stellar_frames[&id].frames;
        frames.step_by(
            SpeedMult::FULL,
            StellarAnimation::of(&animated),
            false,
            five,
            five,
            &mut NeverFires,
        );
        assert_eq!(session.space.stellar_frames[&id].frames, frames);
        assert_eq!(session.stellar_frame(id), frames.shown());
        let anim = GateAnimation::of(&gate(300));
        let after = |id, held| {
            let mut sprite = oracle.space.gate_sprites[&StellarId(id)];
            sprite.step(&anim, false, held, &mut NeverFires);
            sprite
        };
        let (released, held) = (after(300, false), after(301, true));
        assert_ne!(released, held);
        assert_eq!(session.space.gate_sprites[&StellarId(300)], released);
        assert_eq!(
            session.space.gate_sprites[&StellarId(301)],
            held,
            "held by the request"
        );
    }

    #[test]
    fn a_priming_pass_counts_the_line_down_but_moves_no_quote_clock() {
        let (catalog, mut session) = begun();
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &NovaAi::default(),
            &mut NeverFires,
        );
        let clock = session.space.escorts.quote_clock.ticks;
        session.space.comm_line.show(5);
        let mut counted = session.space.comm_line;
        counted.tick();
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &NovaAi::default(),
            rules: Rules::default(),
            chance: &mut NeverFires,
        };
        session.priming_pass(&mut collab);
        assert_eq!(session.space.escorts.quote_clock.ticks, clock, "no time");
        assert_eq!(session.space.comm_line, counted, "the countdown");
    }

    #[test]
    fn a_sound_a_priming_pass_raises_is_taken_after_entry() {
        // The Red Alert's check (@0x68c79) runs in `_HandlePlayer` on the
        // frame counter as it stands: on a check frame a threat sounds it
        // in the first pass, as `PlaySound` does in the original.
        let catalog = crate::testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.space.traffic.add_npc(Npc {
            goal: Goal::Attack(ShipRef::Player),
            ..crate::traffic::fixture::npc(0, crate::stats::ShipStats::default())
        });
        session.space.frame = crate::red_alert::ALERT_CHECK_FRAMES;
        session.take_sounds();
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &crate::testkit::Passive,
            &mut NeverFires,
        );
        let alerts = session
            .take_sounds()
            .into_iter()
            .filter(|&sound| sound == crate::SimSound::RedAlert)
            .count();
        assert_eq!(alerts, 1, "the first pass's, the second on the same frame");
    }

    /// System 131, which [`escorting`]'s failure hides.
    const HIDDEN: crate::catalog::SystemId = crate::catalog::SystemId(131);

    /// A hideable session in flight with mission 140 of escort goal 1
    /// accepted, its two ships here, whose `OnFailure` sets [`HIDDEN`]'s
    /// hiding bit: the first of its ships destroyed fails it at once
    /// (see the `mission_ships` module).
    fn escorting() -> (FakePilotCatalog, Session) {
        use crate::session::mission_ships::tests::{goal_catalog, in_flight};
        let mut catalog = goal_catalog(1, 0);
        let hiding = crate::testkit::hiding_bit(HIDDEN.0).get();
        catalog.missions[0].on_failure = crate::control::Script::parse(&format!("b{hiding}"));
        crate::testkit::hideable(&mut catalog);
        let session = in_flight(&catalog);
        assert!(!session.space.traffic_due, "set up at the take-off");
        (catalog, session)
    }

    /// Whether `session`'s pilot has [`HIDDEN`]'s hiding bit set.
    fn hiding_set(session: &Session) -> bool {
        session.pilot.bits.get(crate::testkit::hiding_bit(HIDDEN.0))
    }

    #[test]
    fn entry_refreshes_the_galaxy_between_its_priming_passes() {
        // `_PlayGame`'s `_PropagateMissionBitEffects` (@0x457e9) reads the
        // bits as the first pass left them: the pass takes a mission ship
        // destroyed out, failing its mission, whose `OnFailure` sets a
        // hiding bit.
        let (catalog, mut session) = escorting();
        session.space.traffic.npcs_mut()[0].condition = crate::combat::hull::Condition::Destroyed;
        assert!(!hiding_set(&session));
        assert!(session.galaxy().visible(HIDDEN));
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &crate::testkit::Passive,
            &mut NeverFires,
        );
        assert!(hiding_set(&session), "the first pass failed it");
        assert!(!session.galaxy().visible(HIDDEN), "refreshed after it");
    }

    #[test]
    fn a_bit_the_second_priming_pass_sets_waits_for_a_later_refresh() {
        // The refresh comes before the second pass (@0x457e9-0x457f3), and
        // nothing after it refreshes the galaxy before the first frame.
        use crate::combat::hull::Condition;
        let (catalog, set_up) = escorting();
        // Mission ship 0 breaking up, `ticks_left` on its death timer,
        // entered.
        let entered = |ticks_left| {
            let mut session = set_up.clone();
            session.space.traffic.npcs_mut()[0].condition = Condition::Dying {
                ticks_left,
                starting: false,
            };
            enter(
                &mut session,
                &catalog,
                SpeedMult::FULL,
                &crate::testkit::Passive,
                &mut NeverFires,
            );
            session
        };
        // Only a pass takes a ship destroyed out, so the longest timer
        // whose breakup still fails the mission at entry is the one that
        // runs out in its last pass, the second.
        let last = (1..=60)
            .filter(|&ticks_left| hiding_set(&entered(ticks_left)))
            .max()
            .expect("a breakup that ends within entry");
        assert!(!hiding_set(&entered(last + 1)), "one that outlasts it");
        let session = entered(last);
        assert!(hiding_set(&session), "the second pass failed it");
        assert!(session.galaxy().visible(HIDDEN), "not refreshed since");
    }

    #[test]
    fn a_landed_entry_sets_up_nothing_and_runs_no_priming_pass() {
        let (catalog, mut session) = begun();
        crate::testkit::land_now(&mut session).expect("lands");
        session.space.traffic_due = true;
        let behaviour = Counting::default();
        enter(
            &mut session,
            &catalog,
            SpeedMult::FULL,
            &behaviour,
            &mut NeverFires,
        );
        assert!(session.space.traffic_due, "the take-off sets them up");
        assert_eq!(behaviour.decisions.get(), 0);
    }
}
