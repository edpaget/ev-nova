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
//!
//! The mark outlives its mission: nothing at a mission's end writes
//! +0x8a, and `_AbortMission` (@0x99a92-0x99bdb) touches only the ships
//! marked as its special ships (+0xc8d2). So a survivor gives one back to
//! whichever mission holds its slot at the drop, as
//! `_DeleteAllComputerShips` tests only the slot's active byte
//! (@0x3bfe8), capped at that mission's count. The original also caps a
//! free slot's dead record (@0x3c00a-0x3c024); the engine skips a free
//! slot, which no one can see, as the next accept reloads the slot
//! (@0xa0e20-0xa0ecb).

use crate::catalog::GovtId;
use crate::catalog::{AuxShip, DudeId, SystemId};
use crate::chance::Chance;
use crate::govt::Governments;
use crate::hyperspace::StarMap;
use crate::mission::select::GovtCode;

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

    /// The restart of the enter-system hook (@0x99fb5-0x99ff4), of the
    /// take-off (@0x634df-0x6353c) and of `_OpenLastPilotFile`'s loop at
    /// launch (@0x765d4-0x76637, `nova_sim::Session::open_at_launch`), for
    /// a mission of `Flags` `flags`:
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

    /// The restart of Open Pilot (`_MainScreenChoice(1)` @0x1a918-0x1a982),
    /// for a mission of `Flags` `flags`: with [`AUX_EVERY_VISIT`] its count
    /// ([`aux_count`]) left to come again; then, only while some are left
    /// to come (+0x6e > 0, @0x1a94a-0x1a94f), the countdown
    /// `Rand(70) + 70`, drawn on `chance` (@0x1a951-0x1a966), and none
    /// arrived. With none left the countdown and those arrived stay as
    /// the save had them. The take-off and the enter-system hook
    /// ([`AuxShips::restart`]) have no such guard, nor has
    /// `_OpenLastPilotFile`'s loop at launch (@0x765d4-0x76637).
    pub fn restart_from_main_screen(&mut self, aux: &AuxShip, flags: u16, chance: &mut dyn Chance) {
        if flags & AUX_EVERY_VISIT != 0 {
            self.left = aux_count(aux);
        }
        if self.left > 0 {
            self.countdown = countdown(chance);
            self.arrived = 0;
        }
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
    /// ([`aux_count`]) (`_DeleteAllComputerShips` @0x3bfce-0x3c028). The
    /// ships an in-flight `M` or `N` leaves behind give nothing back by
    /// the engine: the display pass frees them first
    /// (`_HandleShipDisplay` @0x2ded6-0x2dee7;
    /// [`RuleKey::MoveLeftBehind`](crate::RuleKey::MoveLeftBehind)).
    pub fn give_back(&mut self, aux: &AuxShip) {
        self.left = self.left.saturating_add(1).min(aux_count(aux));
    }
}

/// Whether the auxiliary ships of `AuxShipSyst` `code` may come into the
/// player's system `player`, `holders` being the systems that hold the
/// mission's destination and return stellars now (`None` with no such
/// stop or no holder), over `map` and the relations of `govts`
/// (`_ValidAuxShipSystem` @0x992b5-0x9959d):
///
/// - -1 and -6: any system. The original allows only the system it is
///   passed (@0x992d9-0x992fa), but its one caller, `_EnterMoreShips`,
///   passes the player's (@0x434f0-0x4350d), so that always holds.
/// - -2: when the destination's holder is the player's system; -3: the
///   return's. The holder is the stellar's owner (+0x14) read live, the
///   rescue's included, with no `_FindSystemFromStellar` fallback and no
///   resolution by place, so a stellar no system holds never matches
///   (@0x99307-0x99336).
/// - 128-2175: that system, or the visible system standing for it at its
///   place (`_FindActiveCoLocatedSystem`, @0x99338-0x99361).
/// - 5000-9998: system `code - 4872` (5000 is system 128; the Bible's 5128 is wrong), or any system
///   its raw Con slots list ([`StarMap::con_slots`]), with no stand-in and
///   no visibility test (@0x99363-0x993ac). A system the map doesn't hold
///   lists none; the original reads past its table for codes of 7048 and
///   up (@0x99383-0x9939b), which isn't modelled.
/// - 9999-14999: the player's system is that government's, an independent
///   for 9999 (@0x993b1-0x993dd).
/// - 15000-19999: the player's system is governed, by that government
///   itself or an ally of it (@0x993e2-0x99430). Unlike a `ShipSyst`, its
///   own counts.
/// - 20000-24999: the player's system isn't that government's, an
///   independent included (@0x99435-0x99461).
/// - 25000-29999: the player's system is governed, by an enemy of that
///   government; or that government is xenophobic and the player's
///   system's government isn't it nor an ally of it, which adds a derelict
///   government to its enemies (@0x99466-0x994fc).
/// - 30000-30999: governed, by one sharing a class with that government
///   (@0x994fe-0x9953e); 31000-31999: governed, by one sharing none
///   (@0x99545-0x99589).
/// - Any other code: never.
///
/// The player's system's government is the map's, overlaid, read with no
/// visibility test ([`StarMap::catalog_govt`]), as the original reads it
/// at +0x8 of a hidden system too.
#[must_use]
pub fn aux_system_allows(
    code: i16,
    player: SystemId,
    (destination, return_to): (Option<SystemId>, Option<SystemId>),
    map: &StarMap,
    govts: &Governments,
) -> bool {
    match code {
        -1 | -6 => true,
        -2 => destination == Some(player),
        -3 => return_to == Some(player),
        128..=2175 => {
            let system = SystemId(code);
            system == player || map.places().stand_in(system) == Some(player)
        }
        5000..=9998 => {
            let system = SystemId(code - 4872);
            system == player || map.con_slots(system).contains(&player)
        }
        _ => GovtCode::of(code).is_some_and(|of| govt_allows(of, map.catalog_govt(player), govts)),
    }
}

/// Whether a player's system governed by `govt` is of the relation `code`
/// names, as `_ValidAuxShipSystem` reads it (@0x993b1-0x99589).
fn govt_allows(code: GovtCode, govt: Option<GovtId>, govts: &Governments) -> bool {
    match code {
        GovtCode::Govt(of) => govt == of,
        GovtCode::Ally(of) => govt.is_some() && govts.allies(govt, Some(of)),
        GovtCode::NotGovt(of) => govt != Some(of),
        GovtCode::Enemy(of) => {
            let of = Some(of);
            govt.is_some()
                && ((govts.xenophobic(of) && govt != of && !govts.allies(govt, of))
                    || govts.enemies(govt, of))
        }
        GovtCode::SharedClass(of) => govt.is_some() && govts.shared_class(govt, Some(of)),
        GovtCode::NoSharedClass(of) => govt.is_some() && !govts.shared_class(govt, Some(of)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, StarSystem};
    use crate::govt::{DERELICT, XENOPHOBIC};
    use crate::hyperspace::CoLocated;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::{govt, star};
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
    fn the_main_screens_restart_refills_with_0x0010_and_draws_only_with_some_left() {
        let spent = AuxShips {
            countdown: 5,
            arrived: 2,
            left: 1,
        };
        let mut ships = spent;
        let mut draws = Draws::of(&[9]);
        ships.restart_from_main_screen(&aux(4, 200), 0x0001, &mut draws);
        assert_eq!(
            ships,
            AuxShips {
                countdown: 79,
                arrived: 0,
                left: 1
            },
            "some left: drawn, none arrived"
        );
        assert_eq!(draws.asked, [COUNTDOWN_DRAW]);
        let mut ships = AuxShips { left: 0, ..spent };
        ships.restart_from_main_screen(&aux(4, 200), AUX_EVERY_VISIT, &mut Draws::of(&[0]));
        assert_eq!(
            ships,
            AuxShips {
                countdown: 70,
                arrived: 0,
                left: 4
            },
            "refilled first, then drawn"
        );
    }

    #[test]
    fn the_main_screens_restart_leaves_the_countdown_with_none_left() {
        for left in [0, -1] {
            let before = AuxShips {
                countdown: 5,
                arrived: 2,
                left,
            };
            let mut ships = before;
            let mut draws = Draws::of(&[]);
            ships.restart_from_main_screen(&aux(4, 200), 0x0001, &mut draws);
            assert_eq!(ships, before, "{left}");
            assert!(draws.asked.is_empty(), "nothing drawn ({left})");
        }
        let mut ships = AuxShips {
            countdown: 5,
            arrived: 2,
            left: 3,
        };
        let mut draws = Draws::of(&[]);
        ships.restart_from_main_screen(&aux(4, 700), AUX_EVERY_VISIT, &mut draws);
        assert_eq!(
            ships,
            AuxShips {
                countdown: 5,
                arrived: 2,
                left: 0
            },
            "a düde out of range refills none, so nothing is drawn"
        );
        assert!(draws.asked.is_empty());
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

    const FED: GovtId = GovtId(128);
    const CIVVIES: GovtId = GovtId(129);
    const PIRATES: GovtId = GovtId(130);
    const XENO: GovtId = GovtId(131);
    const WRECKS: GovtId = GovtId(132);
    const XENO_FRIENDS: GovtId = GovtId(133);

    /// The Federation (class 1, allied with 13, at war with 9), the
    /// Civvies (classes 1 and 13), the pirates (class 9), a xenophobe
    /// (class 4), a derelict government (class 5) and the
    /// xenophobe's ally (class 6, allied with 4).
    fn govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [13, -1, -1, -1],
                enemies: [9, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [1, 13, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [9, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                flags: XENOPHOBIC,
                ..govt(131)
            },
            GovtRecord {
                classes: [5, -1, -1, -1],
                flags: DERELICT,
                ..govt(132)
            },
            GovtRecord {
                classes: [6, -1, -1, -1],
                allies: [4, -1, -1, -1],
                ..govt(133)
            },
        ])
    }

    fn system(id: i16, at: (f32, f32), links: &[i16], govt: Option<GovtId>) -> StarSystem {
        StarSystem {
            govt,
            ..star(id, at, links)
        }
    }

    /// 130 (the Federation's) and 131 at one place, 131 hidden; 132 the
    /// Civvies', 133 the pirates', 134 independent, 135 the xenophobe's,
    /// 136 the derelicts', 137 the xenophobe's ally's, 140 independent, each apart. 130's Con lists
    /// 131, 131's 132 and 140, 140's 133.
    fn map() -> StarMap {
        StarMap::with_hidden(
            vec![
                system(130, (0.0, 0.0), &[131], Some(FED)),
                system(131, (0.0, 0.0), &[132, 140], Some(FED)),
                system(132, (100.0, 0.0), &[], Some(CIVVIES)),
                system(133, (200.0, 0.0), &[], Some(PIRATES)),
                system(134, (300.0, 0.0), &[], None),
                system(135, (400.0, 0.0), &[], Some(XENO)),
                system(136, (500.0, 0.0), &[], Some(WRECKS)),
                system(137, (550.0, 0.0), &[], Some(XENO_FRIENDS)),
                system(140, (600.0, 0.0), &[133], None),
            ],
            &BTreeSet::from([SystemId(131)]),
            CoLocated::Lowest,
        )
    }

    /// Whether `code` allows the player in `player`, over [`map`] and
    /// [`govts`], with no stops.
    fn allows(code: i16, player: i16) -> bool {
        aux_system_allows(code, SystemId(player), (None, None), &map(), &govts())
    }

    #[test]
    fn the_aux_system_codes_stock_uses_allow_their_systems() {
        let map = map();
        let govts = govts();
        let holders = (Some(SystemId(132)), Some(SystemId(130)));
        let allows =
            |code, player| aux_system_allows(code, SystemId(player), holders, &map, &govts);
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
        for code in [-4, -5, 0, 127, 2176, 4999, 32_000, i16::MAX, -7] {
            for player in [130, 132, 133, 134] {
                assert!(!allows(code, player), "{code}: none");
            }
        }
    }

    #[test]
    fn codes_5000_to_9998_allow_that_system_or_one_its_con_lists() {
        // 5000 + (ID - 128).
        assert!(allows(5002, 130), "130 itself");
        assert!(allows(5002, 131), "130's Con lists 131");
        assert!(!allows(5002, 132));
        assert!(allows(5003, 131), "131 itself, hidden");
        assert!(allows(5003, 132), "the hidden 131's Con lists 132");
        assert!(allows(5003, 140));
        assert!(
            !allows(5003, 130),
            "130 stands for 131, but the compare is raw"
        );
        assert!(!allows(5003, 133), "listed only by 140");
        assert!(allows(5012, 133), "140's Con lists 133");
        assert!(!allows(5871, 130), "999: not held, no links");
        assert!(!allows(9998, 130));
    }

    #[test]
    fn codes_5000_and_9998_are_the_ends_of_the_con_range() {
        // 128 and 5126, the first and last systems the range names, each
        // with a Con listing 130.
        let map = StarMap::with_hidden(
            vec![
                system(128, (0.0, 0.0), &[130], None),
                system(130, (100.0, 0.0), &[], None),
                system(5126, (200.0, 0.0), &[130], None),
            ],
            &BTreeSet::new(),
            CoLocated::Lowest,
        );
        let allows =
            |code, player| aux_system_allows(code, SystemId(player), (None, None), &map, &govts());
        assert!(allows(5000, 128), "128 itself");
        assert!(allows(5000, 130), "128's Con lists 130");
        assert!(allows(9998, 5126), "5126 itself");
        assert!(allows(9998, 130), "5126's Con lists 130");
    }

    #[test]
    fn government_codes_read_a_hidden_player_system_s_government() {
        assert!(allows(10_000, 131), "hidden 131 is the Federation's");
        assert!(!allows(9999, 131), "not an independent");
        assert!(!allows(20_000, 131));
    }

    #[test]
    fn codes_9999_to_14999_allow_a_player_system_of_that_government() {
        assert!(allows(9999, 134), "9999: an independent");
        assert!(allows(9999, 140));
        assert!(!allows(9999, 130));
        assert!(allows(10_000, 130));
        assert!(!allows(10_000, 132));
        assert!(!allows(10_000, 134));
        assert!(allows(10_002, 133));
        assert!(!allows(14_999, 130));
    }

    #[test]
    fn codes_15000_to_19999_allow_the_government_itself_or_an_ally_never_an_independent() {
        assert!(allows(15_000, 130), "its own counts");
        assert!(allows(15_000, 132), "the Civvies are of an allied class");
        assert!(!allows(15_000, 133));
        assert!(!allows(15_000, 134), "an independent");
        assert!(allows(15_002, 133));
        assert!(!allows(15_002, 134));
        assert!(!allows(19_999, 134));
    }

    #[test]
    fn codes_20000_to_24999_allow_any_other_government_an_independent_included() {
        assert!(!allows(20_000, 130));
        assert!(allows(20_000, 132));
        assert!(allows(20_000, 134), "an independent");
        assert!(allows(24_999, 130));
    }

    #[test]
    fn codes_25000_to_29999_allow_an_enemy_never_an_independent() {
        assert!(allows(25_000, 133), "at war with the pirates");
        assert!(!allows(25_000, 132));
        assert!(!allows(25_000, 130));
        assert!(!allows(25_000, 134));
        assert!(allows(25_003, 130), "the xenophobe's enemy");
        assert!(
            !allows(25_003, 134),
            "a xenophobe is an independent's enemy, but an independent never counts"
        );
        assert!(!allows(25_003, 135), "its own");
    }

    #[test]
    fn a_xenophobe_s_25000_code_allows_a_derelict_non_ally() {
        let govts = govts();
        assert!(!govts.enemies(Some(WRECKS), Some(XENO)), "not by enemies");
        assert!(allows(25_003, 136));
        assert!(!allows(25_000, 136), "the Federation is no xenophobe");
    }

    #[test]
    fn a_xenophobe_s_25000_code_refuses_its_ally() {
        let govts = govts();
        assert!(
            !govts.enemies(Some(XENO_FRIENDS), Some(XENO)),
            "not by enemies"
        );
        assert!(govts.allies(Some(XENO_FRIENDS), Some(XENO)));
        assert!(!allows(25_003, 137));
    }

    #[test]
    fn codes_30000_to_30999_allow_a_shared_class_never_an_independent() {
        assert!(allows(30_000, 130), "its own");
        assert!(allows(30_000, 132), "class 1");
        assert!(!allows(30_000, 133));
        assert!(!allows(30_000, 134));
        assert!(!allows(30_999, 134));
    }

    #[test]
    fn codes_31000_to_31999_allow_no_shared_class_never_an_independent() {
        assert!(allows(31_000, 133));
        assert!(!allows(31_000, 132));
        assert!(!allows(31_000, 130));
        assert!(!allows(31_000, 134), "an independent");
        assert!(
            allows(31_999, 130),
            "a government it doesn't hold shares none"
        );
        assert!(!allows(31_999, 134));
    }

    #[test]
    fn a_missing_stop_or_one_no_system_holds_allows_none() {
        assert!(!allows(-2, 130));
        assert!(!allows(-3, 130));
    }

    #[test]
    fn a_holder_counts_as_it_is_not_by_place() {
        let (map, govts) = (map(), govts());
        let hidden = (Some(SystemId(131)), Some(SystemId(131)));
        assert!(
            !aux_system_allows(-2, SystemId(130), hidden, &map, &govts),
            "130 stands for 131, but the owner is compared as it is"
        );
        assert!(!aux_system_allows(-3, SystemId(130), hidden, &map, &govts));
    }
}
