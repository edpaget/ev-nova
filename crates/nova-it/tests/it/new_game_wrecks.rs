//! The new-game fixture's kits and wrecks reaching the ships that fly and
//! die, over the checked-in fixture (`nova_data::new_game::fixture::folder`)
//! opened as the app opens it, under the new game's rulebook:
//!
//! - **A kit on an NPC.** The Fringe's raider band flies the armoured
//!   skiff, a variant carrying the `welded-plate` kit as its default
//!   item. An NPC flying it holds its hull's `Armor` plus the kit's
//!   `MORE_ARMOR`, as the new game's reading of `NpcDefaultItems` counts
//!   an NPC's default items.
//! - **An NPC's wreck.** A raider skiff the player shoots down breaks up
//!   with its `Explode1`, stays dying for more than half and at most all
//!   of its `DeathDelay`, ejects its `PodCount` escape pods, then makes
//!   its final explosion, its `Explode2`, and is gone. A warden picket,
//!   which names no wreck, dies the same way by the default: no pods.
//! - **The player's death.** Through the app as `main` wires it, a raider
//!   breaks a new pilot's hauler up, and the app stays in flight through
//!   the break-up, for at least the hauler's `DeathDelay`, before its
//!   final explosion and the main menu; the downed beep is heard as the
//!   break-up starts, and the explosion's halt only frames later.
//!
//! What each format word maps to is nova-data's (`nova_data::new_game`),
//! and how a ship dies is nova-sim's, each pinned there. This pins only
//! that the fixture, the mapping and the session meet: every delay, count,
//! explosion and armour value is read from the opened data and the
//! session, and every tick count is a bound read from the record.

use nova::app::Showing;
use nova::chance::SplitMix;
use nova_audio::AudioCommand;
use nova_data::new_game::fixture::folder;
use nova_data::new_game::kits::mod_type;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::{GameData, Registered};
use nova_sim::catalog::Explosion;
use nova_sim::fixture::{bring_in, disable, put_player_at, set_player_armor, set_player_shield};
use nova_sim::{
    AiType, CombatEvent, NovaLaw, NpcId, PLAYER_EXPLOSION_HALTS, PlayerFate, ShipId, ShipRef,
    SimSound, TargetPick, Vec2,
};

use crate::new_game_conversation::flying;
use crate::new_game_play::Play;
use crate::new_game_relations::{armed_at_hearth, step};
use crate::new_game_travel::nova_of;
use crate::pilots::Harness;

/// The seeds the Fringe is filled on, at most, before an armoured skiff
/// comes in.
const SEEDS: u64 = 64;

/// How many steps a wreck is waited on at most.
const STEPS: usize = 600;

/// How many frames the player's death is waited on at most.
const FRAMES: usize = 4000;

/// `id`'s record in the data the app opened.
fn record<T: Registered + Clone>(data: &GameData, id: i16) -> T {
    data.get::<T>(id)
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

/// The armour `kit`'s effects add, from its `MORE_ARMOR` slots.
fn armor_delta(kit: &Outfit) -> f32 {
    [
        (kit.mod_type, kit.mod_val),
        (kit.mod_type2, kit.mod_val2),
        (kit.mod_type3, kit.mod_val3),
        (kit.mod_type4, kit.mod_val4),
    ]
    .into_iter()
    .filter(|&(kind, _)| kind == mod_type::MORE_ARMOR)
    .map(|(_, val)| f32::from(val))
    .sum()
}

#[test]
#[allow(clippy::float_cmp)]
fn an_npc_flying_the_armoured_skiff_holds_its_hull_s_armour_and_its_plate_s() {
    let mut play = Play::new(&folder(), "fringe", |_| {});
    let ids = &play.game.ids;
    let armoured = nova_of(&ids.ships, "armoured-skiff");
    let hull = f32::from(record::<Ship>(&play.data, armoured).armor);
    let plate = armor_delta(&record::<Outfit>(
        &play.data,
        nova_of(&ids.kits, "welded-plate"),
    ));
    assert!(plate != 0.0, "the plate adds armour");
    for seed in 0..SEEDS {
        play.session.populate(&play.data, &mut SplitMix::new(seed));
        if let Some(npc) = play
            .session
            .npcs()
            .iter()
            .find(|npc| npc.ship == ShipId(armoured))
        {
            assert_eq!(npc.reserves.armor.max, hull + plate, "seed {seed}");
            assert_eq!(npc.reserves.armor.now, hull + plate, "seed {seed}: full");
            return;
        }
    }
    panic!("no armoured skiff over {SEEDS} seeds");
}

/// What one NPC's death showed, step by step from its break-up.
#[derive(Debug, Default)]
struct Death {
    /// The explosion of each `BreakingUp` it raised.
    breaking_up: Vec<Option<Explosion>>,
    /// The pods it ejected before its `Destroyed`.
    pods: usize,
    /// Its `Destroyed`'s explosion, and the steps from the break-up it
    /// came on.
    destroyed: Option<(Option<Explosion>, usize)>,
    /// The pods it ejected after its `Destroyed`.
    late_pods: usize,
    /// The steps from the break-up until it was gone from the system.
    gone_after: Option<usize>,
}

/// The fixture's hull `hull`, of no government (so it neither fights nor
/// costs a record), brought into Hearth 80 pixels ahead of an armed
/// player, disabled, targeted and shot until it breaks up, then watched
/// until it is gone.
fn shoot_down(hull: &str) -> (Play, Ship, Death) {
    let mut play = armed_at_hearth();
    let law = NovaLaw::default();
    let ship = ShipId(nova_of(&play.game.ids.ships, hull));
    let at = play.session.player().position;
    let id = bring_in(
        &mut play.session,
        ship,
        None,
        AiType::Warship,
        at + Vec2::new(0.0, -80.0),
    )
    .expect("brought in");
    disable(&mut play.session, id);
    put_player_at(&mut play.session, at);
    assert_eq!(play.session.select_target(TargetPick::Nearest), Some(id));
    let record = record::<Ship>(&play.data, ship.0);
    let mut death = Death::default();
    let mut since: Option<usize> = None;
    for _ in 0..STEPS {
        let events = step(&mut play, law, since.is_none());
        if let Some(steps) = since.as_mut() {
            *steps += 1;
        }
        for event in events {
            match event {
                CombatEvent::BreakingUp {
                    ship: ShipRef::Npc(of),
                    explosion,
                    ..
                } if of == id => {
                    death.breaking_up.push(explosion);
                    since.get_or_insert(0);
                }
                CombatEvent::PodEjected {
                    ship: ShipRef::Npc(of),
                    ..
                } if of == id => {
                    if death.destroyed.is_some() {
                        death.late_pods += 1;
                    } else {
                        death.pods += 1;
                    }
                }
                CombatEvent::Destroyed {
                    ship: ShipRef::Npc(of),
                    explosion,
                    ..
                } if of == id => {
                    assert!(death.destroyed.is_none(), "destroyed once");
                    death.destroyed = Some((explosion, since.expect("after its break-up")));
                }
                _ => {}
            }
        }
        if !alive(&play, id) {
            death.gone_after = since;
            return (play, record, death);
        }
    }
    panic!("{hull} never gone: {death:?}");
}

/// Whether NPC `id` is still in the system.
fn alive(play: &Play, id: NpcId) -> bool {
    play.session.npcs().iter().any(|npc| npc.id == id)
}

/// Asserts that `death` ran the whole of `record`'s wreck: its break-up
/// explosion, its dying for more than half and at most all of its
/// `DeathDelay` in steps from the break-up, its pods before its final
/// explosion, that explosion, then gone.
fn died_by_its_wreck(record: &Ship, death: &Death) {
    let delay = usize::try_from(record.death_delay).expect("a delay");
    let pods = usize::try_from(record.pod_count).expect("a count");
    let puffs = Explosion::decode(record.explode1);
    assert!(puffs.is_some(), "{record:?}");
    assert_eq!(death.breaking_up, [puffs], "breaks up once: {death:?}");
    let (explosion, at) = death.destroyed.expect("a final explosion");
    assert_eq!(explosion, Explosion::decode(record.explode2), "{death:?}");
    assert!(
        at > delay / 2 && at <= delay,
        "exploded {at} steps into a DeathDelay of {delay}"
    );
    assert_eq!(death.gone_after, Some(at), "gone with its explosion");
    assert_eq!((death.pods, death.late_pods), (pods, 0), "{death:?}");
}

#[test]
fn a_raider_skiff_shot_down_breaks_up_ejects_its_pods_and_explodes_by_its_wreck() {
    let (_, record, death) = shoot_down("raider-skiff");
    // What makes it a wreck at all: until the new game mapped wrecks,
    // every ship vanished on the tick its armour went, with no explosion
    // and no pod.
    assert!(record.death_delay > 0, "{record:?}");
    assert!(record.pod_count > 0, "{record:?}");
    assert!(Explosion::decode(record.explode2).is_some(), "{record:?}");
    died_by_its_wreck(&record, &death);
}

#[test]
fn a_warden_picket_naming_no_wreck_dies_by_the_default_with_no_pod() {
    let (play, picket, death) = shoot_down("warden-picket");
    assert!(picket.death_delay > 0, "{picket:?}");
    assert_eq!(picket.pod_count, 0, "{picket:?}");
    assert!(Explosion::decode(picket.explode2).is_some(), "{picket:?}");
    let raider = record::<Ship>(&play.data, nova_of(&play.game.ids.ships, "raider-skiff"));
    assert_ne!(
        (picket.death_delay, picket.explode2),
        (raider.death_delay, raider.explode2),
        "the default, not the raiders' wreck"
    );
    died_by_its_wreck(&picket, &death);
}

/// Changes the session the app flies.
fn change_session(game: &mut Harness, change: impl FnOnce(&mut nova_sim::Session)) {
    game.app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| change(sim.session_mut().expect("a session")));
}

/// Whether `sounds` asks for `snd ` `id` to play.
fn plays(sounds: &[AudioCommand], id: nova_sim::SoundId) -> bool {
    sounds
        .iter()
        .any(|command| matches!(command, AudioCommand::Play { sound, .. } if *sound == id))
}

/// Whether `sounds` halts every one of the player's explosion halts.
fn halts_the_explosion_s(sounds: &[AudioCommand]) -> bool {
    PLAYER_EXPLOSION_HALTS.iter().all(|&id| {
        sounds
            .iter()
            .any(|command| matches!(command, AudioCommand::Halt { sound } if *sound == id))
    })
}

#[test]
fn a_new_pilot_s_hauler_broken_up_by_a_raider_breaks_up_in_flight_before_the_menu() {
    let mut game = flying();
    let game_ids = nova_data::new_game::open(&folder()).expect("lists").ids;
    let hauler = nova_of(&game_ids.ships, "hauler");
    let raider_ship = ShipId(nova_of(&game_ids.ships, "raider-skiff"));
    let raiders = nova_sim::GovtId(nova_of(&game_ids.factions, "raider"));
    assert_eq!(game.ship_session().pilot().ship(), ShipId(hauler));
    let data = crate::new_game_conversation::opened_from(&folder());
    let delay = u32::try_from(record::<Ship>(&data, hauler).death_delay).expect("a delay");
    assert!(delay > 0, "the hauler breaks up for a while");
    let beep = SimSound::ShipDowned.cue().expect("a beep").sound;
    change_session(&mut game, |session| {
        let at = session.player().position + Vec2::new(80.0, 0.0);
        bring_in(session, raider_ship, Some(raiders), AiType::Warship, at).expect("brought in");
    });
    game.sounds();

    // Held one hit from breaking up, until a raider's hit breaks it up.
    let mut first = None;
    for _ in 0..FRAMES {
        change_session(&mut game, |session| {
            set_player_shield(session, 0.0);
            set_player_armor(session, 1.0);
        });
        game.frame();
        let sounds = game.sounds();
        if let PlayerFate::BreakingUp { ticks_left, .. } = game.ship_session().player_fate() {
            first = Some(ticks_left);
            assert!(plays(&sounds, beep), "the downed beep: {sounds:?}");
            assert!(
                !halts_the_explosion_s(&sounds),
                "no explosion yet: {sounds:?}"
            );
            break;
        }
        assert!(
            !plays(&sounds, beep),
            "no beep before the break-up: {sounds:?}"
        );
        assert_eq!(game.showing(), Showing::Flight);
    }
    let unset = first.expect("a raider's hit breaks the hauler up");

    // The break-up, in flight, its timer set (on the hit's frame it may
    // still be unset, at 0) and then falling, then the explosion.
    let mut set = (unset > 0).then_some(unset);
    let mut last = unset;
    let mut exploded = false;
    for _ in 0..FRAMES {
        game.frame();
        assert_eq!(
            game.showing(),
            Showing::Flight,
            "in flight through the break-up"
        );
        let sounds = game.sounds();
        match game.ship_session().player_fate() {
            PlayerFate::BreakingUp { ticks_left, .. } => {
                match set {
                    None => set = (ticks_left > 0).then_some(ticks_left),
                    Some(_) => assert!(
                        0 < ticks_left && ticks_left <= last,
                        "{last} then {ticks_left}"
                    ),
                }
                last = ticks_left;
                assert!(
                    !halts_the_explosion_s(&sounds),
                    "no explosion yet: {sounds:?}"
                );
            }
            PlayerFate::Exploded | PlayerFate::Holding { .. } => {
                assert!(
                    halts_the_explosion_s(&sounds),
                    "the explosion's halt: {sounds:?}"
                );
                exploded = true;
                break;
            }
            other => panic!("{other:?} straight from the break-up"),
        }
    }
    assert!(exploded, "the hauler explodes");
    let set = set.expect("the death timer was set");
    assert!(
        set >= delay,
        "set at {set} ticks of a DeathDelay of {delay}"
    );
    assert!(
        set - last >= delay,
        "broke up from {set} to {last}: at least the DeathDelay of {delay}"
    );

    // Only then the main menu.
    for _ in 0..FRAMES {
        if game.showing() == Showing::MainMenu {
            return;
        }
        game.frame();
    }
    panic!("never back at the main menu: {:?}", game.showing());
}
