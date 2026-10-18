//! Whether an AI ship has an afterburner (the original's +0xbd), and
//! whether it flies a complex manoeuvre: the two readers of the player's
//! combat rating the pursuit substates ask (`_AIHasAfterburner`
//! @0x64f5-0x6594 and `_AIPerformsComplexManeuvers` @0x659a-0x65c1 in
//! the `EV Nova` executable; see [`pursuit`](crate::ai::pursuit)).
//!
//! - **The afterburner** ([`has_afterburner`]), rolled as a ship spawns:
//!   first none, unrolled, at a berth another ship still names as its
//!   swarm leader (@0x64ff-0x6539, [`swarm::named`](crate::ai::swarm::named),
//!   which each roll's caller asks); none for a planet-type class (`shïp` `Flags` 0x0400, @0x6554); one
//!   for a class of [`AFTERBURNER_ALWAYS`] (0x0040, @0x6559); for one of
//!   [`AFTERBURNER_BY_RATING`] (0x0020), one while the rating over the
//!   yardstick is at least `Rand(`[`RATING_ROLL`]`)` +
//!   [`RATING_FLOOR`] (@0x655d-0x6589), the division truncating toward
//!   zero as `idivl` does; none for any other. The yardstick is what
//!   [`RuleKey::AfterburnerYardstick`](crate::RuleKey::AfterburnerYardstick)
//!   says ([`yardstick`]): by the engine the first `shïp` record's
//!   `Strength` (`shïp` 128's, the original reading `*0x3b5098` + 0xc,
//!   @0x6566), the same yardstick the odds measure the rating by
//!   ([`odds::rating_factor`](crate::ai::odds::rating_factor)); by the
//!   other reading the ship's own class's. A yardstick of none divides
//!   by zero and crashes the original; here it gives none.
//! - **A complex manoeuvre** ([`performs_complex_manoeuvres`]):
//!   `Rand(`[`RATING_ROLL`]`)` + [`RATING_FLOOR`] at most the rating.
//! - Where the flag is rolled: a `düde` ship's spawn (`_RandomShipSpawn`
//!   @0x3c5f0), a fleet's lead (`_HyperSpawnFleet` @0x41e14, not its
//!   escorts), a person (`_SpawnPerson` @0x40d62, and always one when its
//!   `përs` `Flags` has [`PERSON_AFTERBURNER`], @0x40d99-0x40da2) and
//!   every fighter launched (`_LaunchFighter` @0x3d736). A mission's ship,
//!   an auxiliary ship and a fleet's escorts have none, as
//!   `_GenericRandomShipSpawn` writes 0 (@0x3cb83) and their spawns roll
//!   nothing ([`asks`]).
//! - Neither routine plays a sound.
//!
//! # Not modelled
//!
//! - The player's escorts' flag, which `_SpawnEscort` (@0x3ff51) and
//!   `_DoShipCapture` (@0x4123e) roll and a jump keeps: here they have
//!   none (rdm phase combat-ai-fidelity/phase-17-left-out-of-phase-7).

use crate::chance::Chance;
use crate::combat::hull::{AFTERBURNER_ALWAYS, AFTERBURNER_BY_RATING, HullSpec};
use crate::rulebook::RuleSource;
use crate::traffic::spawn::NewShip;

/// `Rand(1344)`: the roll a rating is measured against (0x540 @0x656c and
/// @0x65a0).
pub const RATING_ROLL: u16 = 1344;
/// What the roll starts from (0x100 @0x6582 and @0x65b3).
pub const RATING_FLOOR: i64 = 256;
/// The `përs` `Flags` bit that gives a person an afterburner whatever its
/// roll (the person's +0x14, `_SpawnPerson` @0x40d99).
pub const PERSON_AFTERBURNER: u16 = 0x0002;

/// The yardstick `reading` divides the rating by (see the module docs):
/// `first`, the first `shïp` record's `Strength`, by the engine; `own`,
/// the ship's own class's, by the other reading.
#[must_use]
pub fn yardstick(reading: RuleSource, first: i16, own: i16) -> i16 {
    match reading {
        RuleSource::Engine => first,
        RuleSource::Bible => own,
    }
}

/// Whether an AI ship of `hull` has an afterburner against a `rating`
/// measured by `yardstick`, rolling on `chance` (see the module docs).
pub fn has_afterburner(
    hull: &HullSpec,
    rating: i64,
    yardstick: i16,
    chance: &mut (impl Chance + ?Sized),
) -> bool {
    if hull.planet_type {
        return false;
    }
    if hull.afterburner & AFTERBURNER_ALWAYS != 0 {
        return true;
    }
    if hull.afterburner & AFTERBURNER_BY_RATING == 0 || yardstick == 0 {
        return false;
    }
    rating / i64::from(yardstick) >= i64::from(chance.roll(RATING_ROLL)) + RATING_FLOOR
}

/// Whether a ship flies a complex manoeuvre against the player's
/// `rating`, rolling on `chance` (`_AIPerformsComplexManeuvers`).
pub fn performs_complex_manoeuvres(rating: i64, chance: &mut (impl Chance + ?Sized)) -> bool {
    i64::from(chance.roll(RATING_ROLL)) + RATING_FLOOR <= rating
}

/// Whether `ship`'s spawn asks `_AIHasAfterburner` (see the module docs):
/// a `düde` ship, a fleet's lead or a person, not a fleet's escort, a
/// mission's ship or an auxiliary ship.
#[must_use]
pub fn asks(ship: &NewShip) -> bool {
    ship.lead.is_none() && ship.mission.is_none() && ship.aux.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Ranged;

    fn hull(afterburner: u16, planet_type: bool) -> HullSpec {
        HullSpec {
            afterburner,
            planet_type,
            ..HullSpec::default()
        }
    }

    #[test]
    fn the_named_values() {
        assert_eq!(
            (RATING_ROLL, RATING_FLOOR, PERSON_AFTERBURNER),
            (1344, 256, 0x0002)
        );
    }

    #[test]
    fn a_class_of_flags_0x0040_always_has_one_and_rolls_nothing() {
        let mut chance = Ranged::of(&[]);
        assert!(has_afterburner(
            &hull(AFTERBURNER_ALWAYS, false),
            0,
            0,
            &mut chance
        ));
        assert!(has_afterburner(
            &hull(AFTERBURNER_ALWAYS | AFTERBURNER_BY_RATING, false),
            0,
            2,
            &mut chance
        ));
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_planet_type_class_never_has_one_even_with_0x0040() {
        let mut chance = Ranged::of(&[(1344, 0)]);
        assert!(!has_afterburner(
            &hull(AFTERBURNER_ALWAYS | AFTERBURNER_BY_RATING, true),
            1_000_000,
            1,
            &mut chance
        ));
    }

    #[test]
    fn a_class_of_neither_bit_has_none() {
        let mut chance = Ranged::of(&[(1344, 0)]);
        assert!(!has_afterburner(&hull(0, false), 1_000_000, 1, &mut chance));
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_class_of_flags_0x0020_has_one_while_the_rating_over_the_yardstick_reaches_the_roll() {
        let rolled = |rating: i64, yardstick: i16, outcome: u32| {
            let mut chance = Ranged::of(&[(1344, outcome)]);
            let has = has_afterburner(
                &hull(AFTERBURNER_BY_RATING, false),
                rating,
                yardstick,
                &mut chance,
            );
            assert_eq!(chance.asked, [1344]);
            has
        };
        // 1024 / 2 = 512 against 256 + 256.
        assert!(rolled(1024, 2, 256));
        assert!(!rolled(1024, 2, 257));
        // 1025 / 2 truncates to 512.
        assert!(!rolled(1025, 2, 257));
        assert!(rolled(1027, 2, 257));
        // Even the best roll needs 256.
        assert!(!rolled(255, 1, 0));
        assert!(rolled(256, 1, 0));
        // A negative yardstick turns the quotient negative.
        assert!(!rolled(1_000_000, -1, 0));
    }

    #[test]
    fn a_yardstick_of_none_gives_none() {
        let mut chance = Ranged::of(&[(1344, 0)]);
        assert!(!has_afterburner(
            &hull(AFTERBURNER_BY_RATING, false),
            1_000_000,
            0,
            &mut chance
        ));
    }

    #[test]
    fn the_yardstick_is_the_first_records_strength_by_the_engine_and_its_own_by_the_other_reading()
    {
        assert_eq!(yardstick(RuleSource::Engine, 2, 40), 2);
        assert_eq!(yardstick(RuleSource::Bible, 2, 40), 40);
    }

    #[test]
    fn a_complex_manoeuvre_needs_the_rating_at_least_the_roll_and_256() {
        let rolled = |rating: i64, outcome: u32| {
            let mut chance = Ranged::of(&[(1344, outcome)]);
            let does = performs_complex_manoeuvres(rating, &mut chance);
            assert_eq!(chance.asked, [1344]);
            does
        };
        assert!(rolled(256, 0));
        assert!(!rolled(255, 0));
        assert!(rolled(1599, 1343));
        assert!(!rolled(1598, 1343));
    }

    /// A `düde` ship of type 200 placed at the centre.
    fn duede_ship() -> NewShip {
        NewShip {
            ship: crate::catalog::ShipId(200),
            govt: None,
            ai_type: crate::traffic::npc::AiType::WimpyTrader,
            lead: None,
            state: crate::flight::ShipState::default(),
            entry: crate::traffic::spawn::Entry::Placed,
            aggression: 0,
            booty: Some(0),
            info_types: 0,
            person: None,
            mission: None,
            aux: None,
            derelict: false,
            held: false,
            boarded: false,
            cloaked: false,
            quote: None,
            cargo: None,
        }
    }

    #[test]
    fn a_duede_ship_a_fleets_lead_and_a_person_ask_and_an_escort_a_mission_or_an_aux_ship_does_not()
    {
        let duede = duede_ship();
        assert!(asks(&duede));
        assert!(asks(&NewShip {
            booty: None,
            ..duede
        }));
        assert!(asks(&NewShip {
            person: Some(crate::catalog::PersonId(128)),
            ..duede
        }));
        assert!(!asks(&NewShip {
            lead: Some(0),
            ..duede
        }));
        assert!(!asks(&NewShip {
            aux: Some(0),
            ..duede
        }));
        assert!(!asks(&NewShip {
            mission: Some(crate::traffic::npc::MissionMark {
                slot: 0,
                behav: crate::traffic::ShipBehav::Own,
                formation: None,
                order: None,
            }),
            ..duede
        }));
    }
}
