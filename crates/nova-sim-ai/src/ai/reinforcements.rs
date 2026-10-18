//! A system's reinforcement fleet: the call a ship makes for it
//! (`_AICallForReinforcements` @0x7f5a1-0x7f71e in the `EV Nova`
//! executable, [`call`]), the clock that brings it (`_HandleReinforcements`
//! @0x39048-0x392c3, [`tick`]) and the daily countdown before it can be
//! called again (`_IncrementGameTime` @0xb724-0xb759, [`day`]).
//!
//! Each system keeps a [`Clock`]: a timer (`sÿst` runtime +0xc8, a float
//! in the original, whole ticks here, as the frame scale is 1) and a
//! countdown in days (+0xc4). The timer runs while a called fleet is on
//! its way; the countdown keeps the fleet from being called again.
//!
//! - **The call** ([`call`], gates in the original's order). The ship has
//!   a government (@0x7f5be); its system has a fleet (@0x7f5e1); the
//!   timer is not running (≤ 0, @0x7f5f9) and the countdown is 0
//!   (@0x7f607); the ship's odds are above 0 (@0x7f614); the fleet's
//!   government is allied with the ship's (`_GovtAllies` @0x7f63d); and
//!   the odds are above half the **fleet** government's `MaxOdds`, or the
//!   call is forced (@0x7f64e-0x7f6b0). Then an inhibitor (the global one,
//!   or the ship's or the fleet's government's) marks the system called,
//!   the countdown 1, and nothing comes ([`Called::Marked`],
//!   @0x7f6b2-0x7f6ef); otherwise the timer starts at the system's
//!   `ReinfTime` ([`Called::Coming`], @0x7f6fc-0x7f70c).
//! - **The clock** ([`tick`]), once a frame on the player's system: with
//!   a fleet and the timer running, the timer goes down by one first
//!   (@0x39097-0x390a8). At 0 or below the fleet arrives
//!   ([`Event::Arrive`], @0x390c2-0x390e0), the timer reads -1
//!   ([`ARRIVED`], @0x390e9) and the countdown is the interval. Before
//!   that, on the first frame the timer is below a quarter of
//!   `ReinfTime` (strictly, @0x390fb-0x39116) while the countdown is 0
//!   (@0x3911c), the fleet is announced ([`Event::Announce`]) when its
//!   government has a name (@0x3914d), and the countdown is the interval
//!   either way (@0x39291-0x392b0). So `ReinfTime` 1 to 4 arrive
//!   unannounced, and a `ReinfTime` of 0 or less never arrives: the call
//!   leaves the timer not running.
//! - **The interval** is the system's `ReinfIntrval`, and 1 when that is
//!   0 or less (@0x39294-0x392b0; the load has already made it so,
//!   `SystemReinforcements`).
//! - **The day** ([`day`]): each visible system's countdown above 0 goes
//!   down by one (@0xb739-0xb746).
//!
//! A ship's odds retreat waits for twice its government's `MaxOdds`
//! while an allied fleet is coming ([`Clock::coming`]; see the
//! [`warship`](super::warship) module).

use crate::catalog::GovtId;

/// A system's reinforcement fleet, as the call and the clock read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fleet {
    /// The `flët`'s `Govt`, or `None` for independent.
    pub govt: Option<GovtId>,
    /// The system's `ReinfTime`, in ticks (+0xc0).
    pub time: i16,
    /// The system's `ReinfIntrval`, in days (+0xc2).
    pub interval: i16,
}

impl Fleet {
    /// The days the countdown is set to: the interval, or 1 when that is
    /// 0 or less (@0x39294-0x392b0).
    #[must_use]
    pub fn interval_days(self) -> u16 {
        u16::try_from(self.interval).unwrap_or(0).max(1)
    }
}

/// The timer an arrival leaves (-1.0 @0x390e9): not running.
pub const ARRIVED: i32 = -1;

/// A system's reinforcement clock (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Clock {
    /// The ticks until a called fleet arrives (+0xc8); 0 or below when
    /// none is coming.
    pub timer: i32,
    /// The days until the fleet can be called again (+0xc4).
    pub countdown: u16,
}

impl Clock {
    /// Whether a called fleet is on its way: the timer is above 0, as the
    /// doubling of `MaxOdds` reads it (`_WarshipAI` @0x8bd9d,
    /// `_InterceptorAI` @0x8d24b).
    #[must_use]
    pub fn coming(self) -> bool {
        self.timer > 0
    }

    /// The clock after the call answered `called`: marked, the countdown
    /// 1 (@0x7f6ef); coming, the timer set (@0x7f70c); otherwise as it
    /// was.
    pub fn answer(&mut self, called: Called) {
        match called {
            Called::No => {}
            Called::Marked => self.countdown = 1,
            Called::Coming(ticks) => self.timer = ticks,
        }
    }
}

/// The inhibitors a call reads (@0x7f6b2-0x7f6ed): `ModType` 44 outfits
/// the player owns (see `nova_sim`'s `reinforcements` module).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
// Each is its own gate of the original's, read on its own.
#[allow(clippy::struct_excessive_bools)]
pub struct Inhibitors {
    /// The global one (`*0x3b510c`, `ModVal` -1).
    pub global: bool,
    /// The ship's government's (+0x82).
    pub ship_govt: bool,
    /// The fleet's government's (+0x82), never for an independent fleet.
    pub fleet_govt: bool,
}

impl Inhibitors {
    /// Whether any of them is set.
    #[must_use]
    pub fn any(self) -> bool {
        self.global || self.ship_govt || self.fleet_govt
    }
}

/// What `_AICallForReinforcements` asks of a ship and its system (see
/// [`call`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReinforcementFacts {
    /// The ship has a government (+0x98 not -1).
    pub has_govt: bool,
    /// The system's fleet, if it has one.
    pub fleet: Option<Fleet>,
    /// The system's clock.
    pub clock: Clock,
    /// The odds against the ship (+0xc914).
    pub odds: f32,
    /// The fleet's government is allied with the ship's
    /// (`_GovtAllies(ship govt, fleet govt)`).
    pub allied: bool,
    /// The fleet government's `MaxOdds`, as a ratio.
    pub fleet_max_odds: f32,
    /// Whether the call is forced past the strength test (battle help's
    /// call, `_DoCommDialog` @0x9636d).
    pub force: bool,
    /// The inhibitors set.
    pub inhibitors: Inhibitors,
}

/// The share of the fleet government's `MaxOdds` the odds must pass
/// (0.5 @0xdd694).
pub const STRENGTH_SHARE: f32 = 0.5;

/// What a call does ([`call`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Called {
    /// Nothing: a gate failed.
    No,
    /// The system is marked called and nothing comes: an inhibitor.
    Marked,
    /// The fleet is coming, its timer set to these ticks.
    Coming(i32),
}

/// What a call with `facts` does, its gates in the original's order (see
/// the module docs).
#[must_use]
pub fn call(facts: &ReinforcementFacts) -> Called {
    let Some(fleet) = facts.fleet.filter(|_| facts.has_govt) else {
        return Called::No;
    };
    let clock = facts.clock;
    let strong = facts.odds > STRENGTH_SHARE * facts.fleet_max_odds;
    if clock.timer > 0
        || clock.countdown > 0
        || facts.odds <= 0.0
        || !facts.allied
        || !(strong || facts.force)
    {
        return Called::No;
    }
    if facts.inhibitors.any() {
        Called::Marked
    } else {
        Called::Coming(i32::from(fleet.time))
    }
}

/// The share of `ReinfTime` the timer must fall below for the fleet to be
/// announced (0.25 @0xdd130).
pub const ANNOUNCE_SHARE: f64 = 0.25;

/// What a frame of the clock brings ([`tick`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// "Sensors detect … reinforcement fleet approaching.", with
    /// `snd ` 154.
    Announce,
    /// The fleet comes.
    Arrive,
}

/// A frame of `clock` on the player's system, whose fleet is `fleet`
/// (if any) and whose government is `named` or not: what it brings (see
/// the module docs).
pub fn tick(clock: &mut Clock, fleet: Option<Fleet>, named: bool) -> Option<Event> {
    let fleet = fleet?;
    if clock.timer <= 0 {
        return None;
    }
    clock.timer -= 1;
    if clock.timer <= 0 {
        clock.timer = ARRIVED;
        clock.countdown = fleet.interval_days();
        return Some(Event::Arrive);
    }
    if f64::from(clock.timer) >= f64::from(fleet.time) * ANNOUNCE_SHARE || clock.countdown > 0 {
        return None;
    }
    clock.countdown = fleet.interval_days();
    named.then_some(Event::Announce)
}

/// A system's countdown after a day, `countdown` before it: one less,
/// stopping at 0 (@0xb739-0xb746).
#[must_use]
pub fn day(countdown: u16) -> u16 {
    countdown.saturating_sub(1)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// Federation's fleet: `ReinfTime` 900, `ReinfIntrval` 2.
    const FLEET: Fleet = Fleet {
        govt: Some(GovtId(128)),
        time: 900,
        interval: 2,
    };

    /// A ship of a government allied with the fleet's, at odds of 2
    /// against a `MaxOdds` of 2 (above half of it), in a system whose
    /// clock is idle: every gate holds.
    fn facts() -> ReinforcementFacts {
        ReinforcementFacts {
            has_govt: true,
            fleet: Some(FLEET),
            clock: Clock::default(),
            odds: 2.0,
            allied: true,
            fleet_max_odds: 2.0,
            force: false,
            inhibitors: Inhibitors::default(),
        }
    }

    #[test]
    fn a_ship_whose_every_gate_holds_calls_the_fleet_for_its_reinf_time() {
        assert_eq!(call(&facts()), Called::Coming(900));
        let mut clock = Clock::default();
        clock.answer(Called::Coming(900));
        assert_eq!(
            clock,
            Clock {
                timer: 900,
                countdown: 0
            }
        );
    }

    /// A change to the facts that fails one gate.
    type Failing = fn(&mut ReinforcementFacts);

    #[test]
    fn each_gate_failing_in_turn_calls_nothing() {
        let failing: [(&str, Failing); 8] = [
            ("no government", |f| f.has_govt = false),
            ("no fleet", |f| f.fleet = None),
            ("a fleet coming", |f| f.clock.timer = 1),
            ("called lately", |f| f.clock.countdown = 1),
            ("no odds", |f| f.odds = 0.0),
            ("odds of -1", |f| f.odds = -1.0),
            ("not allied", |f| f.allied = false),
            ("odds of half the fleet's MaxOdds", |f| f.odds = 1.0),
        ];
        for (why, fail) in failing {
            let mut facts = facts();
            fail(&mut facts);
            assert_eq!(call(&facts), Called::No, "{why}");
        }
        let mut stronger = facts();
        stronger.odds = 1.01;
        assert_eq!(call(&stronger), Called::Coming(900), "strictly above half");
    }

    #[test]
    fn force_passes_only_the_strength_test() {
        let mut weak = facts();
        weak.odds = 0.1;
        weak.force = true;
        assert_eq!(call(&weak), Called::Coming(900));
        let mut unallied = weak;
        unallied.allied = false;
        assert_eq!(call(&unallied), Called::No, "still allied");
        let mut odd_less = weak;
        odd_less.odds = 0.0;
        assert_eq!(call(&odd_less), Called::No, "still above 0");
        let mut coming = weak;
        coming.clock.timer = 5;
        assert_eq!(call(&coming), Called::No, "still idle");
    }

    #[test]
    fn each_inhibitor_marks_the_call_once_every_other_gate_holds() {
        let each = [
            Inhibitors {
                global: true,
                ..Inhibitors::default()
            },
            Inhibitors {
                ship_govt: true,
                ..Inhibitors::default()
            },
            Inhibitors {
                fleet_govt: true,
                ..Inhibitors::default()
            },
        ];
        for inhibitors in each {
            let mut inhibited = facts();
            inhibited.inhibitors = inhibitors;
            assert_eq!(call(&inhibited), Called::Marked, "{inhibitors:?}");
            let mut clock = Clock::default();
            clock.answer(Called::Marked);
            assert_eq!(
                clock,
                Clock {
                    timer: 0,
                    countdown: 1
                },
                "no timer"
            );
            let mut weak = inhibited;
            weak.odds = 0.5;
            assert_eq!(call(&weak), Called::No, "only after the strength test");
        }
    }

    #[test]
    fn nothing_answered_leaves_the_clock() {
        let mut clock = Clock {
            timer: 3,
            countdown: 2,
        };
        clock.answer(Called::No);
        assert_eq!(
            clock,
            Clock {
                timer: 3,
                countdown: 2
            }
        );
    }

    /// The events of `ticks` frames of a clock started at `time` for a
    /// fleet of that `ReinfTime`, named or not, each with the frame it
    /// came on (from 1), and the clock they leave.
    fn run(time: i16, named: bool, ticks: u32) -> (Vec<(u32, Event)>, Clock) {
        let fleet = Fleet { time, ..FLEET };
        let mut clock = Clock::default();
        clock.answer(Called::Coming(i32::from(time)));
        let events = (1..=ticks)
            .filter_map(|frame| tick(&mut clock, Some(fleet), named).map(|event| (frame, event)))
            .collect();
        (events, clock)
    }

    #[test]
    fn the_fleet_is_announced_below_a_quarter_of_its_time_and_arrives_at_its_end() {
        let (events, clock) = run(900, true, 900);
        assert_eq!(
            events,
            [(676, Event::Announce), (900, Event::Arrive)],
            "the timer 224 < 225 on frame 676"
        );
        assert_eq!(
            clock,
            Clock {
                timer: ARRIVED,
                countdown: 2
            }
        );
    }

    #[test]
    fn the_announcement_sets_the_countdown_to_the_interval() {
        let (_, clock) = run(900, true, 676);
        assert_eq!(
            clock,
            Clock {
                timer: 224,
                countdown: 2
            }
        );
        let (events, _) = run(900, true, 899);
        assert_eq!(events, [(676, Event::Announce)], "once");
    }

    #[test]
    fn an_unnamed_fleet_sets_the_countdown_with_no_announcement() {
        let (events, clock) = run(900, false, 676);
        assert_eq!(events, []);
        assert_eq!(clock.countdown, 2);
        let (events, _) = run(900, false, 900);
        assert_eq!(events, [(900, Event::Arrive)]);
    }

    #[test]
    fn a_reinf_time_of_4_arrives_unannounced_and_5_is_announced_at_1() {
        for time in 1..=4 {
            let (events, _) = run(time, true, 10);
            assert_eq!(
                events,
                [(u32::try_from(time).expect("small"), Event::Arrive)],
                "{time}"
            );
        }
        let (events, _) = run(5, true, 10);
        assert_eq!(
            events,
            [(4, Event::Announce), (5, Event::Arrive)],
            "the timer 1 < 1.25"
        );
    }

    #[test]
    fn a_reinf_time_of_0_or_less_never_arrives() {
        for time in [0, -5] {
            let (events, clock) = run(time, true, 10);
            assert_eq!(events, [], "{time}");
            assert_eq!(clock.timer, i32::from(time));
            assert!(!clock.coming());
        }
    }

    #[test]
    fn with_no_fleet_or_the_timer_idle_nothing_happens() {
        let mut clock = Clock {
            timer: 10,
            countdown: 0,
        };
        assert_eq!(tick(&mut clock, None, true), None);
        assert_eq!(clock.timer, 10, "no fleet: the timer stands");
        for timer in [0, ARRIVED] {
            let mut idle = Clock {
                timer,
                countdown: 0,
            };
            assert_eq!(tick(&mut idle, Some(FLEET), true), None);
            assert_eq!(
                idle,
                Clock {
                    timer,
                    countdown: 0
                }
            );
        }
    }

    #[test]
    fn a_countdown_already_set_holds_the_announcement() {
        let mut clock = Clock {
            timer: 10,
            countdown: 1,
        };
        let fleet = Fleet { time: 100, ..FLEET };
        assert_eq!(tick(&mut clock, Some(fleet), true), None);
        assert_eq!(
            clock,
            Clock {
                timer: 9,
                countdown: 1
            }
        );
    }

    #[test]
    fn an_interval_of_0_or_less_counts_as_1() {
        assert_eq!(
            Fleet {
                interval: 0,
                ..FLEET
            }
            .interval_days(),
            1
        );
        assert_eq!(
            Fleet {
                interval: -3,
                ..FLEET
            }
            .interval_days(),
            1
        );
        assert_eq!(
            Fleet {
                interval: 7,
                ..FLEET
            }
            .interval_days(),
            7
        );
    }

    #[test]
    fn a_coming_fleet_is_one_whose_timer_runs() {
        assert!(
            Clock {
                timer: 1,
                countdown: 0
            }
            .coming()
        );
        assert!(
            !Clock {
                timer: 0,
                countdown: 0
            }
            .coming()
        );
        assert!(
            !Clock {
                timer: ARRIVED,
                countdown: 3
            }
            .coming()
        );
    }

    #[test]
    fn the_day_count_goes_down_by_one_and_stops_at_0() {
        assert_eq!(day(2), 1);
        assert_eq!(day(1), 0);
        assert_eq!(day(0), 0);
    }
}
