//! The shots point defence ends at life 0, kept hidden for the next
//! proximity pass (rdm weapons-fidelity phase 29): `_CheckShotProximities`
//! skips only a life below 0 (@0x371ea-0x371f6), refuses them every ship
//! (`_ShotCanHitShip` @0x447c2-0x447ce), and its asteroid branch, which
//! tests no life (@0x376ab-0x37758), sets them off.

use super::*;

/// The advance pass at single speed, by `rules`, among no sub-munitions
/// and no governments' relations, on frame 0.
fn arena<'a>(arsenal: &'a Arsenal, govts: &'a Governments, rules: Rules<'a>) -> Arena<'a> {
    Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal,
        govts,
        rules,
        field: Field::default(),
    }
}

/// The collision passes alone among `ships`, `rocks` and `stellars`.
fn test_amid(
    combat: &mut Combat,
    ships: &mut [Ship],
    rocks: &mut Rocks,
    stellars: &mut [StellarTarget],
) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let scenery = Scenery {
        stellars,
        asteroids: Some(rocks.lend()),
        hazards: None,
    };
    let arena = arena(&arsenal, &govts, Rules::default());
    combat.test_hits(&mut fighters, arena, scenery, &mut NeverFires);
}

/// `_HandleShot` alone among `ships`.
fn fly_amid(combat: &mut Combat, ships: &mut [Ship]) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let arena = arena(&arsenal, &govts, Rules::default());
    combat.fly_shots(&mut fighters, arena, &mut NeverFires);
}

/// A still homing missile on a 10-pixel sprite that point defence's first
/// hit downs, of `Count` 100, doing 20 mass damage, its fuse reaching
/// `reach` pixels with no safety, exploding in `bööm` 130.
fn fused_missile(reach: i16) -> WeaponRecord {
    WeaponRecord {
        prox_radius: reach,
        explod_type: 2,
        ..still_missile()
    }
}

/// Where `events` say something exploded, in order.
fn explosions(events: &[CombatEvent]) -> Vec<Vec2> {
    events
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::Exploded { at, .. } => Some(at),
            _ => None,
        })
        .collect()
}

/// Whether `combat` draws the shot numbered `id`.
fn drawn_shot(combat: &Combat, id: ShotId) -> bool {
    combat.shots().iter().any(|shot| shot.id == id)
}

/// A's still point-defence shot at the centre, and B's missile of
/// `record` 3 pixels right of it, aimed at A, which it overlaps: their
/// numbers.
fn pd_meets(combat: &mut Combat, record: WeaponRecord) -> (ShotId, ShotId) {
    let pd = put(combat, pd_shot(), A, Vec2::ZERO, Vec2::ZERO);
    let missile = put(combat, record, B, Vec2::new(3.0, 0.0), Vec2::ZERO);
    let shot = combat.shots.last_mut().expect("just put");
    shot.target = Some(A);
    shot.aimed_at = Some(A);
    (pd, missile)
}

/// A 25 pixels above B's missile, within its fuse, and B far off.
fn beside_the_missile() -> [Ship; 2] {
    [Ship::at(1, 3.0, -25.0), Ship::at(2, 1000.0, 1000.0)]
}

#[test]
fn a_missile_a_point_defence_shot_downs_breaks_an_asteroid_in_its_fuses_reach_that_tick() {
    let mut combat = Combat::default();
    let (pd, missile) = pd_meets(&mut combat, fused_missile(30));
    assert!(combat.targeted_damage().contains_key(&A), "live, it counts");
    let mut ships = beside_the_missile();
    // 20 pixels from the missile, beyond its 10-pixel radius.
    let mut rocks = Rocks::of([rock(0, 23.0, 0.0)]);
    test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
    assert_eq!(rocks.get(0), None, "destroyed: {rocks:?}");
    assert_eq!(
        remains_at(&combat.take_asteroid_remains()),
        [Vec2::new(23.0, 0.0)]
    );
    let events = combat.take_events();
    assert_eq!(
        shot_down(&events),
        [Vec2::ZERO],
        "at the point-defence shot"
    );
    assert_eq!(explosions(&events), [Vec2::new(3.0, 0.0)], "at the missile");
    assert_eq!(combat.take_strikes(), [], "no ship hit");
    assert!(!drawn_shot(&combat, pd) && !drawn_shot(&combat, missile));
    assert_eq!(combat.targeted_damage(), BTreeMap::new(), "none counted");
}

#[test]
fn a_light_missile_a_point_defence_shot_downs_pushes_the_asteroid_it_leaves_whole() {
    let mut combat = Combat::default();
    let light = WeaponRecord {
        mass_dmg: 1,
        impact: 10,
        ..fused_missile(30)
    };
    pd_meets(&mut combat, light);
    let mut ships = beside_the_missile();
    let mut rocks = Rocks::of([rock(0, 23.0, 0.0)]);
    test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
    assert_eq!(rocks.at(0).strength, 9, "{rocks:?}");
    assert_ne!(rocks.at(0).velocity, Vec2::ZERO, "pushed");
    assert_eq!(combat.take_asteroid_remains(), [], "none destroyed");
    assert_eq!(combat.take_strikes(), []);
}

#[test]
fn a_point_defence_shot_with_a_proximity_radius_sets_off_an_asteroid_itself() {
    // The Quad's stock `ProxRadius` of 5 (`wëap` 133), spent on a missile
    // that stands it (durability 4) and flies on.
    let mut combat = Combat::default();
    let quad = drawn(
        WeaponRecord {
            speed: 0,
            prox_radius: 5,
            explod_type: 2,
            ..quad()
        },
        10,
    );
    let pd = put(&mut combat, quad, A, Vec2::ZERO, Vec2::ZERO);
    let sturdy = drawn(
        WeaponRecord {
            speed: 0,
            ..missile()
        },
        10,
    );
    let missile = put(&mut combat, sturdy, B, Vec2::new(-3.0, 0.0), Vec2::ZERO);
    let mut ships = [Ship::at(1, 0.0, 500.0), Ship::at(2, 1000.0, 1000.0)];
    let mut rocks = Rocks::of([rock(0, 4.0, 0.0)]);
    // 2 across: a radius of 1.
    rocks.types[0].size = (2.0, 2.0);
    test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
    assert_eq!(rocks.at(0).strength, 9, "the Quad's 1 mass damage");
    assert_eq!(explosions(&combat.take_events()), [Vec2::ZERO]);
    assert!(!drawn_shot(&combat, pd), "spent");
    assert!(drawn_shot(&combat, missile), "the missile flies on");
}

#[test]
fn a_life_zero_fuse_is_armed_by_count_above_prox_safety_not_by_age() {
    // At life 0 the fuse reads `Count` - `ProxSafety` > 0
    // (@0x371fc-0x3721e), whatever the shot's age.
    let cases: [(&str, u16, i16, bool); 4] = [
        ("Count above ProxSafety", 0, 50, true),
        ("ProxSafety at Count", 0, 100, false),
        ("ProxSafety above Count", 0, 120, false),
        ("Seeker 0x0001", 0x0001, 0, false),
    ];
    for (what, seeker, prox_safety, hit) in cases {
        let mut combat = Combat::default();
        let record = WeaponRecord {
            seeker,
            prox_safety,
            ..fused_missile(30)
        };
        pd_meets(&mut combat, record);
        combat.shots[1].age = 3;
        let mut ships = beside_the_missile();
        let mut rocks = Rocks::of([rock(0, 23.0, 0.0)]);
        test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
        assert_eq!(rocks.get(0).is_none(), hit, "{what}: destroyed: {rocks:?}");
    }
}

#[test]
fn a_spent_missile_point_defence_meets_is_put_back_to_life_zero_and_tested() {
    // `_PointDefenseCollisionHandler` tests no life and writes 0
    // (@0x377fa); the hidden shot then has no end of life (@0x35f0a).
    let mut combat = Combat::default();
    pd_meets(&mut combat, fused_missile(30));
    combat.shots[1].age = 100;
    let mut ships = beside_the_missile();
    let mut rocks = Rocks::of([rock(0, 23.0, 0.0)]);
    test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
    fly_amid(&mut combat, &mut ships);
    assert_eq!(rocks.get(0), None, "destroyed: {rocks:?}");
    assert_eq!(
        explosions(&combat.take_events()),
        [Vec2::new(3.0, 0.0)],
        "the asteroid's alone"
    );
}

/// A missile point-defence beams down at once: `Speed` 500, durability 0,
/// its fuse reaching 30 pixels, exploding in `bööm` 130.
fn frail_missile() -> WeaponRecord {
    drawn(
        WeaponRecord {
            speed: 500,
            durability: 0,
            prox_radius: 30,
            explod_type: 2,
            ..missile()
        },
        18,
    )
}

#[test]
fn a_missile_a_point_defence_beam_downs_sets_off_an_asteroid_in_the_next_tick() {
    // B fires on tick 1; on tick 2 A's beam takes it (`_AIDispatch`), the
    // missile moves 5 pixels to (0, -195) (`_HandleShot`) and the beam
    // downs it (`_HandleBeams` @0x30ab1); tick 3's proximity pass sets it
    // off on the asteroid 20 pixels right of it, and `_HandleShot` frees it.
    let mut defender = Ship::at(1, 0.0, 0.0).armed(pd_beam());
    defender.trigger = Trigger {
        point_defence: false,
        ..QUIET
    };
    let attacker = Ship::at(2, 0.0, -200.0)
        .armed(frail_missile())
        .facing(180.0)
        .targeting(A);
    let mut ships = [defender, attacker];
    let mut rocks = Rocks::of([rock(0, 20.0, -195.0)]);
    let mut combat = Combat::default();
    let tick_once = |combat: &mut Combat, ships: &mut [Ship], rocks: &mut Rocks| {
        tick_amid(combat, ships, &Arsenal::default(), Rules::default(), rocks);
        ships[0].trigger = QUIET;
        ships[1].trigger = QUIET;
        combat.take_events()
    };
    tick_once(&mut combat, &mut ships, &mut rocks);
    let missile = combat.shots()[0].id;
    assert!(combat.targeted_damage().contains_key(&A), "live, it counts");
    let events = tick_once(&mut combat, &mut ships, &mut rocks);
    assert_eq!(shot_down(&events).len(), 1, "{events:?}");
    assert_eq!(rocks.at(0).strength, 10, "not yet");
    assert!(!drawn_shot(&combat, missile), "hidden");
    assert_eq!(combat.targeted_damage(), BTreeMap::new());
    let events = tick_once(&mut combat, &mut ships, &mut rocks);
    assert_eq!(rocks.get(0), None, "set off, destroyed");
    let at = explosions(&events);
    assert_eq!(at.len(), 1, "{events:?}");
    assert!((at[0] - Vec2::new(0.0, -195.0)).length() < 1e-3, "{at:?}");
    assert_eq!(combat.take_strikes(), [], "no ship hit");
    let (slot, again) = rock(0, 20.0, -195.0);
    rocks.field.seat(slot, again);
    tick_once(&mut combat, &mut ships, &mut rocks);
    assert_eq!(rocks.at(0).strength, 10, "freed by `_HandleShot`");
}

#[test]
fn a_missile_a_beam_downs_before_its_first_move_is_still_tested() {
    // The proximity pass reads only life and position, not whether the
    // sprite was ever shown.
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = arena(&arsenal, &govts, Rules::default());
    let mut combat = Combat::default();
    let missile = put(
        &mut combat,
        frail_missile(),
        B,
        Vec2::new(0.0, -100.0),
        Vec2::ZERO,
    );
    combat.shots[0].moved_from = None;
    combat.shots[0].target = Some(A);
    combat.shots[0].aimed_at = Some(A);
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(pd_beam()),
        Ship::at(2, 1000.0, 1000.0),
    ];
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.defend_npcs(&mut fighters, arena, None, false, &mut NeverFires);
    combat.close_tick(&mut fighters, arena, &mut [], &mut NeverFires);
    drop(fighters);
    assert_eq!(shot_down(&combat.take_events()).len(), 1);
    assert!(!drawn_shot(&combat, missile));
    let mut rocks = Rocks::of([rock(0, 20.0, -100.0)]);
    ships[0].trigger = QUIET;
    test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
    assert_eq!(rocks.get(0), None, "set off, destroyed");
    assert_eq!(combat.take_strikes(), []);
}

#[test]
fn hidden_shots_are_neither_drawn_nor_counted_nor_chosen_by_point_defence() {
    // B's missile at A, which A's beam downs; C, A's escort, defends A's
    // missiles too (`defence::choose` reads the defender's fleet), and its
    // ready beam takes the missile while it is live.
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = arena(&arsenal, &govts, Rules::default());
    let scene = || {
        let mut combat = Combat::default();
        let missile = put(
            &mut combat,
            frail_missile(),
            B,
            Vec2::new(0.0, -100.0),
            Vec2::ZERO,
        );
        combat.shots[0].target = Some(A);
        combat.shots[0].aimed_at = Some(A);
        let mut escort = Ship::at(3, 0.0, -50.0).armed(pd_beam());
        escort.fleet = A;
        escort.trigger = QUIET;
        let ships = [
            Ship::at(1, 0.0, 0.0).armed(pd_beam()),
            Ship::at(2, 1000.0, 1000.0),
            escort,
        ];
        (combat, missile, ships)
    };
    // C's point defence, run alone.
    let c_takes = |combat: &mut Combat, ships: &mut [Ship; 3], missile: ShotId| {
        ships[0].trigger = QUIET;
        ships[0].trigger.point_defence = false;
        ships[2].trigger = QUIET;
        let beams_before = combat.beams().len();
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.defend_npcs(&mut fighters, arena, None, false, &mut NeverFires);
        drop(fighters);
        combat.beams()[beams_before..]
            .iter()
            .any(|beam| beam.firer == C && beam.aiming == Aiming::Shot(missile))
    };

    let (mut combat, missile, mut ships) = scene();
    assert!(drawn_shot(&combat, missile), "live");
    assert!(
        combat.targeted_damage().contains_key(&A),
        "live, it counts: {:?}",
        combat.targeted_damage()
    );
    assert!(
        c_takes(&mut combat, &mut ships, missile),
        "live, C takes it"
    );

    let (mut combat, missile, mut ships) = scene();
    ships[2].trigger.point_defence = false;
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.defend_npcs(&mut fighters, arena, None, false, &mut NeverFires);
    combat.close_tick(&mut fighters, arena, &mut [], &mut NeverFires);
    drop(fighters);
    assert_eq!(combat.hidden.len(), 1, "kept hidden");
    assert!(!drawn_shot(&combat, missile));
    assert_eq!(combat.targeted_damage(), BTreeMap::new(), "hidden, none");
    assert!(
        !c_takes(&mut combat, &mut ships, missile),
        "{:?}",
        combat.beams()
    );
}

#[test]
fn clearing_the_fight_lets_go_of_the_hidden_shots() {
    // A beam downs a missile beside an asteroid, keeping it hidden; the
    // ship then leaves the system, and the next system's first proximity
    // pass, beside an asteroid at the same spot, tests nothing.
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = arena(&arsenal, &govts, Rules::default());
    let mut combat = Combat::default();
    put(
        &mut combat,
        frail_missile(),
        B,
        Vec2::new(0.0, -100.0),
        Vec2::ZERO,
    );
    combat.shots[0].target = Some(A);
    combat.shots[0].aimed_at = Some(A);
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(pd_beam()),
        Ship::at(2, 1000.0, 1000.0),
    ];
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.defend_npcs(&mut fighters, arena, None, false, &mut NeverFires);
    combat.close_tick(&mut fighters, arena, &mut [], &mut NeverFires);
    drop(fighters);
    assert_eq!(combat.hidden.len(), 1, "kept hidden");
    combat.clear();
    combat.take_events();
    let mut rocks = Rocks::of([rock(0, 20.0, -100.0)]);
    ships[0].trigger = QUIET;
    test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
    assert_eq!(rocks.at(0).strength, 10, "untouched: {rocks:?}");
    assert_eq!(explosions(&combat.take_events()), []);
}

#[test]
fn the_proximity_pass_and_handle_shot_leave_no_shot_hidden() {
    let mut combat = Combat::default();
    pd_meets(&mut combat, fused_missile(30));
    let mut ships = beside_the_missile();
    test_amid(&mut combat, &mut ships, &mut Rocks::of([]), &mut []);
    fly_amid(&mut combat, &mut ships);
    assert_eq!(combat.hidden, [], "freed");
}

#[test]
fn a_players_missile_an_npc_downs_beside_an_asteroid_blasts_the_ships_in_its_square() {
    // `_HandleAsteroidHit` blasts for the player's shot alone
    // (@0x3670d-0x368ca), sparing the player only under `Flags` 0x0100.
    let blasting = |flags: u16| WeaponRecord {
        blast_radius: 40,
        flags,
        ..fused_missile(30)
    };
    for (firer, flags, struck) in [
        (ShipRef::Player, 0, vec![ShipRef::Player, C]),
        (ShipRef::Player, 0x0100, vec![C]),
        (A, 0, vec![]),
    ] {
        let mut combat = Combat::default();
        put(&mut combat, pd_shot(), B, Vec2::ZERO, Vec2::ZERO);
        put(
            &mut combat,
            blasting(flags),
            firer,
            Vec2::new(3.0, 0.0),
            Vec2::ZERO,
        );
        let mut ships = [
            the_player_at(3.0, -25.0),
            Ship::at(1, 1000.0, 0.0),
            Ship::at(2, 1000.0, 1000.0),
            Ship::at(3, 30.0, 10.0),
        ];
        let mut rocks = Rocks::of([rock(0, 23.0, 0.0)]);
        test_amid(&mut combat, &mut ships, &mut rocks, &mut []);
        let strikes = combat.take_strikes();
        let ships_struck: Vec<ShipRef> = strikes.iter().map(|strike| strike.ship).collect();
        assert_eq!(ships_struck, struck, "{firer:?} {flags:#x}");
        assert!(
            strikes
                .iter()
                .all(|strike| strike.by == Some(ShipRef::Player) && !strike.direct),
            "{strikes:?}"
        );
        assert_eq!(rocks.get(0), None, "{firer:?} {flags:#x}: destroyed");
    }
}

#[test]
fn a_planet_type_shot_ended_at_life_zero_strikes_a_stellar_and_then_no_asteroid() {
    // The stellar branch (@0x37224-0x37521) sets life -1 on a hit, and
    // the asteroid branch is not reached (@0x3752d-0x37543).
    for (standing, stellar_hit) in [(100, true), (-1, false)] {
        let mut combat = Combat::default();
        let planet_type = WeaponRecord {
            flags2: 0x0400,
            ..fused_missile(30)
        };
        pd_meets(&mut combat, planet_type);
        let mut ships = beside_the_missile();
        let mut rocks = Rocks::of([rock(0, 23.0, 0.0)]);
        let mut stellars = [StellarTarget {
            stellar: StellarId(128),
            position: Vec2::new(3.0, 0.0),
            radius: 20.0,
            strength: standing,
        }];
        test_amid(&mut combat, &mut ships, &mut rocks, &mut stellars);
        let strikes = combat.take_stellar_strikes();
        assert_eq!(strikes.len(), usize::from(stellar_hit), "{strikes:?}");
        assert_eq!(
            rocks.get(0).is_none(),
            !stellar_hit,
            "{standing}: destroyed"
        );
        if stellar_hit {
            assert_eq!(stellars[0].strength, 70, "30 damage taken");
        }
    }
}

#[test]
fn a_hidden_shot_holds_one_slot_until_handle_shot() {
    // Under the caps, 126 others and the two point defence ends fill the
    // table; each ended shot holds its one slot until `_HandleShot`.
    let mut combat = crowded(MAX_SHOTS - 2, true);
    pd_meets(&mut combat, fused_missile(30));
    let mut ships = beside_the_missile();
    test_amid(&mut combat, &mut ships, &mut Rocks::of([]), &mut []);
    assert!(!combat.shot_room(), "both still held");
    combat.shots.pop();
    assert!(combat.shot_room(), "each held once");
    fly_amid(&mut combat, &mut ships);
    assert_eq!(combat.slots.held().count(), 0, "freed");
}
