//! The sprite of each hypergate shown: which frame it draws, as the
//! [`gate_sprite`](crate::gate_sprite) rules step it.
//!
//! `_DoPlayGameWork` calls `_HandleShip` (@0x44483), then
//! `_HandleShipDisplay` (@0x444ad), then `_HandleStellarSprites`
//! (@0x444d2), so each gate steps after the ships have moved, reading them
//! as this frame left them: at the end of the pass's ship loop
//! ([`Session::ship_loop`]), before the frame is drawn. It steps whenever the ship is not docked, while a
//! jump plays too, as the original's stellars animate then. Outside the
//! play frame each `_PrepareAllSprites`, and game entry, runs the same walk
//! at its own gain and holds (see the `stellar_passes` module).
//!
//! The gates are those of the stellars shown. A gate keeps its sprite
//! across systems, as the original keeps a gate's frame, next frame and
//! timer on its `spöb` record (+0x476, +0x478, +0x490), written only by
//! the sprite tick's gate branch (@0x2ea3d-0x2ec3f), the launch
//! (`_InitObjects` @0x1c8c5-0x1c8d1), a newly made sprite slot
//! (`_HandleStellarSystemVisibility` @0x331e2-0x331fb) and the take-off
//! (`_HandlePlayerDockRequest` @0x675ce-0x67646; see the `takeoff_frames`
//! module, rdm:phase/play-feel-4/phase-14-takeoff-stellar-frame-reset).
//! While not shown it stands still,
//! as the sprite tick and every pass walk only the slots shown
//! (@0x2e713-0x2e73e). It
//! starts closed only the first time a flight shows it, with next 0 once
//! a take-off or a gate's exit has reset the frames: `Space` is not
//! saved, so a load, a resumed pilot and a new pilot start every gate
//! closed (stellar-fidelity phase 12).

use super::stellar_passes::GateHolds;
use super::stellars::NO_SPRITE_SIDE;
use super::{Chart, Session, Setup, Space, Statics};
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::clearance::{CLEARED, DockRequest};
use crate::gate::HYPERGATE;
use crate::gate_sprite::{GateAnimation, approach_reach, held_open};

impl Session {
    /// The frame hypergate `gate` draws, or `None` for a stellar shown
    /// that is not a hypergate, or one not shown.
    #[must_use]
    pub(crate) fn gate_frame(&self, gate: StellarId) -> Option<u16> {
        let site = self.chart.gate_site(gate)?;
        let anim = GateAnimation::of(site);
        Some(
            self.space
                .gate_sprites
                .get(&gate)
                .map_or(0, |sprite| sprite.shown(&anim, site.destroyed)),
        )
    }

    /// Whether hypergate `gate` is open: on its open frame or past it
    /// ([`GateSprite::is_open`](crate::gate_sprite::GateSprite::is_open)).
    ///
    /// For the tests: no production code calls it.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn gate_open(&self, gate: StellarId) -> bool {
        self.chart.gate_site(gate).is_some_and(|site| {
            self.space
                .gate_sprites
                .get(&gate)
                .is_some_and(|sprite| sprite.is_open(&GateAnimation::of(site)))
        })
    }

    /// Keeps a sprite for each hypergate shown ([`Space::keep_gate_sprites`]).
    pub(super) fn keep_gate_sprites(&mut self) {
        let gates = self
            .chart
            .sites
            .iter()
            .filter(|site| is_hypergate(site))
            .map(|site| site.id);
        self.space.keep_gate_sprites(gates);
    }

    /// One tick of every hypergate shown, held open by the NPCs as they
    /// are now or by the player's dock request ([`held_open`]), read after
    /// this pass's approach ran in `player_move`, as `_HandleStellarSprites`
    /// runs after `_HandlePlayer`; its random picks drawn on `chance`. A ship
    /// heading in holds it within [`approach_reach`] of the width of the
    /// its slot ([`Space::slot_sprite_size`], [`NO_SPRITE_SIDE`] with no
    /// frame), grown by
    /// whether it was open before this step.
    pub(super) fn step_gate_sprites(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.step_gates_by(1, GateHolds::AsTheyStand, chance);
    }

    /// One pass of every hypergate shown at `gain` ticks, held as `holds`
    /// says: the walk the play frame and each stellar pass share (see
    /// [`Session::step_gate_sprites`] and the `stellar_passes` module).
    pub(super) fn step_gates_by(
        &mut self,
        gain: u8,
        holds: GateHolds,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.keep_gate_sprites();
        let (dock, npcs) = match holds {
            GateHolds::AsTheyStand => (self.port.dock, self.space.traffic.npcs()),
            GateHolds::Entered(stellar) => (
                Some(DockRequest {
                    stellar,
                    permission: CLEARED,
                }),
                self.space.traffic.npcs(),
            ),
            GateHolds::Nobody => (None, [].as_slice()),
        };
        for site in self.chart.sites.iter().filter(|site| is_hypergate(site)) {
            let width = self
                .space
                .slot_sprite_width(&self.statics, &self.setup, site);
            let anim = GateAnimation::of(site);
            if let Some(sprite) = self.space.gate_sprites.get_mut(&site.id) {
                let reach = approach_reach(width, sprite.is_open(&anim));
                let held = held_open(site, reach, dock, npcs);
                sprite.step_by(gain, &anim, site.destroyed, held, chance);
            }
        }
    }
}

impl Space {
    /// Gives each of `gates`, the hypergates shown, a closed sprite when
    /// the flight has none for it yet: as the launch leaves it, or as the
    /// take-off's reset leaves it once one has run
    /// ([`GateSprite::after_reset`](crate::gate_sprite::GateSprite::after_reset);
    /// see the `takeoff_frames` module). A gate's sprite is kept while it
    /// is not shown, as the original keeps it on the `spöb` record (see
    /// the module docs).
    pub(super) fn keep_gate_sprites(&mut self, gates: impl IntoIterator<Item = StellarId>) {
        for gate in gates {
            self.gate_sprites
                .entry(gate)
                .or_insert(self.gate_sprite_afresh);
        }
    }

    /// The width `_GetSpriteXSize` gives `site`'s sprite slot
    /// ([`Space::slot_sprite_size`]), or [`NO_SPRITE_SIDE`] when the slot
    /// shows no frame (@0x26d4-0x2701).
    fn slot_sprite_width(&self, statics: &Statics, setup: &Setup, site: &LandingSite) -> u32 {
        self.slot_sprite_size(statics, setup, site)
            .map_or(NO_SPRITE_SIDE, |(width, _)| width)
    }
}

impl Chart {
    /// Hypergate `gate`, when it is shown.
    pub(super) fn gate_site(&self, gate: StellarId) -> Option<&LandingSite> {
        self.sites
            .iter()
            .find(|site| site.id == gate && is_hypergate(site))
    }
}

/// Whether `site` is a hypergate (`Flags2` 0x1000, @0x2e832).
fn is_hypergate(site: &LandingSite) -> bool {
    site.flags2 & HYPERGATE != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::NovaAi;
    use crate::catalog::SiteAnimation;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight::Controls;
    use crate::flight::ShipState;
    use crate::gate::WORMHOLE;
    use crate::geometry::Vec2;
    use crate::rulebook::RuleSource;
    use crate::story::rebuild::Fallback;
    use crate::testkit::work_with;
    use crate::testkit::{FAST, FakePilotCatalog, begin_jump_now, catalog, fly_out, planet};
    use crate::traffic::npc::{Mode, Npc};

    /// Where hypergate 300 is in 130.
    const GATE_AT: Vec2 = Vec2::new(500.0, 0.0);

    /// [`catalog`] with system 130 holding planet 128, hypergate 300 of 6
    /// frames stepping every tick (open frame 3), and wormhole 400.
    fn gated() -> FakePilotCatalog {
        let mut catalog = catalog();
        let animation = SiteAnimation {
            delay: 1,
            type_frames: 6,
            ..SiteAnimation::default()
        };
        catalog.sites[0].1 = vec![
            planet(128, 0.0, 0.0),
            LandingSite {
                flags2: HYPERGATE,
                animation,
                ..planet(300, GATE_AT.x, GATE_AT.y)
            },
            LandingSite {
                flags2: WORMHOLE,
                animation,
                ..planet(400, -500.0, 0.0)
            },
        ];
        catalog
    }

    /// In flight in 130, populated.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        tick(&mut session, catalog);
        session
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

    /// Adds a ship of FAST coming out of hypergate 300, waiting `wait`
    /// ticks.
    fn emerging(session: &mut Session, wait: u32) {
        let fields = crate::stats::ShipStats::new(FAST, &[]);
        session.space.traffic.add_npc(Npc {
            state: ShipState {
                position: GATE_AT,
                velocity: Vec2::ZERO,
                heading: 90.0,
            },
            mode: Mode::Emerging { ticks_left: wait },
            emerges_from: Some(StellarId(300)),
            ..crate::testkit::npc(0, fields)
        });
    }

    /// Adds a ship of `fields` at (`x`, 0), at rest facing the gate,
    /// bound to enter hypergate 300.
    fn heading_in(session: &mut Session, fields: crate::handling::ShipFields, x: f32) {
        session.space.traffic.add_npc(Npc {
            state: ShipState {
                position: Vec2::new(x, 0.0),
                velocity: Vec2::ZERO,
                heading: 90.0,
            },
            goal: crate::ai::Goal::EnterGate(StellarId(300)),
            ..crate::testkit::npc(0, crate::stats::ShipStats::new(fields, &[]))
        });
    }

    #[test]
    fn a_gate_opens_while_it_is_the_players_dock_stellar_and_closes_once_the_request_goes() {
        use crate::clearance::{CLEARED, DockRequest, REQUESTED_NEAR};
        let catalog = gated();
        for permission in [REQUESTED_NEAR, CLEARED] {
            let mut session = flying(&catalog);
            session.port.dock = Some(DockRequest {
                stellar: StellarId(300),
                permission,
            });
            let mut frames = Vec::new();
            for _ in 0..10 {
                tick(&mut session, &catalog);
                frames.push(session.gate_frame(StellarId(300)).expect("a gate"));
            }
            assert_eq!(frames.first(), Some(&1), "{permission:#x}: {frames:?}");
            assert!(session.gate_open(StellarId(300)), "{permission:#x}");
            session.port.dock = None;
            for _ in 0..30 {
                tick(&mut session, &catalog);
            }
            assert_eq!(session.gate_frame(StellarId(300)), Some(0));
        }
    }

    #[test]
    fn a_gate_stays_closed_for_a_request_at_0_or_at_another_stellar() {
        use crate::clearance::{CLEARED, DockRequest};
        let catalog = gated();
        for (stellar, permission) in [(300, 0), (128, CLEARED)] {
            let mut session = flying(&catalog);
            session.port.dock = Some(DockRequest {
                stellar: StellarId(stellar),
                permission,
            });
            for _ in 0..30 {
                tick(&mut session, &catalog);
                assert_eq!(session.gate_frame(StellarId(300)), Some(0));
            }
        }
    }

    #[test]
    fn a_gate_opens_as_a_ship_heads_in_to_enter_it_and_closes_once_it_is_gone() {
        let catalog = gated();
        let mut session = flying(&catalog);
        heading_in(&mut session, FAST, -200.0);
        let mut opened_on_the_way = false;
        let mut open_while_entering = true;
        let mut left = None;
        let mut frames = Vec::new();
        for tick in 0..400 {
            self::tick(&mut session, &catalog);
            let frame = session.gate_frame(StellarId(300)).expect("a gate");
            frames.push(frame);
            match session.npcs().first() {
                Some(npc) if npc.entering_gate() => open_while_entering &= frame > 0,
                Some(npc) => {
                    let off = (npc.state.position - GATE_AT).x.abs();
                    if off >= 200.0 {
                        assert_eq!(frame, 0, "{tick}: {off} out, beyond 2w");
                    } else if frame > 0 {
                        opened_on_the_way = true;
                    }
                }
                None => {
                    left.get_or_insert(tick);
                }
            }
        }
        assert!(opened_on_the_way, "{frames:?}");
        assert!(open_while_entering, "{frames:?}");
        let left = left.expect("it left");
        assert_eq!(frames.last(), Some(&0), "closed once it is gone");
        assert!(frames[left..].contains(&0));
    }

    #[test]
    fn a_ship_heading_in_holds_an_open_gate_a_tenth_farther_out() {
        let catalog = gated();
        // Still, 210 out: beyond 2w (200), within 220.
        let still = crate::handling::ShipFields { speed: 0, ..FAST };
        let frames = |open_first: bool| {
            let mut session = flying(&catalog);
            heading_in(&mut session, still, GATE_AT.x + 210.0);
            if open_first {
                let site = *session.chart.gate_site(StellarId(300)).expect("a gate");
                let sprite = session
                    .space
                    .gate_sprites
                    .get_mut(&StellarId(300))
                    .expect("kept");
                for _ in 0..3 {
                    sprite.step(&GateAnimation::of(&site), false, true, &mut NeverFires);
                }
                assert!(session.gate_open(StellarId(300)));
            }
            (0..10)
                .map(|_| {
                    tick(&mut session, &catalog);
                    session.gate_frame(StellarId(300)).expect("a gate")
                })
                .collect::<Vec<u16>>()
        };
        assert_eq!(frames(false), [0; 10], "closed: 210 is beyond 200");
        assert!(
            frames(true).iter().all(|&frame| frame >= 3),
            "held in its loop"
        );
    }

    #[test]
    fn a_gate_slot_showing_no_frame_is_held_open_within_twice_32() {
        let catalog = gated();
        let still = crate::handling::ShipFields { speed: 0, ..FAST };
        let opens = |out: f32, empty_slot: bool| {
            let mut session = flying(&catalog);
            if empty_slot {
                // A rebuild point that could read no frame for the gate.
                let mut unreadable = session.chart.sites.clone();
                for site in &mut unreadable {
                    site.frame_size = None;
                }
                session.space.rebuilt.rebuild(
                    &unreadable,
                    |_| false,
                    Fallback::Unreadable,
                    RuleSource::Engine,
                );
            }
            heading_in(&mut session, still, GATE_AT.x + out);
            tick(&mut session, &catalog);
            session.gate_frame(StellarId(300)) > Some(0)
        };
        assert!(opens(150.0, false), "a 100-wide frame: within 200");
        assert!(!opens(150.0, true), "no frame: beyond 64");
        assert!(opens(63.0, true), "no frame: within 64");
        assert!(!opens(64.0, true));
    }

    #[test]
    fn a_gate_slot_frozen_on_a_frame_its_type_could_not_replace_is_held_open_within_twice_32() {
        // Hypergate 300's `Type` counts no frames; its `DeadType`, 100
        // wide, does.
        let mut catalog = gated();
        let gate = &mut catalog.sites[0].1[1];
        gate.type_counts_frames = false;
        gate.dead_graphic = Some(9);
        gate.dead_frame_size = Some((100, 100));
        let still = crate::handling::ShipFields { speed: 0, ..FAST };
        let opens = |out: f32| {
            let mut session = flying(&catalog);
            // `Y`, a frame, `U`, a frame: the `Type` clone keeps the
            // `DeadType` frame.
            let sites = session.chart.sites.clone();
            let rebuilt = &mut session.space.rebuilt;
            rebuilt.frame(&sites, |id| id == StellarId(300), RuleSource::Engine);
            rebuilt.frame(&sites, |_| false, RuleSource::Engine);
            assert!(session.frame_frozen(StellarId(300)));
            heading_in(&mut session, still, GATE_AT.x + out);
            tick(&mut session, &catalog);
            session.gate_frame(StellarId(300)) > Some(0)
        };
        assert!(!opens(150.0), "a 32-pixel slot, not 100: beyond 64");
        assert!(opens(63.0), "within 64");
        assert!(!opens(64.0));
    }

    #[test]
    fn only_a_hypergate_shown_has_a_gate_frame() {
        let catalog = gated();
        let session = flying(&catalog);
        assert_eq!(session.gate_frame(StellarId(300)), Some(0));
        assert_eq!(session.gate_frame(StellarId(128)), None, "a planet");
        assert_eq!(session.gate_frame(StellarId(400)), None, "a wormhole");
        assert_eq!(session.gate_frame(StellarId(500)), None, "not shown");
        assert!(!session.gate_open(StellarId(300)));
    }

    #[test]
    fn a_gate_with_no_ship_coming_out_stays_closed() {
        let catalog = gated();
        let mut session = flying(&catalog);
        for _ in 0..30 {
            tick(&mut session, &catalog);
            assert_eq!(session.gate_frame(StellarId(300)), Some(0));
        }
    }

    #[test]
    fn a_gate_opens_while_a_ship_waits_in_it_then_closes_once_it_bursts_out() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 10);
        let mut frames = Vec::new();
        let mut open_while_out = true;
        for _ in 0..60 {
            tick(&mut session, &catalog);
            let npc = &session.npcs()[0];
            let frame = session.gate_frame(StellarId(300)).expect("a gate");
            if npc.holds_gate_open(StellarId(300)) {
                open_while_out &= frame > 0;
            }
            frames.push(frame);
        }
        assert_eq!(frames[..5], [1, 2, 3, 3, 4], "opening, then its loop");
        assert!(open_while_out);
        assert!(!session.gate_open(StellarId(300)));
        assert!(
            matches!(session.npcs()[0].mode, Mode::Coasting { .. } | Mode::Flying),
            "{:?}",
            session.npcs()[0].mode
        );
        assert_eq!(frames[59], 0, "closed once it is out: {frames:?}");
        let opened = frames.iter().position(|&f| f >= 3).expect("opened");
        assert!(frames[opened..].contains(&0), "{frames:?}");
    }

    #[test]
    fn a_gate_lets_go_as_the_ship_coming_out_starts_its_burst() {
        let catalog = gated();
        let mut session = flying(&catalog);
        // 12: the release tick shows frame 2 where held it would show 3.
        emerging(&mut session, 12);
        let gate = StellarId(300);
        let site = *session.chart.gate_site(gate).expect("a gate");
        let stepped = |mut sprite: crate::gate_sprite::GateSprite, held: bool| {
            sprite.step(&GateAnimation::of(&site), false, held, &mut NeverFires);
            sprite
        };
        let mut released = false;
        for tick in 0..40 {
            let before = session.space.gate_sprites[&gate];
            self::tick(&mut session, &catalog);
            let after = session.space.gate_sprites[&gate];
            match session.npcs()[0].mode {
                // Waiting, or its wait just run out: goal still 0x15.
                Mode::Emerging { .. } | Mode::LeavingGate { tick: 0 } => {
                    assert!(!released, "{tick}");
                    assert_eq!(after, stepped(before, true), "{tick}: held");
                }
                // Its first burst tick: goal 8, +0x6c = -1 (@0x8dd8a-0x8dd93).
                Mode::LeavingGate { tick: 1 } => {
                    let held = stepped(before, true);
                    let let_go = stepped(before, false);
                    assert_eq!(after, let_go, "{tick}: released");
                    let anim = GateAnimation::of(&site);
                    assert_ne!(
                        let_go.shown(&anim, false),
                        held.shown(&anim, false),
                        "{tick}: a meaningful release"
                    );
                    assert_eq!(session.gate_frame(gate), Some(2));
                    released = true;
                }
                Mode::LeavingGate { .. } => {
                    assert!(released, "{tick}");
                    assert_eq!(after, stepped(before, false), "{tick}: released");
                }
                _ => break,
            }
        }
        assert!(released, "it burst out");
    }

    #[test]
    fn a_gate_is_open_from_its_open_frame() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 1000);
        let open: Vec<bool> = (0..4)
            .map(|_| {
                tick(&mut session, &catalog);
                session.gate_open(StellarId(300))
            })
            .collect();
        assert_eq!(open, [false, false, true, true]);
        assert!(!session.gate_open(StellarId(128)), "a planet");
    }

    /// Hypergate 300's sprite, as `session` keeps it.
    fn gate_300(session: &Session) -> Option<crate::gate_sprite::GateSprite> {
        session.space.gate_sprites.get(&StellarId(300)).copied()
    }

    #[test]
    fn a_gate_no_longer_shown_keeps_its_sprite_until_shown_again() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 1000);
        tick(&mut session, &catalog);
        session.keep_gate_sprites();
        assert_eq!(session.gate_frame(StellarId(300)), Some(1), "kept");
        let left = gate_300(&session);
        let shown = session.chart.sites.clone();
        session.chart.sites.retain(|site| site.id != StellarId(300));
        session.keep_gate_sprites();
        assert_eq!(gate_300(&session), left, "kept while not shown");
        session.chart.sites = shown;
        session.keep_gate_sprites();
        assert_eq!(gate_300(&session), left, "shown again");
        assert_eq!(session.gate_frame(StellarId(300)), Some(1), "where it was");
    }

    #[test]
    fn a_gate_not_shown_does_not_step() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 1000);
        tick(&mut session, &catalog);
        let left = gate_300(&session);
        session.chart.sites.retain(|site| site.id != StellarId(300));
        for _ in 0..5 {
            session.step_gate_sprites(&mut NeverFires);
        }
        assert_eq!(gate_300(&session), left);
    }

    #[test]
    fn a_load_starts_every_gate_closed() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 1000);
        tick(&mut session, &catalog);
        assert_eq!(session.gate_frame(StellarId(300)), Some(1));
        let pilot = session.pilot().clone();
        let closed = Some(crate::gate_sprite::GateSprite::new());
        let rules = crate::rulebook::Rulebook::default();
        let loaded = Session::load(&catalog, pilot.clone(), &rules).expect("loads");
        assert_eq!(gate_300(&loaded), closed, "loaded");
        let flown = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(gate_300(&flown), closed, "resumed");
    }

    #[test]
    fn a_system_laid_out_keeps_a_sprite_for_its_hypergates_alone() {
        let catalog = gated();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session
                .space
                .gate_sprites
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [StellarId(300)]
        );
    }

    #[test]
    fn a_gate_steps_after_the_ships_move_in_the_same_tick() {
        let catalog = gated();
        let mut session = flying(&catalog);
        // Its wait runs out this tick, its goal still 0x15: it still holds the gate.
        emerging(&mut session, 1);
        tick(&mut session, &catalog);
        assert_eq!(session.npcs()[0].mode, Mode::LeavingGate { tick: 0 });
        assert_eq!(session.gate_frame(StellarId(300)), Some(1));
    }

    #[test]
    fn a_gate_keeps_stepping_while_a_jump_plays() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 1000);
        session
            .plot_course(crate::catalog::SystemId(131))
            .expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        let before = session.gate_frame(StellarId(300));
        tick(&mut session, &catalog);
        assert!(session.jumping().is_some());
        assert_ne!(session.gate_frame(StellarId(300)), before);
    }

    #[test]
    fn a_gate_does_not_step_while_the_ship_is_docked() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 1000);
        tick(&mut session, &catalog);
        assert_eq!(session.gate_frame(StellarId(300)), Some(1));
        crate::testkit::land_now(&mut session).expect("lands");
        for _ in 0..5 {
            tick(&mut session, &catalog);
        }
        assert_eq!(session.gate_frame(StellarId(300)), Some(1));
    }
}
