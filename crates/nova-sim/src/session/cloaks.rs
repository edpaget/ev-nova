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
use crate::cloak::{self, Area, Observer, Scanner};
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
