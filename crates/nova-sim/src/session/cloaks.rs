//! The player and cloaked ships (see [`cloak`](crate::cloak)): whether
//! the player sees an NPC, has it on the radar or on screen, and a hit's
//! effect on an NPC's cloak.
//!
//! - **Seeing** ([`Session::sees`], `_ShipVisibleToShip` with the player
//!   observing): an NPC emerging from a gate is unseen, an uncloaked one
//!   seen, and a cloaked one only with a cloak scanner on screen within
//!   200 on both axes. A mission's watched ship is observed only once it
//!   is seen ([`Session::watch_ship`]).
//! - **On screen** ([`Session::on_screen`]): the play area the view shows
//!   ([`Session::set_play_area`], placed on the player), or by default
//!   the original's smallest, [`DEFAULT_PLAY_AREA`](crate::cloak::DEFAULT_PLAY_AREA), inset by three
//!   quarters of the ship's sprite (`_MissionObjectivesCheck`
//!   @0x9ea81-0x9eb09). A watched ship that can cloak is observed only
//!   while it is also on screen.
//! - **Drawn** ([`Session::cloak_shown`], `_HandleShipDisplay`
//!   @0x2d187-0x2d1cd): fully cloaked, a ship the player leads, or any
//!   ship while the player scans on screen, is drawn faint and any other
//!   not at all; short of that, a led ship is drawn, and any other while
//!   the player sees past its cloak.
//! - **The radar** ([`Session::on_radar`], `_DrawStatusRadar`
//!   @0x49ac6-0x49af5): a cloaked ship shows only when its own cloak shows
//!   there or the player scans for cloaked ships on the radar.
//! - **Targeting** (see [`targeting`](crate::targeting)): a cloaked ship
//!   other than the player's escort is skipped, and let go as a target,
//!   unless the player can target cloaked ships
//!   (`_ShipCanTargetCloakedShips` @0x2466; `_HandlePlayer`
//!   @0x68652-0x686f9).
//! - **Hits** ([`Session::cloak_hits`], `_DamageShip`): each strike on an
//!   NPC may cloak it (`Flags2` 0x4000, by a ship it is not attacking) and
//!   uncloak it (a cloak that decloaks when hit), in that order.

use super::Session;
use crate::cloak::{self, Area, CloakShown, Observer, Scanner};
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::traffic::npc::Npc;

impl Session {
    /// What the player's cloak scanners give, from the outfits it owns.
    #[must_use]
    pub fn scanner(&self) -> Scanner {
        Scanner::of(&self.pilot.outfits, &self.outfits)
    }

    /// Whether the player sees `npc` (see the module docs).
    #[must_use]
    pub fn sees(&self, npc: &Npc) -> bool {
        !npc.emerging() && self.sees_past_cloak(npc)
    }

    /// Whether `npc`'s cloak lets the player see it, leaving aside a gate
    /// it waits at, which [`Session::sees`] holds unseen: what the view
    /// draws a ship fading in at its gate by.
    #[must_use]
    pub fn sees_past_cloak(&self, npc: &Npc) -> bool {
        let observer = Observer {
            at: self.player.position,
            scanner: self.scanner().on_screen,
            cloaked_escort: false,
        };
        cloak::visible(npc.cloak, npc.state.position, false, observer)
    }

    /// How `npc`'s cloak lets the player's screen show it, as
    /// `_HandleShipDisplay` decides (@0x2d187-0x2d1cd).
    ///
    /// At a full cloak ([`FADE_FULL`](cloak::FADE_FULL)) a ship the player
    /// leads is drawn faint, and so is any ship while the player scans for
    /// cloaked ships on screen, at any range, as @0x2d1aa has no range test
    /// ([`cloak::full_fade_translucency`]); any other is not drawn. Still
    /// fading, or uncloaked, a ship the player leads is drawn, and any
    /// other while the player sees past its cloak
    /// ([`Session::sees_past_cloak`]). The original draws every ship still
    /// fading jittered and translucent (@0x2d2c0-0x2d648), which
    /// rdm:phase/cloaking-fidelity/phase-1-cloak-fade-and-sounds owns;
    /// until then an opaque led ship is the nearest the engine draws. A
    /// ship waiting at its gate is left to the view's tint, as now.
    #[must_use]
    pub fn cloak_shown(&self, npc: &Npc) -> CloakShown {
        let led = npc.escorts_player();
        if npc.cloak.full() {
            let level = cloak::full_fade_translucency(led, self.scanner().on_screen);
            if level < cloak::TRANSLUCENCY_STEPS {
                CloakShown::Translucent(level)
            } else {
                CloakShown::Hidden
            }
        } else if led || self.sees_past_cloak(npc) {
            CloakShown::Opaque
        } else {
            CloakShown::Hidden
        }
    }

    /// Whether `npc` shows on the player's radar: uncloaked, its cloak
    /// showing there, or the player scanning for cloaked ships there.
    #[must_use]
    pub fn on_radar(&self, npc: &Npc) -> bool {
        !npc.cloak.is_cloaked() || npc.cloak_gear.on_radar || self.scanner().on_radar
    }

    /// Whether `npc` is on the player's screen (see the module docs). A
    /// ship with no sprite counts as a point.
    #[must_use]
    pub fn on_screen(&self, npc: &Npc) -> bool {
        self.npc_in_play_area(npc, cloak::on_screen)
    }

    /// `test` applied to the play area about the player and `npc`'s
    /// sprite: where it is and its width and height, a ship with no
    /// sprite a point. The tests take the size rather than the sprite's
    /// rect, as [`cloak::on_screen`] insets the play area by it.
    pub(super) fn npc_in_play_area(
        &self,
        npc: &Npc,
        test: fn(Area, Vec2, f32, f32) -> bool,
    ) -> bool {
        test(
            self.play_area.offset(self.player.position),
            npc.state.position,
            npc.hull.sprite_size.unwrap_or(0.0),
            npc.hull.sprite_height.unwrap_or(0.0),
        )
    }

    /// Says what the view shows of space, about the player's ship: the
    /// play area, as world units from the player (x right, y down).
    pub fn set_play_area(&mut self, area: Area) {
        self.play_area = area;
    }

    /// The play area the view shows about the player, or
    /// [`DEFAULT_PLAY_AREA`](crate::cloak::DEFAULT_PLAY_AREA) until it says.
    #[must_use]
    pub fn play_area(&self) -> Area {
        self.play_area
    }

    /// Whether a watched `npc` is observed now: seen, and, when it can
    /// cloak, on screen (`_MissionObjectivesCheck` @0x9ea5c-0x9eb15).
    pub(super) fn observes(&self, npc: &Npc) -> bool {
        self.sees(npc) && (!npc.can_cloak() || self.on_screen(npc))
    }

    /// Each of `strikes` on an NPC, as its cloak takes it (see the module
    /// docs).
    pub(super) fn cloak_hits(&mut self, strikes: &[Strike]) {
        for strike in strikes {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
                continue;
            };
            let can_cloak = npc.can_cloak();
            let attacking = npc.goal.attacking() == Some(strike.by);
            cloak::hit(&mut npc.cloak, npc.cloak_gear, can_cloak, attacking);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cloak::{CloakShown, FADE_FULL, SCAN_SCREEN, SEEN_AT_FULL_FADE};
    use crate::escort::FormationSlot;
    use crate::geometry::Vec2;
    use crate::mission::ships::ShipBehav;
    use crate::session::Session;
    use crate::session::tests::{CLOAKED, armed, facing_an_npc, holding_260, scanning};
    use crate::traffic::npc::MissionMark;

    /// Marks `session`'s NPC 0 a mission escort that has joined the
    /// player's formation.
    fn lead(session: &mut Session) {
        session.traffic.npcs_mut()[0].mission = Some(MissionMark {
            slot: 0,
            behav: ShipBehav::EscortPlayer,
            formation: Some(FormationSlot::default()),
            order: None,
        });
        assert!(session.npcs()[0].escorts_player());
    }

    /// How NPC 0 is shown with its cloak at `fade`, fading `direction`.
    fn shown(session: &mut Session, fade: f32, direction: i8) -> CloakShown {
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak.fade = fade;
        npc.cloak.direction = direction;
        session.cloak_shown(&session.npcs()[0])
    }

    const FAINT: CloakShown = CloakShown::Translucent(SEEN_AT_FULL_FADE);

    #[test]
    fn a_fully_cloaked_ship_is_faint_when_led_or_scanned_at_any_range_and_hidden_otherwise() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        assert_eq!(shown(&mut session, FADE_FULL, 0), CloakShown::Hidden);
        lead(&mut session);
        assert_eq!(shown(&mut session, FADE_FULL, 0), FAINT, "led");

        let scans = scanning(SCAN_SCREEN);
        let mut session = holding_260(&scans);
        let player = session.player().position;
        let id = session.npcs()[0].id;
        crate::fixture::cloak(&mut session, id, CLOAKED);
        session.traffic.npcs_mut()[0].state.position = player + Vec2::new(1000.0, -1000.0);
        assert!(!session.sees_past_cloak(&session.npcs()[0]), "out of sight");
        assert_eq!(
            session.cloak_shown(&session.npcs()[0]),
            FAINT,
            "scanned, at any range"
        );
    }

    #[test]
    fn a_led_ship_still_fading_is_drawn_and_a_stranger_only_while_seen() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        assert_eq!(shown(&mut session, 0.0, 0), CloakShown::Opaque, "uncloaked");
        assert_eq!(shown(&mut session, 31.9, 1), CloakShown::Hidden);
        assert_eq!(shown(&mut session, 8.0, -1), CloakShown::Opaque, "seen");
        lead(&mut session);
        assert_eq!(shown(&mut session, 31.9, 1), CloakShown::Opaque, "led");
        assert_eq!(shown(&mut session, 0.0, 0), CloakShown::Opaque);
    }
}
