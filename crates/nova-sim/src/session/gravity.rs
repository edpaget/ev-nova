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
//! **Ordering.** The fight's tick pulls between its shots and its beams
//! ([`stellar_hazards`](crate::combat::stellar_hazards)), as the original
//! pulls before `_HandleBeams` (@0x4453d), so an advance-pass beam's
//! knockback is clamped by `_SpeedLimit` on the velocity the pull left.
//! The pull is a shove the session sets after the tick, with the hits'.
//! None of this plays a sound.

use super::{Chart, PlayerShip, Session, Setup};
use crate::combat::ShipRef;
use crate::combat::stellar_hazards::{GravitySource, HazardResistance, StellarHazards};
use crate::deadly_stellar::resists_deadly;
use crate::flight::{Flyer, ShipState};
#[cfg(doc)]
use crate::gravity::pull;
use crate::gravity::{GravityAfterburner, resists_gravity};
use crate::rulebook::RuleKey;
use crate::traffic::npc::Npc;

impl Session {
    /// Whether any gravity source is in the player's system,
    /// `_gravitySourcesPresent` (*0x3b5568) as `_HandleGravity` leaves it:
    /// any stellar the system lists, shown or not, destroyed or not, of
    /// `Gravity` not none (@0x39c20-0x39c27). None in the stock data.
    #[must_use]
    pub fn gravity_sources_present(&self) -> bool {
        self.chart.gravity_sources_present()
    }

    /// Whether gravity leaves `npc` alone (`_ShipResistsGravity`): its
    /// hull or its inertialess flight, never its outfits.
    fn npc_resists_gravity(&self, npc: &Npc) -> bool {
        let leader = self.leader_state(npc);
        resists_gravity(
            npc.hull.stellar_immune,
            npc.flies_inertialess(leader.as_ref(), self.setup.flight_rules().inertial_dampener),
            npc.stats.gravity_resistance,
            Flyer::Npc,
        )
    }

    /// Where `npc`'s leader is, the player or an NPC still here, which
    /// whether it keeps close formation reads.
    pub(super) fn leader_state(&self, npc: &Npc) -> Option<ShipState> {
        match npc.leader_ref()? {
            ShipRef::Player => Some(self.ship.player),
            ShipRef::Npc(id) => self.space.npc(id).map(|leader| leader.state),
        }
    }

    /// The stellar hazards of a game-work pass, for the fight's tick to
    /// apply between its shots and its beams (see
    /// [`stellar_hazards`](crate::combat::stellar_hazards)): gravity's
    /// sources (`_HandleGravity`), then the deadly stellars' discs
    /// (`_HandleDeadlyStellars`; see the `deadly_stellars` module), as
    /// `_DoPlayGameWork` calls them (@0x44509, @0x4450e).
    pub(super) fn stellar_hazards(&self) -> StellarHazards {
        StellarHazards {
            gravity: self.chart.gravity_sources(),
            deadly: self.deadly_discs(),
        }
    }

    /// Which stellar hazards `npc` resists, its outfits never counting.
    pub(super) fn npc_hazard_resistance(&self, npc: &Npc) -> HazardResistance {
        HazardResistance {
            gravity: self.npc_resists_gravity(npc),
            deadly: resists_deadly(
                npc.hull.stellar_immune,
                npc.stats.deadly_resistance,
                Flyer::Npc,
            ),
        }
    }
}

impl Setup {
    /// Which of the afterburner's widenings a gravity source keeps plain,
    /// by [`RuleKey::GravityAfterburner`].
    pub(super) fn gravity_afterburner(&self) -> GravityAfterburner {
        GravityAfterburner::from(self.rules.source_for(RuleKey::GravityAfterburner))
    }
}

impl PlayerShip {
    /// Whether gravity leaves the player alone (`_ShipResistsGravity`):
    /// its hull, its inertialess flight or its `ModType` 41 outfit.
    fn player_resists_gravity(&self, setup: &Setup) -> bool {
        resists_gravity(
            self.fit.hull.stellar_immune,
            self.player_inertialess(setup),
            self.fit.stats.gravity_resistance,
            Flyer::Player,
        )
    }

    /// Which stellar hazards the player resists: gravity by
    /// `_ShipResistsGravity`, a deadly stellar by
    /// `_ShipResistsDeadlyStellars` ([`resists_deadly`]).
    pub(super) fn player_hazard_resistance(&self, setup: &Setup) -> HazardResistance {
        HazardResistance {
            gravity: self.player_resists_gravity(setup),
            deadly: resists_deadly(
                self.fit.hull.stellar_immune,
                self.fit.stats.deadly_resistance,
                Flyer::Player,
            ),
        }
    }
}

impl Chart {
    /// Whether any gravity source is in the player's system,
    /// `_gravitySourcesPresent` (*0x3b5568) as `_HandleGravity` leaves it:
    /// any stellar the system lists, shown or not, destroyed or not, of
    /// `Gravity` not none (@0x39c20-0x39c27). None in the stock data.
    #[must_use]
    fn gravity_sources_present(&self) -> bool {
        self.listed.iter().any(|site| site.gravity != 0)
    }

    /// `_HandleGravity`'s sources (see the module docs): each listed
    /// stellar of `Gravity` not none, in slot order.
    fn gravity_sources(&self) -> Vec<GravitySource> {
        self.listed
            .iter()
            .filter(|site| site.gravity != 0)
            .map(|site| GravitySource {
                at: site.position,
                gravity: site.gravity,
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{LandingSite, OutfitId, ShipId};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::hull::Condition;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::gravity::{GRAVITY_RESISTANCE, pull};
    use crate::rulebook::{RuleSource, Rulebook};
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
    /// `shown` are shown, under `rules`.
    fn among(
        rules: Rulebook,
        catalog: &FakePilotCatalog,
        listed: &[LandingSite],
        shown: usize,
    ) -> Session {
        let mut session = facing_an_npc(catalog, 180).with_rules(rules);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.chart.listed = listed.to_vec();
        session.chart.sites = listed[..shown].to_vec();
        session
    }

    /// Each reading of [`RuleKey::DeadlyStellarKill`] and
    /// [`RuleKey::GravityAfterburner`], the engine's first.
    fn readings() -> Vec<Rulebook> {
        let mut readings = Vec::new();
        for kill in [RuleSource::Engine, RuleSource::Bible] {
            for afterburner in [RuleSource::Engine, RuleSource::Bible] {
                readings.push(
                    Rulebook::default()
                        .with_override(RuleKey::DeadlyStellarKill, kill)
                        .with_override(RuleKey::GravityAfterburner, afterburner),
                );
            }
        }
        readings
    }

    fn fight(session: &mut Session) {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    fn npc(session: &Session) -> &Npc {
        &session.space.traffic.npcs()[0]
    }

    fn npc_mut(session: &mut Session) -> &mut Npc {
        &mut session.space.traffic.npcs_mut()[0]
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
        for rules in readings() {
            let catalog = armed();
            let hidden = among(
                rules,
                &catalog,
                &[planet(139, 0.0, 0.0), gravity_at(140, 0.0, 0.0, -3)],
                1,
            );
            assert!(hidden.gravity_sources_present());
            let none = among(
                rules,
                &catalog,
                &[planet(139, 0.0, 0.0), planet(140, 9.0, 0.0)],
                2,
            );
            assert!(!none.gravity_sources_present());
        }
    }

    #[test]
    fn the_player_and_an_npc_are_pulled_toward_the_stellar_by_the_fight() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(rules, &catalog, &[east()], 1);
            fight(&mut session);
            let stellar = Vec2::new(2000.0, 0.0);
            let player = pull(Vec2::ZERO, stellar, 40, 1.0);
            assert!(player.x > 0.09, "toward it: {player:?}");
            assert!(
                near(session.ship.player.velocity, player),
                "{:?}",
                session.ship.player
            );
            let at = npc(&session).state.position;
            assert!(near(
                npc(&session).state.velocity,
                pull(at, stellar, 40, 1.0)
            ));
        }
    }

    #[test]
    fn a_hidden_or_destroyed_stellar_pulls_too_and_pulls_add() {
        for rules in readings() {
            let catalog = armed();
            let west = LandingSite {
                destroyed: true,
                ..gravity_at(141, -1000.0, 0.0, 30)
            };
            let mut session = among(rules, &catalog, &[east(), west], 1);
            fight(&mut session);
            let expected = pull(Vec2::ZERO, Vec2::new(2000.0, 0.0), 40, 1.0)
                + pull(Vec2::ZERO, Vec2::new(-1000.0, 0.0), 30, 1.0);
            assert!(expected.x < 0.0, "the west wins: {expected:?}");
            assert!(
                near(session.ship.player.velocity, expected),
                "{:?}",
                session.ship.player
            );
        }
    }

    #[test]
    fn a_dying_or_disabled_npc_is_pulled() {
        for rules in readings() {
            let catalog = armed();
            for condition in [
                Condition::Disabled,
                Condition::Dying {
                    ticks_left: 100,
                    starting: false,
                },
            ] {
                let mut session = among(rules, &catalog, &[east()], 1);
                npc_mut(&mut session).condition = condition;
                fight(&mut session);
                assert!(npc(&session).state.velocity.x > 0.09, "{condition:?}");
            }
        }
    }

    #[test]
    fn a_stellar_immune_or_inertialess_npc_is_not_pulled_but_its_outfit_does_not_count() {
        for rules in readings() {
            let catalog = armed();
            let immune = |session: &mut Session| npc_mut(session).hull.stellar_immune = true;
            let inertialess =
                |session: &mut Session| npc_mut(session).stats.inertialess_hull = true;
            for resist in [immune, inertialess] {
                let mut session = among(rules, &catalog, &[east()], 1);
                resist(&mut session);
                fight(&mut session);
                assert_eq!(npc(&session).state.velocity, Vec2::ZERO);
                assert!(session.ship.player.velocity.x > 0.09, "the player still is");
            }
            let mut outfitted = among(rules, &catalog, &[east()], 1);
            npc_mut(&mut outfitted).stats.gravity_resistance = true;
            fight(&mut outfitted);
            assert!(npc(&outfitted).state.velocity.x > 0.09);
        }
    }

    #[test]
    fn the_player_carrying_a_modtype_41_outfit_is_not_pulled() {
        for rules in readings() {
            let mut catalog = armed();
            catalog
                .outfits
                .push(outfit(197, &[(GRAVITY_RESISTANCE, 0)]));
            catalog.defaults = vec![(ShipId(128), vec![(OutfitId(197), 1)])];
            let mut session = among(rules, &catalog, &[east()], 1);
            assert!(session.ship.fit.stats.gravity_resistance);
            fight(&mut session);
            assert_eq!(session.ship.player.velocity, Vec2::ZERO);
            assert!(npc(&session).state.velocity.x > 0.09, "the NPC still is");
        }
    }

    #[test]
    fn an_inertialess_escort_in_close_formation_with_the_player_is_pulled() {
        for rules in readings() {
            use crate::ai::Goal;
            use crate::escort::{EscortDuty, FormationSlot};
            let catalog = armed();
            let mut session = among(rules, &catalog, &[east()], 1);
            // Away from the centre, so a leader at the centre would not do.
            session.ship.player.position = Vec2::new(1500.0, 1500.0);
            let formation = FormationSlot {
                slot: 0,
                ships: 1,
                spacing: 30.0,
            };
            let slot = formation.position(&session.ship.player);
            let escort = npc_mut(&mut session);
            escort.escort = Some(EscortDuty {
                formation,
                order: None,
            });
            escort.goal = Goal::Formation { guard: None };
            escort.stats.inertialess_hull = true;
            escort.state.position = slot;
            fight(&mut session);
            let pulled = pull(slot, Vec2::new(2000.0, 0.0), 40, 1.0);
            assert!(
                near(npc(&session).state.velocity, pulled),
                "not inertialess there"
            );
        }
    }

    #[test]
    fn a_stellar_immune_or_inertialess_player_is_not_pulled() {
        for rules in readings() {
            let catalog = armed();
            let mut immune = among(rules, &catalog, &[east()], 1);
            immune.ship.fit.hull.stellar_immune = true;
            fight(&mut immune);
            assert_eq!(immune.ship.player.velocity, Vec2::ZERO);
            let mut inertialess = among(rules, &catalog, &[east()], 1);
            inertialess.ship.fit.stats.inertialess_hull = true;
            fight(&mut inertialess);
            assert_eq!(inertialess.ship.player.velocity, Vec2::ZERO);
        }
    }

    #[test]
    fn a_destroyed_player_is_not_pulled() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(rules, &catalog, &[east()], 1);
            session.ship.condition = Condition::Destroyed;
            fight(&mut session);
            assert_eq!(session.ship.player.velocity, Vec2::ZERO);
        }
    }

    #[test]
    fn double_times_second_pass_pulls_again() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(rules, &catalog, &[east()], 1);
            session.ship.settle_box(&session.pilot, &session.setup);
            fight(&mut session);
            let once = session.ship.player.velocity;
            session.tick_again(
                Controls::default(),
                &crate::ai::NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
            let again = pull(
                session.ship.player.position,
                Vec2::new(2000.0, 0.0),
                40,
                1.0,
            );
            assert!(
                near(session.ship.player.velocity, once + again),
                "{:?}",
                session.ship.player
            );
            assert!(
                session.ship.player.velocity.x > 0.19,
                "{:?}",
                session.ship.player
            );
        }
    }

    #[test]
    fn an_npc_is_carried_past_its_top_speed() {
        for rules in readings() {
            let catalog = armed();
            let mut session = among(rules, &catalog, &[gravity_at(140, 600.0, 0.0, 32000)], 1);
            let top = npc(&session).stats.handling.max_speed;
            fight(&mut session);
            let speed = npc(&session).state.velocity.length();
            assert!(speed > top + 1.0, "{speed} past {top}");
            assert_eq!(npc(&session).id, NpcId(0));
        }
    }
}
