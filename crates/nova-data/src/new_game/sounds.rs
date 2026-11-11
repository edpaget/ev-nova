//! The new game's sounds: the clips `sounds/` names, the `snd ` IDs a
//! weapon's `sound` and `game.md`'s roles put them at, and the store's
//! [`SoundFiles`] holding them.
//!
//! **A weapon's sound.** Each `sounds/` entry a mapped weapon's `sound`
//! names is numbered within [`WEAPON_SOUND_IDS`] ([`number`]), in sound id
//! order, each the table pins (`sounds`) at its pin and the others after
//! the highest pin. The weapon's `wëap` `Sound` is that ID less
//! [`FIRST_WEAPON_SOUND`] ([`super::outfits::weapon`]), so it fires the
//! clip. A weapon naming no sound keeps `Sound` -1 and fires silently; a
//! non-weapon's `sound` is ignored, as a non-launcher's `guidance` is.
//!
//! **A role.** Each of `game.md`'s `sounds` fills one of the engine's own
//! cues, and every event that shares the cue plays it:
//!
//! | Role | Cue | Played by |
//! |---|---|---|
//! | `button_down` | [`BUTTON_DOWN`] | a button's press, and the first half of a keyed one |
//! | `button_up` | [`BUTTON_UP`] | a button's release, and the second half of a keyed one |
//! | `landing` | [`LANDING`] | the traffic controller's word, the touchdown, and every other event on that beep |
//! | `refused` | [`REFUSED`] | every refusal (the land and jump keys, a gate's fuel, a buy), and every other event on that beep |
//! | `gate_passage` | [`GATE_PASSAGE`] | the arrival out of a gate, and out of every jump |
//! | `explosion` | [`EXPLOSION`] | the supplied `bööm`s ([`booms`]), and an explosion on board |
//!
//! A role left out is silent, with no problem.
//!
//! **The explosions.** A wreck's explosions name `bööm` 128 (the large)
//! and 129 (the small) ([`super::wrecks`]), which the new game doesn't
//! author. When `game.md` names an `explosion` that is in `sounds/`,
//! [`booms`] supplies both, each playing [`EXPLOSION`] (`SoundIndex` 0)
//! and naming no `spïn` the new game holds, so they are heard but not seen.
//!
//! **The clips** ([`read`]). Each sound a weapon or a role needs is read
//! once through the [`Source`], checked with [`decode_wav`] (at least one
//! frame), and its bytes kept at every ID it fills. Every other ID is
//! silent on purpose: the store's sound files hold only what the data
//! fills (see [`crate::store`]'s `sound_files`).
//!
//! # Problems
//!
//! Each is reported once, and the sound concerned stays silent:
//!
//! - an outfit's `sound` naming no `sounds/` entry, against the outfit;
//! - a role naming no `sounds/` entry, against `game.md`;
//! - a sound past the last weapon sound ID, against the sound;
//! - a sound's file that isn't under `media/export/`, isn't there, can't be
//!   read, doesn't decode as a WAV file (the clip is in a format the
//!   adapter can't play) or holds no samples, against the sound, with the
//!   decoder's error.
//!
//! A sound whose file is a Git LFS pointer is silent and not reported:
//! the load lists it among the pointers ([`super::NewGame::lfs_pointers`]).
//! A file that failed to load isn't reported again.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use nova_galaxy_format::game::SoundRole;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Galaxy, Problem};
use nova_galaxy_format::media::is_lfs_pointer;
use nova_galaxy_format::source::Source;

use super::ids::{IdTable, WEAPON_SOUND_IDS, after_pins, allocate_pinned};
use super::map::absent;
use super::records::unseen_boom;
use crate::decode::Entry;
use crate::registry::AnyRecord;
use crate::sound::decode_wav;
use crate::store::SoundFiles;
use crate::wire::id::SoundId;

/// The `snd ` a weapon's `Sound` of 0 fires: [`WEAPON_SOUND_IDS`]' first,
/// nova-view's `flight::data::FIRST_WEAPON_SOUND`.
pub const FIRST_WEAPON_SOUND: i16 = *WEAPON_SOUND_IDS.start();

/// The `button_down` role's cue: `snd ` 600, "Menu button down", the first
/// of `nova_view::sound::MAIN_SCREEN_SOUNDS`.
pub const BUTTON_DOWN: SoundId = SoundId(600);

/// The `button_up` role's cue: `snd ` 601, "Menu button up", the second of
/// `nova_view::sound::MAIN_SCREEN_SOUNDS`.
pub const BUTTON_UP: SoundId = SoundId(601);

/// The `landing` role's cue: `snd ` 151, "Beep2", `nova_sim::sound::BEEP`'s
/// second, which `SimSound::TrafficControl` and `SimSound::Landed` play.
pub const LANDING: SoundId = SoundId(151);

/// The `refused` role's cue: `snd ` 153, "Beep4", `nova_sim::sound::BEEP`'s
/// fourth, which `SimSound::Refused` plays.
pub const REFUSED: SoundId = SoundId(153);

/// The `gate_passage` role's cue: `snd ` 130, "Warp out", which
/// `SimSound::Arrived` plays out of a gate and every jump.
pub const GATE_PASSAGE: SoundId = SoundId(130);

/// The `explosion` role's cue: `snd ` 300, the first explosion sound
/// (`nova_sim::catalog::FIRST_EXPLOSION_SOUND`), which a `bööm` of
/// `SoundIndex` 0 and `SimSound::OnboardExplosion` play.
pub const EXPLOSION: SoundId = SoundId(300);

/// The `bööm`s [`booms`] supplies: 128, the large explosion, and 129, the
/// small ([`super::wrecks::LARGE_EXPLOSION`], [`super::wrecks::SMALL_EXPLOSION`]).
pub const BOOMS: [(i16, &str); 2] = [(128, "Large explosion"), (129, "Small explosion")];

/// The cue the role `role` fills.
///
/// The match is exhaustive on purpose: the mapper must give every role
/// the format defines ([`EngineSounds::roles`]) a cue, and only this crate
/// knows the cues, so a role added to the format fails to compile here
/// until it is given one.
///
/// [`EngineSounds::roles`]: nova_galaxy_format::game::EngineSounds::roles
fn cue(role: SoundRole) -> SoundId {
    match role {
        SoundRole::ButtonDown => BUTTON_DOWN,
        SoundRole::ButtonUp => BUTTON_UP,
        SoundRole::Landing => LANDING,
        SoundRole::Refused => REFUSED,
        SoundRole::GatePassage => GATE_PASSAGE,
        SoundRole::Explosion => EXPLOSION,
    }
}

/// Numbers the sounds `galaxy`'s weapons (`weapons`, by outfit id) fire
/// within [`WEAPON_SOUND_IDS`] under `pins` (see the module docs).
pub(super) fn number(
    galaxy: &Galaxy,
    weapons: &BTreeMap<Id, i16>,
    pins: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let mut fired = BTreeSet::new();
    for outfit in weapons.keys() {
        let doc = &galaxy.outfits[outfit];
        let Some(sound) = &doc.data.sound else {
            continue;
        };
        if galaxy.sounds.contains_key(sound) {
            fired.insert(sound);
        } else if absent(galaxy, &galaxy.sounds, "sounds", sound) {
            problems.push(Problem::new(
                &doc.path,
                format!("its sound `{sound}` is not in sounds/; it fires silently"),
            ));
        }
    }
    let mut free = after_pins(WEAPON_SOUND_IDS, pins.values());
    let (table, past) = allocate_pinned(fired, pins, &mut free);
    for sound in past {
        problems.push(Problem::new(
            &galaxy.sounds[sound].path,
            format!(
                "it is past the last weapon sound ID, {}; the weapons naming it fire silently",
                WEAPON_SOUND_IDS.end()
            ),
        ));
    }
    table
}

/// The `wëap` `Sound` of a weapon firing `sound`, a weapon sound ID: its
/// index past [`FIRST_WEAPON_SOUND`].
#[must_use]
pub fn weapon_sound(sound: i16) -> i16 {
    sound - FIRST_WEAPON_SOUND
}

/// The clips `galaxy`'s weapons (by `ids.sounds`) and `game.md`'s roles
/// fill, read from `source` (see the module docs).
pub(super) fn read(
    galaxy: &Galaxy,
    ids: &IdTable,
    source: &impl Source,
    problems: &mut Vec<Problem>,
) -> SoundFiles {
    let mut fills: BTreeMap<&Id, Vec<SoundId>> = BTreeMap::new();
    for (sound, &nova) in &ids.sounds {
        fills.entry(sound).or_default().push(SoundId(nova));
    }
    if let Some(game) = &galaxy.game {
        for (role, sound) in game.data.sounds.roles() {
            let Some(sound) = sound else { continue };
            if galaxy.sounds.contains_key(sound) {
                fills.entry(sound).or_default().push(cue(role));
            } else if absent(galaxy, &galaxy.sounds, "sounds", sound) {
                problems.push(Problem::new(
                    &game.path,
                    format!(
                        "its `{}` sound `{sound}` is not in sounds/; it is silent",
                        role.name()
                    ),
                ));
            }
        }
    }
    let mut clips = BTreeMap::new();
    for (sound, at) in fills {
        let doc = &galaxy.sounds[sound];
        let Some(bytes) = clip(&doc.data.file, source, &doc.path, problems) else {
            continue;
        };
        let bytes: Arc<[u8]> = bytes.into();
        for id in at {
            clips.insert(id, Arc::clone(&bytes));
        }
    }
    SoundFiles::new(clips)
}

/// The bytes of the sound `file`, named in `path`, when they are a clip
/// the adapter plays; none, reported in `problems` but for a pointer,
/// otherwise.
fn clip(
    file: &nova_galaxy_format::id::MediaPath,
    source: &impl Source,
    path: &str,
    problems: &mut Vec<Problem>,
) -> Option<Vec<u8>> {
    let mut silent = |why: String| {
        problems.push(Problem::new(
            path,
            format!("its file `{file}` {why}; it is silent"),
        ));
        None
    };
    let bytes = match export_bytes(file, source) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return None,
        Err(why) => return silent(why),
    };
    match decode_wav(&bytes) {
        Ok(pcm) if pcm.frames() > 0 => Some(bytes),
        Ok(_) => silent("holds no samples".to_owned()),
        Err(error) => silent(format!("doesn't decode as a WAV file ({error})")),
    }
}

/// The bytes of a sound's `file`, read from `source`: none when they are
/// a Git LFS pointer, which the load lists among the pointers; otherwise,
/// when the file isn't under `media/export/`, isn't there or can't be
/// read, why, for the caller to report with what follows from it.
pub(super) fn export_bytes(
    file: &nova_galaxy_format::id::MediaPath,
    source: &impl Source,
) -> Result<Option<Vec<u8>>, String> {
    if !file.is_export() {
        return Err("isn't under media/export/".to_owned());
    }
    let bytes = match source.bytes(file.as_str()) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err("isn't there".to_owned()),
        Err(error) => return Err(format!("couldn't be read ({error})")),
    };
    Ok((!is_lfs_pointer(&bytes)).then_some(bytes))
}

/// `bööm` 128 and 129 ([`BOOMS`]), each playing [`EXPLOSION`], when
/// `game.md` names an `explosion` that is in `sounds/`; none otherwise.
pub(super) fn booms(galaxy: &Galaxy) -> Vec<Entry<AnyRecord>> {
    let named = (galaxy.game.as_ref())
        .and_then(|game| game.data.sounds.explosion.as_ref())
        .is_some_and(|sound| galaxy.sounds.contains_key(sound));
    if !named {
        return Vec::new();
    }
    BOOMS
        .iter()
        .map(|&(id, name)| Entry {
            id,
            name: Some(name.to_owned()),
            // `SoundIndex` 0: `snd ` 300, `EXPLOSION`.
            record: AnyRecord::Boom(Box::new(unseen_boom(0))),
        })
        .collect()
}

#[cfg(test)]
mod tests;
