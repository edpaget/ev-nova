//! The sprite layer a shot is in, as `_SpawnShot` puts it
//! (@0x3f0a9-0x3f143 in the `EV Nova` executable).
//!
//! The layer decides two things: which collision passes test the shot
//! (`_DoPlayGameWork` @0x43e75-0x44015; see [`Combat`](super::Combat)'s
//! hit pass and [`Shot::touched`](super::projectile::Shot::touched)), and
//! where it is drawn among the ships (`_SetupSpriteWorld`
//! @0x21d0c-0x21e24; see [`sprite_layer`](crate::sprite_layer)).
//!
//! `_SpawnShot` picks it by the weapon's guidance ([`ShotLayer::spawned`]):
//!
//! - homing (1) goes in 0x3b531c, [`ShotLayer::Homing`];
//! - point defence (9) in 0x3b53bc, [`ShotLayer::PointDefence`];
//! - a turret (4) fired by a ship (a source slot of 0x3f or less, `-0x44`)
//!   whose class has `shïp` `Flags3` 0x0040 (class +0x9ec, filled from the
//!   `shïp` @0x7a204) in 0x3b5308, [`ShotLayer::OverShips`], over every
//!   ship (the Bible: "Ship's turreted shots appear above the ship instead
//!   of below");
//! - everything else, a turret from a non-ship source or a ship without
//!   the bit included, in 0x3b52f0, [`ShotLayer::Shots`], under most
//!   ships.
//!
//! A sub-munition is spawned through `_SpawnShot` too
//! (`_SpawnShotSubmunitions` @0x3f328), so it takes its layer by its own
//! guidance the same way. `_HandleShipPointDefense` moves a point-defence
//! shot fired at a ship to 0x3b52f0 (@0x398a7-0x398c2), where nothing
//! moves it back: after `_SpawnShot` put it in 0x3b53bc, so it always
//! enters 0x3b52f0 at its tail ([`shot_slots`](super::shot_slots)). No
//! layer choice plays a sound.

use super::weapon::Guidance;

/// The sprite layer a shot's sprite is in (see the module docs). The
/// layers compare in the order they are drawn, bottom first
/// ([`SpriteLayer`](crate::sprite_layer::SpriteLayer)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShotLayer {
    /// Point defence's own layer 0x3b53bc, which only the missile pass
    /// walks (`_DoPlayGameWork` @0x43fff-0x44010) and no ship's contact
    /// pass acts on (@0x43fdd-0x43ffa, @0x36fbd-0x36fd6): a point-defence
    /// shot (9), until `_HandleShipPointDefense` moves one fired at a ship
    /// to [`ShotLayer::Shots`] (@0x398a7-0x398c2).
    PointDefence,
    /// The homing layer 0x3b531c: a homing shot (1), the only shots point
    /// defence's layer meets (@0x43fff-0x44010).
    Homing,
    /// The shot layer 0x3b52f0, under most ships: every other shot.
    #[default]
    Shots,
    /// The layer 0x3b5308, over every ship: a turret (4) a ship of `shïp`
    /// `Flags3` 0x0040 fired.
    OverShips,
}

impl ShotLayer {
    /// The layer `_SpawnShot` puts a shot of `guidance` in, fired by a
    /// ship (`ship_source`) whose class has `Flags3` 0x0040
    /// (`turret_over`) or not (@0x3f0a9-0x3f143; see the module docs).
    #[must_use]
    pub fn spawned(guidance: Guidance, ship_source: bool, turret_over: bool) -> Self {
        match guidance {
            Guidance::Homing => Self::Homing,
            Guidance::PointDefence => Self::PointDefence,
            Guidance::Turret if ship_source && turret_over => Self::OverShips,
            _ => Self::Shots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homing_and_point_defence_shots_take_their_own_layers() {
        for (ship_source, turret_over) in [(true, true), (false, false)] {
            assert_eq!(
                ShotLayer::spawned(Guidance::Homing, ship_source, turret_over),
                ShotLayer::Homing
            );
            assert_eq!(
                ShotLayer::spawned(Guidance::PointDefence, ship_source, turret_over),
                ShotLayer::PointDefence
            );
        }
    }

    #[test]
    fn a_ships_turret_of_flags3_0x0040_goes_over_the_ships() {
        assert_eq!(
            ShotLayer::spawned(Guidance::Turret, true, true),
            ShotLayer::OverShips
        );
        assert_eq!(
            ShotLayer::spawned(Guidance::Turret, true, false),
            ShotLayer::Shots,
            "a ship without the bit"
        );
        assert_eq!(
            ShotLayer::spawned(Guidance::Turret, false, true),
            ShotLayer::Shots,
            "no ship's"
        );
    }

    #[test]
    fn every_other_guidance_takes_the_shot_layer() {
        for guidance in [
            Guidance::Unguided,
            Guidance::FreefallBomb,
            Guidance::Rocket,
            Guidance::FrontTurret,
            Guidance::RearTurret,
            Guidance::Other(2),
        ] {
            assert_eq!(
                ShotLayer::spawned(guidance, true, true),
                ShotLayer::Shots,
                "{guidance:?}"
            );
        }
        assert_eq!(ShotLayer::default(), ShotLayer::Shots);
    }

    #[test]
    fn the_shot_layers_compare_as_they_are_drawn() {
        use crate::sprite_layer::SpriteLayer;
        let layers = [
            ShotLayer::OverShips,
            ShotLayer::Homing,
            ShotLayer::Shots,
            ShotLayer::PointDefence,
        ];
        let mut sorted = layers;
        sorted.sort();
        let mut drawn = layers;
        drawn.sort_by_key(|&layer| SpriteLayer::from(layer));
        assert_eq!(sorted, drawn);
    }
}
