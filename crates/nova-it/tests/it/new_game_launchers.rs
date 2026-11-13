//! The new-game fixture's launcher and bay reaching the fight: over the
//! checked-in fixture (`nova_data::new_game::fixture::folder`), opened as
//! the app opens it (`DiskData::open_new_game`), under the new game's
//! rulebook and Nova's combat AI:
//!
//! - Hearth Dock's outfitter sells the seekers only to a hull with a
//!   launcher hardpoint, and the tender comes with a load of them;
//! - the tender's seeker launcher, selected with Weapon Select and fired
//!   on the secondary trigger, spends a seeker a shot and fires a homing
//!   shot at the target, and nothing once the seekers are gone;
//! - the tender's drone bay launches its drones as carried escorts up to
//!   its load, and Return to Hangar takes them back aboard;
//! - a warden tender fighting a raider, brought in through nova-sim's
//!   published `fixture::bring_in`, launches its drones up to its load and
//!   takes them back once the fight is over.
//!
//! What each word maps to is nova-data's (`nova_data::new_game`), and how
//! a weapon fires, a bay launches and a fighter docks is nova-sim's, each
//! pinned there. This pins only that the fixture, the mapping and the
//! fight meet: every id, load, price and reload is read from the opened
//! data and the session, none retyped.

use nova_data::new_game::fixture::folder;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::weapon::Weapon;
use nova_sim::combat::weapon::Guidance;
use nova_sim::fixture::{bring_in, destroy, put_player_at, repair};
use nova_sim::flight_frame::FlightCommand;
use nova_sim::outfitter::OutfitOrder;
use nova_sim::{
    AiType, CombatEvent, Direction, EscortCommand, EscortGroup, GovtId, NovaLaw, NpcId, OutfitId,
    ShipId, ShipRef, TargetPick, Vec2, WeaponId,
};

use crate::new_game_play::Play;
use crate::new_game_relations::step_firing;
use crate::new_game_travel::nova_of;

/// How many frames' steps a scene waits at most.
const STEPS: usize = 1200;

fn ship(play: &Play, id: &str) -> ShipId {
    ShipId(nova_of(&play.game.ids.ships, id))
}

fn outfit(play: &Play, id: &str) -> OutfitId {
    OutfitId(nova_of(&play.game.ids.outfits, id))
}

fn weapon(play: &Play, id: &str) -> WeaponId {
    WeaponId(nova_of(&play.game.ids.weapons, id))
}

fn govt(play: &Play, id: &str) -> GovtId {
    GovtId(nova_of(&play.game.ids.factions, id))
}

/// The drone bay's drones: its fighters' `oütf`.
fn drones(play: &Play) -> OutfitId {
    OutfitId(nova_of(&play.game.ids.fighters, "drone-bay"))
}

/// The opened `shïp` of the fixture's hull `id`.
fn hull(play: &Play, id: &str) -> Ship {
    record(play, ship(play, id).0)
}

fn record<T: nova_data::Registered + Clone>(play: &Play, nova: i16) -> T {
    (play.data.get::<T>(nova))
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

/// The `AmmoLoad` of `ship`'s stock slot of `weapon`.
fn load(ship: &Ship, weapon: WeaponId) -> u16 {
    let types = ship.weap_type1_4.iter().chain(&ship.weap_type5_8);
    let loads = ship.ammo_load1_4.iter().chain(&ship.ammo_load5_8);
    let load = types
        .zip(loads)
        .find(|(held, _)| held.map(|held| held.0) == Some(weapon.0))
        .map(|(_, &load)| load)
        .expect("a slot of the weapon");
    u16::try_from(load).expect("a load")
}

/// `weapon`'s `Reload`, in ticks, as the opened data holds it.
fn reload(play: &Play, weapon: WeaponId) -> usize {
    let reload = record::<Weapon>(play, weapon.0).reload;
    usize::try_from(reload).expect("a reload")
}

/// The fixture's start, landed at Hearth Dock.
fn at_hearth_dock() -> Play {
    let mut play = Play::new(&folder(), "hearth", |_| {});
    play.land("hearth-dock");
    play
}

/// The fixture's start, having bought the tender at Hearth Dock, landed.
fn bought_the_tender() -> Play {
    let mut play = at_hearth_dock();
    let tender = ship(&play, "tender");
    play.session
        .buy_ship(tender, "Tender", &mut play.chance)
        .expect("bought");
    assert_eq!(play.session.pilot().ship(), tender);
    play
}

/// The fixture's start, flying out of Hearth Dock in the tender.
fn in_the_tender() -> Play {
    let mut play = bought_the_tender();
    play.take_off();
    play
}

/// Whether the outfitter where the ship is docked lists `outfit`.
fn listed(play: &mut Play, outfit: OutfitId) -> bool {
    let outfitter = play
        .session
        .outfitter(&mut play.chance)
        .expect("an outfitter");
    outfitter.rows.iter().any(|row| row.id == outfit)
}

fn buy(play: &mut Play, outfit: OutfitId) -> bool {
    let order = OutfitOrder {
        outfit,
        direction: Direction::Buy,
    };
    play.session.outfit(order, &mut play.chance).is_ok()
}

/// How many times `events` say the player fired `weapon`.
fn fired(events: &[CombatEvent], weapon: WeaponId) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(event, CombatEvent::Fired { ship: ShipRef::Player, weapon: fired, .. }
                if *fired == weapon)
        })
        .count()
}

/// Presses Weapon Select, a step after each press, until the secondary is
/// `weapon`: at most once per secondary the tender carries.
fn select(play: &mut Play, weapon: WeaponId) {
    for _ in 0..2 {
        if play.session.secondary() == Some(weapon) {
            return;
        }
        play.command(FlightCommand::SelectWeapon { backwards: false });
        step_firing(play, NovaLaw::default(), (false, false));
    }
    assert_eq!(play.session.secondary(), Some(weapon), "selected");
}

#[test]
fn hearth_dock_sells_seekers_to_a_hull_with_a_launcher_alone() {
    let mut play = at_hearth_dock();
    let seekers = outfit(&play, "seekers");
    assert!(!listed(&mut play, seekers), "the hauler has no launcher");
    assert!(!buy(&mut play, seekers), "nor can it buy them");
    assert_eq!(play.session.pilot().owned(seekers), 0);

    let tender = ship(&play, "tender");
    play.session
        .buy_ship(tender, "Tender", &mut play.chance)
        .expect("bought");
    let launcher = weapon(&play, "seeker-launcher");
    let loaded = load(&hull(&play, "tender"), launcher);
    assert!(loaded > 0);
    assert_eq!(
        play.session.pilot().owned(outfit(&play, "seeker-launcher")),
        1
    );
    assert_eq!(
        play.session.pilot().owned(seekers),
        loaded,
        "the tender comes loaded"
    );

    assert!(
        listed(&mut play, seekers),
        "a launcher hull is offered them"
    );
    let cash = play.session.pilot().cash();
    assert!(buy(&mut play, seekers), "bought");
    assert_eq!(play.session.pilot().owned(seekers), loaded + 1);
    let cost = record::<Outfit>(&play, seekers.0).cost;
    assert_eq!(play.session.pilot().cash(), cash - i64::from(cost));
}

#[test]
fn firing_the_seeker_launcher_spends_a_round_on_a_homing_shot() {
    let mut play = in_the_tender();
    let law = NovaLaw::default();
    let launcher = weapon(&play, "seeker-launcher");
    let seekers = outfit(&play, "seekers");
    let at = play.session.player().position;
    put_player_at(&mut play.session, at);
    let skiff = ship(&play, "raider-skiff");
    let near = at + Vec2::new(0.0, -120.0);
    let target = bring_in(&mut play.session, skiff, None, AiType::Warship, near).expect("brought");
    assert_eq!(
        play.session.select_target(TargetPick::Nearest),
        Some(target)
    );
    select(&mut play, launcher);

    let (mut shots, mut homing) = (0, false);
    for _ in 0..STEPS {
        let before = play.session.pilot().owned(seekers);
        if before == 0 {
            break;
        }
        repair(&mut play.session, target);
        let events = step_firing(&mut play, law, (false, true));
        let spent = usize::from(before - play.session.pilot().owned(seekers));
        assert_eq!(spent, fired(&events, launcher), "a seeker a shot");
        shots += spent;
        homing |= play.session.shots().iter().any(|shot| {
            shot.firer == ShipRef::Player
                && shot.weapon.id == launcher
                && shot.weapon.guidance == Guidance::Homing
                && shot.weapon.turn > 0.0
                && shot.target == Some(ShipRef::Npc(target))
        });
    }
    assert_eq!(play.session.pilot().owned(seekers), 0, "all spent");
    assert!(shots > 0, "it fired");
    assert!(homing, "a homing shot at the target");
    for _ in 0..3 * reload(&play, launcher) {
        let events = step_firing(&mut play, law, (false, true));
        assert_eq!(fired(&events, launcher), 0, "nothing to fire");
    }
}

/// The player's carried escorts, and the NPCs flying out of its bay.
fn out_of_the_player_s_bay(play: &Play) -> (usize, Vec<ShipId>) {
    let carried = (play.session.pilot().escorts().iter())
        .filter(|escort| escort.carried)
        .count();
    let flying = (play.session.npcs().iter())
        .filter(|npc| {
            npc.carrier
                .is_some_and(|carrier| carrier.ship == ShipRef::Player)
        })
        .map(|npc| npc.ship)
        .collect();
    (carried, flying)
}

#[test]
fn the_tender_launches_its_drones_up_to_its_count_and_takes_them_back() {
    let mut play = in_the_tender();
    let law = NovaLaw::default();
    let bay = weapon(&play, "drone-bay");
    let drones = drones(&play);
    let drone = ship(&play, "drone");
    let count = play.session.pilot().owned(drones);
    assert_eq!(count, load(&hull(&play, "tender"), bay), "its load aboard");
    assert!(count > 0);
    let count = usize::from(count);
    select(&mut play, bay);

    // Every drone is aboard or out of the bay as a carried escort flying
    // the drone: a bay never launches more than it holds.
    let check = |play: &Play| {
        let (carried, flying) = out_of_the_player_s_bay(play);
        let aboard = usize::from(play.session.pilot().owned(drones));
        assert_eq!(carried + aboard, count, "{carried} out, {aboard} aboard");
        assert_eq!(flying.len(), carried, "each flies");
        assert!(flying.iter().all(|&ship| ship == drone), "{flying:?}");
        carried
    };
    let (mut emptied, mut launches) = (false, 0);
    for _ in 0..STEPS {
        if play.session.pilot().owned(drones) == 0 {
            emptied = true;
            break;
        }
        let events = step_firing(&mut play, law, (false, true));
        launches += fired(&events, bay);
        check(&play);
    }
    assert!(emptied, "every drone launched");
    assert_eq!(launches, count, "a launch a drone");
    for _ in 0..3 * reload(&play, bay) {
        let events = step_firing(&mut play, law, (false, true));
        assert_eq!(fired(&events, bay), 0, "nothing left to launch");
        check(&play);
    }
    assert_eq!(check(&play), count, "every drone out, and no more");

    play.command(FlightCommand::CommandEscorts {
        group: EscortGroup::All,
        command: EscortCommand::Dock,
    });
    let mut docked = false;
    for _ in 0..STEPS {
        step_firing(&mut play, law, (false, false));
        if check(&play) == 0 {
            docked = true;
            break;
        }
    }
    assert!(docked, "{:?}", out_of_the_player_s_bay(&play));
    assert_eq!(
        usize::from(play.session.pilot().owned(drones)),
        count,
        "aboard again"
    );
    assert!(play.session.npcs().iter().all(|npc| npc.ship != drone));
}

/// The NPCs flying out of NPC `carrier`'s bay, as their hulls and
/// governments.
fn out_of_the_bay_of(play: &Play, carrier: NpcId) -> Vec<(ShipId, Option<GovtId>)> {
    (play.session.npcs().iter())
        .filter(|npc| {
            npc.carrier
                .is_some_and(|out_of| out_of.ship == ShipRef::Npc(carrier))
        })
        .map(|npc| (npc.ship, npc.govt))
        .collect()
}

/// Repairs every NPC in the system, so the fight neither ends nor loses a
/// ship.
fn repair_all(play: &mut Play) {
    let ids: Vec<NpcId> = play.session.npcs().iter().map(|npc| npc.id).collect();
    for id in ids {
        repair(&mut play.session, id);
    }
}

#[test]
fn a_warden_tender_fighting_a_raider_launches_its_drones_up_to_its_count_and_takes_them_back() {
    let mut play = at_hearth_dock();
    play.take_off();
    let law = NovaLaw::default();
    let at = play.session.player().position + Vec2::new(0.0, -400.0);
    let (wardens, raiders) = (govt(&play, "warden"), govt(&play, "raider"));
    let bay = weapon(&play, "drone-bay");
    let drone = ship(&play, "drone");
    let count = usize::from(load(&hull(&play, "tender"), bay));
    assert!(count > 0);
    let (tender_hull, skiff) = (ship(&play, "tender"), ship(&play, "raider-skiff"));
    let tender = bring_in(
        &mut play.session,
        tender_hull,
        Some(wardens),
        AiType::Warship,
        at,
    )
    .expect("brought");
    let raider = bring_in(
        &mut play.session,
        skiff,
        Some(raiders),
        AiType::Warship,
        at + Vec2::new(200.0, 0.0),
    )
    .expect("brought");
    let rounds = |play: &Play| {
        let npc = (play.session.npcs().iter())
            .find(|npc| npc.id == tender)
            .expect("the tender stays");
        usize::try_from(npc.rounds.slot(bay)).expect("rounds")
    };
    assert_eq!(rounds(&play), count, "its class's load aboard");
    // Every drone is aboard or out of the bay, each out a warden drone: a
    // bay never launches more than it holds.
    let check = |play: &Play| {
        let out = out_of_the_bay_of(play, tender);
        assert!(
            out.iter().all(|&flown| flown == (drone, Some(wardens))),
            "{out:?}"
        );
        assert_eq!(out.len() + rounds(play), count, "{out:?}");
        out.len()
    };

    let attacking = |play: &Play| {
        (play.session.npcs().iter())
            .find(|npc| npc.id == tender)
            .and_then(|npc| npc.goal.attacking())
            == Some(ShipRef::Npc(raider))
    };
    let mut attacks = false;
    for _ in 0..STEPS {
        repair_all(&mut play);
        step_firing(&mut play, law, (false, false));
        check(&play);
        if attacking(&play) {
            attacks = true;
            break;
        }
    }
    assert!(attacks, "the tender's goal attacks the raider");
    let mut emptied = false;
    for _ in 0..STEPS {
        repair_all(&mut play);
        step_firing(&mut play, law, (false, false));
        if check(&play) == count {
            emptied = true;
            break;
        }
    }
    assert!(emptied, "every drone out: {} aboard", rounds(&play));

    // With the raider gone, the tender has no target: it stops attacking
    // and calls its drones home.
    destroy(&mut play.session, raider);
    let mut docked = false;
    for _ in 0..STEPS {
        repair_all(&mut play);
        step_firing(&mut play, law, (false, false));
        if check(&play) == 0 {
            docked = true;
            break;
        }
    }
    assert!(docked, "{:?}", out_of_the_bay_of(&play, tender));
    assert_eq!(rounds(&play), count, "aboard again");
    assert!(play.session.npcs().iter().all(|npc| npc.ship != drone));
}
