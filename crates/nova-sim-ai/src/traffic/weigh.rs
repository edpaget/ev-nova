//! Weighing the fight's strikes before anything answers them:
//! `_DamageShip`'s answer flag for each ([`answer_flag`]), the stray
//! provocation each adds to its ship (+0xc910) and zeroes once answered,
//! and that provocation's ease each tick.

use super::{Traffic, World};
use crate::ai::answer_flag::{self, FlagRules, STRAY_EASE};
use crate::chance::Chance;
use crate::combat::{ShipRef, Strike};

impl World<'_> {
    /// The answer flag's readings in this world.
    #[must_use]
    pub fn flag_rules(&self) -> FlagRules {
        FlagRules {
            shoot_penalty: self.stray_shoot_penalty,
            independent_kin: self.independent_kin,
        }
    }
}

impl Traffic {
    /// Weighs each of `strikes` in turn in `world`, rolling on `chance`
    /// ([`answer_flag::verdict`]): its ship gains the stray provocation it
    /// adds, which an answer then zeroes (@0x3bb83), and the strike is
    /// marked [`answered`](Strike::answered) or not. Each strike reads the
    /// NPCs' stray provocation as the strikes before it left it.
    ///
    /// Each verdict reads the targets and goals as the tick found them,
    /// where the original runs the previous hit's reaction before it
    /// weighs the next. It settles in the hit's own step, before the frame
    /// is drawn (see `nova_sim::Session::answer_strikes`), so it crosses no
    /// boundary the player sees.
    pub fn weigh_strikes(
        &mut self,
        world: &World,
        strikes: &mut [Strike],
        chance: &mut dyn Chance,
    ) {
        let rules = world.flag_rules();
        for strike in strikes {
            let verdict = answer_flag::verdict(strike, &world.around(&self.npcs), rules, chance);
            strike.answered = verdict.answers;
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            if let Some(npc) = self.npcs.iter_mut().find(|npc| npc.id == id) {
                npc.stray += verdict.stray;
                if verdict.answers {
                    npc.stray = 0.0;
                }
            }
        }
    }

    /// Eases every NPC's stray provocation by [`STRAY_EASE`] while it is
    /// above none, to no lower than none, as `_HandleShip` does each frame
    /// (@0x35374-0x353b2) after the collision passes and the shots.
    pub fn ease_stray(&mut self) {
        for npc in &mut self.npcs {
            if npc.stray > 0.0 {
                npc.stray = (npc.stray - STRAY_EASE).max(0.0);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{NEUTRAL, PIRATES, TRADERS, govts, n, player, ship};
    use crate::combat::fixture::strike;
    use crate::traffic::npc::{AiType, Npc};
    use nova_sim_base::fixture::Draws;

    /// A traffic of `npcs`.
    fn of(npcs: Vec<Npc>) -> Traffic {
        Traffic {
            npcs,
            ..Traffic::default()
        }
    }

    /// The player's stray hit on NPC `on`, weighing `weight`.
    fn stray(on: u32, weight: f32) -> Strike {
        Strike {
            intended: false,
            answered: false,
            weight,
            ..strike(n(on), ShipRef::Player)
        }
    }

    fn strays(traffic: &Traffic) -> Vec<f32> {
        traffic.npcs.iter().map(|npc| npc.stray).collect()
    }

    #[test]
    fn weighing_marks_each_strike_and_an_answer_clears_its_stray_provocation() {
        let govts = govts(0, 0);
        let mut traffic = of(vec![
            ship(1, TRADERS, AiType::Warship, 0.0, 0.0),
            ship(2, PIRATES, AiType::Warship, 0.0, 0.0),
            ship(3, NEUTRAL, AiType::Warship, 0.0, 0.0),
        ]);
        traffic.npcs[0].stray = 30.0;
        traffic.npcs[2].stray = 30.0;
        let world = World {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..World::new(&[])
        };
        let mut strikes = [
            Strike {
                answered: false,
                ..strike(n(1), n(2))
            },
            stray(3, 4.0),
        ];
        let mut chance = Draws::of(&[]);
        traffic.weigh_strikes(&world, &mut strikes, &mut chance);
        assert_eq!(strikes.map(|strike| strike.answered), [true, false]);
        assert_eq!(strays(&traffic), [0.0, 0.0, 37.0], "answered; 30 + 7");
        assert_eq!(chance.asked, [4], "the neutral's roll: forgiven");
    }

    #[test]
    fn two_stray_hits_in_one_tick_read_each_others_provocation() {
        let govts = govts(0, 0);
        let mut traffic = of(vec![ship(1, NEUTRAL, AiType::Warship, 0.0, 0.0)]);
        traffic.npcs[0].stray = 45.0;
        let world = World {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..World::new(&[])
        };
        // The first, forgiven by the roll, lifts it to 52; the second, not
        // forgiven, is answered from it, which it is not from 45, and
        // clears it.
        let mut strikes = [stray(1, 4.0), stray(1, 4.0)];
        traffic.weigh_strikes(&world, &mut strikes, &mut Draws::of(&[1, 0]));
        assert_eq!(strikes.map(|strike| strike.answered), [false, true]);
        assert_eq!(strays(&traffic), [0.0]);
    }

    #[test]
    fn a_strike_on_the_player_or_a_ship_gone_marks_nothing() {
        let mut traffic = of(vec![ship(1, NEUTRAL, AiType::Warship, 0.0, 0.0)]);
        let mut strikes = [strike(ShipRef::Player, n(1)), strike(n(9), n(1))];
        traffic.weigh_strikes(&World::new(&[]), &mut strikes, &mut Draws::of(&[]));
        assert_eq!(strikes.map(|strike| strike.answered), [false, false]);
        assert_eq!(strays(&traffic), [0.0]);
    }

    #[test]
    fn the_worlds_flag_rules_are_its_two_readings() {
        use crate::rulebook::RuleSource;
        assert_eq!(World::new(&[]).flag_rules(), FlagRules::default());
        let world = World {
            stray_shoot_penalty: RuleSource::Bible,
            ..World::new(&[])
        };
        assert_eq!(world.flag_rules().shoot_penalty, RuleSource::Bible);
        assert_eq!(world.flag_rules().independent_kin, RuleSource::Engine);
        let world = World {
            independent_kin: RuleSource::Bible,
            ..World::new(&[])
        };
        assert_eq!(world.flag_rules().independent_kin, RuleSource::Bible);
        assert_eq!(world.flag_rules().shoot_penalty, RuleSource::Engine);
    }

    #[test]
    fn stray_provocation_eases_half_a_tick_to_0() {
        let mut traffic = of(vec![
            ship(1, NEUTRAL, AiType::Warship, 0.0, 0.0),
            ship(2, NEUTRAL, AiType::Warship, 0.0, 0.0),
            ship(3, NEUTRAL, AiType::Warship, 0.0, 0.0),
        ]);
        traffic.npcs[0].stray = 1.25;
        traffic.npcs[1].stray = 0.0;
        traffic.npcs[2].stray = -1.0;
        traffic.ease_stray();
        assert_eq!(strays(&traffic), [0.75, 0.0, -1.0]);
        traffic.ease_stray();
        assert_eq!(strays(&traffic), [0.25, 0.0, -1.0]);
        traffic.ease_stray();
        assert_eq!(strays(&traffic), [0.0, 0.0, -1.0], "floored at none");
        assert_eq!(STRAY_EASE, 0.5);
    }
}
