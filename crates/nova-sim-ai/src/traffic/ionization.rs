//! An NPC's ionization (see the [`ionization`](crate::ionization)
//! module): what `_HandleShip` does with its ion charge each frame, and
//! the slowed turn and thrust `_ShipTurnRate` and `_ShipAccelRate` give it.
//!
//! **The capacity and rate** ([`Npc::ion_factor`]): its class's
//! `IonizeMax` and deionization rate alone. `_ShipIonizationFactor` and
//! `_ShipDeionizationRate` count the Ion Absorbers and Dissipaters for the
//! player only (+0x86 == 0, @0x3260, @0x6ee9), so an NPC's default items
//! never count (`nova_sim::stats::IonStats`).
//!
//! **The turn and thrust** ([`Npc::ionized_handling`],
//! [`Npc::flying_handling`], [`Npc::turn_rate`]): every reader of an NPC's
//! `_ShipTurnRate` and `_ShipAccelRate` reads them slowed by the charge
//! (`_HandleShip`'s four turn reads and its glow's thrust @0x34f57,
//! `_LowLevelAIHandler`'s, `_HighLevelAIHandler`'s @0x8d955/@0x8d979,
//! `_SuitableMissileType` on the target @0x34f2, the inertialess
//! adjustment @0x334e1 and the escape pod's @0x8fb6d): the flight, the
//! AI's facing and reach tests, the warp-out's heading test, the glow's
//! rush test, the fire window and a missile's fit to its target. The turn
//! is spared while its warp stage runs (+0x50 above 0,
//! [`Warp::under_way`](crate::traffic::warp_out::Warp::under_way)).
//!
//! **The frame's block** ([`deionize`], `_HandleShip` @0x342b2-0x34421):
//! the original runs it after the move (@0x3426e-0x342aa) and before the
//! turn, for every ship `_HandleShip` reaches (only its class checks
//! @0x335a1/@0x335c8 return before it): disabled, warping, waiting and
//! immobile ships all shed. The engine runs it at the start of each NPC's
//! flight, before the next move, which makes the same sequence of moves
//! and pulls; the charge it leaves is what the turn and thrust that follow
//! read, as in the original. The pull's cap is the NPC's own
//! `_ShipMaxSpeed`. None of this plays a sound.

use crate::flight::{FlightRules, Flyer};
use crate::handling::Handling;
use crate::ionization::{Ionization, factor, lowered_top, pull};
use crate::traffic::npc::Npc;

impl Npc {
    /// Its ionization factor, `_ShipIonizationFactor`: its charge over its
    /// class's `IonizeMax` ([`factor`]).
    #[must_use]
    pub fn ion_factor(&self) -> f32 {
        factor(self.ion.charge, self.stats.ion.capacity(Flyer::Npc))
    }

    /// What its handling reads of its ionization now: its charge, its
    /// factor, and whether its warp stage runs.
    #[must_use]
    pub fn ionization(&self) -> Ionization {
        Ionization {
            charge: self.ion.charge,
            factor: self.ion_factor(),
            jumping: self.warp.under_way(),
        }
    }

    /// Its own handling, its turn and thrust slowed by its charge
    /// ([`Ionization::slows`]): its `_ShipTurnRate` and `_ShipAccelRate`.
    #[must_use]
    pub fn ionized_handling(&self) -> Handling {
        self.ionization().slows(self.stats.handling)
    }

    /// Its turn rate, `_ShipTurnRate`, slowed by its charge.
    #[must_use]
    pub fn turn_rate(&self) -> f32 {
        self.ionized_handling().turn_rate
    }

    /// How it flies this tick: its handling at top speed `own`, its thrust
    /// under `flight`, its turn and thrust slowed by its charge.
    #[must_use]
    pub fn flying_handling(&self, own: f32, flight: FlightRules) -> Handling {
        Handling {
            max_speed: own,
            ..self
                .ionization()
                .slows(flight.handling(self.stats.handling))
        }
    }
}

/// `npc`'s frame's ion block, at top speed `top_speed` (see the module
/// docs): a charge above none sheds its class's rate, unfloored, and its
/// velocity is pulled toward the top speed lowered by the charge left;
/// one at or below none is cleared.
pub fn deionize(npc: &mut Npc, top_speed: f32) {
    let rate = npc.stats.ion.rate(Flyer::Npc);
    let Some(charge) = crate::ionization::deionize(npc.ion.charge, rate) else {
        npc.ion.charge = 0.0;
        return;
    };
    npc.ion.charge = charge;
    let cap = lowered_top(top_speed, npc.ion_factor());
    npc.state.velocity = pull(npc.state.velocity, cap);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::hull::Condition;
    use crate::fuel::OutfitMod;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::stats::{ION_ABSORBER, ION_DISSIPATER, ShipStats};
    use crate::traffic::fixture::npc;
    use nova_sim_base::fixture::FAST;

    /// `FAST` of `IonizeMax` 400 and `Deionize` 50, carrying `outfits`.
    fn stats(outfits: &[OutfitMod]) -> ShipStats {
        ShipStats::new(
            ShipFields {
                ionize_max: 400,
                deionize: 50,
                ..FAST
            },
            outfits,
        )
    }

    fn charged(charge: f32) -> Npc {
        let mut ship = npc(1, stats(&[]));
        ship.ion.charge = charge;
        ship
    }

    #[test]
    fn an_ionized_npc_sheds_its_class_rate_and_dips_below_none_for_a_tick() {
        let mut ship = charged(1.0);
        deionize(&mut ship, 6.0);
        assert_eq!(ship.ion.charge, 0.5);
        deionize(&mut ship, 6.0);
        assert_eq!(ship.ion.charge, 0.0);
        ship.ion.charge = 0.25;
        deionize(&mut ship, 6.0);
        assert_eq!(ship.ion.charge, -0.25);
        deionize(&mut ship, 6.0);
        assert_eq!(ship.ion.charge, 0.0);
    }

    #[test]
    fn a_disabled_or_warping_npc_sheds_too() {
        let mut disabled = charged(10.0);
        disabled.condition = Condition::Disabled;
        deionize(&mut disabled, 6.0);
        assert_eq!(disabled.ion.charge, 9.5);
        let mut warping = charged(10.0);
        warping.warp.stage = 3.0;
        deionize(&mut warping, 6.0);
        assert_eq!(warping.ion.charge, 9.5);
    }

    #[test]
    fn an_npc_past_its_lowered_top_is_pulled_back_on_each_axis() {
        let mut ship = charged(200.5);
        ship.state.velocity = Vec2::new(6.0, -6.0);
        deionize(&mut ship, 6.0);
        let pulled = |v: f64| (v - 0.025) as f32;
        assert_eq!(ship.state.velocity, Vec2::new(pulled(6.0), -pulled(6.0)));
        let mut slow = charged(200.5);
        slow.state.velocity = Vec2::new(1.0, -1.0);
        deionize(&mut slow, 6.0);
        assert_eq!(slow.state.velocity, Vec2::new(1.0, -1.0), "within 3");
        let mut clear = charged(0.0);
        clear.state.velocity = Vec2::new(6.0, -6.0);
        deionize(&mut clear, 6.0);
        assert_eq!(clear.state.velocity, Vec2::new(6.0, -6.0), "no charge");
    }

    #[test]
    fn an_ionized_npc_turns_and_thrusts_slower_but_turns_freely_warping() {
        let mut ship = charged(200.0);
        let own = ship.stats.handling;
        let slowed = ship.ionized_handling();
        assert_eq!(slowed.turn_rate, own.turn_rate * 0.5);
        assert_eq!(slowed.accel, (f64::from(own.accel) * 0.5) as f32);
        assert_eq!(ship.turn_rate(), slowed.turn_rate);
        let flying = ship.flying_handling(7.5, FlightRules::default());
        assert_eq!(
            flying,
            Handling {
                max_speed: 7.5,
                ..slowed
            }
        );
        ship.warp.stage = 1.0;
        assert_eq!(ship.turn_rate(), own.turn_rate);
        assert_eq!(ship.ionized_handling().accel, slowed.accel);
        ship.warp.stage = 0.0;
        ship.ion.charge = 0.0;
        assert_eq!(ship.ionized_handling(), own);
    }

    #[test]
    fn an_ionized_npcs_fire_window_narrows_with_its_turn() {
        // 3° a tick: 10° off is within 4 turns, but not within 4 of 1.5°.
        let plain = charged(0.0);
        assert!(crate::ai::fire::faces(&plain, 10, 4.0));
        let ionized = charged(200.0);
        assert!(!crate::ai::fire::faces(&ionized, 10, 4.0));
    }

    #[test]
    fn an_ionized_snipers_gun_facing_narrows_with_its_turn() {
        use crate::flight::ShipState;
        let off = |heading: f32| ShipState {
            position: crate::flight::facing(heading) * 500.0,
            ..ShipState::default()
        };
        let at = off(crate::ai::fire::GUN_FACING * 3.0 * 0.75);
        assert!(crate::traffic::snipe::facing(&charged(0.0), &at));
        assert!(!crate::traffic::snipe::facing(&charged(200.0), &at));
    }

    #[test]
    fn an_ion_dissipator_among_its_default_items_changes_nothing() {
        let dissipators = [
            OutfitMod {
                mod_type: ION_ABSORBER,
                mod_val: 200,
                count: 2,
            },
            OutfitMod {
                mod_type: ION_DISSIPATER,
                mod_val: 50,
                count: 2,
            },
        ];
        let mut carrying = npc(1, stats(&dissipators));
        carrying.ion.charge = 200.0;
        let bare = charged(200.0);
        assert_eq!(carrying.ion_factor(), 0.5);
        assert_eq!(carrying.ionized_handling(), bare.ionized_handling());
        deionize(&mut carrying, 6.0);
        assert_eq!(carrying.ion.charge, 199.5);
    }
}
