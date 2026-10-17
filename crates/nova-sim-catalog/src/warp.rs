//! The player's warp clock: the pieces of the original's warp timing
//! that the session's jump and `_ScanPlayer`'s jump-timing check read.
//!
//! The original (`EV Nova.app/Contents/MacOS/EV Nova`, i386) times the
//! player's warp by three values:
//!
//! - **t**, the Mac ticks (60 a second, `TickCount`) since the warp
//!   stage began: `_HandlePlayer` stamps the player's +0xa4 once as the
//!   stage starts (@0x6b5f0-0x6b5f5), on the first frame of the jump on
//!   which the ship is slow enough or can expedite its jumps, and
//!   restamps it every frame while the ship still brakes
//!   (@0x6b6a2-0x6b6b1). The engine ticks at 30 a second, so t is
//!   [`MAC_TICKS_PER_TICK`] × the engine's ticks since the stamp
//!   ([`mac_ticks`]).
//! - **w**, the ship class's warp factor (class +0x44), which
//!   `_LoadObjectData` sets from the `shïp`'s `Flags` (@0x7ae9d-0x7aef1;
//!   [`warp_factor`]).
//! - **d**, the warp sound's duration in Mac ticks (`_WarpSoundDuration`
//!   @0x8203-0x8224), which `_LoadSounds` works out from `snd ` 128
//!   (@0x1c0b5-0x1c12d; [`sound_duration`]).
//!
//! `_ScanPlayer` (@0x7e562-0x7e633), the player's own acceleration away
//! (`_HandlePlayer` @0x6d1c1-0x6d296) and a disabled player's jump
//! collapse (`_HandlePlayer` @0x688f2-0x6895e) all read the lead-in speed
//! v = t·w / (d·0.01) − 35/w ([`lead_in_speed`]): the scan is held once
//! v > 0 ([`held`]), which is where the player starts to move off. Until
//! then the warp is in its lead-in. The collapse gives the player v when
//! it is above 0.
//!
//! **Double time.** `_LoadSounds` works out a second d from `snd ` 129
//! "Warp up x2" (@0x1c0b5-0x1c12d), and `_WarpSoundDuration` gives it in
//! place of 128's while `_doubleTime` is set (@0x820b-0x8219;
//! [`warp_sound_duration`]), to every reader: the scan hold, the
//! collapse, the player's move-off and an NPC's warp-out. The move-off's
//! term is −52.5/w in double time, where the scan hold and the collapse
//! keep −35/w ([`move_off_speed`]); an NPC's warp-out runs its clock at
//! 0.667 or 0.5 in double time ([`npc_warp_out_speed`]). The clock
//! itself is `TickCount`, real time, which double time's second pass
//! does not move on. Which warp-up is still playing ([`WarpUps`]) holds
//! double time on, or off, and skips a warp-up begun while it plays (see
//! the session's `double_time` module).
//!
//! Left out: `_LoadObjectData`'s guard that sets a negative factor to 0.5
//! (@0x7aefe-0x7af07), which no `Flags` reaches. The engine doesn't move
//! a warping ship yet (rdm:phase/ship-handling-fidelity/phase-8-warp-out),
//! so nothing reads [`move_off_speed`] and [`npc_warp_out_speed`] but
//! their tests.

use crate::catalog::SoundId;
use crate::clock::TICKS_PER_SECOND;
use crate::sound::WARP_UP_HALTS;

/// The Mac's ticks a second (`TickCount`), which the offers' times are
/// counted in.
pub const MAC_TICKS_PER_SECOND: u32 = 60;

/// The Mac's ticks in one of the engine's: 2.
pub const MAC_TICKS_PER_TICK: u32 = MAC_TICKS_PER_SECOND / TICKS_PER_SECOND;

/// The warp sound's duration, in Mac ticks, when there is no `snd ` 128
/// (`_LoadSounds` @0x1c0d2, 0x15e).
pub const NO_SOUND_DURATION: u32 = 350;

/// What the warp factor of every ship class is multiplied by (1.3, f64
/// @0xdd5e8).
const FACTOR_SCALE: f64 = 1.3;

/// What d is multiplied by before t·w is divided by it (0.01, f64
/// @0xdd098).
const DURATION_SCALE: f64 = 0.01;

/// The lead-in's term, over w (−35.0, f64 @0xdd630): the scan hold's,
/// the collapse's, and the warp-out's of a ship the player doesn't lead.
const LEAD_IN: f64 = -35.0;

/// What the player's move-off takes off, over w, in double time (52.5,
/// f64 @0xdd9a0).
const MOVE_OFF_DOUBLE_TIME: f64 = 52.5;

/// What the player's move-off takes off, over w, otherwise (35.0, f64
/// @0xdd9a8).
const MOVE_OFF: f64 = 35.0;

/// The warp-out clock's factor for a ship the player leads, in double
/// time (0.667, f64 @0xdd620).
const LED_DOUBLE_TIME: f64 = 0.667;

/// The warp-out clock's factor for any other ship, in double time (0.5,
/// f64 @0xdd128).
const OWN_DOUBLE_TIME: f64 = 0.5;

/// The warp-out clock's factor out of double time (1.0, f64 @0xdd0c0).
const SINGLE_TIME: f64 = 1.0;

/// The warp-out's term, over w, for a ship the player leads (−45.0, f64
/// @0xdd628).
const LED_WARP_OUT: f64 = -45.0;

/// The fastest a warp-out's speed gets (50.0, f32 @0xdd064).
const WARP_OUT_CAP: f32 = 50.0;

/// A sound's length as `_ParseSndHeader` gives it: its frames, and the
/// whole part of its 16.16 sample rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundLength {
    /// The frames: samples per channel.
    pub frames: u32,
    /// The whole samples a second: the high half of the 16.16 rate.
    pub rate: u32,
}

/// The engine's ticks since the warp stage began, as the original's
/// Mac ticks.
#[must_use]
pub fn mac_ticks(ticks: u32) -> u32 {
    ticks.saturating_mul(MAC_TICKS_PER_TICK)
}

/// A ship class's warp factor, from its `shïp` `Flags` (`_LoadObjectData`
/// @0x7ae9d-0x7aef1): 0.7 for bit 0x0001, else 1.3 for 0x0002, else 1.6
/// for 0x0004, else 1.0, each an f32, × 1.3 in doubles and stored as an
/// f32 again: 0.91, 1.69, 2.08 or 1.3.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn warp_factor(flags: u16) -> f32 {
    let base: f32 = if flags & 0x0001 != 0 {
        0.7
    } else if flags & 0x0002 != 0 {
        1.3
    } else if flags & 0x0004 != 0 {
        1.6
    } else {
        1.0
    };
    (f64::from(base) * FACTOR_SCALE) as f32
}

/// The warp sound's duration, in Mac ticks, from `sound`, the length of
/// `snd ` 128 (`_LoadSounds` @0x1c0b5-0x1c12d): its frames × 60 over the
/// whole part of its rate, rounded down; [`NO_SOUND_DURATION`] without
/// one.
///
/// The original works it in 32 bits and keeps 16 (`imull`, `divl`,
/// `movw`); the engine keeps it wide. A rate under 1 Hz would trap the
/// original's divide: the engine takes it as no sound.
#[must_use]
pub fn sound_duration(sound: Option<SoundLength>) -> u32 {
    sound.and_then(length_ticks).unwrap_or(NO_SOUND_DURATION)
}

/// The warp-out's duration, in Mac ticks, from `sound`, the length of
/// `snd ` 130: its frames × 60 over the whole part of its rate, as
/// [`sound_duration`] works it, but 0 without one. 130 has no
/// `_jumpSoundLength` entry (`_LoadSounds`' duration loop
/// @0x1c0d2-0x1c12d reads 128 and 129 only), so there is no 350-tick
/// fallback: a missing 130 never plays.
#[must_use]
pub fn warp_out_duration(sound: Option<SoundLength>) -> u32 {
    sound.and_then(length_ticks).unwrap_or(0)
}

/// `sound`'s frames × 60 over its rate, in Mac ticks, kept wide; none for
/// a rate under 1 Hz.
#[allow(clippy::cast_possible_truncation)]
fn length_ticks(SoundLength { frames, rate }: SoundLength) -> Option<u32> {
    (rate > 0).then(|| {
        let ticks = u64::from(frames) * u64::from(MAC_TICKS_PER_SECOND) / u64::from(rate);
        ticks.min(u64::from(u32::MAX)) as u32
    })
}

/// The warp sound's duration that `_WarpSoundDuration` (@0x8203-0x8224)
/// gives from `durations`, `_jumpSoundLength`'s two entries (`snd ` 128's
/// and 129's, each by [`sound_duration`]): the second while
/// `double_time` (@0x820b-0x8219), else the first.
#[must_use]
pub fn warp_sound_duration(durations: [u32; 2], double_time: bool) -> u32 {
    durations[usize::from(double_time)]
}

/// Whether `_ScanPlayer`'s jump-timing check (@0x7e562-0x7e633) holds a
/// scan back: `since` the Mac ticks since the warp stage began (none
/// before it, or while the ship still brakes), `factor` the ship class's
/// warp factor (w), `duration` the warp sound's (d) and `disabled`
/// whether the player is disabled.
///
/// The check applies only to a player in the warp stage (+0x50 > 0) that
/// is not disabled (`_IsDisabled`), and holds the scan when the
/// [`lead_in_speed`] is above 0.0.
#[must_use]
pub fn held(since: Option<u32>, factor: f32, duration: u32, disabled: bool) -> bool {
    since.is_some_and(|t| !disabled && lead_in_speed(t, factor, duration) > 0.0)
}

/// The warp's lead-in speed v after `since` Mac ticks of the warp clock,
/// for a ship class's warp `factor` (w) and the warp sound's `duration`
/// (d), which `_ScanPlayer` (@0x7e562-0x7e633), the player's acceleration
/// away (`_HandlePlayer` @0x6d1c1-0x6d296) and the disabled player's jump
/// collapse (`_HandlePlayer` @0x688f2-0x6895e) all work out.
///
/// It is v = f32(f64(f32(t) × w) / (f64(d) × 0.01)), then, when w ≠ 0,
/// v = f32(f64(v) + (−35.0 / f64(w))), in the original's widths. With
/// w = 0 the −35/w term is skipped, so v is 0 or NaN. A d of 0 gives ±∞
/// or NaN, as in the original.
#[must_use]
pub fn lead_in_speed(since: u32, factor: f32, duration: u32) -> f32 {
    less_over_factor(clock_speed(since, factor, duration), factor, LEAD_IN)
}

/// The player's move-off speed v after `since` Mac ticks of the warp
/// clock (`_HandlePlayer` @0x6d1c1-0x6d296), for a ship class's warp
/// `factor` (w) and the warp sound's `duration` (d, by
/// [`warp_sound_duration`]): v = f32(f64(f32(t) × w) / (f64(d) × 0.01)),
/// then, when w ≠ 0, v = f32(f64(v) − K / f64(w)), with K 52.5 while
/// `double_time` (@0x6d264) and 35.0 otherwise (@0x6d26e). The ship moves
/// off while v > 0 (@0x6d293-0x6d296).
///
/// Out of double time it is [`lead_in_speed`] bit for bit: a − b is
/// a + (−b) in IEEE arithmetic. The cap at 50.0 the motion applies after
/// it (@0x6d2c3-0x6d2f8) is the motion's
/// (rdm:phase/ship-handling-fidelity/phase-8-warp-out).
#[must_use]
pub fn move_off_speed(since: u32, factor: f32, duration: u32, double_time: bool) -> f32 {
    let term = if double_time {
        MOVE_OFF_DOUBLE_TIME
    } else {
        MOVE_OFF
    };
    less_over_factor(clock_speed(since, factor, duration), factor, -term)
}

/// v = f32(f64(f32(t) × w) / (f64(d) × 0.01)), the warp clock's speed
/// before its term, for `since` Mac ticks, the warp `factor` and the
/// warp sound's `duration`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn clock_speed(since: u32, factor: f32, duration: u32) -> f32 {
    // The original makes t exact in f32 by its halves (@0x7e598-0x7e5ba,
    // @0x6d1d8-0x6d1fa): each half is exact, and their sum rounds once,
    // as this does.
    let t = since as f32;
    (f64::from(t * factor) / (f64::from(duration) * DURATION_SCALE)) as f32
}

/// `speed` plus `term` over the warp `factor`, f32(f64(v) + term /
/// f64(w)), when w ≠ 0 (a NaN w included, as `ucomiss`'s `jp` takes the
/// term's branch); `speed` as it is when w = 0.
#[allow(clippy::cast_possible_truncation)]
fn less_over_factor(speed: f32, factor: f32, term: f64) -> f32 {
    if factor == 0.0 {
        speed
    } else {
        (f64::from(speed) + term / f64::from(factor)) as f32
    }
}

/// Whose warp clock an NPC's warp-out reads (`_HandleShip`
/// @0x347c1-0x349c1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarpOutClock {
    /// A ship the player leads (+0x9a = 0, @0x347c1): the player's own
    /// clock (+0xa4) and warp factor, ×0.667 in double time, less 45/w.
    Player,
    /// Any other ship: its own clock and its class's warp factor, ×0.5 in
    /// double time, less 35/w.
    Own,
}

/// An NPC's warp-out speed v (`_HandleShip` @0x347c1-0x349c1), after
/// `since` Mac ticks of the `clock` it reads, for the warp `factor` (w)
/// of that clock's ship and the warp sound's `duration` (d, by
/// [`warp_sound_duration`]):
///
/// - v = f32(f64(t) × F × f64(w) / (f64(d) × 0.01)), t the unsigned 32
///   bits made exact in f64 (@0x347f6-0x34808, @0x348d0-0x348e2), with F
///   0.667 ([`WarpOutClock::Player`], @0x34825) or 0.5
///   ([`WarpOutClock::Own`], @0x348ff) while `double_time`, else 1.0;
/// - then, when w ≠ 0, v = f32(f64(v) + T / f64(w)), with T −45.0
///   (@0x348b5) or −35.0 (@0x3498a);
/// - then capped at 50.0 (@0x349a3-0x349be). A NaN passes the cap
///   (`ucomiss`, `jbe`).
///
/// The ship accelerates while v > 0 (@0x349c1). A ship the player leads
/// runs this only while the player's +0x50 > 1.0 (@0x347cf-0x347e3),
/// which is its caller's to check.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn npc_warp_out_speed(
    clock: WarpOutClock,
    since: u32,
    factor: f32,
    duration: u32,
    double_time: bool,
) -> f32 {
    let (rate, term) = match clock {
        WarpOutClock::Player => (LED_DOUBLE_TIME, LED_WARP_OUT),
        WarpOutClock::Own => (OWN_DOUBLE_TIME, LEAD_IN),
    };
    let rate = if double_time { rate } else { SINGLE_TIME };
    let t = f64::from(since) * rate * f64::from(factor);
    let v = (t / (f64::from(duration) * DURATION_SCALE)) as f32;
    let v = less_over_factor(v, factor, term);
    if v > WARP_OUT_CAP { WARP_OUT_CAP } else { v }
}

/// Whether each warp-up sound is still playing: `_warpUpSnd[0]`, `snd `
/// 128, and `_warpUpSnd[1]`, `snd ` 129, which the original asks
/// `ST_IsSoundPlaying` of as the warp stage starts (`_HandlePlayer`
/// @0x6b617-0x6b62b, @0x6b645-0x6b658) and as the double-time switch runs
/// (`_PlayGame` @0x463b3-0x463c1, @0x463ff-0x4640c).
///
/// The session models the play state itself rather than asking the
/// audio side. The original's sound level only scales the volume
/// (`_AdjustSoundVolume` @0x5efb-0x5f2a), so a warp-up plays, and holds,
/// with the sound off too, where the engine's audio plays nothing. The
/// engine's audio plays the warp-up at its own rate, so it lasts its
/// duration in Mac ticks ([`sound_duration`]), which [`WarpUps::start`]
/// counts down by [`MAC_TICKS_PER_TICK`] on each advance pass
/// ([`WarpUps::step`]), as the sound plays in real time. Accepted
/// differences: the countdown follows the session's ticks, so it stands
/// still under a dialog or the map, where the sound plays on; a warp-up
/// still playing as the ship lands is taken as over; and once the
/// warp-up's pitch follows w (`_AdjustWarpSoundSpeed`,
/// rdm:phase/ship-handling-fidelity/phase-8-warp-out), its length will
/// scale with it.
///
/// A jump's arrival halts both ([`SimSound::Halt`] of [`WARP_UP_HALTS`]),
/// as the original's warp end does (`_HandlePlayer` @0x6c69d-0x6c6d2); a
/// hypergate or wormhole exit keeps them playing, as the original's do
/// (@0x63e57, @0x6443d); a relocation's dock ends them for the session
/// only, raising no halt.
///
/// [`SimSound::Halt`]: crate::SimSound::Halt
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WarpUps {
    /// The Mac ticks left of 128 and of 129.
    left: [u32; 2],
}

impl WarpUps {
    /// Starts the warp-up for `double_time` (129, else 128), to play for
    /// `duration` Mac ticks.
    pub fn start(&mut self, double_time: bool, duration: u32) {
        self.left[usize::from(double_time)] = duration;
    }

    /// Plays one engine tick of both: [`MAC_TICKS_PER_TICK`] off each.
    pub fn step(&mut self) {
        for left in &mut self.left {
            *left = left.saturating_sub(MAC_TICKS_PER_TICK);
        }
    }

    /// Halts each warp-up `sounds` names, as `ST_HaltSound` does.
    pub fn halt(&mut self, sounds: &[SoundId]) {
        for (left, id) in self.left.iter_mut().zip(WARP_UP_HALTS) {
            if sounds.contains(&id) {
                *left = 0;
            }
        }
    }

    /// Ends both.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether the warp-up for `double_time` (129, else 128) is playing.
    #[must_use]
    pub fn playing(&self, double_time: bool) -> bool {
        self.left[usize::from(double_time)] > 0
    }
}

/// The warp-out sound `warpOutSnd`, `snd ` 130.
const WARP_OUT: SoundId = SoundId(130);

/// Whether the warp-out, `warpOutSnd`, `snd ` 130, is still playing, which
/// the original asks `ST_IsSoundPlaying` of at a jump's natural end
/// (`_HandlePlayer` @0x6bbe1-0x6bbe9): that end plays 130 only when it is
/// not (@0x6bc08), where the fallback end plays it unasked (@0x6c706).
///
/// Every arrival sound starts it ([`SimSound::Arrived`]): a jump's warp
/// end, and the hypergate's and wormhole's exits, which play 130
/// unconditionally (`_PlayerEnterHypergate` @0x63e57,
/// `_PlayerEnterWormhole` @0x6443d). It lasts [`warp_out_duration`],
/// counted down by [`MAC_TICKS_PER_TICK`] on each advance pass
/// ([`WarpOut::step`]). The player's explosion halts it
/// (`_HandleShipDisplay` @0x2d8d0-0x2d8ea, in [`PLAYER_EXPLOSION_HALTS`]),
/// and the session takes it as over on landing, as it does the warp-ups
/// ([`WarpUps`]).
///
/// As with [`WarpUps`], the session models the play state rather than
/// asking the audio side. Accepted differences: the countdown follows
/// the session's ticks, so it stands still under a dialog or the map,
/// where the sound plays on; and the engine plays 130 unpitched, as
/// `PlaySound` does, so its length is its duration at its own rate.
///
/// [`SimSound::Arrived`]: crate::SimSound::Arrived
/// [`PLAYER_EXPLOSION_HALTS`]: crate::sound::PLAYER_EXPLOSION_HALTS
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WarpOut {
    /// The Mac ticks left of 130.
    left: u32,
}

impl WarpOut {
    /// Starts the warp-out, to play for `duration` Mac ticks.
    pub fn start(&mut self, duration: u32) {
        self.left = duration;
    }

    /// Plays one engine tick of it: [`MAC_TICKS_PER_TICK`] off.
    pub fn step(&mut self) {
        self.left = self.left.saturating_sub(MAC_TICKS_PER_TICK);
    }

    /// Halts it if `sounds` names 130, as `ST_HaltSound` does.
    pub fn halt(&mut self, sounds: &[SoundId]) {
        if sounds.contains(&WARP_OUT) {
            self.left = 0;
        }
    }

    /// Ends it.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether it is playing.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.left > 0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(MAC_TICKS_PER_TICK, 2);
        assert_eq!(NO_SOUND_DURATION, 350);
        assert_eq!(mac_ticks(0), 0);
        assert_eq!(mac_ticks(37), 74);
        assert_eq!(mac_ticks(u32::MAX), u32::MAX, "saturates");
    }

    // The warp factor.

    /// The original's f32 of the f64 product of `base` and 1.3.
    #[allow(clippy::cast_possible_truncation)]
    fn scaled(base: f32) -> f32 {
        (f64::from(base) * 1.3) as f32
    }

    #[test]
    fn each_jump_speed_flag_gives_its_warp_factor_bit_exact() {
        assert_eq!(warp_factor(0x0001).to_bits(), scaled(0.7).to_bits());
        assert_eq!(warp_factor(0x0002).to_bits(), scaled(1.3).to_bits());
        assert_eq!(warp_factor(0x0004).to_bits(), scaled(1.6).to_bits());
        assert_eq!(warp_factor(0).to_bits(), scaled(1.0).to_bits());
        assert!((warp_factor(0x0001) - 0.91).abs() < 1e-6);
        assert!((warp_factor(0x0002) - 1.69).abs() < 1e-6);
        assert!((warp_factor(0x0004) - 2.08).abs() < 1e-6);
        assert_eq!(warp_factor(0), 1.3);
    }

    #[test]
    fn the_lowest_jump_speed_bit_wins_and_other_bits_are_ignored() {
        assert_eq!(warp_factor(0x0007), warp_factor(0x0001));
        assert_eq!(warp_factor(0x0005), warp_factor(0x0001));
        assert_eq!(warp_factor(0x0006), warp_factor(0x0002));
        assert_eq!(warp_factor(0xfff8), warp_factor(0), "no jump speed bit");
        assert_eq!(warp_factor(0x8004), warp_factor(0x0004));
    }

    // The warp sound's duration.

    #[test]
    fn the_duration_is_the_frames_times_60_over_the_rate() {
        let warp_up = SoundLength {
            frames: 134_016,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(warp_up)), 364, "stock snd 128");
        let exact = SoundLength {
            frames: 22_050,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(exact)), 60);
    }

    #[test]
    fn the_duration_divides_by_the_whole_part_of_the_rate() {
        // 22254.54 Hz divides by 22254: 22254 × 100 frames is 6000 ticks
        // exactly, where 22255 would give 5999.
        let sound = SoundLength {
            frames: 2_225_400,
            rate: 22_254,
        };
        assert_eq!(sound_duration(Some(sound)), 6000);
    }

    #[test]
    fn no_sound_or_no_rate_gives_350() {
        assert_eq!(sound_duration(None), 350);
        let still = SoundLength {
            frames: 1000,
            rate: 0,
        };
        assert_eq!(sound_duration(Some(still)), 350);
        let empty = SoundLength {
            frames: 0,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(empty)), 0, "a sound of no frames");
    }

    #[test]
    fn a_long_sound_keeps_its_duration_wide() {
        let long = SoundLength {
            frames: u32::MAX,
            rate: 1,
        };
        assert_eq!(sound_duration(Some(long)), u32::MAX, "capped");
        let wide = SoundLength {
            frames: 1_000_000,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(wide)), 2721, "past an i16's 16 bits");
    }

    // The lead-in speed.

    #[test]
    fn the_lead_in_speed_is_tw_over_d_hundredths_less_35_over_w() {
        let w = warp_factor(0);
        assert!(lead_in_speed(72, w, 350) <= 0.0);
        assert!(lead_in_speed(74, w, 350) > 0.0);
        // 2 × 1.3 / 0.09 − 35 / 1.3, in the original's widths.
        #[allow(clippy::cast_possible_truncation)]
        let expected = {
            let v = (f64::from(2.0_f32 * w) / (9.0 * 0.01)) as f32;
            (f64::from(v) - 35.0 / f64::from(w)) as f32
        };
        assert_eq!(lead_in_speed(2, w, 9).to_bits(), expected.to_bits());
        assert!((lead_in_speed(2, w, 9) - 1.9658).abs() < 1e-3);
        // A factor of 0 skips the −35/w term.
        assert_eq!(lead_in_speed(1000, 0.0, 350), 0.0);
        assert!(lead_in_speed(1000, 0.0, 0).is_nan(), "0 / 0");
        // A duration of 0 gives +∞ once time has passed, NaN before.
        assert_eq!(lead_in_speed(2, w, 0), f32::INFINITY);
        assert!(lead_in_speed(0, w, 0).is_nan());
        assert!(lead_in_speed(1000, f32::NAN, 350).is_nan());
        // The factor moves the zero crossing: 147.9 and 28.3.
        let slow = warp_factor(0x0001);
        assert!(lead_in_speed(146, slow, 350) <= 0.0);
        assert!(lead_in_speed(148, slow, 350) > 0.0);
        let fast = warp_factor(0x0004);
        assert!(lead_in_speed(28, fast, 350) <= 0.0);
        assert!(lead_in_speed(30, fast, 350) > 0.0);
    }

    // The hold.

    #[test]
    fn no_clock_holds_nothing_and_neither_does_a_disabled_player() {
        let w = warp_factor(0);
        assert!(!held(None, w, 350, false));
        assert!(held(Some(1000), w, 350, false));
        assert!(!held(Some(1000), w, 350, true), "disabled");
    }

    #[test]
    fn the_lead_in_ends_past_0_35_d_over_w_squared() {
        // 0.35 × 350 / 1.69 = 72.49 Mac ticks.
        let w = warp_factor(0);
        assert!(!held(Some(0), w, 350, false));
        assert!(!held(Some(72), w, 350, false));
        assert!(held(Some(74), w, 350, false));
    }

    #[test]
    fn the_warp_factor_moves_the_end_of_the_lead_in() {
        // 0.91: 0.35 × 350 / 0.8281 = 147.9.
        let slow = warp_factor(0x0001);
        assert!(!held(Some(146), slow, 350, false));
        assert!(held(Some(148), slow, 350, false));
        // 2.08: 0.35 × 350 / 4.3264 = 28.3.
        let fast = warp_factor(0x0004);
        assert!(!held(Some(28), fast, 350, false));
        assert!(held(Some(30), fast, 350, false));
        // The duration moves it too: 0.35 × 364 / 1.69 = 75.4.
        let w = warp_factor(0);
        assert!(!held(Some(74), w, 364, false));
        assert!(held(Some(76), w, 364, false));
    }

    #[test]
    fn a_warp_factor_of_0_never_holds() {
        for since in [0, 2, 74, 1000, u32::MAX] {
            for duration in [0, 350] {
                assert!(
                    !held(Some(since), 0.0, duration, false),
                    "{since} {duration}"
                );
            }
        }
    }

    #[test]
    fn a_duration_of_0_holds_once_any_time_has_passed() {
        // t·w / 0 is +∞ once t > 0, and +∞ − 35/w is still +∞; at t = 0 it
        // is 0 / 0, NaN, which is never above 0.
        let w = warp_factor(0);
        assert!(!held(Some(0), w, 0, false), "NaN");
        assert!(held(Some(2), w, 0, false), "+∞");
    }

    // The duration by double time (`_WarpSoundDuration`).

    #[test]
    fn double_time_reads_the_second_warp_sound() {
        assert_eq!(warp_sound_duration([364, 252], false), 364, "snd 128");
        assert_eq!(warp_sound_duration([364, 252], true), 252, "snd 129");
        assert_eq!(warp_sound_duration([350, 350], true), 350);
    }

    // The player's move-off.

    /// F5's arithmetic with the term `k`, in the original's widths.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn move_off(since: u32, w: f32, d: u32, k: f64) -> f32 {
        let v = (f64::from(since as f32 * w) / (f64::from(d) * 0.01)) as f32;
        if w == 0.0 {
            v
        } else {
            (f64::from(v) - k / f64::from(w)) as f32
        }
    }

    #[test]
    fn the_move_off_takes_52_5_over_w_in_double_time_and_35_otherwise() {
        let w = warp_factor(0);
        for since in [0, 2, 74, 120, 1000] {
            for d in [252, 350, 364] {
                assert_eq!(
                    move_off_speed(since, w, d, true).to_bits(),
                    move_off(since, w, d, 52.5).to_bits(),
                    "{since} {d}"
                );
                assert_eq!(
                    move_off_speed(since, w, d, false).to_bits(),
                    move_off(since, w, d, 35.0).to_bits(),
                    "{since} {d}"
                );
                assert_eq!(
                    move_off_speed(since, w, d, false).to_bits(),
                    lead_in_speed(since, w, d).to_bits(),
                    "the lead-in's −35/w, bit for bit"
                );
            }
        }
        // 0.525 × 252 / 1.69 = 78.3 Mac ticks in double time, where the
        // 35 of single time gives 0.35 × 252 / 1.69 = 52.2.
        assert!(move_off_speed(78, w, 252, true) <= 0.0);
        assert!(move_off_speed(80, w, 252, true) > 0.0);
        assert!(move_off_speed(54, w, 252, false) > 0.0);
    }

    #[test]
    fn a_move_off_of_no_factor_or_no_duration_is_the_originals() {
        let w = warp_factor(0);
        assert_eq!(move_off_speed(1000, 0.0, 350, true), 0.0, "no term");
        assert!(move_off_speed(1000, 0.0, 0, true).is_nan(), "0 / 0");
        assert_eq!(move_off_speed(2, w, 0, true), f32::INFINITY);
        assert!(move_off_speed(0, w, 0, true).is_nan());
        assert!(move_off_speed(1000, f32::NAN, 350, true).is_nan());
    }

    // An NPC's warp-out.

    /// F6's arithmetic: the clock's `factor`, then the `term` over w and
    /// the cap at 50, in the original's widths.
    #[allow(clippy::cast_possible_truncation)]
    fn warp_out(since: u32, w: f32, d: u32, factor: f64, term: f64) -> f32 {
        let mut v = (f64::from(since) * factor * f64::from(w) / (f64::from(d) * 0.01)) as f32;
        if w != 0.0 {
            v = (f64::from(v) + term / f64::from(w)) as f32;
        }
        if v > 50.0 { 50.0 } else { v }
    }

    #[test]
    fn a_ship_the_player_leads_runs_its_warp_out_at_0_667_less_45_over_w() {
        let w = warp_factor(0);
        for since in [0, 2, 100, 150, 400] {
            assert_eq!(
                npc_warp_out_speed(WarpOutClock::Player, since, w, 252, true).to_bits(),
                warp_out(since, w, 252, 0.667, -45.0).to_bits(),
                "{since}"
            );
            assert_eq!(
                npc_warp_out_speed(WarpOutClock::Player, since, w, 364, false).to_bits(),
                warp_out(since, w, 364, 1.0, -45.0).to_bits(),
                "{since}"
            );
        }
        // 0.45 × 252 / (0.667 × 1.69) = 100.6 Mac ticks in double time.
        assert!(npc_warp_out_speed(WarpOutClock::Player, 100, w, 252, true) <= 0.0);
        assert!(npc_warp_out_speed(WarpOutClock::Player, 101, w, 252, true) > 0.0);
    }

    #[test]
    fn any_other_ship_runs_its_warp_out_at_0_5_less_35_over_w() {
        let w = warp_factor(0x0004);
        for since in [0, 2, 50, 150, 400] {
            assert_eq!(
                npc_warp_out_speed(WarpOutClock::Own, since, w, 252, true).to_bits(),
                warp_out(since, w, 252, 0.5, -35.0).to_bits(),
                "{since}"
            );
            assert_eq!(
                npc_warp_out_speed(WarpOutClock::Own, since, w, 364, false).to_bits(),
                warp_out(since, w, 364, 1.0, -35.0).to_bits(),
                "{since}"
            );
        }
        // 0.35 × 252 / (0.5 × 4.3264) = 40.8 Mac ticks in double time.
        assert!(npc_warp_out_speed(WarpOutClock::Own, 40, w, 252, true) <= 0.0);
        assert!(npc_warp_out_speed(WarpOutClock::Own, 41, w, 252, true) > 0.0);
    }

    #[test]
    fn a_warp_out_is_capped_at_50_and_a_nan_passes() {
        let w = warp_factor(0);
        for clock in [WarpOutClock::Player, WarpOutClock::Own] {
            assert_eq!(npc_warp_out_speed(clock, 100_000, w, 252, true), 50.0);
            assert_eq!(npc_warp_out_speed(clock, 2, w, 0, false), 50.0, "+∞");
            assert!(npc_warp_out_speed(clock, 0, w, 0, false).is_nan(), "0 / 0");
            assert!(npc_warp_out_speed(clock, 100, f32::NAN, 252, true).is_nan());
        }
    }

    #[test]
    fn a_warp_out_of_no_factor_skips_its_term() {
        for clock in [WarpOutClock::Player, WarpOutClock::Own] {
            assert_eq!(npc_warp_out_speed(clock, 1000, 0.0, 252, true), 0.0);
        }
    }

    #[test]
    fn a_warp_out_clock_near_the_top_of_32_bits_is_exact() {
        // The original makes the unsigned t exact in f64 (@0x347f6-0x34808).
        let since = u32::MAX - 1;
        let d = u32::MAX;
        assert_eq!(
            npc_warp_out_speed(WarpOutClock::Own, since, 1.0, d, true).to_bits(),
            warp_out(since, 1.0, d, 0.5, -35.0).to_bits()
        );
        assert!(npc_warp_out_speed(WarpOutClock::Own, since, 1.0, d, true) < 50.0);
    }

    // The warp-up sounds' play state.

    #[test]
    fn a_warp_up_plays_for_its_duration_in_mac_ticks() {
        let mut ups = WarpUps::default();
        assert!(!ups.playing(false) && !ups.playing(true), "none at first");
        ups.start(true, 6);
        assert!(ups.playing(true));
        assert!(!ups.playing(false), "only the one started");
        ups.step();
        ups.step();
        assert!(ups.playing(true), "2 Mac ticks left");
        ups.step();
        assert!(!ups.playing(true), "over after 3 engine ticks");
        ups.step();
        assert!(!ups.playing(true), "stays over");
        ups.start(false, 5);
        ups.step();
        ups.step();
        assert!(ups.playing(false), "1 Mac tick left");
        ups.step();
        assert!(!ups.playing(false));
        ups.start(false, 0);
        assert!(!ups.playing(false), "a sound of no length");
    }

    #[test]
    fn a_halt_ends_the_warp_ups_it_names() {
        let mut ups = WarpUps::default();
        ups.start(false, 364);
        ups.start(true, 252);
        ups.halt(&[SoundId(129)]);
        assert!(ups.playing(false), "128 plays on");
        assert!(!ups.playing(true));
        ups.start(true, 252);
        ups.halt(&[SoundId(371), SoundId(130)]);
        assert!(ups.playing(false) && ups.playing(true), "neither named");
        ups.halt(&crate::sound::WARP_UP_HALTS);
        assert!(!ups.playing(false) && !ups.playing(true));
    }

    #[test]
    fn clearing_ends_both_warp_ups() {
        let mut ups = WarpUps::default();
        ups.start(false, 364);
        ups.start(true, 252);
        ups.clear();
        assert_eq!(ups, WarpUps::default());
    }

    // The warp-out sound's play state.

    #[test]
    fn the_warp_out_lasts_its_sounds_duration_and_nothing_without_one() {
        let sound = SoundLength {
            frames: 44_508,
            rate: 22_254,
        };
        assert_eq!(warp_out_duration(Some(sound)), 120);
        assert_eq!(warp_out_duration(Some(sound)), sound_duration(Some(sound)));
        assert_eq!(warp_out_duration(None), 0, "no 350-tick fallback");
        assert_eq!(
            warp_out_duration(Some(SoundLength {
                frames: 100,
                rate: 0
            })),
            0,
            "no rate, no sound"
        );
    }

    #[test]
    fn the_warp_out_plays_for_its_duration_in_mac_ticks() {
        let mut out = WarpOut::default();
        assert!(!out.playing(), "not at first");
        out.start(5);
        assert!(out.playing());
        out.step();
        out.step();
        assert!(out.playing(), "1 Mac tick left");
        out.step();
        assert!(!out.playing(), "over after 3 engine ticks");
        out.step();
        assert!(!out.playing(), "stays over");
        out.start(0);
        assert!(!out.playing(), "a missing 130 never plays");
    }

    #[test]
    fn a_halt_naming_130_ends_the_warp_out() {
        let mut out = WarpOut::default();
        out.start(200);
        out.halt(&crate::sound::WARP_UP_HALTS);
        assert!(out.playing(), "128 and 129 are not it");
        out.halt(&crate::sound::PLAYER_EXPLOSION_HALTS);
        assert!(!out.playing());
    }

    #[test]
    fn clearing_ends_the_warp_out() {
        let mut out = WarpOut::default();
        out.start(200);
        out.clear();
        assert_eq!(out, WarpOut::default());
    }

    #[test]
    fn a_nan_warp_factor_never_holds() {
        // The original's `jp` takes the −35/w branch for a NaN factor
        // (@0x7e60c), which leaves v NaN.
        assert!(!held(Some(1000), f32::NAN, 350, false));
    }
}
