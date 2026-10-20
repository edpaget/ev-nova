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
//!   uncloak it (a cloak that decloaks when hit), in that order. A
//!   blast's or a death shockwave's strike does neither, being no direct
//!   hit ([`Strike::direct`]): both sit in `_DamageShip`'s blocks that its
//!   arg +0x20 skips (@0x3b44d, @0x3bf1f). Cloak-when-hit is in the
//!   reaction block (@0x3bb95-0x3bbdd), so it needs the strike answered
//!   ([`Strike::answered`], see the `answer_flag` module) too; the decloak
//!   comes after the block, and any direct hit makes it. Cloak-when-hit's
//!   conditions also start the burst weapons of the ship's class on their
//!   burst reload (`_AIForceBurstReload` @0x84c37), cloaked already or
//!   not.

use super::{PlayerShip, Session, Statics};
use crate::cloak::{self, Area, CloakShown, Observer, Scanner};
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::pilot::Pilot;
use crate::traffic::npc::Npc;

impl Session {
    /// What the player's cloak scanners give, from the outfits it owns.
    #[must_use]
    pub fn scanner(&self) -> Scanner {
        scanner(&self.pilot, &self.statics)
    }

    /// Whether the player sees `npc` (see the module docs).
    #[must_use]
    pub fn sees(&self, npc: &Npc) -> bool {
        self.ship.sees(&self.pilot, &self.statics, npc)
    }

    /// Whether `npc`'s cloak lets the player see it, leaving aside a gate
    /// it waits at, which [`Session::sees`] holds unseen: what the view
    /// draws a ship fading in at its gate by.
    #[must_use]
    pub fn sees_past_cloak(&self, npc: &Npc) -> bool {
        self.ship.sees_past_cloak(&self.pilot, &self.statics, npc)
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
        self.ship.cloak_shown(&self.pilot, &self.statics, npc)
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
            self.play_area.offset(self.ship.player.position),
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
    /// docs). A hit that meets cloak-when-hit's conditions first starts
    /// the burst weapons of the ship's class on their burst reload
    /// ([`Armament::force_burst_reload`](crate::combat::armament::Armament::force_burst_reload)),
    /// cloaked already or not, as `_DamageShip` calls
    /// `_AIForceBurstReload` (@0x3bbd5) before `_DoShipCloak` (@0x3bbdd).
    pub(super) fn cloak_hits(&mut self, strikes: &[Strike]) {
        for strike in strikes.iter().filter(|strike| strike.direct) {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let arsenal = &self.statics.arsenal;
            let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
                continue;
            };
            let can_cloak = npc.can_cloak();
            // A hit by no ship never cloaks it: `_DamageShip` cloaks on a
            // hit only for a ship source (@0x3bb9f), as if it were
            // attacking; nor does an unanswered one, whose reaction block
            // is skipped.
            let holds =
                !strike.answered || strike.by.is_none_or(|by| npc.goal.attacking() == Some(by));
            if cloak::hit(&mut npc.cloak, npc.cloak_gear, can_cloak, holds) {
                npc.armament.force_burst_reload(&arsenal.of_class(npc.ship));
            }
        }
    }
}

impl PlayerShip {
    /// Whether the player sees `npc` (see the module docs).
    #[must_use]
    fn sees(&self, pilot: &Pilot, statics: &Statics, npc: &Npc) -> bool {
        !npc.emerging() && self.sees_past_cloak(pilot, statics, npc)
    }

    /// Whether `npc`'s cloak lets the player see it, leaving aside a gate
    /// it waits at, which [`Session::sees`] holds unseen: what the view
    /// draws a ship fading in at its gate by.
    #[must_use]
    fn sees_past_cloak(&self, pilot: &Pilot, statics: &Statics, npc: &Npc) -> bool {
        let observer = Observer {
            at: self.player.position,
            scanner: scanner(pilot, statics).on_screen,
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
    fn cloak_shown(&self, pilot: &Pilot, statics: &Statics, npc: &Npc) -> CloakShown {
        let led = npc.escorts_player();
        if npc.cloak.full() {
            let level = cloak::full_fade_translucency(led, scanner(pilot, statics).on_screen);
            if level < cloak::TRANSLUCENCY_STEPS {
                CloakShown::Translucent(level)
            } else {
                CloakShown::Hidden
            }
        } else if led || self.sees_past_cloak(pilot, statics, npc) {
            CloakShown::Opaque
        } else {
            CloakShown::Hidden
        }
    }
}

/// What the player's cloak scanners give, from the outfits it owns.
#[must_use]
fn scanner(pilot: &Pilot, statics: &Statics) -> Scanner {
    Scanner::of(&pilot.outfits, &statics.outfits)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::cloak::{CloakShown, FADE_FULL, SCAN_SCREEN, SEEN_AT_FULL_FADE};
    use crate::escort::FormationSlot;
    use crate::geometry::Vec2;
    use crate::mission::ships::ShipBehav;
    use crate::session::Session;
    use crate::session::tests::{CAN_CLOAK, CLOAKED, armed, facing_an_npc, holding_260, scanning};
    use crate::traffic::npc::MissionMark;

    #[test]
    fn an_unanswered_hit_never_cloaks_its_ship_but_still_uncloaks_it() {
        // A clear answer flag skips the reaction block, cloak-when-hit
        // with it (@0x3bb95-0x3bbdd), but not the decloak (@0x3bf1f).
        use crate::cloak::{CLOAK_WHEN_HIT, Cloak, CloakGear};
        use crate::combat::fixture::strike;
        use crate::combat::{ShipRef, Strike};
        use crate::traffic::npc::NpcId;
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let hit = strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        let unanswered = Strike {
            answered: false,
            ..hit
        };
        session.traffic.npcs_mut()[0].cloak_gear = CloakGear {
            flags2: CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        session.cloak_hits(&[unanswered]);
        assert_eq!(session.npcs()[0].cloak, Cloak::default());
        session.cloak_hits(&[hit]);
        assert_eq!(session.npcs()[0].cloak.direction, 1, "answered");
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear = CloakGear {
            decloaks_when_hit: true,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        session.cloak_hits(&[unanswered]);
        assert_eq!(session.npcs()[0].cloak.direction, -1);
    }

    /// [`armed`] whose blaster (128, its class's) bursts 15 shots and
    /// reloads 40 after, with a burst weapon 129 no class carries.
    fn bursting() -> nova_sim_catalog::fixture::FakePilotCatalog {
        let mut catalog = armed();
        catalog.weapons[0].burst_count = 15;
        catalog.weapons[0].burst_reload = 40;
        catalog.weapons.push(crate::catalog::WeaponRecord {
            burst_count: 4,
            burst_reload: 20,
            ..catalog.weapons[0]
        });
        catalog.weapons[1].id = crate::catalog::WeaponId(129);
        catalog
    }

    /// Each of NPC 0's mounts' weapon and reload once `hits` land on it,
    /// a cloak-when-hit ship that `can_cloak`, carrying its class's
    /// blaster and weapon 129 too, attacking NPC 9.
    fn reloads_after(
        hits: &[crate::combat::Strike],
        can_cloak: bool,
    ) -> Vec<(crate::catalog::WeaponId, f32)> {
        use crate::cloak::{CLOAK_WHEN_HIT, CloakGear};
        use crate::combat::weapon::WeaponSpec;
        let catalog = bursting();
        let mut session = facing_an_npc(&catalog, 180);
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear = CloakGear {
            flags2: CLOAK_WHEN_HIT,
            device: can_cloak,
            ..CAN_CLOAK
        };
        npc.goal =
            crate::ai::Goal::Attack(crate::combat::ShipRef::Npc(crate::traffic::npc::NpcId(9)));
        npc.armament = crate::combat::armament::Armament::new(
            catalog
                .weapons
                .iter()
                .map(|record| (WeaponSpec::new(record), 1)),
        );
        assert!(
            npc.armament
                .mounts()
                .iter()
                .all(|mount| mount.reload == 0.0),
            "ready"
        );
        session.cloak_hits(hits);
        session.npcs()[0]
            .armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id, mount.reload))
            .collect()
    }

    #[test]
    fn cloaking_when_hit_starts_its_classs_burst_weapons_on_their_burst_reload() {
        // `_AIForceBurstReload` (@0x3bbd5) on the cloak-when-hit
        // conditions, before `_DoShipCloak` (@0x3bbdd); the class's
        // weapons alone (`shïp` +0x5e8, @0x84c5f).
        use crate::catalog::WeaponId;
        use crate::combat::fixture::strike;
        use crate::combat::{ShipRef, Strike};
        use crate::traffic::npc::NpcId;
        let hit = strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        let reset = vec![(WeaponId(128), 40.0), (WeaponId(129), 0.0)];
        let alone = vec![(WeaponId(128), 0.0), (WeaponId(129), 0.0)];
        assert_eq!(reloads_after(&[hit], true), reset);
        let unanswered = Strike {
            answered: false,
            ..hit
        };
        assert_eq!(reloads_after(&[unanswered], true), alone, "unanswered");
        let by_its_target = strike(ShipRef::Npc(NpcId(0)), ShipRef::Npc(NpcId(9)));
        assert_eq!(reloads_after(&[by_its_target], true), alone, "its target");
        assert_eq!(reloads_after(&[hit], false), alone, "it can't cloak");
        let blast = Strike {
            direct: false,
            ..hit
        };
        assert_eq!(reloads_after(&[blast], true), alone, "no direct hit");
    }

    #[test]
    fn a_ship_already_cloaked_starts_its_burst_reload_too() {
        use crate::combat::fixture::strike;
        use crate::combat::{ShipRef, Strike};
        use crate::traffic::npc::NpcId;
        let catalog = bursting();
        let mut session = facing_an_npc(&catalog, 180);
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear = crate::cloak::CloakGear {
            flags2: crate::cloak::CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        let hit: Strike = strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        session.cloak_hits(&[hit]);
        let npc = &session.npcs()[0];
        assert_eq!(npc.cloak, CLOAKED);
        assert_eq!(npc.armament.mounts()[0].reload, 40.0);
    }

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
