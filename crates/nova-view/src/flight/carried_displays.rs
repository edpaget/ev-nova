//! A kept escort's weapon glow and shield flash, carried into its new NPC
//! through a jump's arrival, a gate's exit and a take-off.
//!
//! `_RespawnEscort` (@0x3d4b8-0x3d62c) writes neither the glow (+0xc8e8)
//! nor the flash (+0xc8f0), so an escort the fleet keeps goes on from
//! where its last flight display left both, and each `_PrepareAllSprites`
//! that displays its slot before the first frame steps them once more at
//! its gain. Every other ship's are let go: the old NPCs are gone, and
//! the new ones are zeroed by their spawns.
//!
//! The view lets go of every flash and glow as it lays out a system and as
//! the player lands, holding them here instead ([`HeldDisplays::hold`]).
//! Once the session places the fleet it says which new NPC each kept
//! escort is and the gain of each prepare that displayed it
//! ([`FlightEvent::DisplaysCarried`](nova_sim_frame::flight_frame::FlightEvent::DisplaysCarried)):
//! [`HeldDisplays::carry`] moves each one's flash and glow onto its new
//! NPC, steps them once for each gain, as a pass's display does
//! ([`ShieldFlash::step`], [`WeaponGlow::step`]), and lets go of the rest.
//! The counts and gains are the session's rules, and its tests pin them.

use std::collections::BTreeMap;

use nova_sim_frame::flight_frame::CarriedDisplay;
use nova_sim_frame::{ShipRef, SpeedMult};

use super::flash::{FlashLook, ShieldFlash};
use super::weapon_glow::{WeaponGlow, WeaponGlows};

/// The NPCs' flashes and glows let go of at a change of system or a
/// landing, held for the escorts the fleet keeps (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeldDisplays {
    /// Each NPC's shield flash.
    flashes: BTreeMap<ShipRef, ShieldFlash>,
    /// Each NPC's weapon glow.
    glows: BTreeMap<ShipRef, WeaponGlow>,
}

impl HeldDisplays {
    /// Holds every NPC's flash among `flashes` and glow among `glows`,
    /// leaving both empty: the player's are dropped, as every way into a
    /// system resets them.
    pub fn hold(&mut self, flashes: &mut BTreeMap<ShipRef, ShieldFlash>, glows: &mut WeaponGlows) {
        let npc = |ship: &ShipRef| matches!(ship, ShipRef::Npc(_));
        self.flashes.extend(
            std::mem::take(flashes)
                .into_iter()
                .filter(|(ship, _)| npc(ship)),
        );
        self.glows
            .extend(glows.take_all().into_iter().filter(|(ship, _)| npc(ship)));
    }

    /// Carries each kept escort's held flash and glow among `carried`
    /// into its new NPC, among `flashes` and `glows`, stepped once for
    /// each of its prepares' gains: the flash by its new ship's look,
    /// `look(ship)`, the glow by its class's decay, `decay(ship)`, a ship
    /// of neither being gone. A flash run out, or a glow not shown, is
    /// dropped, as a pass's step drops them. Then whatever else was held
    /// is let go.
    pub fn carry(
        &mut self,
        carried: &[CarriedDisplay],
        (flashes, glows): (&mut BTreeMap<ShipRef, ShieldFlash>, &mut WeaponGlows),
        look: impl Fn(ShipRef) -> Option<FlashLook>,
        decay: impl Fn(ShipRef) -> Option<f32>,
    ) {
        for carry in carried {
            let (from, to) = (ShipRef::Npc(carry.from), ShipRef::Npc(carry.to));
            if let (Some(mut flash), Some(look)) = (self.flashes.remove(&from), look(to)) {
                step_flash(&mut flash, look, &carry.prepares);
                if flash.shown() > 0.0 {
                    flashes.insert(to, flash);
                }
            }
            if let (Some(mut glow), Some(decay)) = (self.glows.remove(&from), decay(to)) {
                for &gain in &carry.prepares {
                    glow.step(decay, gain);
                }
                if glow.drawn().is_some() {
                    glows.insert(to, glow);
                }
            }
        }
        self.flashes.clear();
        self.glows.clear();
    }
}

/// Steps `flash` once for each of `gains`, by `look`.
fn step_flash(flash: &mut ShieldFlash, look: FlashLook, gains: &[SpeedMult]) {
    for &gain in gains {
        flash.step(look.shield, look.base, look.has_layer, gain);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim_frame::NpcId;

    use super::*;

    const OLD: ShipRef = ShipRef::Npc(NpcId(4));
    const NEW: ShipRef = ShipRef::Npc(NpcId(9));
    const HALF: f64 = 0.5;

    /// A full shield of 100 with a shield layer.
    const FULL: FlashLook = FlashLook {
        shield: 100.0,
        base: 100.0,
        has_layer: true,
    };

    /// A glow's decay a frame.
    const DECAY: f32 = 2.0;

    fn half() -> SpeedMult {
        SpeedMult::new(HALF)
    }

    /// A flash just hit, and a glow just lit, for `ship`.
    fn lit(ship: ShipRef) -> (BTreeMap<ShipRef, ShieldFlash>, WeaponGlows) {
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        let mut glows = WeaponGlows::default();
        // Lit after the step, as a late glow is: at 32, not yet shown.
        glows.step(&[(ship, true)], |_| Some(DECAY), SpeedMult::FULL);
        (BTreeMap::from([(ship, flash)]), glows)
    }

    fn carry_of(from: ShipRef, to: ShipRef, prepares: Vec<SpeedMult>) -> CarriedDisplay {
        let npc = |ship| match ship {
            ShipRef::Npc(id) => id,
            ShipRef::Player => unreachable!(),
        };
        CarriedDisplay {
            from: npc(from),
            to: npc(to),
            prepares,
        }
    }

    /// `held`'s displays carried by `carried`, every ship of [`FULL`]'s
    /// look and of [`DECAY`].
    fn carried(
        held: &mut HeldDisplays,
        carried: &[CarriedDisplay],
    ) -> (BTreeMap<ShipRef, ShieldFlash>, WeaponGlows) {
        let (mut flashes, mut glows) = (BTreeMap::new(), WeaponGlows::default());
        held.carry(
            carried,
            (&mut flashes, &mut glows),
            |_| Some(FULL),
            |_| Some(DECAY),
        );
        (flashes, glows)
    }

    #[test]
    fn holding_keeps_the_npcs_flashes_and_glows_and_drops_the_players() {
        let (mut flashes, mut glows) = lit(OLD);
        let (player_flashes, mut player_glows) = lit(ShipRef::Player);
        flashes.extend(player_flashes);
        glows.insert(
            ShipRef::Player,
            player_glows.take(ShipRef::Player).expect("lit"),
        );
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        assert!(flashes.is_empty(), "let go");
        assert_eq!(glows, WeaponGlows::default(), "let go");
        assert_eq!(held.flashes.keys().collect::<Vec<_>>(), [&OLD]);
        assert_eq!(held.glows.keys().collect::<Vec<_>>(), [&OLD]);
    }

    #[test]
    fn a_carry_moves_them_to_the_new_npc_stepped_once_per_gain() {
        let (mut flashes, mut glows) = lit(OLD);
        let (flash, mut glow) = (flashes[&OLD], glows.take(OLD).expect("lit"));
        glows.insert(OLD, glow);
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        let (flashes, mut glows) = carried(
            &mut held,
            &[carry_of(OLD, NEW, vec![half(), SpeedMult::FULL])],
        );
        let mut expected = flash;
        for gain in [half(), SpeedMult::FULL] {
            expected.step(100.0, 100.0, true, gain);
            glow.step(DECAY, gain);
        }
        assert_eq!(flashes, BTreeMap::from([(NEW, expected)]));
        assert_eq!(expected.shown(), 31.5, "32 less a half step");
        assert_eq!(glows.take(NEW), Some(glow));
        assert_eq!(glows, WeaponGlows::default(), "nothing under the old");
        assert_eq!(held, HeldDisplays::default(), "let go");
    }

    #[test]
    fn a_carry_at_gain_0_leaves_them_where_they_were_but_shown() {
        let (mut flashes, mut glows) = lit(OLD);
        let level = glows.take(OLD).expect("lit");
        glows.insert(OLD, level);
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        let gains = vec![SpeedMult::ZERO, SpeedMult::ZERO];
        let (flashes, mut glows) = carried(&mut held, &[carry_of(OLD, NEW, gains)]);
        assert_eq!(flashes[&NEW].shown(), 32.0);
        assert_eq!(flashes[&NEW].drawn(), Some(32));
        let carried = glows.take(NEW).expect("carried");
        assert_eq!(level.drawn(), None, "held unshown");
        assert_eq!(carried.drawn(), Some(32), "shown, by each prepare");
        assert_eq!(carried.shown(), Some(32), "and not fallen");
    }

    #[test]
    fn a_carry_holds_the_flash_to_the_new_ships_shield() {
        let (mut flashes, mut glows) = lit(OLD);
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        let (mut flashes, mut glows) = (BTreeMap::new(), WeaponGlows::default());
        let half_shield = FlashLook {
            shield: 50.0,
            ..FULL
        };
        held.carry(
            &[carry_of(OLD, NEW, vec![SpeedMult::ZERO])],
            (&mut flashes, &mut glows),
            |_| Some(half_shield),
            |_| Some(DECAY),
        );
        assert_eq!(flashes[&NEW].shown(), 16.0, "held to the shield's share");
    }

    #[test]
    fn a_flash_run_out_or_a_glow_gone_out_is_not_carried() {
        let (mut flashes, mut glows) = lit(OLD);
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        // The first shows them, falling past none; the second finds them
        // out.
        let gains = vec![SpeedMult::new(40.0), SpeedMult::FULL];
        let (flashes, glows) = carried(&mut held, &[carry_of(OLD, NEW, gains)]);
        assert!(flashes.is_empty());
        assert_eq!(glows, WeaponGlows::default());
    }

    #[test]
    fn a_flash_shown_at_none_is_not_carried() {
        let (mut flashes, mut glows) = lit(OLD);
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        // Shown at 32, then 1, then at none, as its timer reaches 0.
        let gains = vec![SpeedMult::new(31.0), SpeedMult::FULL, SpeedMult::FULL];
        let (flashes, _) = carried(&mut held, &[carry_of(OLD, NEW, gains)]);
        assert!(flashes.is_empty());
    }

    #[test]
    fn a_new_ship_not_in_the_frame_takes_nothing() {
        let (mut flashes, mut glows) = lit(OLD);
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        let (mut flashes, mut glows) = (BTreeMap::new(), WeaponGlows::default());
        held.carry(
            &[carry_of(OLD, NEW, vec![SpeedMult::FULL])],
            (&mut flashes, &mut glows),
            |_| None,
            |_| None,
        );
        assert!(flashes.is_empty());
        assert_eq!(glows, WeaponGlows::default());
        assert_eq!(held, HeldDisplays::default(), "let go");
    }

    #[test]
    fn what_no_carry_names_is_let_go() {
        let (mut flashes, mut glows) = lit(OLD);
        let mut held = HeldDisplays::default();
        held.hold(&mut flashes, &mut glows);
        let (flashes, glows) = carried(&mut held, &[]);
        assert!(flashes.is_empty());
        assert_eq!(glows, WeaponGlows::default());
        assert_eq!(held, HeldDisplays::default());
    }
}
