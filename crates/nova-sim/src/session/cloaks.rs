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
//!   not at all; short of that, every ship with a fade is drawn jittered
//!   and faint (@0x2d2c0-0x2d648), and one with none as it is.
//! - **The radar** ([`Session::on_radar`], `_DrawStatusRadar`
//!   @0x49ac6-0x49af5): a cloaked ship shows only when its own cloak shows
//!   there or the player scans for cloaked ships on the radar.
//! - **Targeting** (see [`targeting`](crate::targeting)): a cloaked ship
//!   other than the player's escort is skipped, and let go as a target,
//!   unless the player can target cloaked ships
//!   (`_ShipCanTargetCloakedShips` @0x2466; `_HandlePlayer`
//!   @0x68652-0x686f9).
//! - **Hits** ([`Space::cloak_hits`], `_DamageShip`): each strike on an
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

use super::{PlayerShip, Session, Space, Statics};
use crate::cloak::{self, Area, CloakShown, Observer, Scanner};
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};
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
    /// it waits at, which [`Session::sees`] holds unseen.
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
    /// fading (a fade above 0), every ship is drawn jittered and faint,
    /// whoever leads it and whether or not the player sees past its cloak
    /// ([`CloakShown::Fading`], @0x2d2c0-0x2d648); with no fade it is
    /// drawn as it is (@0x2d169-0x2d16c). A ship waiting at its gate is
    /// left to the view's tint, as now.
    #[must_use]
    pub fn cloak_shown(&self, npc: &Npc) -> CloakShown {
        PlayerShip::cloak_shown(&self.pilot, &self.statics, npc)
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
            self.space.play_area.offset(self.ship.player.position),
            npc.state.position,
            npc.hull.sprite_size.unwrap_or(0.0),
            npc.hull.sprite_height.unwrap_or(0.0),
        )
    }

    /// Says what the view shows of space, about the player's ship: the
    /// play area, as world units from the player (x right, y down).
    pub fn set_play_area(&mut self, area: Area) {
        self.space.set_play_area(area);
    }

    /// The play area the view shows about the player, or
    /// [`DEFAULT_PLAY_AREA`](crate::cloak::DEFAULT_PLAY_AREA) until it says.
    #[must_use]
    pub fn play_area(&self) -> Area {
        self.space.play_area()
    }

    /// Whether a watched `npc` is observed now: seen, and, when it can
    /// cloak, on screen (`_MissionObjectivesCheck` @0x9ea5c-0x9eb15).
    pub(super) fn observes(&self, npc: &Npc) -> bool {
        let reserves = self.setup.rules.source_for(RuleKey::CloakReserves);
        self.sees(npc) && (!npc.can_cloak(reserves) || self.on_screen(npc))
    }
}

impl PlayerShip {
    /// Whether the player sees `npc` (see the module docs).
    #[must_use]
    fn sees(&self, pilot: &Pilot, statics: &Statics, npc: &Npc) -> bool {
        !npc.emerging() && self.sees_past_cloak(pilot, statics, npc)
    }

    /// Whether `npc`'s cloak lets the player see it, leaving aside a gate
    /// it waits at, which [`Session::sees`] holds unseen.
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
    /// fading (a fade above 0), every ship is drawn jittered and faint,
    /// whoever leads it and whether or not the player sees past its cloak
    /// ([`CloakShown::Fading`], @0x2d2c0-0x2d648); with no fade it is
    /// drawn as it is (@0x2d169-0x2d16c). A ship waiting at its gate is
    /// left to the view's tint, as now.
    #[must_use]
    fn cloak_shown(pilot: &Pilot, statics: &Statics, npc: &Npc) -> CloakShown {
        let led = npc.escorts_player();
        let level = cloak::full_fade_translucency(led, scanner(pilot, statics).on_screen);
        cloak::shown(npc.cloak, level)
    }
}

impl Space {
    /// Says what the view shows of space, about the player's ship: the
    /// play area, as world units from the player (x right, y down).
    fn set_play_area(&mut self, area: Area) {
        self.play_area = area;
    }

    /// The play area the view shows about the player, or
    /// [`DEFAULT_PLAY_AREA`](crate::cloak::DEFAULT_PLAY_AREA) until it says.
    #[must_use]
    fn play_area(&self) -> Area {
        self.play_area
    }

    /// Each of `strikes` on an NPC, as its cloak takes it (see the module
    /// docs). A hit that meets cloak-when-hit's conditions first starts
    /// the burst weapons of the ship's class on their burst reload
    /// ([`Armament::force_burst_reload`](crate::combat::armament::Armament::force_burst_reload)),
    /// cloaked already or not, as `_DamageShip` calls
    /// `_AIForceBurstReload` (@0x3bbd5) before `_DoShipCloak` (@0x3bbdd).
    ///
    /// Whether the NPC can cloak reads `reserves`
    /// ([`RuleKey::CloakReserves`]).
    pub(super) fn cloak_hits(
        &mut self,
        statics: &Statics,
        reserves: RuleSource,
        strikes: &[Strike],
    ) {
        for strike in strikes.iter().filter(|strike| strike.direct) {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let arsenal = &statics.arsenal;
            let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
                continue;
            };
            let can_cloak = npc.can_cloak(reserves);
            // A hit by no ship never cloaks it: `_DamageShip` cloaks on a
            // hit only for a ship source (@0x3bb9f), as if it were
            // attacking; nor does an unanswered one, whose reaction block
            // is skipped.
            let holds =
                !strike.answered || strike.by.is_none_or(|by| npc.goal.attacking() == Some(by));
            // The fight has already taken the hit's damage, as `_DamageShip`
            // does (@0x3ac11-0x3acbd) before it cloaks (@0x3bbdd), so a cloak
            // that zeroes shields zeroes what the hit left.
            let shield = &mut npc.reserves.shield.now;
            if cloak::hit(&mut npc.cloak, npc.cloak_gear, can_cloak, holds, shield) {
                npc.armament.force_burst_reload(&arsenal.of_class(npc.ship));
            }
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

    /// The engine's reading of `cloak_reserves`.
    const ENGINE: crate::rulebook::RuleSource = crate::rulebook::RuleSource::Engine;

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
        session.space.traffic.npcs_mut()[0].cloak_gear = CloakGear {
            flags2: CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        session
            .space
            .cloak_hits(&session.statics, ENGINE, &[unanswered]);
        assert_eq!(session.npcs()[0].cloak, Cloak::default());
        session.space.cloak_hits(&session.statics, ENGINE, &[hit]);
        assert_eq!(session.npcs()[0].cloak.direction, 1, "answered");
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.cloak_gear = CloakGear {
            decloaks_when_hit: true,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        session
            .space
            .cloak_hits(&session.statics, ENGINE, &[unanswered]);
        assert_eq!(session.npcs()[0].cloak.direction, -1);
    }

    #[test]
    fn a_hit_that_cloaks_a_ship_with_a_zeroing_cloak_zeroes_its_shield() {
        use crate::cloak::{CLOAK_WHEN_HIT, CloakGear};
        use crate::combat::ShipRef;
        use crate::combat::fixture::strike;
        use crate::traffic::npc::NpcId;
        let catalog = armed();
        for (zeroes_shields, left) in [(true, 0.0), (false, 40.0)] {
            let mut session = facing_an_npc(&catalog, 180);
            let npc = &mut session.space.traffic.npcs_mut()[0];
            npc.cloak_gear = CloakGear {
                flags2: CLOAK_WHEN_HIT,
                zeroes_shields,
                ..CAN_CLOAK
            };
            npc.reserves.shield.now = 40.0;
            session.space.cloak_hits(
                &session.statics,
                ENGINE,
                &[strike(ShipRef::Npc(NpcId(0)), ShipRef::Player)],
            );
            let npc = &session.npcs()[0];
            assert_eq!(npc.cloak.direction, 1);
            assert_eq!(npc.reserves.shield.now, left, "zeroes {zeroes_shields}");
        }
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
        let npc = &mut session.space.traffic.npcs_mut()[0];
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
        session.space.cloak_hits(&session.statics, ENGINE, hits);
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
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.cloak_gear = crate::cloak::CloakGear {
            flags2: crate::cloak::CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        let hit: Strike = strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        session.space.cloak_hits(&session.statics, ENGINE, &[hit]);
        let npc = &session.npcs()[0];
        assert_eq!(npc.cloak, CLOAKED);
        assert_eq!(npc.armament.mounts()[0].reload, 40.0);
    }

    /// Marks `session`'s NPC 0 a mission escort that has joined the
    /// player's formation.
    fn lead(session: &mut Session) {
        session.space.traffic.npcs_mut()[0].mission = Some(MissionMark {
            slot: 0,
            behav: ShipBehav::EscortPlayer,
            formation: Some(FormationSlot::default()),
            order: None,
        });
        assert!(session.npcs()[0].escorts_player());
    }

    /// How NPC 0 is shown with its cloak at `fade`, fading `direction`.
    fn shown(session: &mut Session, fade: f32, direction: i8) -> CloakShown {
        let npc = &mut session.space.traffic.npcs_mut()[0];
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
        session.space.traffic.npcs_mut()[0].state.position = player + Vec2::new(1000.0, -1000.0);
        assert!(!session.sees_past_cloak(&session.npcs()[0]), "out of sight");
        assert_eq!(
            session.cloak_shown(&session.npcs()[0]),
            FAINT,
            "scanned, at any range"
        );
    }

    #[test]
    fn every_ship_still_fading_is_drawn_fading_led_or_not_and_no_fade_is_opaque() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        assert_eq!(shown(&mut session, 0.0, 0), CloakShown::Opaque, "uncloaked");
        assert_eq!(
            shown(&mut session, -0.5, 0),
            CloakShown::Opaque,
            "drifted under"
        );
        assert_eq!(shown(&mut session, 31.9, 1), CloakShown::Fading(31.9));
        assert_eq!(shown(&mut session, 8.0, -1), CloakShown::Fading(8.0));
        assert_eq!(shown(&mut session, 0.5, -2), CloakShown::Fading(0.5));
        lead(&mut session);
        assert_eq!(
            shown(&mut session, 31.9, 1),
            CloakShown::Fading(31.9),
            "led"
        );
        assert_eq!(shown(&mut session, 0.0, 0), CloakShown::Opaque);
    }

    #[test]
    fn an_npc_cloaking_and_uncloaking_raises_no_cloak_sound() {
        // `_DoShipCloak` and `_DoShipUncloak` play their sounds only for
        // the player (+0x86 0, @0xdec0, @0xdf9e).
        use crate::cloak::{CLOAK_WHEN_HIT, CloakGear};
        use crate::combat::ShipRef;
        use crate::combat::fixture::strike;
        use crate::traffic::npc::NpcId;
        use nova_sim_catalog::sound::SimSound;
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.space.traffic.npcs_mut()[0].cloak_gear = CloakGear {
            flags2: CLOAK_WHEN_HIT,
            decloaks_when_hit: true,
            ..CAN_CLOAK
        };
        session.take_sounds();
        session.space.cloak_hits(
            &session.statics,
            ENGINE,
            &[strike(ShipRef::Npc(NpcId(0)), ShipRef::Player)],
        );
        assert_eq!(session.npcs()[0].cloak.direction, 1, "cloaking");
        session.space.traffic.npcs_mut()[0].cloak = CLOAKED;
        session.space.cloak_hits(
            &session.statics,
            ENGINE,
            &[strike(ShipRef::Npc(NpcId(0)), ShipRef::Player)],
        );
        assert_eq!(session.npcs()[0].cloak.direction, -1, "uncloaking");
        let sounds = session.take_sounds();
        assert!(
            !sounds
                .iter()
                .any(|s| matches!(s, SimSound::Cloaked | SimSound::Uncloaked)),
            "{sounds:?}"
        );
    }

    /// Keeps each NPC's goal.
    #[derive(Debug)]
    struct Keep;

    impl crate::ai::Behaviour for Keep {
        fn decide(
            &self,
            npc: &crate::traffic::npc::Npc,
            _: &crate::ai::Surroundings,
            _: &mut dyn crate::chance::Chance,
        ) -> crate::ai::Goal {
            npc.goal
        }
    }

    /// NPC 0's cloak after `ticks` of the traffic, keeping its goal
    /// `goal`, set up cloaked with a ship type's `Flags2` `flags2`.
    fn cloak_after(goal: crate::ai::Goal, flags2: u16, ticks: u32) -> crate::cloak::Cloak {
        use nova_sim_base::fixture::Draws;
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.cloak_gear = crate::cloak::CloakGear {
            flags2,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        npc.goal = goal;
        for _ in 0..ticks {
            session.tick_traffic(&catalog, &Keep, &mut Draws::of(&[0; 64]));
        }
        session.npcs()[0].cloak
    }

    #[test]
    fn a_cloaked_npc_cloaking_as_it_travels_stays_cloaked_landing_and_a_plain_one_uncloaks() {
        use crate::ai::Goal;
        use crate::catalog::StellarId;
        use crate::cloak::CLOAK_TRAVELLING;
        let landing = Goal::Land(StellarId(128));
        assert_eq!(cloak_after(landing, CLOAK_TRAVELLING, 5), CLOAKED);
        assert_eq!(cloak_after(landing, 0, 1).direction, -1);
    }

    #[test]
    fn the_traffic_thinks_cloaks_in_a_world_holding_the_rulebooks_cloak_on_reload() {
        use crate::cloak::CLOAK_RELOADING;
        use crate::combat::armament::Armament;
        use crate::combat::weapon::WeaponSpec;
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        use nova_sim_base::fixture::Draws;
        // A blaster whose burst reload is its `Reload`: past `Reload` by
        // neither reading's timer, on its burst reload by the Bible's.
        let mut catalog = armed();
        catalog.weapons[0].burst_count = 3;
        catalog.weapons[0].burst_reload = catalog.weapons[0].reload;
        for (source, cloaks) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
            let rules = Rulebook::default().with_override(RuleKey::CloakOnReload, source);
            let mut session = facing_an_npc(&catalog, 180).with_rules(rules);
            assert_eq!(session.world().cloak_on_reload, source);
            let npc = &mut session.space.traffic.npcs_mut()[0];
            npc.cloak_gear = crate::cloak::CloakGear {
                flags2: CLOAK_RELOADING,
                ..CAN_CLOAK
            };
            npc.goal = crate::ai::Goal::Hold;
            let mut armament = Armament::new([(WeaponSpec::new(&catalog.weapons[0]), 1)]);
            let class = armament.clone();
            armament.force_burst_reload(&class);
            npc.armament = armament;
            session.tick_traffic(&catalog, &Keep, &mut Draws::of(&[0; 64]));
            assert_eq!(session.npcs()[0].cloak.direction == 1, cloaks, "{source:?}");
        }
    }

    /// A cloak whose `ModVal` asks shield (a usage of 8, as the Polaris
    /// organ 268's 0x080b), costing none in upkeep here so only
    /// `_ShipCanCloak` reads it.
    const ASKS_SHIELD: crate::cloak::CloakGear = crate::cloak::CloakGear {
        can_cloak_mod: Some(0x0800),
        ..CAN_CLOAK
    };

    /// `catalog`'s [`facing_an_npc`] under `source`'s reading of
    /// `cloak_reserves`, its NPC 0 of [`ASKS_SHIELD`] with no shield left.
    fn shieldless(
        catalog: &nova_sim_catalog::fixture::FakePilotCatalog,
        source: crate::rulebook::RuleSource,
    ) -> Session {
        use crate::rulebook::{RuleKey, Rulebook};
        let rules = Rulebook::default().with_override(RuleKey::CloakReserves, source);
        let mut session = facing_an_npc(catalog, 180).with_rules(rules);
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.cloak_gear = ASKS_SHIELD;
        npc.reserves.shield.now = 0.0;
        assert!(npc.reserves.fuel.now > 0.0, "fuel left");
        session
    }

    #[test]
    fn the_traffic_holds_npc_cloaks_in_a_world_holding_the_rulebooks_cloak_reserves() {
        use crate::ai::Goal;
        use crate::catalog::StellarId;
        use crate::cloak::CLOAK_TRAVELLING;
        use crate::rulebook::RuleSource;
        use nova_sim_base::fixture::Draws;
        let catalog = armed();
        for (source, holds) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut session = shieldless(&catalog, source);
            assert_eq!(session.world().cloak_reserves, source);
            let npc = &mut session.space.traffic.npcs_mut()[0];
            // Travelling keeps its cloak by the think.
            npc.cloak_gear.flags2 = CLOAK_TRAVELLING;
            npc.cloak = CLOAKED;
            npc.goal = Goal::Land(StellarId(128));
            session.tick_traffic(&catalog, &Keep, &mut Draws::of(&[0; 64]));
            assert_eq!(session.npcs()[0].cloak.direction >= 0, holds, "{source:?}");
        }
    }

    #[test]
    fn the_cloak_reserves_rule_reaches_an_npcs_hold_its_cloak_when_hit_and_the_observe_goal() {
        use crate::cloak::CLOAK_WHEN_HIT;
        use crate::combat::ShipRef;
        use crate::combat::fixture::strike;
        use crate::rulebook::RuleSource;
        use crate::traffic::npc::NpcId;
        let catalog = armed();
        for (source, can_cloak) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut session = shieldless(&catalog, source);
            session.space.traffic.npcs_mut()[0].cloak_gear.flags2 = CLOAK_WHEN_HIT;
            session.space.cloak_hits(
                &session.statics,
                source,
                &[strike(ShipRef::Npc(NpcId(0)), ShipRef::Player)],
            );
            assert_eq!(
                session.npcs()[0].cloak.direction == 1,
                can_cloak,
                "{source:?}: cloak-when-hit"
            );

            // Seen, uncloaked, but off screen: a ship that can cloak is
            // observed only on screen.
            let player = session.player().position;
            let npc = &mut session.space.traffic.npcs_mut()[0];
            npc.cloak = crate::cloak::Cloak::default();
            npc.state.position = player + Vec2::new(2000.0, 0.0);
            assert!(session.sees(&session.npcs()[0]));
            assert_eq!(
                session.observes(&session.npcs()[0]),
                !can_cloak,
                "{source:?}: the observe goal"
            );
        }
    }
}
