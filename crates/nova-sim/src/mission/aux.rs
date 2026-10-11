//! An active mission's auxiliary ships: ordinary traffic of its
//! `AuxShipDude` that comes into the systems its `AuxShipSyst` allows on
//! a countdown ([`AuxShips`], its slot's +0x64-+0x6e). How they spawn is
//! the [`traffic::spawn`](crate::traffic::spawn) module's "Auxiliary
//! ships", and when, the session's `mission_ships` module.
//!
//! They carry no mission mark, so they count toward no goal and get none
//! of a special ship's handling: only an aux mark of their slot (the
//! original's ship +0x8a), which gives a survivor back to its slot as the
//! system's ships are dropped ([`AuxShips::give_back`]).

use crate::catalog::{AuxShip, DudeId, SystemId};
use crate::chance::Chance;
use crate::hyperspace::Places;

use super::offer::Stop;

/// `Flags`: the mission's auxiliary ships come again in full on every
/// restart of their countdown (@0x99fc0, @0x634f8), and none that comes
/// is taken off those left to come (@0x435f9).
pub const AUX_EVERY_VISIT: u16 = 0x0010;
/// `Rand(70) + 70`: the countdown, drawn at accept (@0xa0e5f) and at each
/// restart (@0x99fd9, @0x6350b).
pub const COUNTDOWN_DRAW: u32 = 70;
/// The countdown's highest value that counts down (32000 @0x43af1).
const COUNTED: i16 = 32_000;
/// The `düde` IDs an `AuxShipDude` may name (128-639, @0xa0e88-0xa0e94).
const DUDES: std::ops::RangeInclusive<i16> = 128..=639;

/// The `düde` `aux`'s ships are drawn from: its `AuxShipDude` within
/// 128-639, else none, and no ship comes (@0xa0e88-0xa0ea2).
#[must_use]
pub fn aux_dude(aux: &AuxShip) -> Option<DudeId> {
    aux.dude.filter(|dude| DUDES.contains(&dude.0))
}

/// How many of `aux`'s ships its slot keeps as its count (+0x64): its
/// `AuxShipCount`, or none when it is not above 0 or the `düde` is out of
/// range (@0xa0e9c-0xa0ecb, where the original keeps -1; 0 reads the
/// same, as none come at either).
#[must_use]
pub fn aux_count(aux: &AuxShip) -> i16 {
    if aux_dude(aux).is_some() {
        aux.count.max(0)
    } else {
        0
    }
}

/// `Rand(70) + 70`, drawn on `chance`.
fn countdown(chance: &mut dyn Chance) -> i16 {
    i16::try_from(chance.below(COUNTDOWN_DRAW) + COUNTDOWN_DRAW).unwrap_or(i16::MAX)
}

/// What an active mission's slot keeps of its auxiliary ships.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AuxShips {
    /// The countdown they wait on (+0x6c): it counts down while 1-32000,
    /// and they may come once it is not above 0.
    pub countdown: i16,
    /// How many have come since the countdown last restarted (+0x6a).
    pub arrived: i16,
    /// How many are left to come (+0x6e).
    pub left: i16,
}

impl AuxShips {
    /// A mission's auxiliary ships `aux` as `_LoadCurrentMissionData`
    /// loads them at accept (@0xa0e20-0xa0ecb), for every mission, of
    /// auxiliary ships or not: the countdown `Rand(70) + 70`, drawn on
    /// `chance`; none arrived; its count ([`aux_count`]) left to come.
    pub fn at_accept(aux: &AuxShip, chance: &mut dyn Chance) -> Self {
        Self {
            countdown: countdown(chance),
            arrived: 0,
            left: aux_count(aux),
        }
    }

    /// The restart of the enter-system hook (@0x99fb5-0x99ff4) and of the
    /// take-off (@0x634df-0x6353c), for a mission of `Flags` `flags`:
    /// with [`AUX_EVERY_VISIT`] its count ([`aux_count`]) left to come
    /// again; the countdown `Rand(70) + 70`, drawn on `chance`; none
    /// arrived.
    pub fn restart(&mut self, aux: &AuxShip, flags: u16, chance: &mut dyn Chance) {
        if flags & AUX_EVERY_VISIT != 0 {
            self.left = aux_count(aux);
        }
        self.countdown = countdown(chance);
        self.arrived = 0;
    }

    /// How many of `aux`'s ships come now, once the system allows them
    /// (`_EnterMoreShips` @0x434c9-0x4351f): left to come less those
    /// arrived, fixed before any comes, once the countdown is not above
    /// 0, with a `düde`; none otherwise.
    #[must_use]
    pub fn coming(&self, aux: &AuxShip) -> i16 {
        if self.countdown > 0 || aux_dude(aux).is_none() || self.arrived >= self.left {
            return 0;
        }
        self.left - self.arrived
    }

    /// One ship came, for a mission of `Flags` `flags`: one more arrived
    /// and, without [`AUX_EVERY_VISIT`], one fewer left to come
    /// (@0x435f0-0x435ff).
    pub fn came(&mut self, flags: u16) {
        self.arrived = self.arrived.saturating_add(1);
        if flags & AUX_EVERY_VISIT == 0 {
            self.left = self.left.saturating_sub(1);
        }
    }

    /// One frame of the countdown: down one while it is 1-32000
    /// (@0x43ae8-0x43af8).
    pub fn tick(&mut self) {
        if (1..=COUNTED).contains(&self.countdown) {
            self.countdown -= 1;
        }
    }

    /// One of `aux`'s ships, still in the system as its ships are
    /// dropped, given back: one more left to come, no more than its count
    /// ([`aux_count`]) (`_DeleteAllComputerShips` @0x3bfce-0x3c028).
    pub fn give_back(&mut self, aux: &AuxShip) {
        self.left = self.left.saturating_add(1).min(aux_count(aux));
    }
}

/// Whether the auxiliary ships of `AuxShipSyst` `code` may come into the
/// player's system `player`, the mission going to `destination` and
/// completed at `return_to`, over the map's `places`
/// (`_ValidAuxShipSystem` @0x992b5-0x9959d):
///
/// - -1 and -6: any system (@0x992d9-0x992fa).
/// - -2: the destination's system; -3: the return's. Never with none
///   (@0x99307-0x99336).
/// - 128-2175: that system, or the visible system standing for it at its
///   place (`_FindActiveCoLocatedSystem`, @0x99338-0x99361).
/// - Any other code: never. The original also reads 5000-9998 (that
///   system or one it links to) and 9999-31999 (the player's system's
///   government by relation, @0x99363-0x99589), which no stock mission
///   uses: rdm:phase/mission-ships-fidelity/phase-30-aux-system-codes.
#[must_use]
pub fn aux_system_allows(
    code: i16,
    player: SystemId,
    (destination, return_to): (Option<Stop>, Option<Stop>),
    places: &Places,
) -> bool {
    let at = |stop: Option<Stop>| stop.and_then(|stop| stop.system) == Some(player);
    match code {
        -1 | -6 => true,
        -2 => at(destination),
        -3 => at(return_to),
        128..=2175 => {
            let system = SystemId(code);
            system == player || places.stand_in(system) == Some(player)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::testkit::{Draws, star};
    use std::collections::BTreeSet;

    fn aux(count: i16, dude: i16) -> AuxShip {
        AuxShip {
            count,
            dude: Some(DudeId(dude)),
            syst: -1,
        }
    }

    #[test]
    fn the_dude_is_one_of_128_to_639_and_the_count_none_without_one() {
        assert_eq!(aux_dude(&aux(3, 128)), Some(DudeId(128)));
        assert_eq!(aux_dude(&aux(3, 639)), Some(DudeId(639)));
        assert_eq!(aux_dude(&aux(3, 640)), None);
        assert_eq!(aux_dude(&aux(3, 127)), None);
        assert_eq!(aux_dude(&AuxShip::NONE), None);
        assert_eq!(aux_count(&aux(3, 639)), 3);
        assert_eq!(aux_count(&aux(3, 640)), 0, "out of range");
        assert_eq!(aux_count(&aux(-2, 200)), 0, "below none");
        assert_eq!(
            aux_count(&AuxShip {
                dude: None,
                ..aux(3, 200)
            }),
            0
        );
    }

    #[test]
    fn at_accept_the_countdown_is_drawn_and_its_count_left_to_come() {
        let mut draws = Draws::of(&[9]);
        assert_eq!(
            AuxShips::at_accept(&aux(4, 200), &mut draws),
            AuxShips {
                countdown: 79,
                arrived: 0,
                left: 4
            }
        );
        assert_eq!(draws.asked, [COUNTDOWN_DRAW]);
        assert_eq!(COUNTDOWN_DRAW, 70);
        let mut draws = Draws::of(&[0]);
        assert_eq!(
            AuxShips::at_accept(&AuxShip::NONE, &mut draws),
            AuxShips {
                countdown: 70,
                arrived: 0,
                left: 0
            },
            "drawn for a mission of no auxiliary ships too"
        );
        assert_eq!(draws.asked, [70]);
    }

    #[test]
    fn a_restart_refills_those_left_only_with_flags_0x0010() {
        assert_eq!(AUX_EVERY_VISIT, 0x0010);
        let spent = AuxShips {
            countdown: 0,
            arrived: 2,
            left: 1,
        };
        let mut ships = spent;
        let mut draws = Draws::of(&[69]);
        ships.restart(&aux(4, 200), 0x0001, &mut draws);
        assert_eq!(
            ships,
            AuxShips {
                countdown: 139,
                arrived: 0,
                left: 1
            }
        );
        assert_eq!(draws.asked, [70]);
        let mut ships = spent;
        ships.restart(&aux(4, 200), 0x0011, &mut Draws::of(&[0]));
        assert_eq!(
            ships,
            AuxShips {
                countdown: 70,
                arrived: 0,
                left: 4
            }
        );
        let mut ships = spent;
        ships.restart(&aux(4, 700), AUX_EVERY_VISIT, &mut Draws::of(&[0]));
        assert_eq!(ships.left, 0, "a düde out of range has none");
    }

    #[test]
    fn they_come_once_the_countdown_has_run_out_as_many_as_left_less_arrived() {
        let ships = |countdown, arrived, left| AuxShips {
            countdown,
            arrived,
            left,
        };
        let four = aux(4, 200);
        assert_eq!(ships(0, 0, 4).coming(&four), 4);
        assert_eq!(ships(-1, 1, 4).coming(&four), 3);
        assert_eq!(ships(1, 0, 4).coming(&four), 0, "still counting");
        assert_eq!(ships(0, 4, 4).coming(&four), 0, "all arrived");
        assert_eq!(ships(0, 5, 4).coming(&four), 0);
        assert_eq!(ships(0, 0, 0).coming(&four), 0, "none left");
        assert_eq!(ships(0, 0, 4).coming(&aux(4, 700)), 0, "no düde");
    }

    #[test]
    fn each_that_comes_is_arrived_and_without_0x0010_no_longer_left() {
        let mut ships = AuxShips {
            countdown: 0,
            arrived: 0,
            left: 3,
        };
        ships.came(0);
        assert_eq!((ships.arrived, ships.left), (1, 2));
        ships.came(AUX_EVERY_VISIT);
        assert_eq!((ships.arrived, ships.left), (2, 2));
    }

    #[test]
    fn the_countdown_counts_down_from_32000_and_stops_at_0() {
        let tick = |countdown| {
            let mut ships = AuxShips {
                countdown,
                ..AuxShips::default()
            };
            ships.tick();
            ships.countdown
        };
        assert_eq!(tick(32_000), 31_999);
        assert_eq!(tick(1), 0);
        assert_eq!(tick(0), 0);
        assert_eq!(tick(-1), -1);
        assert_eq!(tick(32_001), 32_001);
        assert_eq!(tick(0x7fff), 0x7fff, "lost at a gate");
    }

    #[test]
    fn a_survivor_given_back_is_left_to_come_again_up_to_the_count() {
        let mut ships = AuxShips {
            countdown: 5,
            arrived: 3,
            left: 1,
        };
        ships.give_back(&aux(3, 200));
        assert_eq!(ships.left, 2);
        ships.give_back(&aux(3, 200));
        ships.give_back(&aux(3, 200));
        assert_eq!(ships.left, 3, "capped at the count");
        assert_eq!((ships.countdown, ships.arrived), (5, 3));
    }

    fn stop(stellar: i16, system: Option<i16>) -> Stop {
        Stop {
            stellar: StellarId(stellar),
            system: system.map(SystemId),
        }
    }

    /// 130 and 131 at one place, 131 hidden; 132 apart.
    fn places() -> Places {
        let systems = [
            star(130, (0.0, 0.0), &[]),
            star(131, (0.0, 0.0), &[]),
            star(132, (5.0, 0.0), &[]),
        ];
        Places::new(&systems, &BTreeSet::from([SystemId(131)]))
    }

    #[test]
    fn the_aux_system_codes_stock_uses_allow_their_systems() {
        let places = places();
        let stops = (Some(stop(200, Some(132))), Some(stop(201, Some(130))));
        let allows = |code, player| aux_system_allows(code, SystemId(player), stops, &places);
        for code in [-1, -6] {
            assert!(allows(code, 130) && allows(code, 132), "{code}: any");
        }
        assert!(allows(-2, 132), "the destination's");
        assert!(!allows(-2, 130));
        assert!(allows(-3, 130), "the return's");
        assert!(!allows(-3, 132));
        assert!(allows(132, 132), "that system");
        assert!(!allows(132, 130));
        assert!(allows(131, 131), "itself, hidden");
        assert!(allows(131, 130), "or its stand-in");
        assert!(!allows(131, 132));
        assert!(allows(2175, 2175));
        for code in [-4, -5, 0, 127, 2176, 5000, 9999, 15_000, 31_999, 32_000] {
            assert!(!allows(code, 130), "{code}: unbuilt or none");
        }
    }

    #[test]
    fn a_missing_stop_or_one_of_no_system_allows_none() {
        let places = places();
        let none = (None, None);
        assert!(!aux_system_allows(-2, SystemId(130), none, &places));
        assert!(!aux_system_allows(-3, SystemId(130), none, &places));
        let nowhere = (Some(stop(200, None)), Some(stop(201, None)));
        assert!(!aux_system_allows(-2, SystemId(130), nowhere, &places));
        assert!(!aux_system_allows(-3, SystemId(130), nowhere, &places));
    }
}
