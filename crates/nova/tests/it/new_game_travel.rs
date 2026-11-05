//! The new game flown over its checked-in fixture
//! (`nova_data::new_game::fixture::folder`), wired as the app wires it: the
//! mapped `GameData`, the rulebook `ruleset_rulebook` gives
//! `Ruleset::NewGame`, and a seeded chance. It covers every kind of link a
//! jump point can make: two roads from `hearth` to `verge`, a dead pair,
//! a one-way back door and `drift`, a system of gates alone.
//!
//! - **New Pilot** starts in the fixture's start system, in its hull.
//! - **The player** goes through every gate of every system and comes out
//!   at its partner's gate, its escort after it.
//! - **The traffic** comes out of and goes into every gate under the new
//!   game's readings; under the stock rulebook the gates past 999 are
//!   never used.
//!
//! Which gate an exit takes and how the traffic picks its gates are
//! nova-sim's and nova-sim-ai's rules, pinned there; this pins that the
//! data, the ruleset and the session meet. [`gates`], [`fly_through`] and
//! [`watch_traffic`] are shared with `sample_travel`, which runs them over
//! game-bible's sample.

use std::collections::BTreeSet;

use nova::chance::SplitMix;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_data::new_game::{self, NewGame};
use nova_data::records::stellar::Stellar;
use nova_data::{GameData, Registered};
use nova_sim::flight_frame::FlightEvent;
use nova_sim::gate::GateKind;
use nova_sim::traffic::destination::usable_gates as usable;
use nova_sim::{
    Controls, Escort, Governments, LandPress, NovaAi, Pilot, PilotCatalog, RuleKey, Rulebook,
    Rules, Session, ShipId, SimMessage, SimSound, StellarId, SystemId, Vec2,
};

/// The new game's rulebook, with no settings.
pub(crate) fn new_game_rules() -> Rulebook {
    ruleset_rulebook(None, Ruleset::NewGame.profile().readings).0
}

/// The checked-in fixture, mapped.
fn fixture() -> NewGame {
    new_game::open(&new_game::fixture::folder()).expect("the fixture lists")
}

/// One jump point's gate, as the mapped data holds it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Gate {
    /// The system it is in.
    pub system: SystemId,
    /// Its hypergate `spöb`.
    pub stellar: StellarId,
    /// Where it sits.
    pub at: Vec2,
    /// Its partner's gate, its system and where it sits.
    pub partner: (StellarId, SystemId, Vec2),
}

/// `stellar`'s record in `data`.
fn record<T: Registered + Clone>(data: &GameData, id: i16) -> T {
    data.get::<T>(id)
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

/// Every jump point's gate in `game`, in (system, point) order, with its
/// partner as its one link names it.
pub(crate) fn gates(game: &NewGame) -> Vec<Gate> {
    let system_of = |nova: i16| {
        game.ids
            .gates
            .iter()
            .find(|&(_, &gate)| gate == nova)
            .map(|(point, _)| SystemId(game.ids.systems[&point.system()]))
            .expect("a gate's partner is a gate")
    };
    let at = |stellar: &Stellar| Vec2::new(f32::from(stellar.x_pos), f32::from(stellar.y_pos));
    game.ids
        .gates
        .iter()
        .map(|(point, &nova)| {
            let spob: Stellar = record(&game.data, nova);
            let partner = spob.hyper_link[0].unwrap_or_else(|| panic!("{point} links"));
            let partner_gate: Stellar = record(&game.data, partner.0);
            Gate {
                system: SystemId(game.ids.systems[&point.system()]),
                stellar: StellarId(nova),
                at: at(&spob),
                partner: (partner, system_of(partner.0), at(&partner_gate)),
            }
        })
        .collect()
}

/// One pass of the game's work with no keys held, the traffic deciding
/// as Nova's AI does by `rules`.
fn pass(session: &mut Session, data: &GameData, rules: &Rulebook, chance: &mut SplitMix) {
    let rescued = session.game_work(
        Controls::default(),
        data,
        &NovaAi::from_rulebook(rules),
        Rules::default(),
        chance,
    );
    assert!(!rescued, "no rescue ends the pass");
}

/// A pilot to fly over `data`: its start's, or, in data with no start, the
/// checked-in fixture's start moved into `data`'s first hull.
pub(crate) fn pilot_for(game: &NewGame) -> Pilot {
    Pilot::new(&game.data, "Tester").unwrap_or_else(|_| {
        let mut pilot = Pilot::new(&fixture().data, "Tester").expect("the fixture has a start");
        let hull = game.ids.ships.values().next().expect("the data has a hull");
        pilot.ship = ShipId(*hull);
        pilot
    })
}

/// `pilot` in flight over `data` in `system`, over `stellar` when one is
/// given (else at the centre), with one escort of its own hull, under
/// `rules`.
fn flying(
    data: &GameData,
    rules: &Rulebook,
    pilot: &Pilot,
    system: SystemId,
    stellar: Option<StellarId>,
) -> Session {
    let mut pilot = pilot.clone();
    pilot.system = system;
    pilot.stellar = stellar;
    pilot.escorts.push(Escort {
        ship: pilot.ship,
        reserves: pilot.reserves,
        order: None,
        carried: false,
        wage: Some(0),
        person: None,
        mark: None,
        voice: None,
        berth: None,
    });
    Session::load(data, pilot, rules).expect("flies")
}

/// Puts `pilot` over `gate` in `data`, presses L, works a pass and presses
/// L again, picks the one system the hypergate map offers and enters it:
/// the ship comes out at the partner's gate, in its system, with
/// `SimSound::Arrived` and `SimMessage::ExitedHypergate`, and its escort
/// comes out of the same gate.
pub(crate) fn fly_through(data: &GameData, rules: &Rulebook, pilot: &Pilot, gate: &Gate) {
    let (partner, partner_system, partner_at) = gate.partner;
    let mut session = flying(data, rules, pilot, gate.system, Some(gate.stellar));
    assert_eq!(session.landed(), None, "{gate:?}: in flight");
    assert_eq!(session.player().position, gate.at, "{gate:?}");
    assert!(session.land().is_ok(), "{gate:?}: cleared");
    pass(&mut session, data, rules, &mut SplitMix::new(1));
    assert_eq!(
        session.land(),
        Ok(LandPress::AtGate {
            stellar: gate.stellar,
            kind: GateKind::Hypergate,
        }),
        "{gate:?}"
    );
    assert_eq!(
        session.open_hypergate(data),
        Ok(vec![partner_system]),
        "{gate:?}"
    );
    session.drain_events(&mut Vec::new());
    assert_eq!(
        session.enter_hypergate(Some(partner_system), data, &mut SplitMix::new(2)),
        Ok(partner_system),
        "{gate:?}"
    );
    assert_eq!(session.system(), partner_system, "{gate:?}");
    assert_eq!(session.player().position, partner_at, "{gate:?}");
    let mut events = Vec::new();
    session.drain_events(&mut events);
    assert!(
        events.contains(&FlightEvent::Sound(SimSound::Arrived)),
        "{gate:?}: {events:?}"
    );
    assert!(
        events.contains(&FlightEvent::Message(SimMessage::ExitedHypergate(
            partner_system
        ))),
        "{gate:?}: {events:?}"
    );
    assert!(
        session
            .npcs()
            .iter()
            .any(|npc| npc.emerges_from == Some(partner) && npc.state.position == partner_at),
        "{gate:?}: the escort comes out of the partner's gate"
    );
}

/// Which gates the traffic came out of and went into.
#[derive(Debug, Default)]
pub(crate) struct GateTraffic {
    /// The gates a ship came out of.
    pub emerged: BTreeSet<StellarId>,
    /// The gates a ship went into.
    pub entered: BTreeSet<StellarId>,
    /// How many passes it watched.
    pub ticks: u32,
}

/// Flies `pilot` over `data` at the centre of `system` for up to `ticks`
/// passes under `rules`, on a chance seeded `seed`, noting each gate a
/// ship comes out of or goes into, until every one of `wanted` has had
/// both. The player's escort is no traffic: it is left out.
pub(crate) fn watch_traffic(
    data: &GameData,
    rules: &Rulebook,
    pilot: &Pilot,
    system: SystemId,
    wanted: &BTreeSet<StellarId>,
    ticks: u32,
    seed: u64,
) -> GateTraffic {
    let mut session = flying(data, rules, pilot, system, None);
    let escort = session.npcs().first().map(|npc| npc.id);
    let mut chance = SplitMix::new(seed);
    let mut watched = GateTraffic::default();
    for tick in 0..ticks {
        watched.ticks = tick + 1;
        pass(&mut session, data, rules, &mut chance);
        for npc in session.npcs().iter().filter(|npc| Some(npc.id) != escort) {
            if let Some(gate) = npc.emerges_from {
                watched.emerged.insert(gate);
            }
            if let nova_sim::traffic::npc::Mode::EnteringGate { gate, .. } = npc.mode {
                watched.entered.insert(gate);
            }
        }
        if !wanted.is_empty()
            && wanted.is_subset(&watched.emerged)
            && wanted.is_subset(&watched.entered)
        {
            break;
        }
    }
    watched
}

/// The gates of `gates` in `system`.
pub(crate) fn of_system(gates: &[Gate], system: SystemId) -> BTreeSet<StellarId> {
    gates
        .iter()
        .filter(|gate| gate.system == system)
        .map(|gate| gate.stellar)
        .collect()
}

/// The systems `gates` lie in, each once.
pub(crate) fn systems(gates: &[Gate]) -> BTreeSet<SystemId> {
    gates.iter().map(|gate| gate.system).collect()
}

/// How many passes a system's traffic gets to use every gate.
pub(crate) const TRAFFIC_TICKS: u32 = 30_000;

/// The Nova ID `table` gives the bible id `id`.
fn nova_of<K: ToString>(table: &std::collections::BTreeMap<K, i16>, id: &str) -> i16 {
    let (_, &nova) = table
        .iter()
        .find(|(key, _)| key.to_string() == id)
        .unwrap_or_else(|| panic!("{id}"));
    nova
}

/// The gates among `system`'s stellars an independent ship may pick to
/// come out of or leave by, as `rules` read the reach: what the traffic's
/// gate picks draw among (`nova_sim::traffic::destination::usable_gates`).
pub(crate) fn usable_gates(
    data: &GameData,
    rules: &Rulebook,
    system: SystemId,
) -> BTreeSet<StellarId> {
    let sites = data.landing_sites(system);
    let reach = rules.source_for(RuleKey::DestinationReach);
    usable(None, &sites, &Governments::empty(), reach)
        .into_iter()
        .map(|site| site.id)
        .collect()
}

#[test]
fn new_pilot_starts_in_the_start_system_in_the_start_hull() {
    let game = fixture();
    let session = Session::start(&game.data).expect("the fixture has a start");
    let hearth = SystemId(nova_of(&game.ids.systems, "hearth"));
    assert_eq!(session.system(), hearth);
    assert_eq!(
        session.pilot().ship,
        ShipId(nova_of(&game.ids.ships, "hauler"))
    );
}

#[test]
fn the_fixture_covers_every_kind_of_link() {
    let game = fixture();
    let gates = gates(&game);
    assert_eq!(gates.len(), 9);
    let hearth = SystemId(nova_of(&game.ids.systems, "hearth"));
    let verge = SystemId(nova_of(&game.ids.systems, "verge"));
    let to_verge: BTreeSet<StellarId> = gates
        .iter()
        .filter(|gate| gate.system == hearth && gate.partner.1 == verge)
        .map(|gate| gate.partner.0)
        .collect();
    assert_eq!(to_verge.len(), 2, "two roads to verge, two partners");
    assert!(
        gates.iter().any(|gate| {
            gates
                .iter()
                .find(|other| other.stellar == gate.partner.0)
                .is_some_and(|partner| partner.partner.0 != gate.stellar)
        }),
        "a one-way gate, whose partner leads elsewhere"
    );
}

#[test]
fn the_player_goes_through_every_gate_and_comes_out_at_its_partner() {
    let game = fixture();
    let rules = new_game_rules();
    let pilot = pilot_for(&game);
    for gate in gates(&game) {
        fly_through(&game.data, &rules, &pilot, &gate);
    }
}

#[test]
fn under_the_new_game_every_gate_is_one_the_traffic_comes_out_of_and_leaves_by() {
    let game = fixture();
    let rules = new_game_rules();
    let gates = gates(&game);
    for system in systems(&gates) {
        assert_eq!(
            usable_gates(&game.data, &rules, system),
            of_system(&gates, system),
            "{system:?}"
        );
    }
}

/// The run over every system took about 3 s, so it runs over two: `drift`,
/// of gates alone, and `hearth`, the busiest. The rest are covered by
/// [`under_the_new_game_every_gate_is_one_the_traffic_comes_out_of_and_leaves_by`].
#[test]
fn under_the_new_game_the_traffic_comes_out_of_and_goes_into_every_gate() {
    let game = fixture();
    let rules = new_game_rules();
    let gates = gates(&game);
    let pilot = pilot_for(&game);
    for name in ["drift", "hearth"] {
        let system = SystemId(nova_of(&game.ids.systems, name));
        let wanted = of_system(&gates, system);
        let seen = watch_traffic(
            &game.data,
            &rules,
            &pilot,
            system,
            &wanted,
            TRAFFIC_TICKS,
            7,
        );
        assert_eq!(seen.emerged, wanted, "{name}: out of every gate");
        assert_eq!(seen.entered, wanted, "{name}: into every gate");
    }
}

/// Over as many passes as the new game's run of `hearth` takes to use every
/// gate, the stock rulebook's traffic there never uses its gates past 999.
#[test]
fn under_the_stock_rulebook_the_traffic_never_uses_a_gate_past_999() {
    let game = fixture();
    let gates = gates(&game);
    let hearth = SystemId(nova_of(&game.ids.systems, "hearth"));
    let far: BTreeSet<StellarId> = gates
        .iter()
        .filter(|gate| gate.system == hearth && (gate.at.x > 999.0 || gate.at.y > 999.0))
        .map(|gate| gate.stellar)
        .collect();
    assert_eq!(far.len(), 2, "precondition: hearth's +x and +y roads");
    let wanted = of_system(&gates, hearth);
    let pilot = pilot_for(&game);
    let new_game = watch_traffic(
        &game.data,
        &new_game_rules(),
        &pilot,
        hearth,
        &wanted,
        TRAFFIC_TICKS,
        7,
    );
    let stock = Rulebook::default();
    let seen = watch_traffic(
        &game.data,
        &stock,
        &pilot,
        hearth,
        &BTreeSet::new(),
        new_game.ticks,
        7,
    );
    let used: BTreeSet<StellarId> = seen.emerged.union(&seen.entered).copied().collect();
    assert!(!used.is_empty(), "precondition: the near gate sees traffic");
    assert_eq!(used.intersection(&far).count(), 0, "{used:?}");
    let usable = usable_gates(&game.data, &stock, hearth);
    assert!(usable.is_disjoint(&far), "{usable:?}");
    for system in systems(&gates) {
        assert!(
            usable_gates(&game.data, &stock, system)
                .iter()
                .all(|gate| !far.contains(gate)),
            "{system:?}"
        );
    }
}
