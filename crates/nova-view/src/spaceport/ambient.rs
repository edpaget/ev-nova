//! The stellar's ambient sound while the spaceport is frontmost, as
//! `_DoPortDialog` @0x5f911 loads it and `_PortFilter` @0x4f041 repeats it.
//!
//! - **Which sound.** `_DoPortDialog` loads the `spöb`'s `CustSndID` as
//!   `_planetSnd` when it is 128 or more (`cmpw $0x7f; jle` @0x5fa3f) and
//!   the `AmbientSounds` setting is on (`_settings+0x12`, @0x5fa45-0x5fa4e,
//!   which `_LoadPrefs` @0xced4f defaults to on). Ours has no setting of
//!   its own for it: the Preferences dialog's "Ambient Sounds" is the
//!   general Sound toggle, and with sound off the audio side plays
//!   nothing, this included. Hypergates and wormholes keep an angle in
//!   that field, but they are never landed on, so they never get here.
//! - **When.** On every filter call (`_MyModalDialog` @0x59fee calls it on
//!   each `WaitNextEvent` pass, null events included), while the port
//!   itself is the frontmost dialog (@0x4f088-0x4f159):
//!   1. an activate with a deadline set rearms it: now + 480 + `Rand(480)`
//!      Mac ticks;
//!   2. a stellar whose `Flags2` has [`LOOPS`] plays the sound whenever it
//!      is not playing (`ST_IsSoundPlaying` @0x4f0d6); any other plays it
//!      once the deadline passes and rearms it (@0x4f0ef-0x4f12c);
//!   3. an activate rearms it again.
//!
//!   The deadline starts at 0 (`_ambientNoiseDelay`, @0x5fa6f), so the
//!   first filter call plays it. That call comes only once the landing's
//!   texts and the spaceport's own offer are gone (`_MissionLandCheck`
//!   @0x5fbeb, `_OfferMissionFromPort(3)` @0x5fc37 run before the port's
//!   loop). Once it has played, the port coming back to the front (a
//!   sub-screen, a text or the Preferences closing over it) restarts the
//!   wait without playing. Nothing else repeats it: the bar, the shops,
//!   the map, the player info and every text or offer run their own
//!   filters, so a sound already playing finishes under them, but none
//!   starts.
//! - **The gap** is `AMBIENT_GAP_TICKS + Rand(AMBIENT_GAP_TICKS)`: 480-959
//!   Mac ticks, 8-16 s (`_Rand(n)` is `(n * u16) >> 16`, 0 to n - 1,
//!   @0xa4c76). Ours draws it on the flight's chance; the range is the
//!   original's, the draws are not.
//! - **A press on the landscape** (`_PortFilter` maps it to item 5,
//!   @0x4f2b4-0x4f2d1) plays it once more, unless it loops
//!   (`_DoPortDialog` @0x5fe2a-0x5fe5e). It does not touch the wait.
//! - **Leaving** halts it (`ST_HaltSound` @0x601d2-0x601eb).
//!
//! `_PlanetSoundCallback` @0x55f5d, which would replay a looped sound from
//! a completion callback, is referenced by no code, so it is not ported.
//!
//! No stock stellar sets [`LOOPS`]. Three stock ambient sounds last longer
//! than the shortest gap: `snd ` 10032 (9.2 s), 10033 (12.2 s) and 10034
//! (17.8 s). Ours plays a repeat over one still playing; whether the
//! original's `ST_PlaySoundParam` restarts the sound on its one handle
//! instead is library code we can't read.
//!
//! The wait runs down by the spaceport's frames, not on each event: a lag
//! of under a frame, which can't be heard. Likewise the port's first
//! frontmost frame after a cover is a frame after the cover closed.

use std::time::Duration;

use nova_sim::Chance;
use nova_sim::mission::screens::mac_ticks;

use crate::sound::{SoundId, UiSound};

/// The lowest `CustSndID` that is an ambient sound (`_DoPortDialog`
/// @0x5fa3f): below it, none.
pub const FIRST_AMBIENT_SOUND: i16 = 128;

/// The `spöb` `Flags2` bit of a stellar whose ambient sound loops: it
/// plays again whenever it stops, rather than every 8-16 s. `_PortFilter`
/// tests `0x34(_dockedPort) & 0x10` (@0x4f0cd), the low byte of `Flags2`,
/// which `_PlayerEnterWormhole` confirms by testing its 0x2000, wormhole,
/// as `0x35 & 0x20` (@0x64122).
pub const LOOPS: u16 = 0x0010;

/// The fewest Mac ticks between one play of the ambient sound and the
/// next, and the range of the random part on top (`_PortFilter`
/// @0x4f09f-0x4f0b2, @0x4f110-0x4f12c).
pub const AMBIENT_GAP_TICKS: u32 = 480;

/// The wait before the ambient sound next plays, drawn on `chance`:
/// [`AMBIENT_GAP_TICKS`] + `Rand(AMBIENT_GAP_TICKS)` Mac ticks.
pub fn ambient_gap(chance: &mut dyn Chance) -> Duration {
    mac_ticks(AMBIENT_GAP_TICKS + chance.below(AMBIENT_GAP_TICKS))
}

/// A stellar's ambient sound, and when it next plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ambient {
    sound: SoundId,
    looped: bool,
    /// How long until it next plays: none at first, so it plays at once.
    wait: Duration,
    /// Whether it has played since the spaceport opened.
    played: bool,
    /// Whether something covered the spaceport at its last frame.
    covered: bool,
}

impl Ambient {
    /// The ambient sound of a stellar with this `CustSndID` and `Flags2`:
    /// none below [`FIRST_AMBIENT_SOUND`].
    #[must_use]
    pub fn of(cust_snd_id: i16, flags2: u16) -> Option<Self> {
        (cust_snd_id >= FIRST_AMBIENT_SOUND).then_some(Self {
            sound: SoundId(cust_snd_id),
            looped: flags2 & LOOPS != 0,
            wait: Duration::ZERO,
            played: false,
            covered: false,
        })
    }

    /// The `snd `.
    #[must_use]
    pub fn sound(&self) -> SoundId {
        self.sound
    }

    /// One frame of `dt`, with the spaceport `covered` by something open
    /// over it or not: the sound to raise, if any. Drawn on `chance` for
    /// each new wait.
    ///
    /// Covered, nothing happens but that it is remembered. Frontmost, a
    /// looped sound asks to play unless still playing, every frame. Any
    /// other: the first frame after a cover, the original's activate,
    /// rearms the wait, and plays first only when it has never played;
    /// otherwise the wait runs down by `dt`, and once it is out the sound
    /// plays and the wait is rearmed.
    pub fn tick(
        &mut self,
        dt: Duration,
        covered: bool,
        chance: &mut dyn Chance,
    ) -> Option<UiSound> {
        if covered {
            self.covered = true;
            return None;
        }
        let activated = std::mem::take(&mut self.covered);
        if self.looped {
            return Some(self.play(true));
        }
        let due = if activated {
            !self.played
        } else {
            self.wait = self.wait.saturating_sub(dt);
            self.wait.is_zero()
        };
        if activated || due {
            self.wait = ambient_gap(chance);
        }
        due.then(|| {
            self.played = true;
            self.play(false)
        })
    }

    /// The spaceport was covered by something that doesn't tick it, such
    /// as the Preferences dialog: its next frontmost frame counts as the
    /// port coming back to the front.
    pub fn cover(&mut self) {
        self.covered = true;
    }

    /// A press on the landscape: the sound to raise, once, unless it
    /// loops. The wait is left as it is.
    #[must_use]
    pub fn pressed(&self) -> Option<UiSound> {
        (!self.looped).then(|| self.play(false))
    }

    /// The spaceport closing: the sound halts.
    #[must_use]
    pub fn halted(&self) -> UiSound {
        UiSound::PortAmbientHalted(self.sound)
    }

    fn play(&self, until_stopped: bool) -> UiSound {
        UiSound::PortAmbient {
            sound: self.sound,
            until_stopped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source whose every draw is `draw`, recording each `n` asked.
    struct Fixed {
        draw: u32,
        asked: Vec<u32>,
    }

    impl Fixed {
        fn new(draw: u32) -> Self {
            Self {
                draw,
                asked: Vec::new(),
            }
        }
    }

    impl Chance for Fixed {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draw
        }
    }

    const PORT_KANE: i16 = 10_032;

    #[allow(clippy::unnecessary_wraps)] // compared with what a tick raises
    fn plays(until_stopped: bool) -> Option<UiSound> {
        Some(UiSound::PortAmbient {
            sound: SoundId(PORT_KANE),
            until_stopped,
        })
    }

    fn kane() -> Ambient {
        Ambient::of(PORT_KANE, 0).expect("a sound")
    }

    /// A frame: a twentieth of a second, which adds up to whole seconds.
    const FRAME: Duration = Duration::from_millis(50);

    /// The last Mac tick of a 480-tick wait, after its first 479.
    fn last_tick() -> Duration {
        mac_ticks(480).saturating_sub(mac_ticks(479))
    }

    #[test]
    fn the_gap_is_480_ticks_and_rand_480_more() {
        let mut low = Fixed::new(0);
        assert_eq!(ambient_gap(&mut low), mac_ticks(480));
        assert_eq!(low.asked, [480]);
        assert_eq!(ambient_gap(&mut Fixed::new(479)), mac_ticks(959));
    }

    #[test]
    fn a_custsndid_under_128_is_no_sound() {
        for id in [127, 0, -1, i16::MIN] {
            assert_eq!(Ambient::of(id, 0), None, "{id}");
        }
        assert_eq!(Ambient::of(128, 0).map(|a| a.sound()), Some(SoundId(128)));
        assert_eq!(kane().sound(), SoundId(PORT_KANE));
    }

    #[test]
    fn a_fresh_ambient_sound_plays_on_its_first_frontmost_frame() {
        let mut ambient = kane();
        assert_eq!(ambient.tick(FRAME, false, &mut Fixed::new(0)), plays(false));
    }

    /// Plays it once, then runs `ticks` Mac ticks in one frame: whether
    /// that frame played.
    fn replays_after(draw: u32, ticks: u32) -> bool {
        let mut chance = Fixed::new(draw);
        let mut ambient = kane();
        assert_eq!(ambient.tick(FRAME, false, &mut chance), plays(false));
        ambient.tick(mac_ticks(ticks), false, &mut chance).is_some()
    }

    #[test]
    fn it_plays_again_after_480_ticks_at_the_least() {
        assert!(!replays_after(0, 479));
        assert!(replays_after(0, 480));
    }

    #[test]
    fn it_plays_again_after_959_ticks_at_the_most() {
        assert!(!replays_after(479, 958));
        assert!(replays_after(479, 959));
    }

    #[test]
    fn the_wait_runs_down_frame_by_frame_and_rearms() {
        let mut chance = Fixed::new(0);
        let mut ambient = kane();
        ambient.tick(FRAME, false, &mut chance);
        let mut played = Vec::new();
        for frame in 1..=600 {
            if ambient.tick(FRAME, false, &mut chance).is_some() {
                played.push(frame);
            }
        }
        // 8 s apart, at 20 frames a second.
        assert_eq!(played, [160, 320, 480]);
    }

    #[test]
    fn a_covered_frame_neither_runs_down_nor_plays() {
        let mut chance = Fixed::new(0);
        let mut ambient = kane();
        assert_eq!(ambient.tick(FRAME, true, &mut chance), None);
        assert_eq!(
            ambient.tick(Duration::from_secs(60), true, &mut chance),
            None
        );
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn coming_back_to_the_front_plays_only_if_it_never_played() {
        let mut chance = Fixed::new(0);
        let mut ambient = kane();
        ambient.tick(FRAME, true, &mut chance);
        assert_eq!(
            ambient.tick(FRAME, false, &mut chance),
            plays(false),
            "covered at entry, it plays once uncovered"
        );
        ambient.tick(mac_ticks(470), false, &mut chance);
        ambient.tick(FRAME, true, &mut chance);
        assert_eq!(
            ambient.tick(mac_ticks(20), false, &mut chance),
            None,
            "the activate rearms instead of playing"
        );
        assert_eq!(ambient.tick(mac_ticks(479), false, &mut chance), None);
        assert_eq!(ambient.tick(last_tick(), false, &mut chance), plays(false));
    }

    #[test]
    fn cover_counts_as_a_covered_frame() {
        let mut chance = Fixed::new(0);
        let mut ambient = kane();
        ambient.tick(FRAME, false, &mut chance);
        ambient.tick(mac_ticks(470), false, &mut chance);
        ambient.cover();
        assert_eq!(ambient.tick(mac_ticks(20), false, &mut chance), None);
        assert_eq!(ambient.tick(mac_ticks(479), false, &mut chance), None);
        assert_eq!(ambient.tick(last_tick(), false, &mut chance), plays(false));
    }

    #[test]
    fn a_looped_sound_asks_every_frontmost_frame_until_stopped() {
        let mut chance = Fixed::new(0);
        let mut ambient = Ambient::of(PORT_KANE, LOOPS | 0x2000).expect("a sound");
        for _ in 0..3 {
            assert_eq!(ambient.tick(FRAME, false, &mut chance), plays(true));
        }
        assert_eq!(ambient.tick(FRAME, true, &mut chance), None, "covered");
        assert_eq!(ambient.tick(FRAME, false, &mut chance), plays(true));
        assert!(chance.asked.is_empty(), "no wait is drawn");
    }

    #[test]
    fn a_press_on_the_landscape_plays_it_unless_it_loops() {
        let mut chance = Fixed::new(0);
        let mut ambient = kane();
        assert_eq!(ambient.pressed(), plays(false));
        ambient.tick(FRAME, false, &mut chance);
        assert_eq!(ambient.pressed(), plays(false));
        assert_eq!(
            ambient.tick(mac_ticks(480), false, &mut chance),
            plays(false),
            "the press leaves the wait alone"
        );
        let looped = Ambient::of(PORT_KANE, LOOPS).expect("a sound");
        assert_eq!(looped.pressed(), None);
    }

    #[test]
    fn closing_halts_it_looped_or_not() {
        let halted = Some(UiSound::PortAmbientHalted(SoundId(PORT_KANE)));
        assert_eq!(Some(kane().halted()), halted);
        assert_eq!(
            Ambient::of(PORT_KANE, LOOPS).map(|ambient| ambient.halted()),
            halted
        );
    }
}
