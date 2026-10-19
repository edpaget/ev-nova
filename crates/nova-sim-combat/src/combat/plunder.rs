//! A plundering pirate's fire: disable-only as it leaves, and unable to
//! kill as it hits (`_SpawnShot`, `_SpawnBeam` and `_DamageShip` in the
//! `EV Nova` executable). A pirate plunders in its goal 0xd (see
//! `nova_sim::ai::pirate`), which [`Fighter::plundering`] carries.
//!
//! **At launch**, a shot's or beam's flag starts as its weapon's
//! disable-only bit ([`WeaponSpec::disable_only`](super::weapon::WeaponSpec::disable_only))
//! and then:
//!
//! - **A shot** ([`shot_disables`]) of an NPC is disable-only when the NPC
//!   is in goal 0xd, whatever the shot's target, at another ship, at none
//!   or a point-defence shot (`_AIIsShipAttemptingDisable`,
//!   `_SpawnShot` @0x3e9fb-0x3ea37); or when it plunders the shot's target
//!   ([`plunders`]) and that target is not disabled (@0x3e826-0x3e899).
//!   The target is the one `_SpawnShot` is given: the firer's +0x70 for its
//!   weapons (`_FireAIShipWeapon` @0x889cb), none for a front turret firing
//!   ahead (@0x88bf0) and for point defence (@0x396d3). A sub-munition is
//!   spawned by its parent's owner (+0x2c) at its own target
//!   (`_SpawnShotSubmunitions` @0x3f30b-0x3f328), so it reads its owner's
//!   plunder as it is released.
//! - **A beam** ([`beam_disables`]) of an NPC is disable-only only when the
//!   NPC plunders its target and that target is not disabled
//!   (`_SpawnBeam` @0x45270-0x452d1; its sixth argument, tested ≠ 1
//!   @0x45284, is -1 at every caller). There is no goal-0xd branch for
//!   beams.
//!
//! **At the hit** ([`cant_kill`], `_DamageShip` @0x3a895-0x3a9a3): a real
//! hit (a shot's direct hit, by contact or proximity, a beam's, or a
//! missile lock, which can't kill already; never a blast, a shockwave or
//! sabotage) by an NPC attacker whose target (+0x70) is the ship hit can't
//! kill it when the attacker is in goal 0xd, or its leader (+0x9a), an
//! NPC, is (`_AIShipIsPlundering` @0x89841, the leader's own target not
//! read).
//!
//! `_AIShipIsPlunderingShip`'s other half (@0x83f6e) is goal 4 with
//! sub-goal 0xf, a pirate docked with the ship it plunders
//! ([`Fighter::docked`]): it plunders its target too ([`plunders`]), so
//! its shots and beams at that target leave disable-only while it is
//! intact. It isn't `_AIShipIsPlundering` (goal 0xd alone, @0x89841), so
//! neither the shot's any-target branch nor the can't-kill reads it.
//! Nothing here plays a sound.

use super::{Fighter, ShipRef, Target};

/// What a ship launching a shot or beam reads of its own plunder: whether
/// it is an NPC (slots 1-63), whether it plunders (goal 0xd), and the ship
/// it targets (+0x70).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plunderer {
    /// Whether it is an NPC: the player's fire never is disable-only by
    /// the plunder.
    pub npc: bool,
    /// Whether it plunders ([`Fighter::plundering`]).
    pub plundering: bool,
    /// Whether it is docked with the ship it plunders
    /// ([`Fighter::docked`]).
    pub docked: bool,
    /// The ship it targets ([`Fighter::target`]).
    pub target: Option<ShipRef>,
}

impl Plunderer {
    /// `fighter` as its launches read it.
    #[must_use]
    pub fn of(fighter: &Fighter) -> Self {
        Self {
            npc: matches!(fighter.ship, ShipRef::Npc(_)),
            plundering: fighter.plundering,
            docked: fighter.docked,
            target: fighter.target,
        }
    }
}

/// Whether `firer` plunders `ship`, `_AIShipIsPlunderingShip`
/// (@0x83f5e-0x83f94): it is in goal 0xd, or docked in goal 4 with
/// sub-goal 0xf, and targets it.
#[must_use]
pub fn plunders(firer: Plunderer, ship: ShipRef) -> bool {
    (firer.plundering || firer.docked) && firer.target == Some(ship)
}

/// Whether a shot `firer` launches at `target`, as `targets` see it, is
/// disable-only by the plunder (see the module docs).
#[must_use]
pub fn shot_disables(firer: Plunderer, target: Option<ShipRef>, targets: &[Target]) -> bool {
    firer.npc && (firer.plundering || beam_disables(firer, target, targets))
}

/// Whether a beam `firer` launches at `target`, as `targets` see it, is
/// disable-only by the plunder (see the module docs).
#[must_use]
pub fn beam_disables(firer: Plunderer, target: Option<ShipRef>, targets: &[Target]) -> bool {
    firer.npc && target.is_some_and(|ship| !disabled(ship, targets) && plunders(firer, ship))
}

/// Whether `ship` is disabled among `targets`, as `_IsDisabled` reads it:
/// disabled or breaking up. A ship not among them is not.
fn disabled(ship: ShipRef, targets: &[Target]) -> bool {
    targets
        .iter()
        .any(|target| target.ship == ship && target.condition.disabled())
}

/// Whether a real hit on `victim` by `attacker` can't kill it by the
/// plunder, among `fighters` (see the module docs).
#[must_use]
pub fn cant_kill(attacker: Option<ShipRef>, victim: ShipRef, fighters: &[Fighter]) -> bool {
    let find = |ship: ShipRef| fighters.iter().find(|fighter| fighter.ship == ship);
    let Some(attacker) = attacker
        .filter(|ship| matches!(ship, ShipRef::Npc(_)))
        .and_then(find)
    else {
        return false;
    };
    let led_by_plunderer = attacker
        .leader
        .filter(|leader| matches!(leader, ShipRef::Npc(_)))
        .and_then(find)
        .is_some_and(|leader| leader.plundering);
    attacker.target == Some(victim) && (attacker.plundering || led_by_plunderer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ShipId;
    use crate::combat::NpcId;
    use crate::combat::hull::Condition;
    use crate::combat::sides::Ties;
    use crate::geometry::Vec2;

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    /// NPC `id` as shots see it, `condition`.
    fn target(id: u32, condition: Condition) -> Target {
        Target {
            ship: n(id),
            ship_type: ShipId(128),
            fleet: n(id),
            ties: Ties::default(),
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            radius: 5.0,
            sprite_x: 15.0,
            sheet: true,
            condition,
            disabled: condition != Condition::Intact,
            miss_class: 100,
        }
    }

    /// NPC 1, plundering as `plundering` says, targeting `target`.
    fn pirate(plundering: bool, target: Option<ShipRef>) -> Plunderer {
        Plunderer {
            npc: true,
            plundering,
            target,
            docked: false,
        }
    }

    const DYING: Condition = Condition::Dying {
        ticks_left: 2,
        starting: false,
    };

    #[test]
    fn a_ship_plunders_the_ship_it_targets_in_goal_0xd() {
        assert!(plunders(pirate(true, Some(n(2))), n(2)));
        assert!(!plunders(pirate(true, Some(n(3))), n(2)), "another target");
        assert!(!plunders(pirate(true, None), n(2)));
        assert!(!plunders(pirate(false, Some(n(2))), n(2)), "attacking");
    }

    #[test]
    fn a_ship_docked_in_goal_4_plunders_the_ship_it_targets_too() {
        let docked = |target| Plunderer {
            docked: true,
            ..pirate(false, target)
        };
        assert!(plunders(docked(Some(n(2))), n(2)));
        assert!(!plunders(docked(Some(n(3))), n(2)), "another target");
        assert!(!plunders(docked(None), n(2)));
    }

    #[test]
    fn a_docked_pirates_shots_and_beams_spare_only_its_intact_target() {
        let targets = [target(2, Condition::Intact), target(3, Condition::Disabled)];
        let docked = Plunderer {
            docked: true,
            ..pirate(false, Some(n(2)))
        };
        assert!(shot_disables(docked, Some(n(2)), &targets));
        assert!(beam_disables(docked, Some(n(2)), &targets));
        for aimed in [Some(n(4)), None] {
            assert!(
                !shot_disables(docked, aimed, &targets),
                "{aimed:?}: no goal-0xd branch"
            );
        }
        let at_wreck = Plunderer {
            target: Some(n(3)),
            ..docked
        };
        assert!(!shot_disables(at_wreck, Some(n(3)), &targets), "disabled");
        assert!(!beam_disables(at_wreck, Some(n(3)), &targets));
    }

    #[test]
    fn a_plundering_npcs_every_shot_is_disable_only_whatever_its_target() {
        let targets = [target(2, Condition::Intact), target(3, Condition::Disabled)];
        let plundering = pirate(true, Some(n(2)));
        for aimed in [
            Some(n(2)),
            Some(n(3)),
            Some(n(4)),
            None,
            Some(ShipRef::Player),
        ] {
            assert!(shot_disables(plundering, aimed, &targets), "{aimed:?}");
        }
        let attacking = pirate(false, Some(n(2)));
        for aimed in [Some(n(2)), None] {
            assert!(!shot_disables(attacking, aimed, &targets), "{aimed:?}");
        }
        let player = Plunderer {
            npc: false,
            ..plundering
        };
        assert!(
            !shot_disables(player, Some(n(2)), &targets),
            "the player's never"
        );
    }

    #[test]
    fn a_beam_is_disable_only_only_at_the_intact_ship_its_npc_plunders() {
        let targets = [
            target(2, Condition::Intact),
            target(3, Condition::Disabled),
            target(4, DYING),
        ];
        let at = |ship: Option<ShipRef>| pirate(true, ship);
        assert!(beam_disables(at(Some(n(2))), Some(n(2)), &targets));
        assert!(
            !beam_disables(at(Some(n(2))), Some(n(5)), &targets),
            "another ship"
        );
        assert!(
            !beam_disables(at(Some(n(2))), None, &targets),
            "aimed at none"
        );
        assert!(
            !beam_disables(at(Some(n(3))), Some(n(3)), &targets),
            "disabled"
        );
        assert!(
            !beam_disables(at(Some(n(4))), Some(n(4)), &targets),
            "breaking up"
        );
        assert!(
            beam_disables(at(Some(n(9))), Some(n(9)), &targets),
            "a ship gone is no disabled ship"
        );
        assert!(!beam_disables(
            pirate(false, Some(n(2))),
            Some(n(2)),
            &targets
        ));
        let player = Plunderer {
            npc: false,
            ..at(Some(n(2)))
        };
        assert!(!beam_disables(player, Some(n(2)), &targets));
        assert!(
            shot_disables(
                Plunderer {
                    plundering: false,
                    ..at(Some(n(2)))
                },
                Some(n(2)),
                &targets
            ) == beam_disables(
                Plunderer {
                    plundering: false,
                    ..at(Some(n(2)))
                },
                Some(n(2)),
                &targets
            ),
            "a shot's own test is the beam's"
        );
    }
}
