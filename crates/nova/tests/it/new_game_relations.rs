//! The new-game fixture's faction relations reaching the fight: over the
//! checked-in fixture (`nova_data::new_game::fixture::folder`), opened as
//! the app opens it (`DiskData::open_new_game`), under the new game's
//! rulebook and Nova's combat AI, with ships of the wardens and the
//! raiders brought into Hearth through nova-sim's published
//! `fixture::bring_in`:
//!
//! - a raider skiff attacks the player on sight, the raiders' faction
//!   naming `attacks_player: always`;
//! - a warden picket and a raider skiff, their factions naming each other
//!   as enemies, fight: the warden attacks the raider and wears it down,
//!   and the raider answers;
//! - the player destroying a warden picket lowers its legal record with
//!   the wardens by what the wardens' `penalties` make the law's
//!   penalty.
//!
//! What each word maps to is nova-data's (`nova_data::new_game`), and whom
//! a warship attacks and what a crime costs are nova-sim's, each pinned
//! there. This pins only that the fixture, the mapping and the fight meet:
//! every id, gauge and penalty is read from the opened data and the
//! session, none retyped.

use nova_data::new_game::fixture::folder;
use nova_sim::fixture::{bring_in, disable, put_player_at, set_player_shield};
use nova_sim::flight_frame::{StepInput, StepReport};
use nova_sim::{
    AiType, CombatEvent, Crime, Governments, GovtId, LegalCode, NovaAi, NovaLaw, Npc, NpcId, Rules,
    ShipId, ShipRef, StepCollaborators, TargetPick, Vec2,
};

use crate::new_game_play::Play;
use crate::new_game_travel::{new_game_rules, nova_of};

/// How many frames' steps a scene waits at most.
const STEPS: usize = 600;

/// The fixture's start, landed at Hearth Dock, flying out of it in the
/// racked hauler, whose nose turret carries a mass driver.
pub(crate) fn armed_at_hearth() -> Play {
    let mut play = Play::new(&folder(), "hearth", |_| {});
    play.land("hearth-dock");
    let racked = ship(&play, "racked-hauler");
    play.session
        .buy_ship(racked, "Rack", &mut play.chance)
        .expect("bought");
    play.take_off();
    assert_eq!(play.session.pilot().ship(), racked);
    play
}

fn ship(play: &Play, id: &str) -> ShipId {
    ShipId(nova_of(&play.game.ids.ships, id))
}

fn govt(play: &Play, id: &str) -> GovtId {
    GovtId(nova_of(&play.game.ids.factions, id))
}

/// Brings a warship of the fixture's hull `hull`, of the faction
/// `faction`, into the system at `at`.
fn warship(play: &mut Play, hull: &str, faction: &str, at: Vec2) -> NpcId {
    let (ship, govt) = (ship(play, hull), govt(play, faction));
    let id = bring_in(&mut play.session, ship, Some(govt), AiType::Warship, at).expect("brought");
    let npc = npc(play, id).expect("in the system");
    assert_eq!((npc.ship, npc.govt), (ship, Some(govt)), "{hull}");
    id
}

fn npc(play: &Play, id: NpcId) -> Option<&Npc> {
    play.session.npcs().iter().find(|npc| npc.id == id)
}

/// NPC `id`'s shield and armour points, while it is in the system.
fn worn(play: &Play, id: NpcId) -> Option<f32> {
    npc(play, id).map(|npc| npc.reserves.shield.now + npc.reserves.armor.now)
}

/// One frame's step under Nova's AI and law, the player's primary
/// trigger held when `fire`, its shields topped up first so no scene ends
/// with it shot down. A step, not a bare pass of the game's work: the
/// take-off leaves the frame's gain at zero, which only a step's ramp
/// lifts. The fight's events its passes raised, in order: a step reports
/// them, rather than leaving them to `Session::take_combat_events`.
pub(crate) fn step(play: &mut Play, law: NovaLaw, fire: bool) -> Vec<CombatEvent> {
    let most = play.session.reserves().shield.max;
    set_player_shield(&mut play.session, most);
    let input = StepInput {
        window_width: 1024,
        fire: (fire, false),
        ..StepInput::default()
    };
    let behaviour = NovaAi::from_rulebook(&new_game_rules());
    let mut collab = StepCollaborators {
        catalog: &play.data,
        behaviour: &behaviour,
        rules: Rules {
            law: &law,
            ..Rules::default()
        },
        chance: &mut play.chance,
    };
    let mut report = StepReport::default();
    play.session.step(&input, 1, &mut collab, &mut report);
    report
        .steps
        .into_iter()
        .flat_map(|run| run.passes)
        .flat_map(|pass| pass.combat)
        .collect()
}

/// Whether a shot `firer` fired is in flight.
fn fired(play: &Play, firer: NpcId) -> bool {
    play.session
        .shots()
        .iter()
        .any(|shot| shot.firer == ShipRef::Npc(firer))
}

/// The ship NPC `id`'s goal attacks.
fn attacking(play: &Play, id: NpcId) -> Option<ShipRef> {
    npc(play, id).and_then(|npc| npc.goal.attacking())
}

#[test]
fn a_raider_attacks_the_player_on_sight() {
    let mut play = armed_at_hearth();
    let law = NovaLaw::default();
    let at = play.session.player().position;
    let raider = warship(
        &mut play,
        "raider-skiff",
        "raider",
        at + Vec2::new(200.0, 0.0),
    );
    let (mut attacks, mut fires) = (false, false);
    for _ in 0..STEPS {
        step(&mut play, law, false);
        attacks |= attacking(&play, raider) == Some(ShipRef::Player);
        fires |= fired(&play, raider);
        if attacks && fires {
            break;
        }
    }
    assert!(attacks, "its goal attacks the player");
    assert!(fires, "it fires");
}

#[test]
fn a_warden_and_a_raider_in_one_system_fight() {
    let mut play = armed_at_hearth();
    let law = NovaLaw::default();
    let at = play.session.player().position;
    let raider_at = at + Vec2::new(200.0, 0.0);
    let raider = warship(&mut play, "raider-skiff", "raider", raider_at);
    let warden = warship(
        &mut play,
        "warden-picket",
        "warden",
        raider_at + Vec2::new(150.0, 0.0),
    );
    let fresh = worn(&play, raider).expect("in the system");
    let (mut hunts, mut fires, mut wears, mut answers) = (false, false, false, false);
    for _ in 0..STEPS {
        step(&mut play, law, false);
        // A ship that has left (through a gate, or broken up) has nothing
        // more to show.
        let Some(now) = worn(&play, raider) else {
            break;
        };
        hunts |= attacking(&play, warden) == Some(ShipRef::Npc(raider));
        fires |= fired(&play, warden);
        let struck = now < fresh;
        // Only the warden fires on the raider: the player holds its fire.
        wears |= struck;
        answers |= struck && attacking(&play, raider) == Some(ShipRef::Npc(warden));
        if hunts && fires && wears && answers {
            break;
        }
    }
    assert!(hunts, "the warden's goal attacks the raider");
    assert!(fires, "the warden fires");
    assert!(wears, "the raider is worn down");
    assert!(answers, "the raider answers the warden's strike");
}

#[test]
fn destroying_a_warden_lowers_the_player_s_record_with_the_wardens() {
    let mut play = armed_at_hearth();
    let law = NovaLaw::default();
    let wardens = govt(&play, "warden");
    let at = play.session.player().position;
    let warden = warship(
        &mut play,
        "warden-picket",
        "warden",
        at + Vec2::new(0.0, -80.0),
    );
    // No raider: only the player's strikes break the warden up.
    let before = play.session.pilot().legal_record(wardens);
    // Disabled first, so the player's one strike that breaks it up is a
    // kill alone.
    disable(&mut play.session, warden);
    put_player_at(&mut play.session, at);
    assert_eq!(
        play.session.select_target(TargetPick::Nearest),
        Some(warden)
    );
    for _ in 0..STEPS {
        if npc(&play, warden).is_none() {
            break;
        }
        step(&mut play, law, true);
    }
    assert!(npc(&play, warden).is_none(), "broken up");
    let after = play.session.pilot().legal_record(wardens);
    assert!(after < before, "{before} then {after}");
    let penalty = law
        .penalties(Crime::Kill, Some(wardens), &Governments::read(&play.data))
        .into_iter()
        .find(|&(govt, _)| govt == wardens)
        .map_or(0, |(_, change)| change);
    assert_eq!(i32::from(after), i32::from(before) + penalty);
}
