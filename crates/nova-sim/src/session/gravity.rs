//! Gravity in flight (see [`gravity`](crate::gravity)): `_HandleGravity`
//! (@0x39bc3-0x39ccc), which `_DoPlayGameWork` calls after the player,
//! the collision passes, the shots and every ship (@0x44509), on both
//! passes, so in double time ships are pulled twice a frame.
//!
//! **Which stellars.** Every one the player's system lists, in slot order
//! ([`Session::gravity_sources_present`]), of `Gravity` not none. The
//! original tests neither whether it is shown nor whether it is destroyed,
//! so a hidden or destroyed stellar still pulls, and still counts as a
//! gravity source.
//!
//! **Which ships.** The player, unless destroyed, and every NPC not
//! destroyed (the original's active ships), that does not resist
//! ([`resists_gravity`]): disabled, breaking-up, cloaked and jumping ships
//! and escape pods are pulled too. The player resists by its hull's
//! `Flags3` 0x0020, its inertialess flight or a `ModType` 41 outfit; an
//! NPC by its hull or its inertialess flight, never its outfits.
//!
//! **The pull** ([`pull`]) is added to the velocity and capped by nothing:
//! the player's velocity meets its box on its next move, and an NPC can be
//! carried past its top speed.
//!
//! **The afterburner.** `_gravitySourcesPresent` is what the afterburner's
//! readers test; which of them see it is
//! [`RuleKey::GravityAfterburner`]'s ([`GravityAfterburner`]; see the
//! `afterburner` module).
//!
//! **Ordering.** The engine pulls after the whole fight's tick, where the
//! original pulls before `_HandleBeams`: see the phase's follow-up for the
//! one case that shows. None of this plays a sound.

use super::Session;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::{Flyer, ShipState};
use crate::gravity::{GravityAfterburner, SPEED_MULT, pull, resists_gravity};
use crate::rulebook::RuleKey;
use crate::traffic::npc::Npc;

impl Session {
    /// Whether any gravity source is in the player's system,
    /// `_gravitySourcesPresent` (*0x3b5568) as `_HandleGravity` leaves it:
    /// any stellar the system lists, shown or not, destroyed or not, of
    /// `Gravity` not none (@0x39c20-0x39c27). None in the stock data.
    #[must_use]
    pub fn gravity_sources_present(&self) -> bool {
        self.listed.iter().any(|site| site.gravity != 0)
    }

    /// Which of the afterburner's widenings a gravity source keeps plain,
    /// by [`RuleKey::GravityAfterburner`].
    pub(super) fn gravity_afterburner(&self) -> GravityAfterburner {
        GravityAfterburner::from(self.rules.source_for(RuleKey::GravityAfterburner))
    }

    /// `_HandleGravity`'s pass (see the module docs): each listed stellar
    /// of `Gravity` not none, in slot order, pulls the player and each NPC
    /// that is not destroyed and does not resist.
    pub(super) fn pull_by_gravity(&mut self) {
        let sources: Vec<_> = self
            .listed
            .iter()
            .filter(|site| site.gravity != 0)
            .map(|site| (site.position, site.gravity))
            .collect();
        if sources.is_empty() {
            return;
        }
        let player = self.condition != Condition::Destroyed && !self.player_resists_gravity();
        let npcs: Vec<bool> = self
            .traffic
            .npcs()
            .iter()
            .map(|npc| npc.condition != Condition::Destroyed && !self.npc_resists_gravity(npc))
            .collect();
        for (stellar, gravity) in sources {
            if player {
                self.player.velocity =
                    self.player.velocity + pull(self.player.position, stellar, gravity, SPEED_MULT);
            }
            let pulled = self.traffic.npcs_mut().iter_mut().zip(&npcs);
            for (npc, _) in pulled.filter(|(_, pulled)| **pulled) {
                npc.state.velocity =
                    npc.state.velocity + pull(npc.state.position, stellar, gravity, SPEED_MULT);
            }
        }
    }

    /// Whether gravity leaves the player alone (`_ShipResistsGravity`):
    /// its hull, its inertialess flight or its `ModType` 41 outfit.
    fn player_resists_gravity(&self) -> bool {
        resists_gravity(
            self.hull.stellar_immune,
            self.player_inertialess(),
            self.stats.gravity_resistance,
            Flyer::Player,
        )
    }

    /// Whether gravity leaves `npc` alone (`_ShipResistsGravity`): its
    /// hull or its inertialess flight, never its outfits.
    fn npc_resists_gravity(&self, npc: &Npc) -> bool {
        let leader = self.leader_state(npc);
        resists_gravity(
            npc.hull.stellar_immune,
            npc.flies_inertialess(leader.as_ref(), self.flight_rules().inertial_dampener),
            npc.stats.gravity_resistance,
            Flyer::Npc,
        )
    }

    /// Where `npc`'s leader is, the player or an NPC still here, which
    /// whether it keeps close formation reads.
    pub(super) fn leader_state(&self, npc: &Npc) -> Option<ShipState> {
        match npc.leader_ref()? {
            ShipRef::Player => Some(self.player),
            ShipRef::Npc(id) => self.npc(id).map(|leader| leader.state),
        }
    }

    /// The stellar hazards of a game-work pass, after the fight's tick:
    /// gravity's pull (`_HandleGravity`).
    pub(super) fn stellar_hazards(&mut self) {
        self.pull_by_gravity();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{LandingSite, OutfitId, ShipId};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::gravity::GRAVITY_RESISTANCE;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{FakePilotCatalog, outfit, planet};
    use crate::traffic::npc::NpcId;

    /// Stellar `id` at (`x`, `y`) of `gravity`.
    fn gravity_at(id: i16, x: f32, y: f32, gravity: i16) -> LandingSite {
        LandingSite {
            gravity,
            ..planet(id, x, y)
        }
    }

    /// `catalog`'s session with the player at rest at the centre and its
    /// NPC at rest 100 above, the system listing `listed`, of which
    /// `shown` are shown.
    fn among(catalog: &FakePilotCatalog, listed: &[LandingSite], shown: usize) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.player.position = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        session.listed = listed.to_vec();
        session.sites = listed[..shown].to_vec();
        session
    }

    fn fight(session: &mut Session) {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    fn npc(session: &Session) -> &Npc {
        &session.traffic.npcs()[0]
    }

    fn npc_mut(session: &mut Session) -> &mut Npc {
        &mut session.traffic.npcs_mut()[0]
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-6
    }

    /// Stellar 140 2000 to the right of the centre, of `Gravity` 40.
    fn east() -> LandingSite {
        gravity_at(140, 2000.0, 0.0, 40)
    }

    #[test]
    fn a_listed_stellar_of_any_gravity_is_a_source_shown_or_not() {
        let catalog = armed();
        let hidden = among(
            &catalog,
            &[planet(139, 0.0, 0.0), gravity_at(140, 0.0, 0.0, -3)],
            1,
        );
        assert!(hidden.gravity_sources_present());
        let none = among(&catalog, &[planet(139, 0.0, 0.0), planet(140, 9.0, 0.0)], 2);
        assert!(!none.gravity_sources_present());
    }

    #[test]
    fn the_player_and_an_npc_are_pulled_toward_the_stellar_by_the_fight() {
        let catalog = armed();
        let mut session = among(&catalog, &[east()], 1);
        fight(&mut session);
        let stellar = Vec2::new(2000.0, 0.0);
        let player = pull(Vec2::ZERO, stellar, 40, 1.0);
        assert!(player.x > 0.09, "toward it: {player:?}");
        assert!(
            near(session.player.velocity, player),
            "{:?}",
            session.player
        );
        let at = npc(&session).state.position;
        assert!(near(
            npc(&session).state.velocity,
            pull(at, stellar, 40, 1.0)
        ));
    }

    #[test]
    fn a_hidden_or_destroyed_stellar_pulls_too_and_pulls_add() {
        let catalog = armed();
        let west = LandingSite {
            destroyed: true,
            ..gravity_at(141, -1000.0, 0.0, 30)
        };
        let mut session = among(&catalog, &[east(), west], 1);
        fight(&mut session);
        let expected = pull(Vec2::ZERO, Vec2::new(2000.0, 0.0), 40, 1.0)
            + pull(Vec2::ZERO, Vec2::new(-1000.0, 0.0), 30, 1.0);
        assert!(expected.x < 0.0, "the west wins: {expected:?}");
        assert!(
            near(session.player.velocity, expected),
            "{:?}",
            session.player
        );
    }

    #[test]
    fn a_dying_or_disabled_npc_is_pulled() {
        let catalog = armed();
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 100,
                starting: false,
            },
        ] {
            let mut session = among(&catalog, &[east()], 1);
            npc_mut(&mut session).condition = condition;
            fight(&mut session);
            assert!(npc(&session).state.velocity.x > 0.09, "{condition:?}");
        }
    }

    #[test]
    fn a_stellar_immune_or_inertialess_npc_is_not_pulled_but_its_outfit_does_not_count() {
        let catalog = armed();
        let immune = |session: &mut Session| npc_mut(session).hull.stellar_immune = true;
        let inertialess = |session: &mut Session| npc_mut(session).stats.inertialess_hull = true;
        for resist in [immune, inertialess] {
            let mut session = among(&catalog, &[east()], 1);
            resist(&mut session);
            fight(&mut session);
            assert_eq!(npc(&session).state.velocity, Vec2::ZERO);
            assert!(session.player.velocity.x > 0.09, "the player still is");
        }
        let mut outfitted = among(&catalog, &[east()], 1);
        npc_mut(&mut outfitted).stats.gravity_resistance = true;
        fight(&mut outfitted);
        assert!(npc(&outfitted).state.velocity.x > 0.09);
    }

    #[test]
    fn the_player_carrying_a_modtype_41_outfit_is_not_pulled() {
        let mut catalog = armed();
        catalog
            .outfits
            .push(outfit(197, &[(GRAVITY_RESISTANCE, 0)]));
        catalog.defaults = vec![(ShipId(128), vec![(OutfitId(197), 1)])];
        let mut session = among(&catalog, &[east()], 1);
        assert!(session.stats.gravity_resistance);
        fight(&mut session);
        assert_eq!(session.player.velocity, Vec2::ZERO);
        assert!(npc(&session).state.velocity.x > 0.09, "the NPC still is");
    }

    #[test]
    fn a_stellar_immune_or_inertialess_player_is_not_pulled() {
        let catalog = armed();
        let mut immune = among(&catalog, &[east()], 1);
        immune.hull.stellar_immune = true;
        fight(&mut immune);
        assert_eq!(immune.player.velocity, Vec2::ZERO);
        let mut inertialess = among(&catalog, &[east()], 1);
        inertialess.stats.inertialess_hull = true;
        fight(&mut inertialess);
        assert_eq!(inertialess.player.velocity, Vec2::ZERO);
    }

    #[test]
    fn a_destroyed_player_is_not_pulled() {
        let catalog = armed();
        let mut session = among(&catalog, &[east()], 1);
        session.condition = Condition::Destroyed;
        fight(&mut session);
        assert_eq!(session.player.velocity, Vec2::ZERO);
    }

    #[test]
    fn double_times_second_pass_pulls_again() {
        let catalog = armed();
        let mut session = among(&catalog, &[east()], 1);
        session.settle_box();
        fight(&mut session);
        let once = session.player.velocity;
        session.tick_again(Controls::default(), Rules::default(), &mut NeverFires);
        let again = pull(session.player.position, Vec2::new(2000.0, 0.0), 40, 1.0);
        assert!(
            near(session.player.velocity, once + again),
            "{:?}",
            session.player
        );
        assert!(session.player.velocity.x > 0.19, "{:?}", session.player);
    }

    #[test]
    fn an_npc_is_carried_past_its_top_speed() {
        let catalog = armed();
        let mut session = among(&catalog, &[gravity_at(140, 600.0, 0.0, 32000)], 1);
        let top = npc(&session).stats.handling.max_speed;
        fight(&mut session);
        let speed = npc(&session).state.velocity.length();
        assert!(speed > top + 1.0, "{speed} past {top}");
        assert_eq!(npc(&session).id, NpcId(0));
    }
}
