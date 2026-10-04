//! Damage: what a hit does to a ship's reserves, and which ships a blast
//! reaches.
//!
//! The values are the original's (`_DamageShip` and `_HandleShipHit` in
//! the `EV Nova` executable):
//!
//! - A weapon that passes through shields takes its mass damage off the
//!   armour, ignoring its energy damage and leaving the shields as they
//!   are.
//! - Any other takes its energy damage (when it has any) off the shields,
//!   then, only if they are then down (at or below none), its mass damage
//!   off the armour. While the shields hold, mass damage never reaches the
//!   armour, and energy damage keeps draining a shield that is down.
//! - Either way the shield is then kept above [`SHIELD_FLOOR`] of what it
//!   holds below none, so a shield driven negative must first recharge up
//!   through none.
//! - A blast reaches every ship within the square `radius` on each side of
//!   it, but the one hit directly and the firer, unless the firer is the
//!   player and the weapon's blast does not spare the player.

use super::ShipRef;
use super::weapon::WeaponSpec;
use crate::geometry::Vec2;
use crate::reserves::Reserves;

/// How far below none a shield can be driven, as a fraction of what it
/// holds.
pub const SHIELD_FLOOR: f32 = 0.1;

/// What one hit does.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hit {
    /// Damage to armour.
    pub mass: f32,
    /// Damage to shields.
    pub energy: f32,
    /// Whether it passes through shields.
    pub passes_shields: bool,
}

impl Hit {
    /// A full hit from `weapon`.
    #[must_use]
    pub fn of(weapon: &WeaponSpec) -> Self {
        Self {
            mass: weapon.mass_damage,
            energy: weapon.energy_damage,
            passes_shields: weapon.passes_shields(),
        }
    }
}

/// Takes `hit` off `reserves` (see the module docs).
pub fn apply(reserves: &mut Reserves, hit: Hit) {
    let shield = &mut reserves.shield;
    if hit.passes_shields {
        reserves.armor.now -= hit.mass;
    } else {
        if hit.energy > 0.0 {
            shield.now -= hit.energy;
        }
        if shield.now <= 0.0 {
            reserves.armor.now -= hit.mass;
        }
    }
    shield.now = shield.now.max(-SHIELD_FLOOR * shield.max);
}

/// Where a blast goes off and whose it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    /// Its centre.
    pub at: Vec2,
    /// How far it reaches on each side, in pixels.
    pub radius: f32,
    /// The ship hit directly, which it leaves alone.
    pub direct: Option<ShipRef>,
    /// The ship that fired it.
    pub firer: ShipRef,
    /// Whether it spares the player who fired it.
    pub spares_player: bool,
}

impl Blast {
    /// Which of `ships`, each at its position, the blast reaches (see the
    /// module docs); none without a radius.
    #[must_use]
    pub fn reaches(&self, ships: impl IntoIterator<Item = (ShipRef, Vec2)>) -> Vec<ShipRef> {
        if self.radius <= 0.0 {
            return Vec::new();
        }
        ships
            .into_iter()
            .filter(|&(ship, at)| {
                let off = at - self.at;
                off.x.abs() <= self.radius
                    && off.y.abs() <= self.radius
                    && Some(ship) != self.direct
                    && self.may_hurt(ship)
            })
            .map(|(ship, _)| ship)
            .collect()
    }

    /// Whether it may hurt `ship`, as far as who fired it goes.
    fn may_hurt(&self, ship: ShipRef) -> bool {
        ship != self.firer || (ship == ShipRef::Player && !self.spares_player)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::reserves::Gauge;
    use crate::traffic::npc::NpcId;

    /// 30 of 30 shields and 45 of 45 armour, with fuel.
    fn full() -> Reserves {
        Reserves::full(30.0, 45.0, 300.0)
    }

    fn hit(mass: f32, energy: f32) -> Hit {
        Hit {
            mass,
            energy,
            passes_shields: false,
        }
    }

    fn after(mut reserves: Reserves, hits: &[Hit]) -> (f32, f32) {
        for &hit in hits {
            apply(&mut reserves, hit);
        }
        (reserves.shield.now, reserves.armor.now)
    }

    #[test]
    fn energy_damage_takes_the_shields_and_mass_damage_is_held_off_while_they_hold() {
        assert_eq!(after(full(), &[hit(5.0, 10.0)]), (20.0, 45.0));
        assert_eq!(after(full(), &[hit(5.0, 29.0)]), (1.0, 45.0));
        assert_eq!(after(full(), &[hit(50.0, 0.0)]), (30.0, 45.0), "mass alone");
    }

    #[test]
    fn once_the_shields_are_down_mass_damage_takes_the_armour() {
        assert_eq!(after(full(), &[hit(5.0, 30.0)]), (0.0, 40.0), "at none");
        assert_eq!(after(full(), &[hit(5.0, 32.0)]), (-2.0, 40.0));
        assert_eq!(
            after(full(), &[hit(5.0, 30.0), hit(5.0, 0.0), hit(5.0, 0.0)]),
            (0.0, 30.0)
        );
    }

    #[test]
    fn energy_keeps_draining_a_downed_shield_to_a_tenth_below_none() {
        assert_eq!(
            after(full(), &[hit(1.0, 30.0), hit(1.0, 2.0)]),
            (-2.0, 43.0)
        );
        assert_eq!(after(full(), &[hit(1.0, 100.0)]), (-3.0, 44.0), "floored");
        assert_eq!(
            after(full(), &[hit(1.0, 100.0), hit(1.0, 100.0)]),
            (-3.0, 43.0)
        );
        assert_eq!(SHIELD_FLOOR, 0.1);
        let shieldless = Reserves::full(0.0, 10.0, 0.0);
        assert_eq!(after(shieldless, &[hit(4.0, 9.0)]), (0.0, 6.0));
    }

    #[test]
    fn a_shield_passing_hit_takes_only_the_armour_and_ignores_its_energy() {
        let through = Hit {
            mass: 12.0,
            energy: 12.0,
            passes_shields: true,
        };
        assert_eq!(after(full(), &[through]), (30.0, 33.0));
        let mut down = full();
        down.shield.now = -1.0;
        assert_eq!(after(down, &[through]), (-1.0, 33.0));
    }

    #[test]
    fn no_damage_leaves_the_gauges_alone() {
        assert_eq!(after(full(), &[hit(0.0, 0.0)]), (30.0, 45.0));
        let mut down = full();
        down.shield.now = -1.0;
        down.armor.now = 7.0;
        assert_eq!(after(down, &[hit(0.0, 0.0)]), (-1.0, 7.0));
        assert_eq!(
            after(down, &[hit(0.0, -5.0)]),
            (-1.0, 7.0),
            "negative energy"
        );
        let mut fuel = full();
        apply(&mut fuel, hit(9.0, 99.0));
        assert_eq!(fuel.fuel, Gauge::full(300.0), "fuel untouched");
    }

    #[test]
    fn a_hit_is_its_weapons_damage() {
        let weapon = WeaponSpec::new(&crate::catalog::WeaponRecord {
            mass_dmg: 12,
            energy_dmg: 8,
            flags: 0x0020,
            ..crate::testkit::weapon(232)
        });
        assert_eq!(
            Hit::of(&weapon),
            Hit {
                mass: 12.0,
                energy: 8.0,
                passes_shields: true
            }
        );
    }

    const A: ShipRef = ShipRef::Npc(NpcId(1));
    const B: ShipRef = ShipRef::Npc(NpcId(2));
    const C: ShipRef = ShipRef::Npc(NpcId(3));

    fn blast(firer: ShipRef, direct: Option<ShipRef>, spares_player: bool) -> Blast {
        Blast {
            at: Vec2::new(100.0, 100.0),
            radius: 10.0,
            direct,
            firer,
            spares_player,
        }
    }

    #[test]
    fn a_blast_reaches_the_square_around_it_edges_included() {
        let ships = [
            (A, Vec2::new(110.0, 90.0)),
            (B, Vec2::new(110.5, 100.0)),
            (C, Vec2::new(100.0, 89.0)),
            (ShipRef::Player, Vec2::new(91.0, 109.0)),
        ];
        assert_eq!(
            blast(C, None, false).reaches(ships),
            [A, ShipRef::Player],
            "B and C beyond the edge; the firer C skipped anyway"
        );
        let corner = [(A, Vec2::new(90.0, 110.0)), (B, Vec2::new(100.0, 110.01))];
        assert_eq!(blast(C, None, false).reaches(corner), [A]);
        let none = Blast {
            radius: 0.0,
            ..blast(C, None, false)
        };
        assert_eq!(none.reaches([(A, Vec2::new(100.0, 100.0))]), []);
    }

    #[test]
    fn a_blast_skips_the_ship_hit_directly_and_an_npc_firer() {
        let ships = [
            (A, Vec2::new(100.0, 100.0)),
            (B, Vec2::new(101.0, 100.0)),
            (C, Vec2::new(99.0, 100.0)),
        ];
        assert_eq!(blast(A, Some(B), false).reaches(ships), [C]);
        assert_eq!(blast(A, Some(B), true).reaches(ships), [C]);
    }

    #[test]
    fn a_players_blast_hurts_the_player_unless_it_spares_the_player() {
        let ships = [
            (ShipRef::Player, Vec2::new(100.0, 100.0)),
            (A, Vec2::new(100.0, 100.0)),
        ];
        assert_eq!(
            blast(ShipRef::Player, None, false).reaches(ships),
            [ShipRef::Player, A]
        );
        assert_eq!(blast(ShipRef::Player, None, true).reaches(ships), [A]);
        assert_eq!(
            blast(A, None, true).reaches(ships),
            [ShipRef::Player],
            "an NPC's blast that spares the player still hurts the player"
        );
        assert_eq!(blast(A, Some(ShipRef::Player), false).reaches(ships), []);
    }
}
