//! The crate's own test support: the lower crates' builders and fakes
//! re-exported, and the session-driving helpers.

/// An active mission carrying cargo, from the mission's test support.
pub(crate) use crate::mission::fixture::carrying;
/// The default NPC and the count-waiting one, from the traffic's test
/// support.
pub(crate) use crate::traffic::fixture::{count_waiting, npc};
/// The [`Chance`] mocks and the fast ship class, from the foundation
/// crate's test support.
pub(crate) use nova_sim_base::fixture::{Draws, FAST, Ranged, Scripted};
/// The catalog's record builders and control-bit mocks, from the catalog
/// crate's test support.
pub(crate) use nova_sim_catalog::fixture::{
    AllowAll, FakePilotCatalog, RefuseBits, START, catalog, cron, govt, hull, mission, nebula,
    outfit, person, planet, rank, ship, star, starting, weapon,
};

use crate::ai::{Behaviour, Goal, Reaction, Surroundings};
use crate::catalog::{
    PilotCatalog, ShipId, StellarId, StellarLife, StellarMark, SystemId, SystemVisibility,
    TrafficCatalog,
};
use crate::chance::{Chance, NeverFires};
use crate::combat::armament::Trigger;
use crate::combat::{Rules, ShipRef, Strike};
use crate::control::Test;
use crate::flight::{Controls, Turn};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::hyperspace::{JumpRefusal, MIN_JUMP_DISTANCE};
use crate::landing::{LandOutcome, LandingRefusal};
use crate::session::LandPress;
use crate::session::Session;
use crate::traffic::npc::{Npc, NpcId};

/// A bar, stellar 128, of tech level 7 (above 5, so no low-tech
/// discount), at the centre, so a ship taking off flies out as one
/// starting.
pub(crate) fn bar() -> crate::catalog::LandingSite {
    use crate::landing::StellarFlags;
    crate::catalog::LandingSite {
        flags: StellarFlags::CAN_LAND | StellarFlags::BAR,
        tech_level: 7,
        ..planet(128, 0.0, 0.0)
    }
}

/// Stellar `id` of `Strength` 100 and `dead_time`, starting the game
/// whole, with no `OnRegen`.
pub(crate) fn life(id: i16, dead_time: i16) -> StellarLife {
    StellarLife {
        id: StellarId(id),
        strength: 100,
        dead_time,
        flags2: 0,
        explosion: None,
        on_destroy: crate::control::Script::default(),
        on_regen: crate::control::Script::default(),
    }
}

/// `catalog` with system 130's planet 129 at (-2000, 0): as far out as
/// at (2000, 0), but on the side the 999 bound on a destination does not
/// reach (see [`destination`](crate::traffic::destination)), so an idle
/// NPC drawing the last candidate heads for it, far from 128.
pub(crate) fn far_129_reachable(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
    for (_, sites) in &mut catalog.sites {
        for site in sites.iter_mut().filter(|site| site.id == StellarId(129)) {
            site.position = Vec2::new(-2000.0, 0.0);
        }
    }
    catalog
}

/// `catalog` with variants in system 130 marked as `_StellarsAreIdentical`
/// reads them: 150 at 128's place with its name "Home" (30,-40), 151 at
/// 129's with its name "Varia" (2000,0), and two that miss 129 by one:
/// 152 at its place named "Other", 153 named "Varia" at 2001,0.
pub(crate) fn with_variants(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
    let sites = &mut catalog
        .sites
        .iter_mut()
        .find(|(system, _)| *system == SystemId(130))
        .expect("130 listed")
        .1;
    sites.extend([
        planet(150, -2000.0, 0.0),
        planet(151, 0.0, 2000.0),
        planet(152, 0.0, -2000.0),
        planet(153, -2000.0, -2000.0),
    ]);
    let mark = |id, place, name: &str| StellarMark {
        id: StellarId(id),
        place,
        name: name.to_owned(),
    };
    catalog.marks = vec![
        mark(128, (30, -40), "Home"),
        mark(129, (2000, 0), "Varia"),
        mark(150, (30, -40), "Home"),
        mark(151, (2000, 0), "Varia"),
        mark(152, (2000, 0), "Other"),
        mark(153, (2001, 0), "Varia"),
    ];
    catalog
}

/// `catalog` with a system 134 at 131's map place, (600, 0), listing a
/// stellar 141 at the centre, as 131 lists 140: 131 shows while b50 is
/// clear, 134 once it is set. With `marked`, 140 and 141 are marked with
/// one place and name, so b50 swaps a stellar for its variant (Tri'niphu
/// 212 for 657 after b1502, in miniature); without, they differ by name.
pub(crate) fn swapped(mut catalog: FakePilotCatalog, marked: bool) -> FakePilotCatalog {
    catalog.systems.push(SystemId(134));
    catalog.star_map.push(star(134, (600.0, 0.0), &[]));
    catalog
        .sites
        .push((SystemId(134), vec![planet(141, 0.0, 0.0)]));
    let mark = |id, name: &str| StellarMark {
        id: StellarId(id),
        place: (0, 0),
        name: name.to_owned(),
    };
    let other = if marked { "Tre'ar Zalom" } else { "Elsewhere" };
    catalog
        .marks
        .extend([mark(140, "Tre'ar Zalom"), mark(141, other)]);
    catalog.visibility.extend([
        SystemVisibility {
            id: SystemId(131),
            visibility: Test::parse("!b50"),
        },
        SystemVisibility {
            id: SystemId(134),
            visibility: Test::parse("b50"),
        },
    ]);
    catalog
}

/// The control bit that hides `system` in a [`hideable`] catalog: 9000
/// past its ID.
pub(crate) fn hiding_bit(system: i16) -> crate::control::Bit {
    u16::try_from(system)
        .ok()
        .and_then(|id| crate::control::Bit::new(9000 + id))
        .expect("a system of a hideable catalog has a hiding bit")
}

/// Makes `catalog` hideable: each system on its map that has no
/// `Visibility` yet is visible unless its [`hiding_bit`] is set. A test
/// hides it by setting the bit and refreshing the galaxy, as play does, so
/// every later refresh keeps it hidden.
pub(crate) fn hideable(catalog: &mut FakePilotCatalog) {
    for system in &catalog.star_map {
        if !catalog.visibility.iter().any(|test| test.id == system.id) {
            catalog.visibility.push(SystemVisibility {
                id: system.id,
                visibility: Test::parse(&format!("!b{}", hiding_bit(system.id.0).get())),
            });
        }
    }
}

/// Every NPC keeps its goal, its target and its trigger: the traffic of a
/// pass whose test is about something else.
#[derive(Debug)]
pub(crate) struct Passive;

impl Behaviour for Passive {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
        npc.trigger
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.target
    }
}

/// Every NPC keeps its goal, its target and its trigger, as [`Passive`],
/// but answers the hits it takes as Nova's AI does
/// ([`NovaAi`](crate::ai::NovaAi)): the traffic of a pass whose test is
/// about the answers.
#[derive(Debug, Default)]
pub(crate) struct Answering(crate::ai::NovaAi);

impl Behaviour for Answering {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        Passive.decide(npc, around, chance)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        Passive.trigger(npc, around)
    }

    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        Passive.target(npc, around)
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        self.0.react(npc, strike, around)
    }
}

/// One advance pass under `controls` ([`Session::game_work`]), the traffic
/// drawn from `catalog` keeping its goals ([`Passive`]), the fight by the
/// default rules, no chance firing. Asserts no rescue ended it.
pub(crate) fn work(
    session: &mut Session,
    catalog: &(impl PilotCatalog + TrafficCatalog),
    controls: Controls,
) {
    work_with(
        session,
        catalog,
        controls,
        &Passive,
        Rules::default(),
        &mut NeverFires,
    );
}

/// One advance pass under `controls` ([`Session::game_work`]), the traffic
/// drawn from `catalog` deciding as `behaviour` says, the fight by
/// `rules`, drawing on `chance`. Asserts no rescue ended it.
pub(crate) fn work_with(
    session: &mut Session,
    catalog: &(impl PilotCatalog + TrafficCatalog),
    controls: Controls,
    behaviour: &(impl Behaviour + ?Sized),
    rules: Rules<'_>,
    chance: &mut (impl Chance + ?Sized),
) {
    let rescued = session.game_work(controls, catalog, behaviour, rules, chance);
    assert!(!rescued, "no rescue ends the pass");
}

/// One advance pass of the way to a jump under `controls`, the traffic
/// drawn from `catalog` deciding as Nova's does ([`NovaAi`]), so the
/// ships the player leads follow it, the fight by the default rules, no
/// chance firing.
///
/// [`NovaAi`]: crate::ai::NovaAi
fn fly(session: &mut Session, catalog: &(impl PilotCatalog + TrafficCatalog), controls: Controls) {
    work_with(
        session,
        catalog,
        controls,
        &crate::ai::NovaAi::default(),
        Rules::default(),
        &mut NeverFires,
    );
}

/// Flies the ship out from the centre until it is at least
/// [`MIN_JUMP_DISTANCE`] away, a pass at a time ([`fly`], the traffic
/// drawn from `catalog`): it first turns to face away from the centre
/// (Down faces against its motion), then thrusts once within a turn of
/// it.
pub(crate) fn fly_out(session: &mut Session, catalog: &(impl PilotCatalog + TrafficCatalog)) {
    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
        face: crate::flight::Face::Off,
        afterburner: false,
    };
    for _ in 0..3000 {
        let player = *session.player();
        if player.position.length() >= MIN_JUMP_DISTANCE {
            return;
        }
        let outward = if player.position.length() > 0.0 {
            crate::flight::heading_of(player.position)
        } else {
            0.0
        };
        let off = crate::flight::shortest_turn(player.heading, outward).abs();
        // Down comes within a turn of the heading against the motion,
        // which is outward while the ship drifts in, and stops there.
        let turn = session.handling().turn_rate.trunc().max(1.0);
        let controls = if off <= turn {
            THRUST
        } else if crate::player_turn::reverse_aim(player.velocity).is_some() {
            Controls {
                reverse: true,
                ..Controls::default()
            }
        } else {
            Controls {
                turn: Turn::Right,
                ..Controls::default()
            }
        };
        fly(session, catalog, controls);
    }
    panic!("never got out: {:?}", session.player());
}

/// Presses L until it lands or is refused, the dock approach alone run
/// between presses (not a whole tick, whose motion, fuel and timers the
/// tests calling this do not expect): a press that requests is followed
/// by the approach and another press, up to three. The dock request's
/// own messages and sounds raised on the way (its line, "No response.",
/// the greeting and traffic control's sound) are forgotten, as the
/// `clearance` module's tests pin them. Gives the stellar landed on, or
/// the refusal.
pub(crate) fn land_now(session: &mut Session) -> Result<StellarId, LandingRefusal> {
    let before = session.chatter_marks();
    let landed = (|| {
        for press in 0..3 {
            if press > 0 {
                session.dock_approach();
            }
            if let LandPress::Outcome(LandOutcome::Landed(stellar)) = session.land()? {
                return Ok(stellar);
            }
        }
        panic!(
            "L requested three times and never landed: {:?}",
            session.nav_target()
        );
    })();
    session.forget_dock_chatter(before);
    landed
}

/// The stellar a session just flown from a save resumed on: asserts it
/// flies, at rest, nothing about a landing run, and gives the stellar the
/// pilot last landed on, which the load keeps (see [`Session::load`]).
pub(crate) fn resumed_on(session: &Session) -> Option<StellarId> {
    assert_eq!(session.landed(), None, "resumed in flight");
    assert_eq!(session.player().velocity, Vec2::ZERO, "at rest");
    session.pilot().stellar()
}

/// Plots a course to `to`, flies out and jumps, and arrives, no chance
/// firing on the way.
pub(crate) fn jump(session: &mut Session, catalog: &FakePilotCatalog, to: i16) -> Option<SystemId> {
    jump_with(session, catalog, to, &mut NeverFires)
}

/// Plots a course to `to`, flies out and jumps, and arrives, the day's
/// chances rolled on `chance`.
pub(crate) fn jump_with(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    to: i16,
    chance: &mut impl Chance,
) -> Option<SystemId> {
    if session.course().last() != Some(&SystemId(to)) {
        session.plot_course(SystemId(to)).expect("a route");
    }
    fly_out(session, catalog);
    begin_jump_now(session, catalog).expect("jumps");
    session.arrive(catalog, chance)
}

/// Presses J, and if the jump is accepted, works passes with no keys held
/// through the pre-jump stage until the jump has begun ([`fly`], the
/// traffic drawn from `catalog`). Gives what J gave.
pub(crate) fn begin_jump_now(
    session: &mut Session,
    catalog: &(impl PilotCatalog + TrafficCatalog),
) -> Result<SystemId, JumpRefusal> {
    let next = session.begin_jump()?;
    for _ in 0..1000 {
        if session.jumping().is_some() {
            return Ok(next);
        }
        fly(session, catalog, Controls::default());
    }
    panic!("the jump never began: {:?}", session.player());
}

/// A jump's arrival's draws: the roll table's
/// [`ROLLED`](crate::mission::ROLLED), each its last outcome, then
/// `draws`.
pub(crate) fn arriving(draws: &[u32]) -> Draws {
    let table = std::iter::repeat_n(99, crate::mission::ROLLED);
    Draws::of(&table.chain(draws.iter().copied()).collect::<Vec<_>>())
}

/// A slow ship, 0.6 of a pixel a tick (0.9 led, under the 1 a landing
/// allows), gaining a unit a tick (its `Flags` 0x0008 letting its hull
/// regenerate): a jump from 130 arrives at its top speed, slow enough to
/// land, over planet 140 at 131's edge, where the original's arrival puts
/// it (see the `jump_arrival` module).
pub(crate) fn edge_lander() -> FakePilotCatalog {
    FakePilotCatalog {
        ships: vec![(
            ShipId(128),
            Ok(ShipFields {
                speed: 60,
                fuel_regen: 1,
                flags: crate::stats::FUEL_REGEN_HULL,
                ..FAST
            }),
        )],
        sites: vec![(SystemId(131), vec![planet(140, -1350.0, 0.0)])],
        ..catalog()
    }
}

/// No ship drawn anywhere, so a click in space falls through to the
/// stellars.
pub(crate) struct NoShips;

impl crate::hover::DrawnShips for NoShips {
    fn npc(&self, _id: NpcId) -> Option<crate::hover::DrawnBox> {
        None
    }

    fn player(&self) -> Option<crate::hover::DrawnBox> {
        None
    }
}

/// The stellar a click at `at` in space, with no ship drawn, selects.
pub(crate) fn click_stellar(session: &mut Session, at: Vec2) -> Option<StellarId> {
    match session.click_in_space(at, &NoShips) {
        Some(crate::hover::Hover::Stellar(id)) => Some(id),
        _ => None,
    }
}

/// A bomb outfit (`ModType` [`SABOTAGE_BOMB`](crate::SABOTAGE_BOMB)), with
/// no `bööm`: the sabotage tests' and the message line's.
pub(crate) const BOMB: i16 = 262;

/// The [`BOMB`] outfit's record.
pub(crate) fn bomb_outfit() -> crate::catalog::OutfitRecord {
    outfit(BOMB, &[(crate::SABOTAGE_BOMB, 0)])
}

/// The fighter bay whose fighter is an escape ship.
pub(crate) const ESCAPE_BAY: i16 = 160;

/// The outfit of the escape ships aboard, the bay's ammunition.
pub(crate) const ESCAPE_FIGHTERS: i16 = 310;

/// The escape ship, a fighter of `Flags` 0x8000.
pub(crate) const ESCAPE_SHIP: i16 = 144;

/// `catalog` with a fighter bay of [`ESCAPE_SHIP`]s carried as
/// [`ESCAPE_FIGHTERS`], an escape ship of 100 shield, 200 armour and 300
/// fuel, long named "Federation Viper", so a player owning the fighters
/// ejects into one.
pub(crate) fn with_escape_ship(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
    catalog.weapons.push(crate::catalog::WeaponRecord {
        guidance: crate::bay::FIGHTER_BAY,
        ammo_type: ESCAPE_SHIP,
        ..weapon(ESCAPE_BAY)
    });
    catalog.outfits.push(outfit(
        ESCAPE_FIGHTERS,
        &[(crate::combat::armament::MOD_AMMO, ESCAPE_BAY)],
    ));
    let mut viper = ship(
        ESCAPE_SHIP,
        ShipFields {
            shield: 100,
            armor: 200,
            fuel: 300,
            ..FAST
        },
    );
    viper.flags = crate::escape::ESCAPE_SHIP_FLAG;
    viper.long_name = "Federation Viper".to_owned();
    catalog.ship_records.push(viper);
    catalog
}
