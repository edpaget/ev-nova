//! The hit-turns port (rdm play-feel-9 phase 10, see [`hit_turns`]): a
//! fake keeper over the test ships, which answers each stretch's strikes
//! with a canned rewrite of the ships, and the phases that stop after each
//! direct hit for it.

use super::collision_walk::{D, fly};
use super::escort_lost_in_place::{cluster_seeking, seekers_of};
use super::plunder_fire::spared;
use super::*;
use crate::combat::hit_turns::{self, HitTurns, Stretch, StretchFn, run_in_turn};

/// What the fake keeper's answer saw of one strike: the strike, each
/// ship's armour as it was answered, and how many shots were in flight.
#[derive(Clone, Debug, PartialEq)]
struct Answered {
    strike: Strike,
    armour: Vec<(ShipRef, f32)>,
    in_flight: usize,
}

impl Answered {
    /// The armour of `ship` as the strike was answered.
    fn armour_of(&self, ship: ShipRef) -> f32 {
        self.armour
            .iter()
            .find(|&&(armoured, _)| armoured == ship)
            .map_or(f32::NAN, |&(_, armour)| armour)
    }
}

/// How the fake keeper answers one strike: a rewrite of the ships.
type Rewrite = fn(&Strike, &mut [Ship]);

/// The default rewrite: the player's direct hit on an NPC turns it on the
/// player, as the reaction block does (`_AIMakeShipAttackPlayer`
/// @0x89d4d, goal 0xd to 4 @0x89d3e).
fn turning(strike: &Strike, ships: &mut [Ship]) {
    if !strike.direct || strike.by != Some(ShipRef::Player) {
        return;
    }
    if let Some(ship) = ships.iter_mut().find(|ship| ship.id == strike.ship) {
        ship.target = Some(ShipRef::Player);
        ship.plundering = false;
        ship.attacking = Some(ShipRef::Player);
    }
}

/// A rewrite that answers nothing.
fn silent(_: &Strike, _: &mut [Ship]) {}

/// A fake keeper of the test ships: it lends each stretch fighters built
/// afresh from its ships, and answers each strike with its rewrite,
/// recording what it saw.
struct Keeper {
    combat: Combat,
    ships: Vec<Ship>,
    arsenal: Arsenal,
    govts: Governments,
    rewrite: Rewrite,
    lends: usize,
    answers: usize,
    answered: Vec<Answered>,
}

impl Keeper {
    /// A keeper of `ships`, answering by `rewrite`.
    fn of(ships: Vec<Ship>, rewrite: Rewrite) -> Self {
        Self {
            combat: Combat::default(),
            ships,
            arsenal: Arsenal::default(),
            govts: Governments::default(),
            rewrite,
            lends: 0,
            answers: 0,
            answered: Vec::new(),
        }
    }

    /// The ship `id` among its ships.
    fn ship(&self, id: ShipRef) -> &Ship {
        self.ships
            .iter()
            .find(|ship| ship.id == id)
            .expect("a ship of the keeper")
    }
}

impl HitTurns for Keeper {
    fn lend(&mut self, stretch: &mut StretchFn<'_>) -> Stretch {
        self.lends += 1;
        let mut fighters: Vec<Fighter> = self.ships.iter_mut().map(Ship::fighter).collect();
        let arena = phase::arena(&self.arsenal, &self.govts, Rules::default());
        stretch(&mut self.combat, &mut fighters, arena, &mut NeverFires)
    }

    fn answer(&mut self) {
        self.answers += 1;
        let (strikes, _, _) = self.combat.take_landed();
        for strike in strikes {
            let armour = self
                .ships
                .iter()
                .map(|ship| (ship.id, ship.reserves.armor.now))
                .collect();
            self.answered.push(Answered {
                strike,
                armour,
                in_flight: self.combat.shots().len(),
            });
            (self.rewrite)(&strike, &mut self.ships);
        }
    }
}

#[test]
fn run_in_turn_lends_until_a_stretch_runs_to_the_end_answering_each_stop() {
    let mut keeper = Keeper::of(vec![Ship::at(1, 0.0, 0.0)], silent);
    let mut script = vec![Stretch::Done, Stretch::Answer, Stretch::Answer];
    run_in_turn(&mut keeper, |_, _, _, _| script.pop().expect("a stretch"));
    assert_eq!((keeper.lends, keeper.answers), (3, 2));
    let mut keeper = Keeper::of(vec![Ship::at(1, 0.0, 0.0)], silent);
    run_in_turn(&mut keeper, |_, _, _, _| Stretch::Done);
    assert_eq!((keeper.lends, keeper.answers), (1, 0), "no stop, no answer");
}

#[test]
fn the_fake_keeper_keeps_the_contract() {
    let ships = vec![Ship::at(1, 0.0, 0.0), Ship::at(2, 100.0, 0.0)];
    let mut keeper = Keeper::of(ships, turning);
    assert_eq!(hit_turns::check_contract(&mut keeper), Ok(()));
    assert_eq!(keeper.ship(A).reserves.shield.now, 10.0, "given back");
}

impl Keeper {
    /// The ships struck in the answers, as (ship hit, by), in order.
    fn answered_strikes(&self) -> Vec<(ShipRef, Option<ShipRef>)> {
        self.answered
            .iter()
            .map(|answered| (answered.strike.ship, answered.strike.by))
            .collect()
    }

    /// Runs `phase` once on its ships, nothing answered: a phase that sets
    /// a test up.
    fn run(
        &mut self,
        mut phase: impl FnMut(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
    ) {
        let _ = self.lend(&mut |combat, fighters, arena, chance| {
            phase(combat, fighters, arena, chance);
            Stretch::Done
        });
    }

    /// The collision passes, each direct hit answered before the next.
    fn test_hits(&mut self) {
        run_in_turn(self, |combat, fighters, arena, chance| {
            combat.test_hits_in_turn(fighters, arena, Scenery::default(), chance)
        });
    }

    /// `_HandleBeams` and the late steps, each beam's hit answered before
    /// the next.
    fn close_tick(&mut self) {
        run_in_turn(self, |combat, fighters, arena, chance| {
            combat.close_tick_in_turn(fighters, arena, &mut [], None, chance)
        });
    }
}

/// The player at (`x`, `y`), facing right.
fn player_at(x: f32, y: f32) -> Ship {
    Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, x, y)
    }
}

/// A beam of 200 pixels and three ticks doing `mass` and `energy` a tick.
fn ray(mass: i16, energy: i16) -> WeaponRecord {
    WeaponRecord {
        mass_dmg: mass,
        energy_dmg: energy,
        ..laser()
    }
}

/// A shell that would destroy a ship of 10 shield and 30 armour.
fn shell() -> WeaponRecord {
    WeaponRecord {
        mass_dmg: 50,
        energy_dmg: 50,
        ..blaster()
    }
}

/// `ship` walked first in the contact passes: below every test ship in
/// its layer.
fn walked_first(ship: Ship) -> Ship {
    Ship {
        sprite: Some(SpriteRank {
            layer: ShipLayer::Ships,
            entry: -100,
        }),
        ..ship
    }
}

/// `record`'s shot by `firer` whose last move, from 10 to the left, ended
/// on (`x`, `y`).
fn ending_on(combat: &mut Combat, (record, firer): (WeaponRecord, ShipRef), x: f32, y: f32) {
    let to = Vec2::new(x, y);
    fly(
        combat,
        (record, firer, ShotLayer::Shots),
        (to - Vec2::new(10.0, 0.0), to),
        None,
    );
}

/// The leader's read (@0x3a97a-0x3a99a): E (C) escorts L (A), which
/// plunders V (B). E doesn't plunder, so its beam at V leaves able to kill
/// (`_SpawnBeam` @0x45270-0x452d1). The player's beam hits L first, and E's
/// hits V after it, in the beams' order: the ships after `_HandleBeams`,
/// with what the keeper's answers saw, each beam's hit answered by
/// `rewrite`.
fn beamed_under_a_plunderer(rewrite: Rewrite) -> (Keeper, Vec<CombatEvent>) {
    let lead = Ship {
        plundering: true,
        ..Ship::at(1, 100.0, 0.0).targeting(B)
    };
    let escort = Ship {
        leader: Some(A),
        fleet: A,
        ..Ship::at(3, 0.0, 300.0).armed(ray(50, 50)).targeting(B)
    };
    let ships = vec![
        player_at(0.0, 0.0).armed(ray(0, 5)),
        lead,
        Ship::at(2, 100.0, 300.0),
        escort,
    ];
    let mut keeper = Keeper::of(ships, rewrite);
    keeper.run(|combat, fighters, arena, chance| combat.fire_player(fighters, arena, chance));
    keeper.run(|combat, fighters, arena, chance| combat.handle_npcs(fighters, arena, chance));
    let beams: Vec<(ShipRef, bool)> = keeper
        .combat
        .beams()
        .iter()
        .map(|beam| (beam.firer, beam.disable_only))
        .collect();
    assert_eq!(beams, [(ShipRef::Player, false), (C, false)]);
    keeper.combat.take_events();
    keeper.close_tick();
    let events = keeper.combat.take_events();
    (keeper, events)
}

#[test]
fn a_beam_after_one_that_turned_its_firers_plundering_leader_can_kill() {
    let (keeper, events) = beamed_under_a_plunderer(turning);
    assert!(broke_up(&events, &[B]), "L no longer plunders");
    assert_eq!(keeper.answered_strikes()[0], (A, Some(ShipRef::Player)));
    assert_eq!(
        keeper.answered[0].armour_of(B),
        30.0,
        "answered before E's beam"
    );
    let (keeper, events) = beamed_under_a_plunderer(silent);
    assert!(spared(keeper.ship(B), &events), "L still plunders");
}

#[test]
fn a_tick_of_beams_answered_hit_by_hit_opens_once() {
    // Each stretch resumes the walk: the beams spend a tick of their life
    // and follow their firers once, however many stops the tick makes.
    let (keeper, _) = beamed_under_a_plunderer(silent);
    assert_eq!((keeper.lends - 2, keeper.answers), (3, 2), "a stop a hit");
    let lives: Vec<u32> = keeper.combat.beams().iter().map(|beam| beam.life).collect();
    assert_eq!(lives, [2, 2], "of 3");
    assert_eq!(keeper.combat.ended_beams, 0);
    assert_eq!(keeper.combat.walk, None, "the walk ran to its end");
}

/// The ships of a contact pass in which the player's shot meets P (A, walked
/// first) and P's shell, fired while it attacked, so able to kill, meets
/// V (B): P plunders V by the pass, as the reaction block's writers would
/// leave it (@0x3bcf6, goal 4 to 0xd). The keeper after the pass, its
/// direct hits answered by `rewrite`, and the events.
fn shelled_in_one_pass(rewrite: Rewrite) -> (Keeper, Vec<CombatEvent>) {
    let plunderer = Ship {
        plundering: true,
        ..walked_first(Ship::at(1, 0.0, 0.0)).targeting(B)
    };
    let ships = vec![player_at(0.0, 500.0), plunderer, Ship::at(2, 100.0, 0.0)];
    let mut keeper = Keeper::of(ships, rewrite);
    ending_on(&mut keeper.combat, (blaster(), ShipRef::Player), 0.0, 0.0);
    ending_on(&mut keeper.combat, (shell(), A), 100.0, 0.0);
    assert!(keeper.combat.shots().iter().all(|shot| !shot.disable_only));
    keeper.test_hits();
    let events = keeper.combat.take_events();
    (keeper, events)
}

#[test]
fn a_plunderer_the_player_turned_earlier_in_the_pass_kills_its_quarry() {
    // `_DamageShip`'s prologue reads P's +0x70 and goal live
    // (@0x3a95b-0x3a964), after the player's hit's reaction wrote them.
    let (keeper, events) = shelled_in_one_pass(turning);
    assert!(broke_up(&events, &[B]), "P targets the player now");
    assert_eq!(keeper.answered_strikes()[0], (A, Some(ShipRef::Player)));
    assert_eq!(keeper.answered[0].armour_of(B), 30.0, "before P's shell");
    let (keeper, events) = shelled_in_one_pass(silent);
    assert!(spared(keeper.ship(B), &events), "P still plunders V");
}

/// The leader's read in a contact pass (@0x3a97a-0x3a99a): E (C) escorts
/// L (A, walked first), which plunders V (B); the player's shot meets L,
/// then E's shell meets V. The keeper after the pass, its direct hits
/// answered by `rewrite`, and the events.
fn escort_shelled_in_one_pass(rewrite: Rewrite) -> (Keeper, Vec<CombatEvent>) {
    let lead = Ship {
        plundering: true,
        ..walked_first(Ship::at(1, 0.0, 0.0)).targeting(B)
    };
    let escort = Ship {
        leader: Some(A),
        fleet: A,
        ..Ship::at(3, 0.0, 300.0).targeting(B)
    };
    let ships = vec![player_at(0.0, 500.0), lead, Ship::at(2, 100.0, 0.0), escort];
    let mut keeper = Keeper::of(ships, rewrite);
    ending_on(&mut keeper.combat, (blaster(), ShipRef::Player), 0.0, 0.0);
    ending_on(&mut keeper.combat, (shell(), C), 100.0, 0.0);
    keeper.test_hits();
    let events = keeper.combat.take_events();
    (keeper, events)
}

#[test]
fn an_escort_whose_leader_the_player_turned_earlier_in_the_pass_kills_its_target() {
    let (_, events) = escort_shelled_in_one_pass(turning);
    assert!(broke_up(&events, &[B]), "L no longer plunders");
    let (keeper, events) = escort_shelled_in_one_pass(silent);
    assert!(spared(keeper.ship(B), &events), "L still plunders");
}

#[test]
fn a_direct_hit_is_answered_before_its_blast_lands() {
    // `_HandleShipHit` damages the blast's ships (@0x36ec0) after the
    // direct hit's `_DamageShip` returns (@0x36c7a).
    let bomb = WeaponRecord {
        blast_radius: 30,
        ..blaster()
    };
    let ships = vec![
        player_at(0.0, 500.0),
        Ship::at(2, 100.0, 0.0),
        Ship::at(3, 100.0, 25.0),
    ];
    let mut keeper = Keeper::of(ships, silent);
    ending_on(&mut keeper.combat, (bomb, ShipRef::Player), 100.0, 0.0);
    keeper.test_hits();
    assert_eq!(keeper.answered_strikes(), [(B, Some(ShipRef::Player))]);
    assert_eq!(keeper.answered[0].armour_of(C), 30.0, "Y untouched");
    let (left, _, _) = keeper.combat.take_landed();
    let blasted: Vec<(ShipRef, bool)> = left
        .iter()
        .map(|strike| (strike.ship, strike.direct))
        .collect();
    assert_eq!(blasted, [(C, false)], "the blast, after");
}

#[test]
fn the_proximity_pass_answers_a_hit_before_its_blast_and_sub_munitions_and_walks_on_by_slot() {
    // `_HandleShipHit(shot, ship, 1)` (@0x3768a): the direct hit's
    // `_DamageShip` (@0x36c7a), then the blast (@0x36ec0), then the
    // sub-munitions (@0x36f47); then the next slot's shot.
    let (shell, _) = cluster_seeking();
    let blasting = WeaponRecord {
        blast_radius: 40,
        mass_dmg: 4,
        ..shell
    };
    let fused = WeaponRecord {
        reload: 1000,
        count: 3,
        speed: 1000,
        mass_dmg: 4,
        prox_radius: 40,
        ..weapon(183)
    };
    let ships = vec![
        player_at(0.0, 500.0),
        Ship::at(1, -300.0, 0.0),
        Ship::at(2, 0.0, -150.0),
        Ship::at(3, 0.0, -100.0),
        Ship::at(4, 300.0, -150.0),
    ];
    let mut keeper = Keeper::of(ships, silent);
    keeper.arsenal = seekers_of(blasting);
    ending_on(&mut keeper.combat, (blasting, ShipRef::Player), 0.0, -122.0);
    ending_on(&mut keeper.combat, (fused, A), 300.0, -120.0);
    keeper.test_hits();
    assert_eq!(
        keeper.answered_strikes(),
        [
            (B, Some(ShipRef::Player)),
            (C, Some(ShipRef::Player)),
            (D, Some(A))
        ],
        "X, then the blast's Y and the next slot's hit on Z"
    );
    let first = &keeper.answered[0];
    assert_eq!(first.armour_of(C), 30.0, "the blast after the answer");
    assert_eq!(
        first.in_flight, 1,
        "only the next slot's shot, no sub-munition"
    );
    assert_eq!(keeper.answered[2].in_flight, 2, "the two sub-munitions");
}

/// A rewrite that turns the walk's ships about: it reverses their sprites'
/// order and puts B in A's fleet, which no walk under way reads again.
fn turning_about(_: &Strike, ships: &mut [Ship]) {
    for ship in ships.iter_mut() {
        if let (ShipRef::Npc(NpcId(id)), Some(sprite)) = (ship.id, ship.sprite.as_mut()) {
            sprite.entry = i64::from(id);
        }
        if ship.id == B {
            ship.leader = Some(A);
            ship.fleet = A;
        }
    }
}

#[test]
fn a_walk_keeps_its_order_across_its_stops() {
    let ships = vec![
        player_at(0.0, 500.0),
        Ship::at(1, 0.0, 0.0),
        Ship::at(2, 100.0, 0.0),
        Ship::at(3, 200.0, 0.0),
    ];
    let mut keeper = Keeper::of(ships, turning_about);
    for x in [0.0, 100.0, 200.0] {
        ending_on(&mut keeper.combat, (blaster(), ShipRef::Player), x, 0.0);
    }
    keeper.test_hits();
    let by = Some(ShipRef::Player);
    assert_eq!(
        keeper.answered_strikes(),
        [(C, by), (B, by), (A, by)],
        "C, B, A, as their sprites stood as the passes began"
    );
    assert_eq!((keeper.lends, keeper.answers), (4, 3));
    let mut keeper = Keeper::of(vec![Ship::at(1, 0.0, 0.0)], turning_about);
    keeper.test_hits();
    assert_eq!((keeper.lends, keeper.answers), (1, 0), "no hit, no stop");
}
