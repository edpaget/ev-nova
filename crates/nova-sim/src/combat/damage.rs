//! Damage: what a hit does to a ship's reserves, and which ships a blast
//! reaches.
//!
//! The values are the original's (`_DamageShip` and `_HandleShipHit` in
//! the `EV Nova` executable):
//!
//! - A weapon that passes through shields takes its mass damage off the
//!   armour (@0x3ab8c-0x3abfb), ignoring its energy damage and leaving
//!   the shields as they are.
//! - Any other takes its energy damage off the shields, then, only if
//!   they are then down (at or below none), its mass damage off the
//!   armour (@0x3ac00-0x3ac81). While the shields hold, mass damage never
//!   reaches the armour, and energy damage keeps draining a shield that
//!   is down. A shield it leaves down is kept above [`SHIELD_FLOOR`] of
//!   what it holds below none (@0x3ac86-0x3acbd), so a shield driven
//!   negative must first recharge up through none.
//! - Damage at or below none is skipped, on either path (@0x3ab9b,
//!   @0x3ac08, @0x3ac38): it never heals.
//! - A ship type that holds no armour (stock `shïp` 895, plug-in or test
//!   data) has none to lose: its armour is never taken below none.
//! - A blast reaches every ship within the square `radius` on each side of
//!   it, but the one hit directly and the firer, unless the firer is the
//!   player and the weapon's blast does not spare the player.
//!
//! **Spared hits** ([`take`]). `_DamageShip`'s arguments +0x28 and +0x2c
//! ([`Hit::cant_kill`], [`Hit::cant_disable`]) each keep a hit, on its
//! own, from destroying or newly disabling its ship, on either path
//! (shields first, or armour-direct):
//!
//! - **It never destroys** (+0x28: @0x3ac3a-0x3ac64 after the shield
//!   path, @0x3abd6 on the armour-direct one): armour above none that the
//!   hit takes to none or below is left at [`SPARED_ARMOR`]; armour
//!   already at none or below takes it unfloored. A disable-only weapon's
//!   shots, blasts and beams carry it (`Flags2` 0x1000), and the
//!   shockwave and sabotage bombs.
//! - **It never newly disables** (+0x2c, @0x3acc2-0x3ad4a): a ship not
//!   disabled before the hit (`_IsDisabled` @0x3a9ad) that is after it
//!   has its armour set to what a self-repair leaves
//!   ([`repaired_armor`], the same 0.3333, 0.1 and 1.0). `_IsDisabled`
//!   reads armour at none or below as disabled, so a killing blow is
//!   reset too. That clears the disabled line only while the ship holds
//!   33,333 armour or less: a non-`TOUGH` hull that holds more is left
//!   disabled, and counts as newly disabled. The shockwave and sabotage
//!   bombs carry it.
//!
//! The caller says whether the ship was disabled before the hit: a
//! shockwave reads it from the ship's condition, a sabotage bomb from its
//! armour, and a weapon's hit from both (see
//! [`Combat`](super::Combat)'s steps 3 and 8).

use super::ShipRef;
use super::hull::{DisableRule, HullSpec};
use super::weapon::WeaponSpec;
use crate::geometry::Vec2;
use crate::reserves::{Gauge, Reserves};
use crate::self_repair::repaired_armor;

/// The armour a spared hit leaves a ship it would destroy (0x3f800000
/// @0x3ac64, @0x3abd6).
pub const SPARED_ARMOR: f32 = 1.0;

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
    /// Whether it never destroys its ship (`_DamageShip`'s arg +0x28): a
    /// disable-only weapon's (see the module docs' **Spared hits**).
    pub cant_kill: bool,
    /// Whether it never newly disables its ship (`_DamageShip`'s arg
    /// +0x2c).
    pub cant_disable: bool,
}

impl Hit {
    /// A full hit from `weapon`, which may kill and disable: whether it
    /// is disable-only is its shot's or beam's to say
    /// ([`Shot::disable_only`](super::projectile::Shot::disable_only)).
    #[must_use]
    pub fn of(weapon: &WeaponSpec) -> Self {
        Self {
            mass: weapon.mass_damage,
            energy: weapon.energy_damage,
            passes_shields: weapon.passes_shields(),
            ..Self::default()
        }
    }
}

impl Hit {
    /// The hit with `steps` of decay taken off its mass and its energy,
    /// each floored at none, as `_HandleShipHit` takes a shot's steps off
    /// its direct hit (@0x36add-0x36afd); with no steps, the hit as it is.
    #[must_use]
    pub fn decayed(self, steps: u32) -> Self {
        if steps == 0 {
            return self;
        }
        // Steps are ticks of a shot's life, far within an `f32`'s whole
        // numbers.
        #[allow(clippy::cast_precision_loss)]
        let steps = steps as f32;
        Self {
            mass: (self.mass - steps).max(0.0),
            energy: (self.energy - steps).max(0.0),
            ..self
        }
    }
}

/// Takes `hit` off `reserves` with neither of its floors (see the module
/// docs): what [`take`] does to a hit that may kill and disable.
pub fn apply(reserves: &mut Reserves, hit: Hit) {
    let shield = &mut reserves.shield;
    if hit.passes_shields {
        if hit.mass > 0.0 {
            reserves.armor.now -= hit.mass;
        }
    } else {
        if hit.energy > 0.0 {
            shield.now -= hit.energy;
        }
        if shield.now <= 0.0 {
            if hit.mass > 0.0 {
                reserves.armor.now -= hit.mass;
            }
            shield.now = shield.now.max(-SHIELD_FLOOR * shield.max);
        }
    }
    if reserves.armor.max <= 0.0 {
        reserves.armor.now = reserves.armor.now.max(0.0);
    }
}

/// Takes `hit` off `reserves` as [`apply`] does, with each floor it
/// carries (see the module docs' **Spared hits**): it is on the ship of
/// `hull`, which `was_disabled` before it or not, and `rule` says when a
/// ship is disabled.
pub fn take(
    reserves: &mut Reserves,
    hit: Hit,
    hull: &HullSpec,
    rule: &dyn DisableRule,
    was_disabled: bool,
) {
    let whole = reserves.armor.now > 0.0;
    apply(reserves, hit);
    let armor = &mut reserves.armor;
    if hit.cant_kill && whole && armor.now <= 0.0 {
        armor.now = SPARED_ARMOR;
    }
    if hit.cant_disable && !was_disabled && disabled(*armor, hull, rule) {
        armor.now = repaired_armor(armor.max, hull.tough);
    }
}

/// Whether a ship of `hull` with `armor` is disabled, as `rule` says: never
/// one that holds no armour.
pub(crate) fn disabled(armor: Gauge, hull: &HullSpec, rule: &dyn DisableRule) -> bool {
    armor.max > 0.0 && rule.disabled(armor, hull)
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
    /// The ship that fired it, which it spares; none for a shot no ship
    /// launched, whose blast spares no one (the blast pass skips the
    /// source slot, @0x35fd5-0x36003, and -1 is none).
    pub firer: Option<ShipRef>,
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
        Some(ship) != self.firer || (ship == ShipRef::Player && !self.spares_player)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::hull::NovaDisable;
    use crate::traffic::npc::NpcId;

    /// What an armour-direct spared hit of `mass` leaves of 50 of 100
    /// shields and `armor` of 300 armour, on a hull `tough` or not that
    /// `was_disabled` or not.
    fn spared(armor: f32, mass: f32, tough: bool, was_disabled: bool) -> (f32, f32) {
        let mut reserves = Reserves {
            shield: Gauge {
                now: 50.0,
                max: 100.0,
            },
            armor: Gauge {
                now: armor,
                max: 300.0,
            },
            fuel: Gauge::full(100.0),
        };
        let hull = HullSpec {
            tough,
            ..HullSpec::default()
        };
        let hit = Hit {
            mass,
            energy: 0.0,
            passes_shields: true,
            cant_kill: true,
            cant_disable: true,
        };
        take(&mut reserves, hit, &hull, &NovaDisable, was_disabled);
        (reserves.shield.now, reserves.armor.now)
    }

    #[test]
    fn a_decayed_hit_loses_its_steps_off_mass_and_energy_each_floored_at_none() {
        let hit = Hit {
            mass: 10.0,
            energy: 3.0,
            passes_shields: true,
            cant_kill: true,
            cant_disable: false,
        };
        assert_eq!(
            hit.decayed(4),
            Hit {
                mass: 6.0,
                energy: 0.0,
                ..hit
            }
        );
        assert_eq!(hit.decayed(0), hit);
        let negative = Hit {
            mass: -2.0,
            energy: -1.0,
            ..hit
        };
        assert_eq!(negative.decayed(0), negative, "no step, no floor");
        assert_eq!(
            negative.decayed(1),
            Hit {
                mass: 0.0,
                energy: 0.0,
                ..hit
            },
            "a step floors even what it did not take"
        );
    }

    #[test]
    fn an_armour_direct_spared_hit_leaves_shields_alone_and_armour_at_the_repair_share() {
        let (shield, armor) = spared(300.0, 450.0, false, false);
        assert_eq!(shield, 50.0);
        assert!((armor - 100.99).abs() < 1e-4, "300 x 0.3333 + 1: {armor}");
        assert_eq!(
            spared(300.0, 100.0, false, false),
            (50.0, 200.0),
            "a hit that leaves it intact is taken whole"
        );
    }

    #[test]
    fn an_armour_direct_spared_hit_on_a_tough_hull_ends_at_31() {
        assert_eq!(spared(300.0, 300.0, true, false), (50.0, 31.0));
    }

    #[test]
    fn an_armour_direct_spared_hit_on_an_already_disabled_ship_ends_at_1() {
        assert_eq!(spared(90.0, 300.0, false, true), (50.0, SPARED_ARMOR));
        assert_eq!(
            spared(90.0, 30.0, false, true),
            (50.0, 60.0),
            "a disabled ship stays as the hit leaves it"
        );
        assert_eq!(
            spared(-5.0, 30.0, false, true),
            (50.0, -35.0),
            "a dying ship takes it unfloored"
        );
        assert_eq!(SPARED_ARMOR, 1.0);
    }

    /// What `hit` leaves of 50 of 100 shields and `armor` of 300 armour,
    /// taken by [`take`] on a hull `tough` or not that `was_disabled` or
    /// not.
    fn taken(armor: f32, hit: Hit, tough: bool, was_disabled: bool) -> (f32, f32) {
        let mut reserves = Reserves {
            shield: Gauge {
                now: 50.0,
                max: 100.0,
            },
            armor: Gauge {
                now: armor,
                max: 300.0,
            },
            fuel: Gauge::full(100.0),
        };
        let hull = HullSpec {
            tough,
            ..HullSpec::default()
        };
        take(&mut reserves, hit, &hull, &NovaDisable, was_disabled);
        (reserves.shield.now, reserves.armor.now)
    }

    /// A hit of `mass` armour damage and 60 shield damage, through the
    /// shields or not, that can't kill.
    fn cant_kill(mass: f32, passes_shields: bool) -> Hit {
        Hit {
            mass,
            energy: 60.0,
            passes_shields,
            cant_kill: true,
            cant_disable: false,
        }
    }

    #[test]
    fn a_cant_kill_hit_leaves_armour_it_would_take_to_none_at_1() {
        assert_eq!(
            taken(100.0, cant_kill(450.0, true), false, true),
            (50.0, SPARED_ARMOR),
            "armour-direct"
        );
        assert_eq!(
            taken(100.0, cant_kill(100.0, false), false, false),
            (-10.0, SPARED_ARMOR),
            "shields first, to exactly none, and not reset to the repair share"
        );
        assert_eq!(
            taken(100.0, cant_kill(30.0, true), false, true),
            (50.0, 70.0),
            "a survivable hit is taken whole"
        );
        assert_eq!(
            taken(-5.0, cant_kill(30.0, false), false, true),
            (-10.0, -35.0),
            "a ship already at none or below takes it unfloored"
        );
    }

    /// A hit of `mass` armour damage through the shields that can't
    /// disable.
    fn cant_disable(mass: f32) -> Hit {
        Hit {
            mass,
            energy: 0.0,
            passes_shields: true,
            cant_kill: false,
            cant_disable: true,
        }
    }

    #[test]
    fn a_cant_disable_hit_alone_sets_a_newly_disabled_intact_ship_back_to_the_repair_share() {
        let (_, armor) = taken(300.0, cant_disable(250.0), false, false);
        assert_eq!(armor, repaired_armor(300.0, false));
        let gauge = Gauge {
            now: armor,
            max: 300.0,
        };
        assert!(!NovaDisable.disabled(gauge, &HullSpec::default()));
        assert_eq!(taken(300.0, cant_disable(290.0), true, false), (50.0, 31.0));
        assert_eq!(
            taken(300.0, cant_disable(400.0), false, false).1,
            repaired_armor(300.0, false),
            "a killing blow disables too (`_IsDisabled` reads armour at none as disabled)"
        );
        assert_eq!(
            taken(90.0, cant_disable(30.0), false, true),
            (50.0, 60.0),
            "an already disabled ship is left as the hit leaves it"
        );
        assert_eq!(
            taken(90.0, cant_disable(100.0), false, true),
            (50.0, -10.0),
            "and may be killed"
        );
        assert_eq!(
            taken(300.0, cant_disable(100.0), false, false),
            (50.0, 200.0),
            "a hit that doesn't disable is taken whole"
        );
    }

    #[test]
    fn damage_at_or_below_none_is_skipped() {
        let heal = |passes_shields| Hit {
            mass: -20.0,
            energy: -20.0,
            passes_shields,
            ..Hit::default()
        };
        let mut down = full();
        down.shield.now = -1.0;
        down.armor.now = 7.0;
        assert_eq!(after(down, &[heal(false)]), (-1.0, 7.0), "shields first");
        assert_eq!(after(down, &[heal(true)]), (-1.0, 7.0), "armour-direct");
    }

    #[test]
    fn a_shield_passing_hit_leaves_a_shield_below_its_floor_as_it_is() {
        let mut sunk = full();
        sunk.shield.now = -9.0;
        let through = Hit {
            mass: 5.0,
            passes_shields: true,
            ..Hit::default()
        };
        assert_eq!(after(sunk, &[through]), (-9.0, 40.0));
        assert_eq!(
            after(sunk, &[hit(5.0, 0.0)]),
            (-3.0, 40.0),
            "the shield path floors it"
        );
    }

    /// 30 of 30 shields and 45 of 45 armour, with fuel.
    fn full() -> Reserves {
        Reserves::full(30.0, 45.0, 300.0)
    }

    fn hit(mass: f32, energy: f32) -> Hit {
        Hit {
            mass,
            energy,
            ..Hit::default()
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
            ..Hit::default()
        };
        assert_eq!(after(full(), &[through]), (30.0, 33.0));
        let mut down = full();
        down.shield.now = -1.0;
        assert_eq!(after(down, &[through]), (-1.0, 33.0));
    }

    #[test]
    fn a_ship_that_holds_no_armour_loses_none() {
        let mut armourless = Reserves::full(30.0, 0.0, 0.0);
        armourless.shield.now = -1.0;
        let through = Hit {
            passes_shields: true,
            ..hit(5.0, 0.0)
        };
        assert_eq!(after(armourless, &[hit(5.0, 2.0)]), (-3.0, 0.0));
        assert_eq!(after(armourless, &[through]), (-1.0, 0.0), "through");
        let mut spare = armourless;
        spare.armor.now = 2.0;
        assert_eq!(after(spare, &[hit(5.0, 0.0)]), (-1.0, 0.0), "to none");
        let mut sunk = armourless;
        sunk.armor.now = -2.0;
        assert_eq!(after(sunk, &[hit(5.0, 0.0)]), (-1.0, 0.0), "back to none");
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
                passes_shields: true,
                cant_kill: false,
                cant_disable: false,
            },
            "its weapon's disable-only is the shot's or beam's to pass"
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
            firer: Some(firer),
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
