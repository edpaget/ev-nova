//! Each thrust window the autopilot flies reads `_ShipTurnRate` at the
//! ship's slot of the AI loop, and `_HandleShip` turns by its own at the
//! ship's step (see [`slot_word`](super::slot_word)): the low level reads
//! `_ShipTurnRate` 24 times, all at the slot (@0x852ff, @0x85492, @0x856bc,
//! @0x85817, @0x85b6c, @0x85d14, @0x85f3d, @0x85fa6, @0x86539, @0x866b4,
//! @0x8671a, @0x868b0, @0x8691c, @0x86bc4, @0x87210, @0x872d0, @0x875ab,
//! @0x878a6, @0x87a40, @0x87eb6, @0x88186, @0x88407, @0x88545, @0x886b9),
//! and the step's turn reads it after its ion block (@0x344b0). Between the
//! two the shot loop's `_IonizeShip` (@0x3612d) and the step's shed
//! (@0x342b2-0x34421) move the charge, which `_ShipTurnRate` reads.

#![allow(clippy::float_cmp)]

use super::autopilot::{self, Beside, Readings, TopSpeeds};
use super::npc::Npc;
use super::slot_word::AtStep;
use crate::ai::Goal;
use crate::ai::fire::Manoeuvre;
use crate::ai::pursuit::{Body, Frame};
use crate::bay::{Carrier, DOCK_TURN_SLACK};
use crate::catalog::LandingSite;
use crate::chance::NeverFires;
use crate::combat::ShipRef;
use crate::combat::armament::Arsenal;
use crate::escort::{EscortDuty, FormationSlot};
use crate::flight::{self, FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::npc::NpcId;
use nova_sim_base::fixture::FAST;
use nova_sim_catalog::fixture::planet;
use nova_sim_combat::asteroid::Asteroid;
use std::collections::BTreeMap;

/// The charge a hit leaves, of an `IonizeMax` of 400: its turn slowed.
const CHARGED: f32 = 300.0;

/// One window: a ship in a substate, the ship its flight is about, what it
/// reads beside it, the stellars, and the window against its turn rate.
struct Row {
    /// The substate, for the failure messages.
    name: &'static str,
    /// The ship, its heading set by the test.
    npc: Npc,
    /// The ship its flight is about.
    other: Option<ShipState>,
    /// What it reads beside it.
    beside: Beside,
    /// The system's stellars.
    sites: Vec<LandingSite>,
    /// Its window, in whole degrees off the heading it wants, strictly,
    /// for a turn rate.
    window: fn(f32) -> f32,
}

/// A ship of [`FAST`] at `position` moving `velocity` in `goal`, of
/// `IonizeMax` 400.
fn ship(goal: Goal, position: Vec2, velocity: Vec2) -> Npc {
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.goal = goal;
    npc.state = ShipState {
        position,
        velocity,
        heading: 0.0,
    };
    npc.stats.ion.class_capacity = 400.0;
    npc
}

/// `npc` fighting the player by a frame of one `manoeuvre` body wanting
/// 0.
fn fighting(goal: Goal, manoeuvre: Manoeuvre) -> Npc {
    let mut npc = ship(goal, Vec2::ZERO, Vec2::ZERO);
    npc.pursuit.frame = Some(Frame {
        goal,
        bodies: [
            Some(Body {
                manoeuvre,
                heading: 0,
            }),
            None,
        ],
        speed_cap: None,
    });
    npc
}

const FOE: ShipRef = ShipRef::Player;

/// The fights' windows, each body wanting 0 of a target far off.
fn fights() -> Vec<Row> {
    let far = Some(ShipState {
        position: Vec2::new(0.0, -3000.0),
        ..ShipState::default()
    });
    let plain = |name, npc, window| Row {
        name,
        npc,
        other: far,
        beside: Beside::default(),
        sites: Vec::new(),
        window,
    };
    vec![
        // Sub 0x12 (window @0x852ff), sub 6 (@0x85f3d), sub 5 (@0x85d14)
        // and sub 7 (@0x868b0-0x8695e).
        plain(
            "sub 0x12",
            fighting(Goal::Attack(FOE), Manoeuvre::Swarm),
            |turn| turn + 20.0,
        ),
        plain(
            "sub 6",
            fighting(Goal::Attack(FOE), Manoeuvre::Dogfight),
            |turn| turn + 15.0,
        ),
        plain(
            "sub 5",
            fighting(Goal::Flee(FOE), Manoeuvre::Flee),
            |turn| turn + 20.0,
        ),
        plain(
            "sub 7",
            fighting(Goal::Attack(FOE), Manoeuvre::Approach),
            |turn| 4.0 * turn,
        ),
        // Sub 0xe moving, its retro heading 0 (@0x87210).
        plain(
            "sub 0xe",
            {
                let mut sniper = fighting(Goal::Snipe(FOE), Manoeuvre::Snipe);
                sniper.state.velocity = Vec2::new(0.0, 2.0);
                sniper
            },
            |turn| turn + 1.0,
        ),
    ]
}

/// The other flights' windows.
fn flights() -> Vec<Row> {
    let mut docking = ship(Goal::Dock(ShipRef::Npc(NpcId(2))), Vec2::ZERO, Vec2::ZERO);
    docking.carrier = Some(Carrier {
        ship: ShipRef::Npc(NpcId(2)),
        window: 100.0,
        reach: 10.0,
    });
    let mut escort = ship(
        Goal::Formation { guard: None },
        Vec2::new(10.0, 0.0),
        Vec2::new(0.0, 1.0),
    );
    escort.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: None,
    });
    let mut miner = ship(Goal::Mine, Vec2::ZERO, Vec2::ZERO);
    miner.miner = crate::ai::miner::MINES;
    miner.hull.class_turn = 2.0;
    vec![
        // Sub 2 toward its stellar (@0x856bc).
        Row {
            name: "sub 2",
            npc: ship(
                Goal::Land(crate::catalog::StellarId(140)),
                Vec2::ZERO,
                Vec2::ZERO,
            ),
            other: None,
            beside: Beside::default(),
            sites: vec![planet(140, 0.0, -3000.0)],
            window: |turn| turn + 5.0,
        },
        // Sub 3 out from the centre (@0x85817).
        Row {
            name: "sub 3",
            npc: ship(Goal::JumpOut, Vec2::new(0.0, -500.0), Vec2::ZERO),
            other: None,
            beside: Beside::default(),
            sites: Vec::new(),
            window: |turn| turn + 3.0,
        },
        // Sub 1's brake, its retro heading 0, a full brake (@0x85492).
        Row {
            name: "sub 1",
            // A wait's stop (@0x8f101): goal 0 flies sub 0, no brake.
            npc: ship(Goal::Wait, Vec2::ZERO, Vec2::new(0.0, 3.0)),
            other: None,
            beside: Beside::default(),
            sites: Vec::new(),
            window: |turn| turn + 1.0,
        },
        // Sub 8 inside its dock window, the carrier above it (@0x87eb6).
        Row {
            name: "sub 8",
            npc: docking,
            other: Some(ShipState {
                position: Vec2::new(0.0, -50.0),
                ..ShipState::default()
            }),
            beside: Beside::default(),
            sites: Vec::new(),
            window: |turn| turn + DOCK_TURN_SLACK,
        },
        // Sub 0xc's counter-thrust against its velocity off its leader's
        // (@0x87a40).
        Row {
            name: "sub 0xc",
            npc: escort,
            other: Some(ShipState::default()),
            beside: Beside::default(),
            sites: Vec::new(),
            window: |turn| turn + 1.0,
        },
        // Sub 0x13's chase (@0x88186) and sub 0x15's steer to its box
        // (@0x88545).
        Row {
            name: "sub 0x13",
            npc: miner,
            other: None,
            beside: Beside {
                rock: Some(Asteroid {
                    position: Vec2::new(0.0, -2000.0),
                    velocity: Vec2::ZERO,
                    frame: 0.0,
                    spin: 0.0,
                    strength: 30,
                    kind: 0,
                }),
                ..Beside::default()
            },
            sites: Vec::new(),
            window: |turn| turn + crate::ai::mine_orders::THRUST_SLACK,
        },
        Row {
            name: "sub 0x15",
            npc: ship(Goal::Scoop, Vec2::ZERO, Vec2::ZERO),
            other: None,
            beside: Beside {
                scoop_box: Some(Vec2::new(0.0, -2000.0)),
                ..Beside::default()
            },
            sites: Vec::new(),
            window: |turn| turn + crate::ai::mine_orders::THRUST_SLACK,
        },
    ]
}

/// The windows the autopilot flies, one a substate.
fn rows() -> Vec<Row> {
    let mut rows = fights();
    rows.extend(flights());
    rows
}

/// `npc`'s turn rate charged `charge`.
fn turn_at(npc: &Npc, charge: f32) -> f32 {
    let mut ship = npc.clone();
    ship.ion.charge = charge;
    let top = ship.stats.handling.max_speed;
    ship.flying_handling(top, FlightRules::default()).turn_rate
}

/// What `row`'s ship does off the heading it wants by `off` whole degrees:
/// its low level at its slot charged `at_slot`, then its step charged
/// `at_step`. Whether its word thrust, its velocity as its slot left it and
/// after its step, and its heading after its step; with the heading it
/// wanted.
fn flown(row: &Row, off: i16, at_slot: f32, at_step: f32) -> (bool, [Vec2; 2], f32, i16) {
    let weapons = Arsenal::default();
    let rules = (FlightRules::default(), Readings::default(), &weapons);
    let speeds = TopSpeeds::own_of(&row.npc);
    let quarry = (row.other.as_ref(), false, row.beside);
    let widths = BTreeMap::new();
    // The heading it wants: none of the flights here reads its own.
    let wanted = {
        let mut dry = row.npc.clone();
        let (word, _) = autopilot::low_level(
            &mut dry,
            &row.sites,
            &widths,
            quarry,
            speeds,
            rules,
            &mut NeverFires,
        );
        word.wanted().expect("a heading")
    };
    let mut npc = row.npc.clone();
    npc.state.heading = flight::normalized(f32::from(wanted + off));
    npc.ion.charge = at_slot;
    let (word, outcome) = autopilot::low_level(
        &mut npc,
        &row.sites,
        &widths,
        quarry,
        speeds,
        rules,
        &mut NeverFires,
    );
    assert_eq!((row.name, outcome), (row.name, None));
    assert_eq!(word.wanted(), Some(wanted), "{}", row.name);
    let slot = npc.state.velocity;
    npc.ion.charge = at_step;
    let adjust = npc
        .flying_handling(speeds.own, FlightRules::default())
        .accel;
    let at_step = AtStep::of_npc(&npc, 1.0, FlightRules::default(), adjust);
    let by = (FlightRules::default(), RuleSource::Engine, false);
    autopilot::step(&mut npc, &word, at_step, speeds, by, &mut NeverFires);
    (
        word.thrusts(),
        [slot, npc.state.velocity],
        npc.state.heading,
        wanted,
    )
}

/// The heading a ship at `heading` has once `_HandleShip` turns it toward
/// `wanted` at `rate`.
fn turned(heading: f32, wanted: i16, rate: f32) -> f32 {
    let mut state = ShipState {
        heading,
        ..ShipState::default()
    };
    let turn = flight::turn_to(&mut state, wanted, rate);
    flight::turned(state.heading, turn, rate)
}

#[test]
fn every_thrust_window_reads_the_turn_rate_at_the_slot() {
    for row in rows() {
        let (slot, step) = (turn_at(&row.npc, CHARGED), turn_at(&row.npc, 0.0));
        assert!(slot < step, "{}: the charge slows its turn", row.name);
        // Charged at its slot and shed by its step: just outside its
        // slot's window, inside its step's.
        #[allow(clippy::cast_possible_truncation)]
        let outside = (row.window)(slot).ceil() as i16;
        assert!(
            f32::from(outside) < (row.window)(step),
            "{}: a gap between the two",
            row.name
        );
        let (thrust, [at_slot, after], heading, wanted) = flown(&row, outside, CHARGED, 0.0);
        assert!(!thrust, "{}: out of its slot's window", row.name);
        assert_eq!(after, at_slot, "{}: no thrust", row.name);
        let from = flight::normalized(f32::from(wanted + outside));
        assert_eq!(heading, turned(from, wanted, step), "{}", row.name);
        // Uncharged at its slot and hit before its step: inside its slot's
        // window, so it thrusts, and it turns by its step's slowed rate.
        let inside = outside - 1;
        let (thrust, [at_slot, after], heading, wanted) = flown(&row, inside, 0.0, CHARGED);
        assert!(thrust, "{}: in its slot's window", row.name);
        assert_ne!(after, at_slot, "{}: it thrust", row.name);
        let from = flight::normalized(f32::from(wanted + inside));
        assert_eq!(heading, turned(from, wanted, slot), "{}", row.name);
        assert_ne!(heading, turned(from, wanted, step), "{}", row.name);
    }
}

#[test]
fn a_docking_ships_window_reads_the_turn_rate_at_its_slot_too() {
    // Sub 9's window (@0x878a6) was already the slot's
    // (`plunder_dock::low_level`, the traffic's `dispatch_dock`): charged
    // at its slot and shed by its step, just outside its slot's turn and
    // one, it thrusts nothing and turns by its step's rate.
    use crate::traffic::plunder_dock::{self, DockSub, Docking};
    use crate::traffic::slot_word;
    let victim = ShipRef::Npc(NpcId(9));
    let mut pirate = ship(
        Goal::Plunder {
            ship: victim,
            hopeless: false,
        },
        Vec2::ZERO,
        Vec2::ZERO,
    );
    pirate.docking = Some(Docking {
        ship: victim,
        sub: DockSub::Approach,
    });
    let (slot, step) = (turn_at(&pirate, CHARGED), turn_at(&pirate, 0.0));
    #[allow(clippy::cast_possible_truncation)]
    let outside = (slot + plunder_dock::THRUST_SLACK).ceil() as i16;
    assert!(f32::from(outside) < step + plunder_dock::THRUST_SLACK);
    pirate.state.heading = f32::from(outside);
    pirate.ion.charge = CHARGED;
    let far = ShipState {
        position: Vec2::new(0.0, -3000.0),
        ..ShipState::default()
    };
    let rules = FlightRules::default();
    let handling = pirate.flying_handling(pirate.stats.handling.max_speed, rules);
    let (word, _) =
        plunder_dock::low_level(&mut pirate, &far, &handling, rules, None, &mut NeverFires);
    assert!(!word.thrusts());
    pirate.ion.charge = 0.0;
    let at_step = AtStep::of_npc(&pirate, 1.0, rules, handling.accel);
    slot_word::handle_ship(&mut pirate, &word, at_step, rules);
    assert_eq!(pirate.state.velocity, Vec2::ZERO);
    assert_eq!(pirate.state.heading, turned(f32::from(outside), 0, step));
}
