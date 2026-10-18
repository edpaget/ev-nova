//! How strong a ship and its friends are, the odds against a ship, and
//! whether a chase is hopeless.
//!
//! The rules are the original's (`_AIShipFriendStrength` @0x825a9,
//! `_AICalculateOddsAgainst` @0x84a2f and `_AIEvalHopelessChase`
//! @0x80eaa in the `EV Nova` executable). A ship's strength is its
//! `shïp`'s `Strength` ([`HullSpec::strength`](crate::combat::hull::HullSpec)).
//!
//! - **Friend strength** ([`friend_strength`]): a ship's strength times
//!   its shield factor ([`shield_factor`]: its shield over its most,
//!   held between [`WEAKEST`] and 1; the raw shield, so the weakest, for a
//!   ship that holds none), plus the same for each other ship not
//!   breaking up that escorts it or whose government is allied with its
//!   own. The player counts only itself: it has no escorts and no
//!   government yet, and no NPC counts the player as a friend. The
//!   original's doubling for an ally threatened by the ship's enemy is
//!   left out (a placeholder).
//! - **Odds against** a ship ([`odds_against`]): its enemies' strength
//!   over its friends' (no less than 1), in raw strength. Its friends are
//!   itself and every other intact ship allied with it; its enemies every
//!   other intact ship, not allied, of a government at war with its own
//!   or attacking it, and the player, while intact, when the ship
//!   threatens the player or a ship the player leads
//!   ([`Npc::threatens_player_among`]: `_ExtendedIsThreatToShip` @0x81a63,
//!   called @0x84a63, asks of the player only while it is active and not
//!   disabled, @0x81a33-0x81a4a, and then asks `_IsThreatToPlayer` of the
//!   ship, @0x81a9c), and the ship's government doesn't bless the player
//!   ([`Surroundings::blessed`]: `_PlayerBlessedByGovt(G, 0)` @0x81a89
//!   makes the player no threat before the threat test, @0x81a90). So a
//!   blessing government's ship counts no player in its odds, even while
//!   it attacks the player, under either reading of `rank_govt_reach` and
//!   `blessing_refresh`.
//!   The NPC loop (@0x84af9-0x84bfd) never asks `_ExtendedIsThreatToShip`:
//!   an NPC is an enemy by its government or while it attacks or snipes
//!   at the ship (AI state 4, @0x84bc3, which a warship's hopeless
//!   chase keeps, setting only the sub-state, @0x8e614), so a ship
//!   attacking one the ship leads is no enemy in the odds (that clause
//!   reaches only [`select_target`](crate::ai::target::select_target)'s
//!   threat step).
//!   The original sums strengths in 16 bits (`cvttss2si` @0x84af1,
//!   `addw` @0x84b7e, @0x84bee); here they are not truncated.
//!   The player's strength is multiplied by its [`rating_factor`]: its
//!   combat rating over 6400 times the raw `Strength` of `shïp` 128 (its
//!   `yardstick`, @0x84a7e), in whole numbers (`idivl` @0x84a91), held
//!   between 1 (0xdd068) and 2 (0xdd074) (@0x84a93-0x84ac8). So the
//!   factor is a step: 1 below twice 6400 yardsticks, and 2 from there.
//!   With a yardstick of 0 the original would divide by zero; here, as
//!   for one below 0, the factor is 1. Only the odds weigh the rating, not
//!   [`friend_strength`].
//! - **A hopeless chase** ([`hopeless_chase`]): the target is the player
//!   or is fleeing; the chaser is over [`HEAVY`] tons; their relative
//!   velocity (the target's less the chaser's) has a component over
//!   [`OPENING_SPEED`] either way and points more than [`OPENING_ANGLE`]
//!   off the bearing from the target back to the chaser, so the gap is
//!   opening; and the chaser's class is no faster than the target's, or
//!   slower when it holds a homing missile with the reach. Both are the
//!   classes' top speeds ([`HullSpec::class_speed`](crate::combat::hull::HullSpec::class_speed),
//!   class +0x40 @0x810d3-0x81151), so neither ship's outfits count. A
//!   missile reaches when its launcher's own slot of rounds holds some or
//!   is exactly [`UNLIMITED_SLOT`] (@0x8103a-0x8104e), it suits the
//!   target and has its ammunition, and its range down its sub-munition
//!   chain ([`WeaponSpec::range`](crate::combat::weapon::WeaponSpec::range))
//!   squared is at least [`MISSILE_REACH`] of the distance squared. The
//!   original compares the two angles without wrapping them; here they are
//!   compared the short way round.

use crate::ai::Surroundings;
use crate::ai::fire::{has_ammo, suitable_missile};
use crate::combat::ShipRef;
use crate::combat::aim::{angle_off, bearing};
use crate::combat::hull::Condition;
use crate::combat::weapon::Guidance;
use crate::flight::heading_of;
use crate::reserves::Reserves;
use crate::traffic::npc::Npc;

/// The least a ship's shield factor counts for (0.25 @0xdd684).
pub const WEAKEST: f32 = 0.25;
/// The mass, in tons, a chaser must be over for its chase to be hopeless.
pub const HEAVY: f32 = 99.0;
/// The relative speed, in pixels a tick on either axis, a gap must open
/// faster than (0.35 @0xdda68).
pub const OPENING_SPEED: f32 = 0.35;
/// How far off the bearing, in degrees, the relative velocity must point
/// for the gap to be opening.
pub const OPENING_ANGLE: f32 = 89.0;
/// The share of the distance squared a homing missile's range squared
/// must reach (0.8 @0xdd6f8).
pub const MISSILE_REACH: f32 = 0.8;
/// The slot of rounds a missile counts with in a chase though it holds
/// none: an `AmmoLoad` of -1, as stock gives most weapons that spend
/// nothing (`_AIEvalHopelessChase` @0x81044-0x81049).
pub const UNLIMITED_SLOT: i32 = -1;

/// A ship's shield factor: its shield over its most, held between
/// [`WEAKEST`] and 1; its raw shield, so held the same, when it holds
/// none.
#[must_use]
pub fn shield_factor(reserves: &Reserves) -> f32 {
    let shield = reserves.shield;
    let share = if shield.max > 0.0 {
        shield.now / shield.max
    } else {
        shield.now
    };
    share.clamp(WEAKEST, 1.0)
}

/// How many times its strength the player counts for in the odds: its
/// combat `rating` over 6400 times the `yardstick`, in whole numbers,
/// held between 1 and 2; 1 for a yardstick of 0 or less (see the module
/// docs).
#[must_use]
pub fn rating_factor(rating: i64, yardstick: i16) -> f32 {
    if yardstick <= 0 {
        return 1.0;
    }
    let quotient = rating / (6400 * i64::from(yardstick));
    quotient.clamp(1, 2) as f32
}

/// The strength of `ship` and its friends among `around` (see the
/// module docs).
#[must_use]
pub fn friend_strength(ship: ShipRef, around: &Surroundings) -> f32 {
    let weighed = |strength: f32, reserves: &Reserves| strength * shield_factor(reserves);
    match ship {
        ShipRef::Player => around.player.map_or(0.0, |player| {
            weighed(player.hull.strength, &player.reserves)
        }),
        ShipRef::Npc(id) => {
            let Some(npc) = around.npc(id) else {
                return 0.0;
            };
            let friends: f32 = around
                .npcs
                .iter()
                .filter(|other| {
                    other.id != id
                        && other.condition.standing()
                        && (other.leader == Some(id) || around.govts.allies(npc.govt, other.govt))
                })
                .map(|other| weighed(other.hull.strength, &other.reserves))
                .sum();
            weighed(npc.hull.strength, &npc.reserves) + friends
        }
    }
}

/// The odds against `npc` among `around` (see the module docs).
#[must_use]
pub fn odds_against(npc: &Npc, around: &Surroundings) -> f32 {
    let me = ShipRef::Npc(npc.id);
    let mut friends = npc.hull.strength;
    let mut enemies = match around.player {
        Some(player)
            if player.condition == Condition::Intact
                && !around.blessed(npc.govt)
                && npc.threatens_player_among(around.npcs) =>
        {
            player.hull.strength * rating_factor(player.rating, player.yardstick)
        }
        _ => 0.0,
    };
    for other in around
        .npcs
        .iter()
        .filter(|other| other.id != npc.id && other.condition == Condition::Intact)
    {
        if around.govts.allies(npc.govt, other.govt) {
            friends += other.hull.strength;
        } else if around.govts.enemies(npc.govt, other.govt) || other.goal.attacking() == Some(me) {
            enemies += other.hull.strength;
        }
    }
    enemies / friends.max(1.0)
}

/// Whether `npc`'s chase of `target` among `around` is hopeless (see the
/// module docs).
#[must_use]
pub fn hopeless_chase(npc: &Npc, target: ShipRef, around: &Surroundings) -> bool {
    let (quarry, handling, top_speed) = match target {
        ShipRef::Player => match around.player {
            Some(player) => (player.state, player.handling, player.hull.class_speed),
            None => return false,
        },
        ShipRef::Npc(id) => match around.npc(id) {
            Some(other) if matches!(other.goal, crate::ai::Goal::Flee(_)) => {
                (other.state, other.stats.handling, other.hull.class_speed)
            }
            _ => return false,
        },
    };
    if npc.hull.mass <= HEAVY {
        return false;
    }
    let gap = quarry.velocity - npc.state.velocity;
    if gap.x.abs() <= OPENING_SPEED && gap.y.abs() <= OPENING_SPEED {
        return false;
    }
    let opening = angle_off(
        heading_of(gap),
        bearing(quarry.position, npc.state.position),
    );
    if opening <= OPENING_ANGLE {
        return false;
    }
    let to = quarry.position - npc.state.position;
    let distance = to.x * to.x + to.y * to.y;
    let in_reach = npc.armament.mounts().iter().any(|mount| {
        let range = mount.spec.range();
        let slot = npc.rounds.slot(mount.spec.id);
        (slot > 0 || slot == UNLIMITED_SLOT)
            && mount.spec.guidance == Guidance::Homing
            && suitable_missile(&mount.spec, handling.turn_rate)
            && has_ammo(npc, mount)
            && range * range >= MISSILE_REACH * distance
    });
    let speed = npc.hull.class_speed;
    if in_reach {
        speed < top_speed
    } else {
        speed <= top_speed
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::combat::armament::LauncherRounds;

    use super::*;
    use crate::ai::Blessing;
    use crate::ai::fixture::blessing_of;
    use crate::ai::{Goal, PlayerSide};
    use crate::catalog::{GovtId, GovtRecord, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::{Condition, HullSpec};
    use crate::combat::weapon::WeaponSpec;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::govt::Governments;
    use crate::reserves::Gauge;
    use crate::stats::ShipStats;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::{govt, weapon};

    /// Government 140 (class 1), its ally 141, its enemy 142 and a
    /// neutral, 143.
    fn govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..govt(142)
            },
            govt(143),
        ])
    }

    /// NPC `id` of `govt`, of `strength`, its shields full (30), at rest
    /// at the centre.
    fn ship(id: u32, govt: i16, strength: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]));
        npc.govt = Some(GovtId(govt));
        npc.hull.strength = strength;
        npc
    }

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    /// The player at the centre, of `strength`, its shields full.
    fn player(strength: f32) -> PlayerSide {
        PlayerSide {
            state: ShipState::default(),
            condition: Condition::Intact,
            reserves: Reserves::full(100.0, 100.0, 100.0),
            hull: HullSpec {
                strength,
                ..HullSpec::default()
            },
            handling: ShipStats::new(FAST, &[]).handling,
            rating: 0,
            yardstick: 0,
            untargetable: false,
            blessing: Blessing::NONE,
            target: None,
            class: crate::catalog::EscortClass::Freighter,
        }
    }

    #[test]
    fn the_shield_factor_is_the_shield_held_between_a_quarter_and_whole() {
        let with = |now: f32, max: f32| {
            shield_factor(&Reserves {
                shield: Gauge { now, max },
                ..Reserves::full(0.0, 10.0, 10.0)
            })
        };
        assert_eq!(with(30.0, 30.0), 1.0);
        assert_eq!(with(15.0, 30.0), 0.5);
        assert_eq!(with(7.5, 30.0), 0.25);
        assert_eq!(with(3.0, 30.0), 0.25, "no less");
        assert_eq!(with(-3.0, 30.0), 0.25);
        assert_eq!(with(45.0, 30.0), 1.0, "no more");
        assert_eq!(with(0.0, 0.0), 0.25, "none held: the raw shield");
        assert_eq!(with(5.0, 0.0), 1.0);
        assert_eq!(WEAKEST, 0.25);
    }

    #[test]
    fn friend_strength_adds_escorts_and_allies_each_by_its_shields() {
        let govts = govts();
        let mut me = ship(1, 140, 100.0);
        me.reserves.shield.now = 15.0;
        let mut escort = ship(2, 143, 40.0);
        escort.leader = Some(NpcId(1));
        let ally = ship(3, 141, 20.0);
        let kin = ship(4, 140, 10.0);
        let enemy = ship(5, 142, 1000.0);
        let neutral = ship(6, 143, 1000.0);
        let mut dying = ship(7, 140, 1000.0);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let mut disabled = ship(8, 141, 5.0);
        disabled.condition = Condition::Disabled;
        disabled.reserves.shield.now = 0.0;
        let npcs = [me, escort, ally, kin, enemy, neutral, dying, disabled];
        let around = Surroundings {
            govts: &govts,
            player: Some(player(500.0)),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            friend_strength(n(1), &around),
            100.0 * 0.5 + 40.0 + 20.0 + 10.0 + 5.0 * 0.25,
            "the player, enemies, neutrals and the dying are not friends"
        );
        assert_eq!(
            friend_strength(n(6), &around),
            1000.0 + 40.0,
            "its own kind"
        );
        assert_eq!(friend_strength(ShipRef::Player, &around), 500.0, "alone");
        assert_eq!(friend_strength(n(9), &around), 0.0, "not there");
        let alone = Surroundings::new(&[], &npcs);
        assert_eq!(friend_strength(ShipRef::Player, &alone), 0.0, "no player");
    }

    #[test]
    fn the_odds_against_are_enemies_over_friends_in_raw_strength() {
        let govts = govts();
        let mut me = ship(1, 140, 100.0);
        me.reserves.shield.now = 1.0;
        let ally = ship(2, 141, 50.0);
        let enemy = ship(3, 142, 300.0);
        let mut attacker = ship(4, 143, 60.0);
        attacker.goal = Goal::Attack(n(1));
        let mut sniper = ship(5, 143, 30.0);
        sniper.goal = Goal::Snipe(n(1));
        let mut allied_attacker = ship(6, 141, 1000.0);
        allied_attacker.goal = Goal::Attack(n(1));
        let mut disabled_enemy = ship(7, 142, 1000.0);
        disabled_enemy.condition = Condition::Disabled;
        let neutral = ship(8, 143, 1000.0);
        let mut fleeing = ship(9, 143, 1000.0);
        fleeing.goal = Goal::Flee(n(1));
        let mut disabled_ally = ship(10, 141, 1000.0);
        disabled_ally.condition = Condition::Disabled;
        let npcs = [
            me,
            ally,
            enemy,
            attacker,
            sniper,
            allied_attacker,
            disabled_enemy,
            neutral,
            fleeing,
            disabled_ally,
        ];
        let around = Surroundings {
            govts: &govts,
            player: Some(player(500.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let friends = 100.0 + 50.0 + 1000.0;
        assert_eq!(
            odds_against(&npcs[0], &around),
            (300.0 + 60.0 + 30.0) / friends
        );
        let mut hunting = npcs[0].clone();
        hunting.goal = Goal::Attack(ShipRef::Player);
        assert_eq!(
            odds_against(&hunting, &around),
            (300.0 + 60.0 + 30.0 + 500.0) / friends,
            "the player, while it is threatened"
        );
    }

    #[test]
    fn the_odds_against_count_friends_as_no_less_than_1() {
        let govts = govts();
        let npcs = [ship(1, 140, 0.0), ship(2, 142, 3.0)];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(odds_against(&npcs[0], &around), 3.0);
        let lone = [ship(1, 140, 4.0)];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &lone)
        };
        assert_eq!(odds_against(&lone[0], &around), 0.0);
    }

    #[test]
    fn the_rating_factor_steps_from_1_to_2_at_twice_6400_yardsticks() {
        let s = 10;
        let one = 6400 * i64::from(s);
        assert_eq!(rating_factor(0, s), 1.0);
        assert_eq!(rating_factor(1, s), 1.0);
        assert_eq!(rating_factor(one, s), 1.0);
        assert_eq!(rating_factor(2 * one - 1, s), 1.0, "the quotient is whole");
        assert_eq!(rating_factor(2 * one, s), 2.0);
        assert_eq!(rating_factor(10 * one, s), 2.0, "no more");
        assert_eq!(rating_factor(-2 * one, s), 1.0, "no less");
        assert_eq!(rating_factor(i64::MAX, s), 2.0);
        assert_eq!(rating_factor(i64::MAX, 0), 1.0, "no yardstick");
        assert_eq!(rating_factor(i64::MAX, -5), 1.0);
        assert_eq!(rating_factor(2 * 6400, 1), 2.0);
    }

    #[test]
    fn the_odds_against_a_ship_threatening_the_player_scale_with_its_rating() {
        let govts = govts();
        let mut hunting = ship(1, 140, 50.0);
        hunting.goal = Goal::Attack(ShipRef::Player);
        let npcs = [hunting];
        let odds = |rating| {
            let around = Surroundings {
                govts: &govts,
                player: Some(PlayerSide {
                    rating,
                    yardstick: 10,
                    ..player(100.0)
                }),
                ..Surroundings::new(&[], &npcs)
            };
            odds_against(&npcs[0], &around)
        };
        assert_eq!(odds(0), 2.0);
        assert_eq!(odds(127_999), 2.0);
        assert_eq!(odds(128_000), 4.0);
        let mut calm = npcs[0].clone();
        calm.goal = Goal::default();
        let around = Surroundings {
            govts: &govts,
            player: Some(PlayerSide {
                rating: 128_000,
                yardstick: 10,
                ..player(100.0)
            }),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(odds_against(&calm, &around), 0.0, "not threatening");
        assert_eq!(
            friend_strength(ShipRef::Player, &around),
            100.0,
            "the rating weighs only in the odds"
        );
    }

    #[test]
    fn the_odds_count_the_player_against_a_ship_attacking_one_the_player_leads() {
        let govts = govts();
        let mut raider = ship(1, 143, 50.0);
        raider.goal = Goal::Attack(n(2));
        let mut escort = ship(2, 143, 0.0);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let odds = |escort: &Npc| {
            let npcs = [raider.clone(), escort.clone()];
            let around = Surroundings {
                govts: &govts,
                player: Some(PlayerSide {
                    rating: 128_000,
                    yardstick: 10,
                    ..player(100.0)
                }),
                ..Surroundings::new(&[], &npcs)
            };
            odds_against(&npcs[0], &around)
        };
        assert_eq!(odds(&escort), 4.0, "the player, at its rating, against 50");
        let stranger = Npc {
            escort: None,
            ..escort
        };
        assert_eq!(odds(&stranger), 0.0, "a ship nobody leads");
    }

    #[test]
    fn a_blessing_government_counts_no_player_in_its_odds() {
        let govts = govts();
        let mut hunting = ship(1, 140, 50.0);
        hunting.goal = Goal::Attack(ShipRef::Player);
        let mut raider = ship(1, 143, 50.0);
        raider.goal = Goal::Attack(n(2));
        let mut escort = ship(2, 143, 0.0);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        for (npcs, rating, unblessed, case) in [
            (vec![hunting], 0, 2.0, "hunting the player"),
            (
                vec![raider, escort],
                128_000,
                4.0,
                "attacking a ship the player leads, at the rating",
            ),
        ] {
            let odds = |blessing| {
                let around = Surroundings {
                    govts: &govts,
                    player: Some(PlayerSide {
                        rating,
                        yardstick: 10,
                        blessing,
                        ..player(100.0)
                    }),
                    ..Surroundings::new(&[], &npcs)
                };
                odds_against(&npcs[0], &around)
            };
            let own = npcs[0].govt.expect("governed");
            assert_eq!(odds(Blessing::NONE), unblessed, "{case}: unblessed");
            assert_eq!(odds(blessing_of(own)), 0.0, "{case}");
            assert_eq!(
                odds(blessing_of(GovtId(141))),
                unblessed,
                "{case}: another government's blessing"
            );
        }
    }

    #[test]
    fn a_disabled_or_breaking_up_player_is_no_enemy_in_the_odds() {
        let govts = govts();
        let mut hunting = ship(1, 140, 50.0);
        hunting.goal = Goal::Attack(ShipRef::Player);
        let npcs = [hunting];
        let odds = |condition| {
            let around = Surroundings {
                govts: &govts,
                player: Some(PlayerSide {
                    condition,
                    ..player(100.0)
                }),
                ..Surroundings::new(&[], &npcs)
            };
            odds_against(&npcs[0], &around)
        };
        assert_eq!(odds(Condition::Intact), 2.0);
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 1,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            assert_eq!(odds(condition), 0.0, "{condition:?}");
        }
    }

    /// A heavy chaser (100 tons) of class speed 6 at the centre, chasing
    /// the player 300 pixels above, also of class speed 6, which opens the
    /// gap upwards at `speed`.
    fn chase(speed: f32) -> (Npc, PlayerSide) {
        let mut chaser = ship(1, 140, 0.0);
        chaser.hull.mass = 100.0;
        chaser.hull.class_speed = 6.0;
        chaser.goal = Goal::Attack(ShipRef::Player);
        let mut target = player(0.0);
        target.state.position = Vec2::new(0.0, -300.0);
        target.state.velocity = Vec2::new(0.0, -speed);
        target.hull.class_speed = 6.0;
        (chaser, target)
    }

    /// Whether `chaser`'s chase of `target` is hopeless, both moved to
    /// (100, 200) from where they are and drifting left at 0.5 on top of
    /// their own velocities, which changes nothing.
    fn hopeless(chaser: &Npc, mut target: PlayerSide) -> bool {
        let (offset, drift) = (Vec2::new(100.0, 200.0), Vec2::new(-0.5, 0.0));
        let mut chaser = chaser.clone();
        chaser.state.position = chaser.state.position + offset;
        chaser.state.velocity = chaser.state.velocity + drift;
        target.state.position = target.state.position + offset;
        target.state.velocity = target.state.velocity + drift;
        let npcs = [chaser.clone()];
        let around = Surroundings {
            player: Some(target),
            ..Surroundings::new(&[], &npcs)
        };
        hopeless_chase(&chaser, ShipRef::Player, &around)
    }

    #[test]
    fn a_chase_of_a_player_opening_the_gap_at_no_less_speed_is_hopeless() {
        let (chaser, target) = chase(1.0);
        assert!(hopeless(&chaser, target));
        let mut slower = target;
        slower.hull.class_speed = 5.9;
        assert!(!hopeless(&chaser, slower), "the chaser is faster");
    }

    #[test]
    fn a_chase_compares_the_class_speeds_not_the_outfitted_top_speeds() {
        let (mut chaser, mut target) = chase(1.0);
        chaser.stats.handling.max_speed = 20.0;
        target.handling.max_speed = 1.0;
        assert!(
            hopeless(&chaser, target),
            "equal class speeds, whatever their outfits give"
        );
        target.hull.class_speed = 5.9;
        target.handling.max_speed = 50.0;
        assert!(!hopeless(&chaser, target), "the chaser's class is faster");
    }

    #[test]
    fn a_light_chaser_is_never_hopeless() {
        let (mut chaser, target) = chase(1.0);
        chaser.hull.mass = 99.0;
        assert!(!hopeless(&chaser, target));
        assert_eq!(HEAVY, 99.0);
    }

    #[test]
    fn the_gap_must_open_faster_than_0_35_on_an_axis() {
        let (chaser, mut target) = chase(0.35);
        assert!(!hopeless(&chaser, target));
        target.state.velocity = Vec2::new(0.0, -0.36);
        assert!(hopeless(&chaser, target));
        target.state.velocity = Vec2::new(0.36, -0.36);
        assert!(hopeless(&chaser, target), "on either axis");
        assert_eq!(OPENING_SPEED, 0.35);
    }

    #[test]
    fn the_gap_opens_only_more_than_89_degrees_off_the_bearing() {
        let (chaser, mut target) = chase(1.0);
        // The target is straight up: the bearing from it back to the
        // chaser is 180, and the relative velocity points round from
        // there.
        let off = |degrees: f32| crate::flight::facing(degrees);
        target.state.velocity = off(91.0);
        assert!(!hopeless(&chaser, target), "89 off");
        target.state.velocity = off(90.5);
        assert!(hopeless(&chaser, target), "89.5 off");
        target.state.velocity = off(269.5);
        assert!(hopeless(&chaser, target), "the other way round");
        target.state.velocity = off(269.0);
        assert!(!hopeless(&chaser, target));
        target.state.velocity = off(180.0);
        assert!(!hopeless(&chaser, target), "closing");
        assert_eq!(OPENING_ANGLE, 89.0);
    }

    /// A homing missile launcher 140 reaching 100 pixels, firing rounds of
    /// weapon 138, one of them held on the launcher.
    fn missile() -> (Armament, LauncherRounds) {
        let record = WeaponRecord {
            guidance: 1,
            count: 10,
            speed: 1000,
            ammo_type: 10,
            ..weapon(140)
        };
        let rounds = LauncherRounds::from([(WeaponId(140), 1)]);
        (Armament::new([(WeaponSpec::new(&record), 1)]), rounds)
    }

    #[test]
    fn a_homing_missile_in_reach_makes_equal_speeds_no_hopeless_chase() {
        let (mut chaser, mut target) = chase(1.0);
        let (armament, rounds) = missile();
        chaser.armament = armament;
        chaser.rounds = rounds;
        // 100 pixels' range: 10000 >= 0.8 d² up to d² = 12500.
        target.state.position = Vec2::new(0.0, -111.8);
        assert!(!hopeless(&chaser, target), "equal speeds, in reach");
        target.hull.class_speed = 6.1;
        assert!(hopeless(&chaser, target), "slower, in reach");
        target.hull.class_speed = 6.0;
        target.state.position = Vec2::new(0.0, -111.81);
        assert!(hopeless(&chaser, target), "out of reach");
        target.state.position = Vec2::new(70.0, -80.0);
        assert!(!hopeless(&chaser, target), "in reach both ways");
        target.state.position = Vec2::new(80.0, -80.0);
        assert!(hopeless(&chaser, target), "out of reach both ways");
        target.state.position = Vec2::new(0.0, -100.0);
        chaser.rounds = LauncherRounds::default();
        assert!(hopeless(&chaser, target), "no rounds");
        assert_eq!(MISSILE_REACH, 0.8);
    }

    #[test]
    fn a_chase_counts_a_missile_only_while_its_launchers_slot_holds_rounds_or_minus_one() {
        let (mut chaser, mut target) = chase(1.0);
        let unlimited = WeaponRecord {
            guidance: 1,
            count: 10,
            speed: 1000,
            ammo_type: -1,
            ..weapon(141)
        };
        chaser.armament = Armament::new([(WeaponSpec::new(&unlimited), 1)]);
        target.state.position = Vec2::new(0.0, -100.0);
        for (slot, in_reach) in [(0, false), (-1, true), (2, true), (-2, false)] {
            chaser.rounds = LauncherRounds::from([(WeaponId(141), slot)]);
            assert_eq!(
                !hopeless(&chaser, target),
                in_reach,
                "an unlimited missile with {slot} on its slot"
            );
        }
        let (armament, _) = missile();
        chaser.armament = armament;
        chaser.rounds = LauncherRounds::from([(WeaponId(138), 5)]);
        assert!(
            hopeless(&chaser, target),
            "a launcher needs its own slot, not its ammunition's"
        );
        chaser.rounds = LauncherRounds::from([(WeaponId(140), -1)]);
        assert!(
            hopeless(&chaser, target),
            "-1 passes the slot test, but a launcher fires only rounds it holds"
        );
    }

    #[test]
    fn a_chase_reaches_with_a_missiles_chain_range() {
        let (mut chaser, mut target) = chase(1.0);
        let torpedo = WeaponRecord {
            guidance: 1,
            count: 10,
            speed: 1000,
            sub_count: 5,
            sub_type: Some(WeaponId(148)),
            ..weapon(182)
        };
        let homer = WeaponRecord {
            guidance: 1,
            count: 30,
            speed: 1000,
            ..weapon(148)
        };
        let arsenal = crate::combat::armament::Arsenal::new(&[torpedo, homer], Vec::new());
        chaser.armament = Armament::new([(*arsenal.weapon(WeaponId(182)).expect("read"), 1)]);
        chaser.rounds = LauncherRounds::from([(WeaponId(182), -1)]);
        // 400 pixels' range: 160000 >= 0.8 d² up to d² = 200000.
        target.state.position = Vec2::new(0.0, -447.0);
        assert!(!hopeless(&chaser, target), "in reach down the chain");
        target.state.position = Vec2::new(0.0, -448.0);
        assert!(hopeless(&chaser, target), "beyond it");
    }

    #[test]
    fn a_missile_unsuitable_for_an_agile_target_is_out_of_reach() {
        let (mut chaser, mut target) = chase(1.0);
        let (armament, rounds) = missile();
        chaser.armament = armament;
        chaser.rounds = rounds;
        target.state.position = Vec2::new(0.0, -100.0);
        target.handling.turn_rate = 3.99;
        assert!(!hopeless(&chaser, target), "slow enough");
        target.handling.turn_rate = 4.0;
        assert!(hopeless(&chaser, target), "its 0 turn is too slow");
        let mut agile = chaser.armament.mounts()[0].spec;
        agile.turn = 2.1;
        chaser.armament = Armament::new([(agile, 1)]);
        assert!(!hopeless(&chaser, target), "it turns enough");
    }

    #[test]
    fn a_missile_ai_ships_wont_use_is_out_of_reach() {
        let (mut chaser, mut target) = chase(1.0);
        let (armament, rounds) = missile();
        chaser.armament = armament;
        chaser.rounds = rounds;
        target.state.position = Vec2::new(0.0, -100.0);
        assert!(!hopeless(&chaser, target));
        let mut wont = chaser.armament.mounts()[0].spec;
        wont.flags2 = crate::combat::weapon::AI_WONT_USE;
        chaser.armament = Armament::new([(wont, 1)]);
        assert!(hopeless(&chaser, target));
    }

    #[test]
    fn only_the_player_or_a_fleeing_npc_is_chased_hopelessly() {
        let (chaser, _) = chase(1.0);
        let mut quarry = ship(2, 142, 0.0);
        quarry.hull.class_speed = 6.0;
        quarry.state.position = Vec2::new(0.0, -300.0);
        quarry.state.velocity = Vec2::new(0.0, -1.0);
        let npcs = [chaser.clone(), quarry.clone()];
        let around = Surroundings::new(&[], &npcs);
        assert!(!hopeless_chase(&chaser, n(2), &around), "not fleeing");
        quarry.goal = Goal::Flee(n(1));
        let npcs = [chaser.clone(), quarry];
        let around = Surroundings::new(&[], &npcs);
        assert!(hopeless_chase(&chaser, n(2), &around));
        assert!(!hopeless_chase(&chaser, n(9), &around), "not there");
    }
}
