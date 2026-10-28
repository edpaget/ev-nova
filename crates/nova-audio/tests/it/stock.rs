//! The original's sounds and the soundtrack over the stock data: every
//! `snd ` a sim or interface sound's cue names is the one its source
//! names, and decodes; every
//! stellar ambient sound is there; and the music decodes, on kira's
//! device-free backend. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use kira::backend::mock::MockBackend;
use kira::sound::PlaybackState;
use kira::{AudioManager, AudioManagerSettings};
use nova_audio::kira::SoundCache;
use nova_audio::{Audio, AudioCommand, KiraAudio, SoundBank, Volume};
use nova_data::music::open_music;
use nova_data::records::stellar::Stellar;
use nova_data::{GameData, SoundId};
use nova_rsrc::ResType;
use nova_view::sound::{SimSound, UiSound};

/// A shared buffer the warnings go to.
#[derive(Clone, Default)]
struct Warnings(Rc<RefCell<Vec<u8>>>);

impl Warnings {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("UTF-8")
    }
}

impl Write for Warnings {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_cued_sounds_are_the_named_stock_sounds_and_decode() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let sim = |sound: SimSound| sound.cue().map(|cue| cue.sound);
    let ui = |sound: UiSound| sound.cue().map(|cue| cue.sound);
    let named = [
        (sim(SimSound::Landed), "Beep2"),
        (sim(SimSound::TrafficControl), "Beep2"),
        (sim(SimSound::JumpBegan { rate: 0x1_0000 }), "Warp up"),
        (
            sim(SimSound::JumpBeganInDoubleTime { rate: 0x1_0000 }),
            "Warp up.x2",
        ),
        (sim(SimSound::Arrived), "Warp out"),
        (ui(UiSound::ButtonDown), "Menu button down"),
        (ui(UiSound::ButtonUp), "Menu button up"),
        (ui(UiSound::MapOpened), "Beep3"),
        (ui(UiSound::MapClosed), "Beep1"),
        (ui(UiSound::MapCentred), "Beep5"),
        (ui(UiSound::MapSystemFound), "Beep5"),
        (ui(UiSound::MapSystemNotFound), "Beep4"),
        (sim(SimSound::MissionShipLost), "Beep2"),
        (sim(SimSound::MissionShipBoarded), "Beep5"),
    ];
    let (mut cache, mut warnings) = (SoundCache::default(), Warnings::default());
    for (sound, name) in named {
        let id = sound.expect("the cue names a sound");
        let resource = data
            .resource(ResType::new(*b"snd "), id.0)
            .expect("in the stock data");
        assert_eq!(resource.resource.name(), Some(name), "snd {}", id.0);
        assert!(
            cache.get(&data, id, &mut warnings).is_some(),
            "snd {}",
            id.0
        );
    }
    assert_eq!(warnings.text(), "");
}

/// Every landable stellar's ambient sound, its `CustSndID` from 128 up
/// (`_DoPortDialog` @0x5fa3f). Hypergates and wormholes keep an angle in
/// that field, and are never landed on.
#[test]
fn every_stellar_ambient_sound_is_a_stock_sound_that_decodes() {
    const CAN_LAND: u32 = 0x0001;
    const GATES: u16 = 0x1000 | 0x2000;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let sounds: Vec<SoundId> = data
        .records::<Stellar>()
        .filter_map(|(_, stellar)| stellar.ok())
        .map(|stellar| stellar.record)
        .filter(|stellar| {
            stellar.flags.bits() & CAN_LAND != 0 && stellar.flags2.bits() & GATES == 0
        })
        .map(|stellar| stellar.cust_snd_id)
        .filter(|&id| id >= 128)
        .map(SoundId)
        .collect();
    assert!(sounds.contains(&SoundId(10_032)), "Port Kane's");
    let (mut cache, mut warnings) = (SoundCache::default(), Warnings::default());
    for id in sounds {
        assert!(data.snd(id).is_some(), "snd {}", id.0);
        assert!(
            cache.get(&data, id, &mut warnings).is_some(),
            "snd {}",
            id.0
        );
    }
    assert_eq!(warnings.text(), "");
}

#[test]
fn the_stock_soundtrack_decodes_and_plays() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let music = open_music(&dir).expect("Nova Music.mp3 is there");
    let manager =
        AudioManager::<MockBackend>::new(AudioManagerSettings::default()).expect("a mock opens");
    let warnings = Warnings::default();
    let mut audio =
        KiraAudio::with_manager(manager, data, Some(music)).with_warnings(warnings.clone());
    audio.run(AudioCommand::StartMusic {
        volume: Volume::FULL,
    });
    assert_eq!(audio.music_state(), Some(PlaybackState::Playing));
    assert_eq!(warnings.text(), "");
}
