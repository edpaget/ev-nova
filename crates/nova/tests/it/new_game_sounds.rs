//! The new game's sounds reach the audio side: over the checked-in
//! fixture (`nova_data::new_game::fixture::folder`), opened as the app
//! opens it (`DiskData::open_new_game`), the data `main` hands
//! `KiraAudio::open` as `Rc<GameData>`.
//!
//! - Firing the mass driver, whose file names `slug-thump`, raises its
//!   sound event as the app issues it (a `Play` of its weapon look's
//!   sound, through the view's `CombatSound`), and the sound bank resolves
//!   that ID to `slug-thump.wav`'s samples.
//! - Each role `game.md` fills (the button's two halves, the landing, the
//!   gate passage, the explosion) resolves, at the cue its events play, to
//!   its own clip's samples; the refusal, left empty, is silent, and the
//!   adapter's cache plays nothing for it and warns of nothing.
//! - The mass driver's sound lies in the engine's weapon sounds
//!   (`nova_sim::sound::WEAPON_SOUNDS`), and the mapper's whole range
//!   within them.
//! - The opened data, and its `Rc`, keep the sound port's contract
//!   (`nova_audio::recording::check_bank_contract`).
//!
//! Every ID, cue and clip is read from the opened data, the fixture's
//! files and the crates that own them, none retyped. What each word maps
//! to is nova-data's, what each event plays nova-sim's and nova-view's,
//! and how a clip decodes and plays nova-audio's, each pinned there; this
//! pins that they meet.

use std::rc::Rc;

use nova::app::{Control, WindowEvent};
use nova::chance::SplitMix;
use nova_audio::AudioCommand;
use nova_audio::bank::{Clip, SoundBank, pcm};
use nova_audio::kira::SoundCache;
use nova_audio::recording::check_bank_contract;
use nova_data::GameData;
use nova_data::new_game::NewGame;
use nova_data::new_game::fixture::folder;
use nova_data::records::ship::Ship;
use nova_data::sound::{Pcm, decode_wav};
use nova_sim::catalog::Explosion;
use nova_sim::fixture::{place, put_player_at};
use nova_sim::sound::{SimSound, WEAPON_SOUNDS};
use nova_sim::{SetExpr, SoundId, TargetPick, Vec2, WeaponId};
use nova_view::Key;
use nova_view::flight::CombatLooks;
use nova_view::sound::UiSound;

use crate::common::draw;
use crate::new_game_conversation::{flying, opened_from};
use crate::new_game_travel::{fixture, nova_of};
use crate::pilots::Harness;

/// The bytes of the fixture's sound `sound`'s file, read from the folder
/// at the path its `sounds/` entry names.
pub(crate) fn clip_bytes(game: &NewGame, sound: &str) -> Vec<u8> {
    let (_, doc) = (game.galaxy.sounds.iter())
        .find(|(id, _)| id.as_str() == sound)
        .unwrap_or_else(|| panic!("sounds/{sound}.md"));
    let path = (doc.data.file.as_str().split('/')).fold(folder(), |path, part| path.join(part));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The samples of the fixture's sound `sound`, decoded from its file.
fn samples_of(game: &NewGame, sound: &str) -> Pcm {
    decode_wav(&clip_bytes(game, sound)).expect("a clip the fixture plays")
}

/// What `bank` plays at `id`, decoded.
fn resolved(bank: &dyn SoundBank, id: SoundId) -> Option<Pcm> {
    let clip = bank
        .clip(id)
        .unwrap_or_else(|| panic!("snd {} is missing", id.0));
    pcm(&clip).unwrap_or_else(|error| panic!("snd {}: {error}", id.0))
}

/// The sound the mass driver's file names.
fn mass_driver_sound_name(game: &NewGame) -> String {
    let (_, doc) = (game.galaxy.outfits.iter())
        .find(|(id, _)| id.as_str() == "mass-driver")
        .expect("outfits/mass-driver.md");
    doc.data
        .sound
        .as_ref()
        .expect("the mass driver names its sound")
        .to_string()
}

/// The `snd ` the mass driver fires, as the view's weapon look reads it
/// from the opened data.
fn mass_driver_sound(data: &GameData, game: &NewGame) -> SoundId {
    let weapon = WeaponId(nova_of(&game.ids.weapons, "mass-driver"));
    data.weapon_look(weapon)
        .expect("the mass driver's look")
        .sound
        .expect("a sound")
}

/// The cue `sound` plays.
fn cue(sound: SimSound) -> SoundId {
    sound.cue().expect("a cue").sound
}

/// The effects `game`'s audio was asked to play since the last call.
fn played(game: &Harness) -> Vec<SoundId> {
    game.sounds()
        .into_iter()
        .filter_map(|command| match command {
            AudioCommand::Play { sound, .. } | AudioCommand::PlayUnlessPlaying { sound, .. } => {
                Some(sound)
            }
            _ => None,
        })
        .collect()
}

/// Presses (`held`) or lets go of the fire key.
fn hold_fire(game: &mut Harness, held: bool) {
    let event = WindowEvent::Key {
        key: Key::Space,
        pressed: held,
        repeat: false,
    };
    assert_eq!(game.handle(event), Control::Continue);
}

#[test]
fn firing_the_mass_driver_plays_its_sound_and_the_bank_resolves_it_to_its_clip() {
    let game = fixture();
    let data = opened_from(&folder());
    let sound = mass_driver_sound(&data, &game);
    let (ring, driver) = (
        nova_of(&game.ids.kits, "turret-ring"),
        nova_of(&game.ids.outfits, "mass-driver"),
    );
    let mut harness = flying();
    // The hauler with a turret ring and a mass driver on it, and a ship
    // of the system's traffic to aim at.
    let target = harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            let session = sim.session_mut().expect("a session");
            for outfit in [ring, driver] {
                session.run_set(
                    &SetExpr::parse(&format!("G{outfit}")).expect("parses"),
                    &mut nova_sim::NeverFires,
                );
            }
            (0..200)
                .find_map(|seed| {
                    if session.npcs().is_empty() {
                        session.populate(&*data, &mut SplitMix::new(seed));
                    }
                    session.npcs().first().map(|npc| npc.id)
                })
                .expect("a ship to aim at")
        });
    let stand = |harness: &mut Harness| {
        harness
            .app
            .screen_mut()
            .flight_view_mut()
            .expect("flying")
            .change_sim(|sim| {
                let session = sim.session_mut().expect("a session");
                put_player_at(session, Vec2::ZERO);
                place(session, target, Vec2::new(60.0, -100.0));
                assert_eq!(session.select_target(TargetPick::Nearest), Some(target));
            });
    };
    for _ in 0..30 {
        stand(&mut harness);
        draw(&mut harness);
    }
    assert!(
        !played(&harness).contains(&sound),
        "nothing fires before the trigger"
    );
    // Space pressed again each frame until it takes, as the flight may
    // not step its first frames.
    let fired = (0..600).any(|_| {
        stand(&mut harness);
        hold_fire(&mut harness, true);
        draw(&mut harness);
        played(&harness).contains(&sound)
    });
    hold_fire(&mut harness, false);
    assert!(fired, "the mass driver's snd {} plays", sound.0);
    let name = mass_driver_sound_name(&game);
    assert_eq!(resolved(&data, sound), Some(samples_of(&game, &name)));
}

#[test]
fn each_role_the_fixture_fills_resolves_to_its_clip_and_the_empty_one_to_silence() {
    let game = fixture();
    let data = opened_from(&folder());
    let roles = &game.galaxy.game.as_ref().expect("game.md").data.sounds;
    let named = |role: &Option<_>| -> String {
        role.as_ref()
            .map(ToString::to_string)
            .expect("the fixture fills it")
    };
    // The explosion the hauler's wreck sets off, as the view hears it.
    let hauler = (game.data)
        .get::<Ship>(nova_of(&game.ids.ships, "hauler"))
        .expect("present")
        .expect("decoded")
        .record
        .clone();
    let blast = Explosion::decode(hauler.explode2).expect("an explosion");
    let explosion = data
        .boom_look(blast.boom)
        .expect("a bööm")
        .expect("its look")
        .sound
        .expect("heard");
    let button_down = UiSound::ButtonDown.cue().expect("a cue").sound;
    let filled = [
        (button_down, named(&roles.button_down)),
        (
            UiSound::ButtonUp.cue().expect("a cue").sound,
            named(&roles.button_up),
        ),
        (cue(SimSound::Landed), named(&roles.landing)),
        (cue(SimSound::TrafficControl), named(&roles.landing)),
        (cue(SimSound::Arrived), named(&roles.gate_passage)),
        (explosion, named(&roles.explosion)),
    ];
    let mut clips: Vec<Vec<u8>> = filled
        .iter()
        .map(|(_, sound)| clip_bytes(&game, sound))
        .collect();
    clips.sort();
    clips.dedup();
    assert_eq!(clips.len(), 5, "the five filled roles' clips differ");
    let shared: Rc<GameData> = Rc::clone(&data);
    for (id, sound) in &filled {
        assert_eq!(
            resolved(&shared, *id),
            Some(samples_of(&game, sound)),
            "snd {} plays {sound}",
            id.0
        );
    }
    assert_eq!(roles.refused, None, "left empty on purpose");
    let refused = cue(SimSound::Refused);
    assert_eq!(shared.clip(refused), Some(Clip::Silent));
    let mut warnings = Vec::new();
    assert!(
        SoundCache::default()
            .get(&*shared, refused, &mut warnings)
            .is_none()
    );
    assert_eq!(String::from_utf8_lossy(&warnings), "", "no warning");
}

#[test]
fn the_mass_driver_s_sound_lies_in_the_engine_s_weapon_sounds() {
    let game = fixture();
    let data = opened_from(&folder());
    let sound = mass_driver_sound(&data, &game);
    assert!(WEAPON_SOUNDS.contains(&sound.0), "snd {}", sound.0);
    let mapped = nova_data::new_game::ids::WEAPON_SOUND_IDS;
    assert!(
        WEAPON_SOUNDS.contains(mapped.start()) && WEAPON_SOUNDS.contains(mapped.end()),
        "{mapped:?} within {WEAPON_SOUNDS:?}"
    );
}

#[test]
fn the_opened_data_and_its_shared_one_keep_the_sound_port_s_contract() {
    let game = fixture();
    let data = opened_from(&folder());
    let held = mass_driver_sound(&data, &game);
    let unheld = cue(SimSound::Refused);
    assert_eq!(check_bank_contract(&*data, held, unheld), Ok(()));
    assert_eq!(check_bank_contract(&data, held, unheld), Ok(()));
}
