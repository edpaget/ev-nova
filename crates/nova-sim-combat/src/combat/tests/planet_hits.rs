//! The fight's planet-type rules (stellar-fidelity phase 2, see the
//! [`planet_type`](crate::combat::planet_type) module): a planet-type
//! weapon's shots, blasts and beams against planet-type and ordinary
//! ships.

use super::rock_blasts::shot_into_rock;
use super::*;
use crate::combat::planet_type::{BeamPlanetFlag, PlanetBlasts};
use crate::combat::weapon::DETONATES;

/// `ship`, its class planet-type.
fn planet(ship: Ship) -> Ship {
    let mut ship = ship;
    ship.hull.planet_type = true;
    ship
}

/// Whether `ship` has taken anything off its 10 shield and 30 armour.
fn hurt(ship: &Ship) -> bool {
    left(ship) != (10.0, 30.0)
}

/// The collision passes alone among `ships` and `stellars`, with
/// `arsenal`.
fn hits_amid(
    combat: &mut Combat,
    ships: &mut [Ship],
    arsenal: &Arsenal,
    stellars: &mut [StellarTarget],
) {
    let govts = Governments::default();
    let arena = phase::arena(arsenal, &govts, Rules::default());
    phase::run_in(
        combat,
        ships,
        arena,
        &mut NeverFires,
        |combat, fighters, arena, chance| {
            let scenery = Scenery {
                stellars,
                ..Scenery::default()
            };
            combat.test_hits(fighters, arena, scenery, chance);
        },
    );
}

/// A's shot of `record`, whose last move ran from (90, 0) to (160, 0),
/// across B, 100 pixels right of A, and D 50 pixels behind B, meets them
/// in the collision passes. Whether B and D were hurt.
fn through(
    record: WeaponRecord,
    b: impl Fn(Ship) -> Ship,
    d: impl Fn(Ship) -> Ship,
) -> (bool, bool) {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0),
        b(Ship::at(2, 100.0, 0.0)),
        d(Ship::at(4, 150.0, 0.0)),
    ];
    put(
        &mut combat,
        record,
        A,
        Vec2::new(90.0, 0.0),
        Vec2::new(70.0, 0.0),
    );
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    (hurt(&ships[1]), hurt(&ships[2]))
}

#[test]
fn a_planet_type_shot_passes_through_an_ordinary_ship_unspent_and_hits_a_planet_type_one() {
    assert_eq!(
        through(planet_gun(), |ship| ship, planet),
        (false, true),
        "through B to D"
    );
    assert_eq!(
        through(planet_gun(), planet, |ship| ship),
        (true, false),
        "B, planet-type, takes it"
    );
}

#[test]
fn a_planet_type_shot_passes_an_ordinary_ship_to_strike_the_stellar_behind_it() {
    // Its last move, from (60, 0) to (90, 0), crosses B and the
    // stellar's edge at (80, 0).
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0), Ship::at(2, 70.0, 0.0)];
    put(
        &mut combat,
        planet_gun(),
        A,
        Vec2::new(60.0, 0.0),
        Vec2::new(30.0, 0.0),
    );
    hits_amid(&mut combat, &mut ships, &Arsenal::default(), &mut stellars);
    assert!(!hurt(&ships[1]), "passed through");
    assert_eq!(combat.take_stellar_strikes(), [strike(14, false)]);
}

#[test]
fn an_ordinary_shot_passes_through_a_planet_type_ship_unspent() {
    assert_eq!(through(blaster(), planet, |ship| ship), (false, true));
    assert_eq!(through(blaster(), |ship| ship, planet), (true, false));
}

#[test]
fn an_ordinary_shot_passes_through_the_player_when_its_class_is_planet_type() {
    // B's shot's last move, from (90, 0) to (160, 0), crosses the player
    // in its path and C behind it.
    let mut combat = Combat::default();
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, 100.0, 0.0)
    };
    let mut ships = [
        planet(player),
        Ship::at(2, 0.0, 0.0),
        Ship::at(3, 150.0, 0.0),
    ];
    put(
        &mut combat,
        blaster(),
        B,
        Vec2::new(90.0, 0.0),
        Vec2::new(70.0, 0.0),
    );
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    assert!(!hurt(&ships[0]), "the player, planet-type, passed");
    assert!(hurt(&ships[2]), "C behind it");
}

#[test]
fn an_ordinary_ship_does_not_set_off_a_planet_type_shots_proximity_fuse() {
    // As `a_proximity_hit_releases_its_submunitions_at_the_ship_hit`: B
    // sits within the fuse's reach of the shell's second move, from
    // (10, 0) to (20, 0).
    let fuse_struck = |b: fn(Ship) -> Ship| {
        let (shell, arsenal) = cluster(0x0020 | PLANET_TYPE);
        let fused = WeaponRecord {
            prox_radius: 20,
            ..shell
        };
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0), b(Ship::at(2, 40.0, 20.0))];
        put(
            &mut combat,
            fused,
            A,
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 0.0),
        );
        hits_amid(&mut combat, &mut ships, &arsenal, &mut []);
        hurt(&ships[1])
    };
    assert!(!fuse_struck(|ship| ship), "an ordinary ship");
    assert!(fuse_struck(planet), "a planet-type ship");
}

#[test]
fn a_planet_type_shot_turned_on_its_ordinary_firer_passes_through_it() {
    let struck = |firer: fn(Ship) -> Ship| {
        let mut combat = Combat::default();
        let mut ships = [firer(Ship::at(3, 30.0, 0.0))];
        combat.put_in_flight(Shot {
            source: Source::Other,
            target: Some(C),
            aimed_at: Some(C),
            ..flown_by_c(planet_gun(), Vec2::new(35.0, 0.0))
        });
        // Its last move, from (20, 0) to (35, 0), crosses C.
        phase::test_hits(&mut combat, &mut ships, &NovaDisable);
        hurt(&ships[0])
    };
    assert!(
        !struck(|ship| ship),
        "rule 8 holds for a shot no ship fired"
    );
    assert!(struck(planet), "a planet-type firer is hit");
}

// Blasts.

/// Ionizing flak of `flags2`, detonating with a 40-pixel blast where its
/// third move ends, at (30, 0).
fn flak(flags2: u16) -> WeaponRecord {
    WeaponRecord {
        count: 3,
        speed: 1000,
        explod_type: 2,
        blast_radius: 40,
        flags: DETONATES,
        flags2,
        ..ionizing(blaster(), 80, GREEN)
    }
}

/// A fight whose blasts reach the ships as `reading` of [`PlanetBlasts`]
/// says.
fn blasting_by(reading: RuleSource) -> Combat {
    let mut combat = Combat::default();
    combat.set_planet_blasts(PlanetBlasts::from(reading));
    combat
}

/// Whether the expiry of [`flak`] of `flags2`, by `reading`, blasts an
/// ordinary ship and a planet-type one beside where it ends: each
/// whether it was hurt and whether it was ionized. A fires it in the ship
/// loop, and `_HandleShot` flies it to its end of life.
fn expiry_blasts(flags2: u16, reading: RuleSource) -> [(bool, bool); 2] {
    let mut combat = blasting_by(reading);
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(flak(flags2)),
        Ship::at(3, 30.0, 35.0),
        planet(Ship::at(4, 30.0, -35.0)),
    ];
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    let mut events = combat.take_events();
    // Its third move ends at (30, 0), and the visit after ends its life.
    for _ in 0..4 {
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        events.extend(combat.take_events());
    }
    assert_eq!(combat.shots(), [], "spent");
    assert_eq!(explosions(&events).len(), 1, "it explodes: {events:?}");
    [
        (hurt(&ships[1]), ships[1].ion.charge > 0.0),
        (hurt(&ships[2]), ships[2].ion.charge > 0.0),
    ]
}

#[test]
fn a_planet_type_shots_expiry_explodes_but_blasts_and_ionizes_no_ship() {
    assert_eq!(
        expiry_blasts(PLANET_TYPE, RuleSource::Engine),
        [(false, false); 2],
        "@0x35fb9"
    );
}

#[test]
fn an_ordinary_shots_expiry_blasts_the_ordinary_ships_but_not_a_planet_type_one() {
    assert_eq!(
        expiry_blasts(0, RuleSource::Engine),
        [(true, true), (false, false)],
        "@0x3602e"
    );
}

/// Whether a 30-pixel blast shell of `flags2` hitting B at (100, 0), of
/// `b`, blasts C, ordinary, at (100, 25), and D, planet-type, at
/// (100, -25), by `reading`.
fn on_hit_blasts(flags2: u16, b: fn(Ship) -> Ship, reading: RuleSource) -> (bool, bool, bool) {
    let shell = WeaponRecord {
        explod_type: 0,
        blast_radius: 30,
        flags2,
        ..blaster()
    };
    let mut ships = [
        Ship::at(1, 0.0, 0.0),
        b(Ship::at(2, 100.0, 0.0)),
        Ship::at(3, 100.0, 25.0),
        planet(Ship::at(4, 100.0, -25.0)),
    ];
    let mut combat = blasting_by(reading);
    // Its last move, from (90, 0) to (105, 0), crosses B.
    put(
        &mut combat,
        shell,
        A,
        Vec2::new(90.0, 0.0),
        Vec2::new(15.0, 0.0),
    );
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    (hurt(&ships[1]), hurt(&ships[2]), hurt(&ships[3]))
}

#[test]
fn a_planet_type_shot_hitting_a_planet_type_ship_blasts_the_ordinary_ship_beside_it() {
    assert_eq!(
        on_hit_blasts(PLANET_TYPE, planet, RuleSource::Engine),
        (true, true, true),
        "`_HandleShipHit` tests neither bit"
    );
}

#[test]
fn an_ordinary_shots_hit_blast_reaches_a_planet_type_ship_beside_its_victim() {
    assert_eq!(
        on_hit_blasts(0, |ship| ship, RuleSource::Engine),
        (true, true, true)
    );
}

/// The ships struck when the player shoots `weapon` of `flags2`'s blast
/// at an asteroid at (100, 0), by `reading`, with B, ordinary, and D,
/// planet-type, in the blast's square.
fn asteroid_blasts(flags2: u16, reading: RuleSource) -> Vec<ShipRef> {
    let mut combat = Combat::default();
    combat.set_planet_blasts(PlanetBlasts::from(reading));
    let others = vec![Ship::at(2, 110.0, 20.0), planet(Ship::at(4, 100.0, -25.0))];
    shot_into_rock(&mut combat, ShipRef::Player, blasting(0, flags2), others);
    combat
        .take_strikes()
        .iter()
        .map(|strike| strike.ship)
        .collect()
}

#[test]
fn by_the_engine_an_asteroid_hits_blast_spares_planet_type_ships_and_weapons() {
    assert_eq!(asteroid_blasts(0, RuleSource::Engine), [B]);
    assert_eq!(asteroid_blasts(PLANET_TYPE, RuleSource::Engine), []);
}

#[test]
fn by_the_bible_every_blast_reaches_the_ships_of_its_weapons_planet_type() {
    let d = ShipRef::Npc(NpcId(4));
    assert_eq!(
        expiry_blasts(PLANET_TYPE, RuleSource::Bible),
        [(false, false), (true, true)]
    );
    assert_eq!(
        expiry_blasts(0, RuleSource::Bible),
        [(true, true), (false, false)]
    );
    assert_eq!(
        on_hit_blasts(PLANET_TYPE, planet, RuleSource::Bible),
        (true, false, true)
    );
    assert_eq!(
        on_hit_blasts(0, |ship| ship, RuleSource::Bible),
        (true, true, false)
    );
    assert_eq!(asteroid_blasts(0, RuleSource::Bible), [B]);
    assert_eq!(asteroid_blasts(PLANET_TYPE, RuleSource::Bible), [d]);
}

// Beams.

/// `wëap` `Flags` 0x0400, the "big smoke" bit, which the original's beam
/// filter reads as the planet-type bit (@0x30d34-0x30d82).
const BIG_SMOKE: u16 = 0x0400;

/// [`laser`] with `flags` and `flags2`.
fn beam_of(flags: u16, flags2: u16) -> WeaponRecord {
    WeaponRecord {
        flags,
        flags2,
        ..laser()
    }
}

/// Whether A's beam of `flags` and `flags2`, by `reading` of
/// [`BeamPlanetFlag`], fired to the right in the ship loop, hurts B, 100
/// pixels off, and D, 50 pixels behind B, at the beams' step.
fn beam_through(
    (flags, flags2): (u16, u16),
    reading: RuleSource,
    b: fn(Ship) -> Ship,
    d: fn(Ship) -> Ship,
) -> (bool, bool) {
    let mut combat = Combat::default();
    combat.set_beam_planet_flag(BeamPlanetFlag::from(reading));
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(beam_of(flags, flags2)),
        b(Ship::at(2, 100.0, 0.0)),
        d(Ship::at(4, 150.0, 0.0)),
    ];
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.beams().len(), 1, "fired");
    phase::close_tick(&mut combat, &mut ships, &NovaDisable);
    (hurt(&ships[1]), hurt(&ships[2]))
}

/// Each of a beam of `bits` by `reading`: whether it hits an ordinary B
/// before D, a planet-type one; and whether it passes a planet-type B to
/// hit D, ordinary.
fn beam_hits(bits: (u16, u16), reading: RuleSource) -> [(bool, bool); 2] {
    [
        beam_through(bits, reading, |ship| ship, planet),
        beam_through(bits, reading, planet, |ship| ship),
    ]
}

/// An ordinary beam hits the ordinary B and passes the planet-type B to
/// the ordinary D behind it.
const ORDINARY_HITS: [(bool, bool); 2] = [(true, false), (false, true)];
/// A planet-type beam passes the ordinary B to the planet-type D behind
/// it, and hits the planet-type B.
const PLANET_HITS: [(bool, bool); 2] = [(false, true), (true, false)];

#[test]
fn by_the_engine_a_beam_matches_its_flags_0x0400_against_the_ships_planet_type() {
    let engine = RuleSource::Engine;
    assert_eq!(beam_hits((BIG_SMOKE, 0), engine), PLANET_HITS);
    assert_eq!(beam_hits((0, 0), engine), ORDINARY_HITS);
    assert_eq!(
        beam_hits((0, PLANET_TYPE), engine),
        ORDINARY_HITS,
        "Flags2 does nothing for a beam"
    );
    assert_eq!(beam_hits((BIG_SMOKE, PLANET_TYPE), engine), PLANET_HITS);
}

#[test]
fn by_the_bible_a_beam_matches_its_flags2_0x0400_against_the_ships_planet_type() {
    let bible = RuleSource::Bible;
    assert_eq!(beam_hits((0, PLANET_TYPE), bible), PLANET_HITS);
    assert_eq!(beam_hits((0, 0), bible), ORDINARY_HITS);
    assert_eq!(beam_hits((BIG_SMOKE, 0), bible), ORDINARY_HITS);
    assert_eq!(beam_hits((BIG_SMOKE, PLANET_TYPE), bible), PLANET_HITS);
}

// Sub-munitions on their release tick.

/// A shell releasing 2 planet-type sub-munitions (weapon 148, 5 pixels a
/// tick, of 1 mass damage and `ProxSafety` 0), and the arsenal holding
/// them.
fn planet_cluster() -> (WeaponRecord, Arsenal) {
    cluster_of(WeaponRecord {
        count: 50,
        speed: 500,
        mass_dmg: 1,
        prox_safety: 0,
        flags2: PLANET_TYPE,
        ..weapon(148)
    })
}

/// The strikes on stellar 128, at (20, 0) and 60 across, of the shots in
/// flight in a collision pass among `ships`.
fn stellar_strikes(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal) -> usize {
    let mut stellars = [stellar_at(20.0, 0.0, 60.0, 1000)];
    hits_amid(combat, ships, arsenal, &mut stellars);
    combat.take_stellar_strikes().len()
}

/// `_HandleShot` among `ships`, each sub-munition read from `arsenal`.
fn fly_with(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal) {
    let govts = Governments::default();
    let arena = phase::arena(arsenal, &govts, Rules::default());
    phase::run_in(
        combat,
        ships,
        arena,
        &mut NeverFires,
        |combat, fighters, arena, chance| {
            combat.fly_shots(fighters, arena, chance);
        },
    );
}

/// Whether each shot in flight has moved since it was put in flight.
fn moved(combat: &Combat) -> Vec<(usize, bool)> {
    combat
        .shots()
        .iter()
        .map(|shot| (shot.slot, shot.moved_from.is_some()))
        .collect()
}

#[test]
fn a_planet_type_sub_munition_a_fuse_releases_over_a_stellar_strikes_it_the_tick_after() {
    // An ordinary rocket's fuse is set off by B, ordinary, inside the
    // stellar's disc, in the hit pass; the stellar pass that ends it
    // finds the sub-munitions unmoved. They move later that tick, in
    // `_HandleShot`, and the next tick's pass strikes the stellar
    // (`Count - ProxSafety > life`, @0x371fc-0x3721e).
    let (shell, arsenal) = planet_cluster();
    let fused = WeaponRecord {
        prox_radius: 20,
        ..shell
    };
    let mut combat = Combat::default();
    combat.put_in_flight(Shot {
        target: Some(B),
        aimed_at: Some(B),
        ..flown_by_c(fused, Vec2::new(20.0, 0.0))
    });
    let mut ships = [Ship::at(2, 40.0, 20.0)];
    assert_eq!(
        stellar_strikes(&mut combat, &mut ships, &arsenal),
        0,
        "none in the pass that releases them"
    );
    assert_eq!(moved(&combat), [(1, false), (2, false)], "released unmoved");
    fly_with(&mut combat, &mut ships, &arsenal);
    assert_eq!(moved(&combat), [(1, true), (2, true)]);
    assert_eq!(
        stellar_strikes(&mut combat, &mut ships, &arsenal),
        2,
        "both, once moved"
    );
    fly_with(&mut combat, &mut ships, &arsenal);
    assert_eq!(combat.shots(), [], "spent on it");
}

/// A shot of C's of `record`, at `at`, whose last move spent it.
fn spent_by_c(record: WeaponRecord, at: Vec2) -> Shot {
    let shot = flown_by_c(record, at);
    Shot {
        age: shot.weapon.lifetime,
        ..shot
    }
}

#[test]
fn a_sub_munition_an_expiry_seats_in_an_earlier_slot_strikes_a_tick_later_than_one_in_a_later_slot()
{
    // Slot 0 holds a shot of no life, which `_HandleShot` drops at its
    // visit; slot 1 the planet-type shell, spent over the stellar, whose
    // end of life seats its first sub-munition in slot 0, already visited,
    // and its second in slot 2. Only the second moves in that visit, so
    // only it strikes in the next collision pass; the first moves in the
    // next `_HandleShot` and strikes in the pass after that: the
    // original's frame order.
    let (shell, arsenal) = planet_cluster();
    let shell = WeaponRecord {
        flags2: PLANET_TYPE,
        ..shell
    };
    let lifeless = WeaponRecord {
        count: 0,
        ..blaster()
    };
    let mut combat = Combat::default();
    combat.put_in_flight(flown_by_c(lifeless, Vec2::new(500.0, 500.0)));
    combat.put_in_flight(spent_by_c(shell, Vec2::new(20.0, 0.0)));
    assert_eq!(stellar_strikes(&mut combat, &mut [], &arsenal), 0, "spent");
    fly_with(&mut combat, &mut [], &arsenal);
    assert_eq!(moved(&combat), [(0, false), (2, true)], "slot 0 visited");
    assert_eq!(stellar_strikes(&mut combat, &mut [], &arsenal), 1);
    fly_with(&mut combat, &mut [], &arsenal);
    assert_eq!(moved(&combat), [(0, true)], "slot 2's spent on it");
    assert_eq!(stellar_strikes(&mut combat, &mut [], &arsenal), 1);
    fly_with(&mut combat, &mut [], &arsenal);
    assert_eq!(combat.shots(), [], "both spent on it");
}

// Beams on stellars.

/// A planet-type [`laser`] of 2 mass and 5 energy damage, exploding in
/// `bööm` 130: 200 pixels long, held 3 ticks.
fn planet_laser() -> WeaponRecord {
    WeaponRecord {
        flags2: PLANET_TYPE,
        mass_dmg: 2,
        explod_type: 2,
        ..laser()
    }
}

/// One planet-type reading set on a fight.
type Setter = fn(&mut Combat);

#[test]
fn the_fight_reads_its_planet_type_rules_from_the_rulebook() {
    use crate::rulebook::{RuleKey, Rulebook};
    let read = |rulebook: Rulebook| {
        let mut combat = Combat::default();
        combat.set_planet_type_rules(&rulebook);
        combat
    };
    assert_eq!(read(Rulebook::default()), Combat::default(), "the engine's");
    let cases: [(RuleKey, Setter); 3] = [
        (RuleKey::PlanetBlasts, |combat| {
            combat.set_planet_blasts(PlanetBlasts::Matching);
        }),
        (RuleKey::BeamPlanetFlag, |combat| {
            combat.set_beam_planet_flag(BeamPlanetFlag::Flags2);
        }),
        (RuleKey::BeamStellars, |combat| {
            combat.set_beam_stellars(RuleSource::Bible);
        }),
    ];
    for (key, set) in cases {
        let mut expected = Combat::default();
        set(&mut expected);
        assert_ne!(expected, Combat::default());
        assert_eq!(
            read(Rulebook::default().with_override(key, RuleSource::Bible)),
            expected,
            "{key:?}"
        );
    }
}

/// A fires `record` once to the right among `others` in the ship loop;
/// then the beams' step runs 5 times among `stellars`, by `reading` of
/// [`RuleKey::BeamStellars`](crate::RuleKey::BeamStellars). The ships,
/// the events of the beams' steps, and the strikes on stellars.
fn beamed(
    record: WeaponRecord,
    others: Vec<Ship>,
    stellars: &mut [StellarTarget],
    reading: RuleSource,
) -> (Vec<Ship>, Vec<CombatEvent>, Vec<StellarStrike>) {
    let mut combat = Combat::default();
    combat.set_beam_stellars(reading);
    let mut ships = vec![Ship::at(1, 0.0, 0.0).armed(record)];
    ships.extend(others);
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    assert_eq!(combat.beams().len(), 1, "fired");
    combat.take_events();
    let (mut events, mut strikes) = (Vec::new(), Vec::new());
    for _ in 0..5 {
        phase::run(
            &mut combat,
            &mut ships,
            &NovaDisable,
            |combat, fighters, arena, chance| {
                combat.close_tick_in(fighters, arena, stellars, None, chance);
            },
        );
        events.extend(combat.take_events());
        strikes.extend(combat.take_stellar_strikes());
    }
    (ships, events, strikes)
}

/// Whether each of `seen` is `explosion` at (`x`, 0).
fn all_at(seen: &[(Vec2, Explosion)], x: f32) -> bool {
    seen.iter().all(|(at, explosion)| {
        (*at - Vec2::new(x, 0.0)).length() < 1e-3 && Some(*explosion) == Explosion::decode(2)
    })
}

#[test]
fn by_the_engine_a_planet_type_beam_held_across_a_stellar_never_damages_it() {
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
    let (_, events, strikes) = beamed(
        planet_laser(),
        Vec::new(),
        &mut stellars,
        RuleSource::Engine,
    );
    assert_eq!(strikes, []);
    assert_eq!(stellars[0].strength, 100);
    assert_eq!(explosions(&events), [], "it hit nothing");
}

#[test]
fn by_the_bible_a_planet_type_beam_strikes_a_stellar_each_tick_it_is_held() {
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
    let (_, events, strikes) = beamed(planet_laser(), Vec::new(), &mut stellars, RuleSource::Bible);
    assert_eq!(strikes, [strike(7, false); 3], "2 mass and 5 energy a tick");
    assert_eq!(stellars[0].strength, 79);
    let seen = explosions(&events);
    assert_eq!(seen.len(), 3);
    assert!(all_at(&seen, 80.0), "on its edge: {seen:?}");
}

#[test]
fn by_the_bible_a_beam_shoots_a_stellar_down_below_none_and_passes_it_after() {
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 10)];
    let behind = Ship::at(2, 150.0, 0.0);
    let (ships, _, strikes) = beamed(
        planet_laser(),
        vec![behind],
        &mut stellars,
        RuleSource::Bible,
    );
    assert_eq!(strikes, [strike(7, false), strike(7, true)]);
    assert_eq!(stellars[0].strength, -4);
    assert!(hurt(&ships[1]), "its third tick reaches the ship behind");
}

#[test]
fn by_the_bible_a_beam_stops_at_whichever_of_a_ship_and_a_stellar_is_nearer() {
    // A beam of `Flags2` 0x0400 alone is ordinary to ships by the
    // engine's `beam_planet_flag`, so it may hit an ordinary ship.
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
    let (ships, _, strikes) = beamed(
        planet_laser(),
        vec![Ship::at(2, 50.0, 0.0)],
        &mut stellars,
        RuleSource::Bible,
    );
    assert_eq!(strikes, [], "the ship is nearer");
    assert!(hurt(&ships[1]));
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
    let (ships, _, strikes) = beamed(
        planet_laser(),
        vec![Ship::at(2, 150.0, 0.0)],
        &mut stellars,
        RuleSource::Bible,
    );
    assert_eq!(strikes.len(), 3, "the stellar is nearer");
    assert!(!hurt(&ships[1]));
}

#[test]
fn by_the_bible_an_ordinary_beam_never_hits_a_stellar() {
    let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
    let ordinary = WeaponRecord {
        flags2: 0,
        ..planet_laser()
    };
    let (_, _, strikes) = beamed(ordinary, Vec::new(), &mut stellars, RuleSource::Bible);
    assert_eq!(strikes, []);
    assert_eq!(stellars[0].strength, 100);
}
