//! The fight's in-phase reads of what a reaction writes (rdm play-feel-9
//! phase 11, see [`hit_turns`](crate::combat::hit_turns)): each read
//! after a direct hit in the same phase reads the ships as that hit's
//! answer left them, two hits in one phase, each answered by the fake
//! keeper before the next. Each scene runs twice: answered by `turning`,
//! the player's hit turning its NPC on the player, and by `silent`, which
//! answers nothing, so the read is what decides the outcome.

use super::escort_lost_in_place::cluster_seeking;
use super::hit_turns::{
    Keeper, Rewrite, ending_on, player_at, ray, shell, silent, turning, walked_first,
};
use super::*;

/// Read 1, the beam's intended ship: N (A) targets M (B) and fires its
/// beam at M with no ship aimed at, as a beam fired at nothing
/// (`_SpawnBeam` +0x18 = -1, @0x3146b). The player's beam comes first in
/// the beams' order and hits N. The keeper after `_HandleBeams`, each
/// beam's hit answered by `rewrite`.
fn beamed_by_a_firer_aimed_at_nothing(rewrite: Rewrite) -> Keeper {
    let ships = vec![
        player_at(0.0, 0.0).armed(ray(0, 5)),
        Ship::at(1, 100.0, 0.0).armed(ray(0, 5)).targeting(B),
        Ship::at(2, 200.0, 0.0),
    ];
    let mut keeper = Keeper::of(ships, rewrite);
    keeper.run(|combat, fighters, arena, chance| combat.fire_player(fighters, arena, chance));
    keeper.run(|combat, fighters, arena, chance| combat.handle_npcs(fighters, arena, chance));
    for beam in &mut keeper.combat.beams {
        beam.aimed_at = None;
    }
    let firers: Vec<ShipRef> = keeper
        .combat
        .beams()
        .iter()
        .map(|beam| beam.firer)
        .collect();
    assert_eq!(firers, [ShipRef::Player, A]);
    keeper.close_tick();
    keeper
}

#[test]
fn a_beam_fired_at_nothing_reads_its_firers_target_as_an_earlier_beams_answer_left_it() {
    // With no ship aimed at, `_HandleBeams` reads its firer's live +0x70
    // at the hit (@0x31597-0x315b4) for `_DamageShip`'s intended ship.
    let keeper = beamed_by_a_firer_aimed_at_nothing(turning);
    assert_eq!(
        keeper.answered_strikes(),
        [(A, Some(ShipRef::Player)), (B, Some(A))]
    );
    assert!(!keeper.answered[1].strike.intended, "N targets the player");
    let keeper = beamed_by_a_firer_aimed_at_nothing(silent);
    assert_eq!(keeper.answered_strikes()[1], (B, Some(A)));
    assert!(keeper.answered[1].strike.intended, "N still targets M");
}

/// Read 2, a miner's beam: M (A) mines, and its beam's reach holds X (B).
/// The player's beam hits M first. The keeper after `_HandleBeams`, each
/// beam's hit answered by `rewrite`.
fn beamed_from_a_miner(rewrite: Rewrite) -> Keeper {
    let miner = Ship {
        mining: true,
        ..Ship::at(1, 100.0, 0.0).armed(ray(0, 5)).targeting(B)
    };
    let ships = vec![
        player_at(0.0, 0.0).armed(ray(0, 5)),
        miner,
        Ship::at(2, 160.0, 0.0),
    ];
    let mut keeper = Keeper::of(ships, rewrite);
    keeper.run(|combat, fighters, arena, chance| combat.fire_player(fighters, arena, chance));
    keeper.run(|combat, fighters, arena, chance| combat.handle_npcs(fighters, arena, chance));
    assert_eq!(keeper.combat.beams().len(), 2);
    keeper.close_tick();
    keeper
}

#[test]
fn a_miners_beam_after_one_that_turned_it_meets_the_ship_in_its_reach() {
    // `_AIShipIsAttackingAsteroid(firer)` (@0x30f71-0x30f8c) reads the
    // firer's goal live, as the answer to the player's beam left it.
    let keeper = beamed_from_a_miner(turning);
    assert_eq!(
        keeper.answered_strikes(),
        [(A, Some(ShipRef::Player)), (B, Some(A))],
        "M mines no more"
    );
    let keeper = beamed_from_a_miner(silent);
    assert_eq!(
        keeper.answered_strikes(),
        [(A, Some(ShipRef::Player))],
        "M's beam passes X"
    );
    assert_eq!(keeper.ship(B).reserves.shield.now, 10.0);
}

/// Read 4, a miner's shot in the contact walk: M (A, walked first) mines;
/// the player's shot ends on M, and M's shell on X (B). The keeper after
/// the passes, each direct hit answered by `rewrite`.
fn shelled_by_a_miner(rewrite: Rewrite) -> Keeper {
    let miner = Ship {
        mining: true,
        ..walked_first(Ship::at(1, 0.0, 0.0))
    };
    let ships = vec![player_at(0.0, 500.0), miner, Ship::at(2, 100.0, 0.0)];
    let mut keeper = Keeper::of(ships, rewrite);
    ending_on(&mut keeper.combat, (blaster(), ShipRef::Player), 0.0, 0.0);
    ending_on(&mut keeper.combat, (shell(), A), 100.0, 0.0);
    keeper.test_hits();
    keeper
}

#[test]
fn a_miners_shot_later_in_the_contact_walk_meets_the_ship_once_the_player_turned_it() {
    // `_ShotCanHitShip` (@0x4477e, called @0x37002) reads the source
    // slot's goal live (`_AIShipIsAttackingAsteroid` @0x44a54-0x44a6f).
    let keeper = shelled_by_a_miner(turning);
    assert_eq!(
        keeper.answered_strikes(),
        [(A, Some(ShipRef::Player)), (B, Some(A))],
        "M mines no more"
    );
    let keeper = shelled_by_a_miner(silent);
    assert_eq!(keeper.answered_strikes(), [(A, Some(ShipRef::Player))]);
    assert_eq!(keeper.ship(B).reserves.armor.now, 30.0, "X untouched");
    assert_eq!(keeper.ship(B).reserves.shield.now, 10.0);
}

/// Read 5, a proximity hit's sub-munitions: P (A, walked first) plunders
/// V (B), and its fused shell with sub-munitions ends beside V. The
/// player's shot meets P in the contact walk; then the shell fuses at V
/// in the proximity pass and releases. Each released sub-munition's
/// disable-only flag, and the strikes answered, each direct hit answered
/// by `rewrite`.
fn released_beside_the_quarry(rewrite: Rewrite) -> (Vec<bool>, Vec<(ShipRef, Option<ShipRef>)>) {
    let (cluster, arsenal) = cluster_seeking();
    let plunderer = Ship {
        plundering: true,
        ..walked_first(Ship::at(1, 0.0, 0.0)).targeting(B)
    };
    let ships = vec![player_at(0.0, 500.0), plunderer, Ship::at(2, 100.0, 0.0)];
    let mut keeper = Keeper::of(ships, rewrite);
    keeper.arsenal = arsenal;
    ending_on(&mut keeper.combat, (blaster(), ShipRef::Player), 0.0, 0.0);
    ending_on(&mut keeper.combat, (cluster, A), 100.0, -25.0);
    keeper.test_hits();
    let released = keeper
        .combat
        .shots()
        .iter()
        .filter(|shot| shot.weapon.id == WeaponId(148))
        .map(|shot| shot.disable_only)
        .collect();
    (released, keeper.answered_strikes())
}

#[test]
fn a_proximity_hits_sub_munitions_read_their_owner_as_an_earlier_contact_hits_answer_left_it() {
    // Each `_SpawnShot` of `_SpawnShotSubmunitions` (@0x36f47) reads its
    // source slot's goal (`_AIIsShipAttemptingDisable` @0x3ea1b) for the
    // disable-only byte (@0x3ea32), by `RuleKey::SubmunitionSourceSlot`'s
    // engine reading, the default.
    let (released, answered) = released_beside_the_quarry(turning);
    assert_eq!(released, [false, false]);
    let by_p = [(A, Some(ShipRef::Player)), (B, Some(A))];
    assert_eq!(answered, by_p, "the contact hit, then the fuse's");
    let (released, answered) = released_beside_the_quarry(silent);
    assert_eq!(released, [true, true], "P still plunders");
    assert_eq!(answered, by_p);
}
