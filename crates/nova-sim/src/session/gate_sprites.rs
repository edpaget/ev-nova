//! The sprite of each hypergate shown: which frame it draws, as the
//! [`gate_sprite`](crate::gate_sprite) rules step it.
//!
//! `_DoPlayGameWork` calls `_HandleShip` (@0x44483), then
//! `_HandleShipDisplay` (@0x444ad), then `_HandleStellarSprites`
//! (@0x444d2), so each gate steps after the ships have moved, reading them
//! as this frame left them: at the end of [`Session::tick_traffic`], before
//! the frame is drawn. It steps whenever the ship is not docked, while a
//! jump plays too, as the original's stellars animate then.
//!
//! The gates are those of the stellars shown, kept whenever they change:
//! a gate still shown keeps its sprite, a gate newly shown starts closed,
//! and one no longer shown is let go.

use super::Session;
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::gate::HYPERGATE;
use crate::gate_sprite::{GateAnimation, held_open};

impl Session {
    /// The frame hypergate `gate` draws, or `None` for a stellar shown
    /// that is not a hypergate, or one not shown.
    #[must_use]
    pub fn gate_frame(&self, gate: StellarId) -> Option<u16> {
        let site = self.gate_site(gate)?;
        let anim = GateAnimation::of(site);
        Some(
            self.gate_sprites
                .get(&gate)
                .map_or(0, |sprite| sprite.shown(&anim, site.destroyed)),
        )
    }

    /// Whether hypergate `gate` is open: on its open frame or past it
    /// ([`GateSprite::is_open`](crate::gate_sprite::GateSprite::is_open)).
    #[must_use]
    pub fn gate_open(&self, gate: StellarId) -> bool {
        self.gate_site(gate).is_some_and(|site| {
            self.gate_sprites
                .get(&gate)
                .is_some_and(|sprite| sprite.is_open(&GateAnimation::of(site)))
        })
    }

    /// Hypergate `gate`, when it is shown.
    fn gate_site(&self, gate: StellarId) -> Option<&LandingSite> {
        self.sites
            .iter()
            .find(|site| site.id == gate && is_hypergate(site))
    }

    /// Keeps a sprite for each hypergate shown, and only for those.
    pub(super) fn keep_gate_sprites(&mut self) {
        let gates: Vec<StellarId> = self
            .sites
            .iter()
            .filter(|site| is_hypergate(site))
            .map(|site| site.id)
            .collect();
        self.gate_sprites.retain(|id, _| gates.contains(id));
        for gate in gates {
            self.gate_sprites.entry(gate).or_default();
        }
    }

    /// One tick of every hypergate shown, held open by the NPCs as they
    /// are now ([`held_open`]), its random picks drawn on `chance`.
    pub(super) fn step_gate_sprites(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.keep_gate_sprites();
        for site in self.sites.iter().filter(|site| is_hypergate(site)) {
            let held = held_open(site.id, self.traffic.npcs());
            if let Some(sprite) = self.gate_sprites.get_mut(&site.id) {
                sprite.step(&GateAnimation::of(site), site.destroyed, held, chance);
            }
        }
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
    use crate::flight::ShipState;
    use crate::gate::WORMHOLE;
    use crate::geometry::Vec2;
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
        session.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
    }

    /// Adds a ship of FAST coming out of hypergate 300, waiting `wait`
    /// ticks.
    fn emerging(session: &mut Session, wait: u32) {
        let fields = crate::stats::ShipStats::new(FAST, &[]);
        session.traffic.add_npc(Npc {
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
    fn a_gate_opens_while_a_ship_waits_and_bursts_out_of_it_then_closes() {
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

    #[test]
    fn a_gate_still_shown_keeps_its_sprite_and_one_no_longer_shown_is_let_go() {
        let catalog = gated();
        let mut session = flying(&catalog);
        emerging(&mut session, 1000);
        tick(&mut session, &catalog);
        session.keep_gate_sprites();
        assert_eq!(session.gate_frame(StellarId(300)), Some(1), "kept");
        let shown = session.sites.clone();
        session.sites.retain(|site| site.id != StellarId(300));
        session.keep_gate_sprites();
        assert!(session.gate_sprites.is_empty(), "let go");
        session.sites = shown;
        session.keep_gate_sprites();
        assert_eq!(
            session.gate_sprites.get(&StellarId(300)),
            Some(&crate::gate_sprite::GateSprite::new()),
            "shown again, closed"
        );
    }

    #[test]
    fn a_system_laid_out_keeps_a_sprite_for_its_hypergates_alone() {
        let catalog = gated();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.gate_sprites.keys().copied().collect::<Vec<_>>(),
            [StellarId(300)]
        );
    }

    #[test]
    fn a_gate_steps_after_the_ships_move_in_the_same_tick() {
        let catalog = gated();
        let mut session = flying(&catalog);
        // Its wait ends this tick: it bursts out, still holding the gate.
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
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
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
