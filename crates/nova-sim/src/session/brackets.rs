//! The session's target brackets (see [`brackets`](crate::brackets)):
//! which stellar and which ship are bracketed, how far out, and in which
//! frames.
//!
//! - **Each flight tick** ([`Session::tick`]), the jump's and a destroyed
//!   player's too, steps each fly-in while there is a target and settles
//!   it while there is none, as `_HandleNavTargetSprites` (@0x2fcaf) and
//!   `_HandleWeapTargetSprites` (@0x2ff1a) do on every pass. A landed
//!   tick doesn't, and landing clears the nav target.
//! - **A nav pick** that changes the nav target, by a stellar slot key,
//!   the nearest-stellar key or a click ([`Session::select_stellar`]), or
//!   the land key selecting one ([`Session::land`]), starts the nav
//!   fly-in.
//! - **A target pick** that changes the target
//!   ([`Session::select_target`]) starts the ship fly-in when the new
//!   target's sprite is on screen, and settles it otherwise
//!   ([`Session::retargeted`]).
//! - **The mission ship swapped in for a hailed person**
//!   ([`Session::swap_in_mission_ship`]) is targeted settled (@0x62538).
//!   A plain hail touches nothing.
//!
//! Nav Off, Hyper Select, docking and system entry change no fly-in: the
//! next tick settles it, and with no target there are no brackets.

use super::Session;
use crate::brackets::{self, NavBrackets, TargetBrackets};
use crate::traffic::npc::{Npc, NpcId};

impl Session {
    /// The brackets round the nav target, if there is one.
    ///
    /// They show while [`Session::nav_target`] is set, which is the
    /// original's nav mode 2 with a stellar (@0x2fcbf-0x2fccf):
    /// rdm:phase/hud-fidelity/phase-11-track-nav-mode must keep them keyed
    /// to mode 2. A dominated stellar takes the green frames: the same
    /// +0x46 byte `_ColorCodeStellar` reads (@0x2fd40-0x2fd76), which
    /// [`Session::stellar_iff`]'s standing models.
    #[must_use]
    pub fn nav_brackets(&self) -> Option<NavBrackets> {
        let stellar = self.nav_target?;
        let dominated = self
            .sites
            .iter()
            .find(|site| site.id == stellar)
            .is_some_and(|site| Self::iff_standing(site).dominated);
        Some(NavBrackets {
            stellar,
            expand: self.nav_fly_in.expand(),
            dominated,
        })
    }

    /// The brackets round the ship target, if there is one, in the frames
    /// of its [`brackets::standing`], led by its leader if that is still
    /// in the system.
    #[must_use]
    pub fn target_brackets(&self) -> Option<TargetBrackets> {
        let npc = self.target()?;
        let leader = npc.leader.and_then(|id| self.npc(id));
        Some(TargetBrackets {
            npc: npc.id,
            expand: self.target_fly_in.expand(),
            standing: brackets::standing(npc, leader, self.npcs()),
        })
    }

    /// Whether `npc`'s sprite is in the play area about the player
    /// ([`brackets::sprite_in_area`]); a ship with no sprite counts as a
    /// point.
    fn sprite_on_screen(&self, npc: &Npc) -> bool {
        self.npc_in_play_area(npc, brackets::sprite_in_area)
    }

    /// The brackets' part of a flight tick (see the module docs).
    pub(super) fn brackets_tick(&mut self) {
        if self.nav_target.is_some() {
            self.nav_fly_in.step();
        } else {
            self.nav_fly_in.settle();
        }
        if self.target.is_some() {
            self.target_fly_in.step();
        } else {
            self.target_fly_in.settle();
        }
    }

    /// After a pick that had `before` as the target: when the target
    /// changed to a ship, starts its fly-in if its sprite is on screen,
    /// else settles it (@0x6a5ac-0x6a5e4, @0x6aa95-0x6aab6). A pick that
    /// leaves the target as it was writes nothing.
    pub(super) fn retargeted(&mut self, before: Option<NpcId>) {
        if self.target == before {
            return;
        }
        let Some(on_screen) = self.target().map(|npc| self.sprite_on_screen(npc)) else {
            return;
        };
        if on_screen {
            self.target_fly_in.start();
        } else {
            self.target_fly_in.settle();
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::brackets::{FlyIn, Standing};
    use crate::catalog::StellarId;
    use crate::cloak::Area;
    use crate::combat::ShipRef;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::landing::LandOutcome;
    use crate::market;
    use crate::session::tests::{hub, three_stellars};
    use crate::session::{LandPress, StellarPick};
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{FakePilotCatalog, npc};

    const FLIGHT: [f32; 5] = [196.0, 136.0, 76.0, 16.0, 0.0];

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog).expect("starts")
    }

    /// The nav brackets' expand after each of `ticks` ticks.
    fn nav_over(session: &mut Session, ticks: usize) -> Vec<f32> {
        (0..ticks)
            .map(|_| {
                session.tick(Controls::default());
                session.nav_brackets().expect("bracketed").expand
            })
            .collect()
    }

    /// The ship brackets' expand after each of `ticks` ticks.
    fn target_over(session: &mut Session, ticks: usize) -> Vec<f32> {
        (0..ticks)
            .map(|_| {
                session.tick(Controls::default());
                session.target_brackets().expect("bracketed").expand
            })
            .collect()
    }

    fn nav_expand(session: &Session) -> f32 {
        session.nav_brackets().expect("bracketed").expand
    }

    fn target_expand(session: &Session) -> f32 {
        session.target_brackets().expect("bracketed").expand
    }

    #[test]
    fn selecting_a_stellar_flies_its_brackets_in_over_five_ticks() {
        let mut session = session(&three_stellars());
        assert_eq!(session.nav_brackets(), None);
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(
            session.nav_brackets(),
            Some(NavBrackets {
                stellar: StellarId(131),
                expand: 196.0,
                dominated: false,
            }),
            "a frame drawn before the next tick"
        );
        assert_eq!(nav_over(&mut session, 5), FLIGHT);
        assert_eq!(nav_over(&mut session, 2), [0.0, 0.0]);
    }

    #[test]
    fn selecting_the_current_stellar_again_does_not_restart() {
        let mut session = session(&three_stellars());
        session.select_stellar(StellarPick::Slot(0));
        nav_over(&mut session, 2);
        assert_eq!(
            session.select_stellar(StellarPick::Slot(0)),
            Some(StellarId(131))
        );
        assert_eq!(nav_expand(&session), 136.0);
        assert_eq!(nav_over(&mut session, 3), [76.0, 16.0, 0.0]);
    }

    #[test]
    fn selecting_another_stellar_restarts_the_fly_in() {
        let mut session = session(&three_stellars());
        session.select_stellar(StellarPick::Slot(0));
        nav_over(&mut session, 5);
        session.select_stellar(StellarPick::Slot(1));
        assert_eq!(nav_expand(&session), 196.0);
        assert_eq!(nav_over(&mut session, 5), FLIGHT);
        // 128, in slot 1, is the nearest.
        session.select_stellar(StellarPick::Nearest);
        assert_eq!(session.nav_target(), Some(StellarId(128)));
        assert_eq!(nav_expand(&session), 0.0, "the nearest key: the same");
        session.select_stellar(StellarPick::Slot(0));
        nav_over(&mut session, 5);
        session.select_stellar(StellarPick::Nearest);
        assert_eq!(nav_expand(&session), 196.0, "the nearest key: another");
    }

    #[test]
    fn the_land_key_selecting_a_stellar_flies_it_in() {
        let mut session = session(&three_stellars());
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected { .. }))
        ));
        assert_eq!(nav_expand(&session), 196.0);
        assert_eq!(nav_over(&mut session, 5), FLIGHT);
    }

    #[test]
    fn nav_off_and_hyper_select_hide_the_brackets_and_a_later_pick_flies_in_again() {
        let mut session = session(&hub());
        session.select_stellar(StellarPick::Slot(0));
        nav_over(&mut session, 1);
        assert!(session.clear_nav_target());
        assert_eq!(session.nav_brackets(), None);
        session.tick(Controls::default());
        assert_eq!(session.nav_fly_in, FlyIn::default(), "settled");
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(nav_over(&mut session, 5), FLIGHT);
        assert!(session.select_next_system().is_some());
        assert_eq!(session.nav_brackets(), None);
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(nav_expand(&session), 196.0);
    }

    #[test]
    fn an_always_dominated_stellar_shows_dominated() {
        let mut catalog = three_stellars();
        catalog.sites[0].1[1].flags2 = market::DOMINATED;
        let mut session = session(&catalog);
        session.select_stellar(StellarPick::Slot(1));
        assert!(session.nav_brackets().expect("bracketed").dominated);
        session.select_stellar(StellarPick::Slot(0));
        assert!(!session.nav_brackets().expect("bracketed").dominated);
    }

    /// Adds an idle NPC at `at` with a 40 x 20 sprite, giving its number.
    fn ship_at(session: &mut Session, at: Vec2) -> NpcId {
        let mut ship = npc(0, ShipStats::default());
        ship.state.position = at;
        ship.hull.sprite_size = Some(40.0);
        ship.hull.sprite_height = Some(20.0);
        session.traffic.add_npc(ship)
    }

    fn attack(session: &mut Session, id: NpcId) {
        session.npc_mut(id).expect("there").goal = Goal::Attack(ShipRef::Player);
    }

    #[test]
    fn a_new_target_on_screen_flies_in_from_256() {
        let mut session = session(&three_stellars());
        let near = ship_at(&mut session, Vec2::new(50.0, 0.0));
        let threat = ship_at(&mut session, Vec2::new(-100.0, 0.0));
        attack(&mut session, threat);
        assert_eq!(session.target_brackets(), None);
        for (pick, expected) in [
            (TargetPick::Next, near),
            (TargetPick::NearestThreat, threat),
            (TargetPick::Nearest, near),
        ] {
            assert_eq!(session.select_target(pick), Some(expected), "{pick:?}");
            assert_eq!(target_expand(&session), 196.0, "{pick:?}");
            assert_eq!(target_over(&mut session, 5), FLIGHT, "{pick:?}");
        }
    }

    #[test]
    fn a_new_target_off_screen_starts_settled() {
        let mut session = session(&three_stellars());
        session.set_play_area(Area::centred(Vec2::ZERO, 200.0, 200.0));
        let near = ship_at(&mut session, Vec2::new(50.0, 0.0));
        // Its left edge, 100 + 20, just touches the area's right edge.
        let edge = ship_at(&mut session, Vec2::new(120.0, 0.0));
        session.select_target(TargetPick::Next);
        assert_eq!(session.target, Some(near));
        session.select_target(TargetPick::Next);
        assert_eq!(session.target, Some(edge));
        assert_eq!(target_expand(&session), 0.0, "off screen");
        assert_eq!(target_over(&mut session, 1), [0.0]);
        session.npc_mut(edge).expect("there").state.position = Vec2::new(119.0, 0.0);
        session.target = Some(near);
        assert_eq!(session.select_target(TargetPick::Next), Some(edge));
        assert_eq!(target_expand(&session), 196.0, "a pixel in");
    }

    #[test]
    fn the_play_area_is_about_the_player() {
        let mut session = session(&three_stellars());
        session.set_play_area(Area::centred(Vec2::ZERO, 200.0, 200.0));
        let ship = ship_at(&mut session, Vec2::new(1000.0, 0.0));
        session.player.position = Vec2::new(950.0, 0.0);
        assert_eq!(session.select_target(TargetPick::Next), Some(ship));
        assert_eq!(target_expand(&session), 196.0);
    }

    #[test]
    fn a_pick_that_keeps_the_target_does_not_restart() {
        let mut session = session(&three_stellars());
        let threat = ship_at(&mut session, Vec2::new(50.0, 0.0));
        attack(&mut session, threat);
        session.select_target(TargetPick::NearestThreat);
        target_over(&mut session, 2);
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(threat)
        );
        assert_eq!(target_expand(&session), 136.0);
        session.select_target(TargetPick::Nearest);
        assert_eq!(target_expand(&session), 136.0, "the nearest is the same");
        assert_eq!(target_over(&mut session, 3), [76.0, 16.0, 0.0]);
    }

    #[test]
    fn a_pick_settled_off_screen_cuts_a_fly_in_short() {
        let mut session = session(&three_stellars());
        let near = ship_at(&mut session, Vec2::new(50.0, 0.0));
        let far = ship_at(&mut session, Vec2::new(5000.0, 0.0));
        session.select_target(TargetPick::Next);
        assert_eq!(session.target, Some(near));
        assert_eq!(target_expand(&session), 196.0);
        session.select_target(TargetPick::Next);
        assert_eq!(session.target, Some(far));
        assert_eq!(target_expand(&session), 0.0);
    }

    #[test]
    fn losing_the_target_settles_the_fly_in() {
        let mut session = session(&three_stellars());
        let ship = ship_at(&mut session, Vec2::new(50.0, 0.0));
        session.select_target(TargetPick::Next);
        target_over(&mut session, 1);
        session.traffic.remove(ship);
        session.target = None;
        assert_eq!(session.target_brackets(), None);
        session.tick(Controls::default());
        assert_eq!(session.target_fly_in, FlyIn::default());
    }

    #[test]
    fn a_landed_tick_moves_no_brackets() {
        let mut session = session(&three_stellars());
        session.select_stellar(StellarPick::Slot(0));
        session.landed = Some(StellarId(131));
        session.tick(Controls::default());
        assert_eq!(nav_expand(&session), 196.0);
    }

    #[test]
    fn the_target_brackets_standing_reads_the_leader() {
        let mut session = session(&three_stellars());
        let escort = ship_at(&mut session, Vec2::new(50.0, 0.0));
        let fighter = ship_at(&mut session, Vec2::new(60.0, 0.0));
        session.npc_mut(fighter).expect("there").leader = Some(escort);
        session.target = Some(fighter);
        let standing = |session: &Session| session.target_brackets().expect("bracketed").standing;
        assert_eq!(standing(&session), Standing::Neutral);
        session.npc_mut(escort).expect("there").escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        assert_eq!(standing(&session), Standing::Escort);
        assert_eq!(session.target_brackets().expect("bracketed").npc, fighter);
        session.traffic.remove(escort);
        attack(&mut session, fighter);
        assert_eq!(standing(&session), Standing::Hostile, "its leader gone");
    }
}
