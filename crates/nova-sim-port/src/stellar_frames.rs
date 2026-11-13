//! A stellar's animation by the sequential branch of the original's
//! sprite tick: which frame of its sheet shows, stepped pass by pass, as
//! `_HandleStellarSprites` (@0x2e77b-0x2ee35, stellar-fidelity phase 1)
//! steps it in every game-work pass (@0x444d2), the double-time second
//! pass's included, and in each pass outside one, under a
//! `_PrepareAllSprites` or at game entry ([`StellarFrames::step_by`], at
//! the gain the pass reads). The session keeps each stellar's
//! [`StellarFrames`] and steps it (`nova_sim::Session`'s
//! `stellar_frames` module); the system viewer, which runs no session,
//! steps its own.
//!
//! A stellar animates when the frame count it animates over (its
//! counted sheet's) is above 1, it is not a hypergate, and its destroyed
//! state is the one its flags name: a stellar marked to animate only when
//! destroyed (`Flags2` 0x0080) animates only while destroyed, and any
//! other only while not (@0x2e784-0x2e7bb, @0x2e802). A stellar that does
//! not animate shows frame 0 (@0x2ee20), its timer and frames frozen. A
//! hypergate (`Flags2` 0x1000, @0x2e832) takes its own branch
//! ([`gate_sprite`](crate::gate_sprite), system-traffic-fidelity phase 7).
//!
//! Each pass an animating stellar's timer gains the pass's gain (the
//! original's `_gSpeedMult`, @0x2e810-0x2e825: 1.0 a frame at 30 frames
//! a second, less on the frames that ramp up after a landing or a dialog;
//! see `nova_sim_base::speed_mult`). Once it reaches `AnimDelay`, or
//! `AnimDelay` x `Frame0Bias` while the current frame is 0 and the bias
//! is above 1 (@0x2ec4b-0x2ec83), the timer starts again and the frames
//! advance. A threshold of 0 or less is met every pass: an `AnimDelay` of
//! 0 or less advances every pass, as nothing in the original gates on
//! it. The product is kept wide where the original's 16-bit `imul` wraps.
//!
//! How the frames advance (@0x2ec9b-0x2edf4):
//!
//! - Plainly, the current frame takes the next, and the next becomes the
//!   one after it modulo the counted frame count.
//! - `Flags2` 0x0001 (alternate) shows frame 0 between every other frame:
//!   from any other frame the current goes back to 0, the next kept; from
//!   frame 0 it takes the next, and the next becomes the one after it,
//!   its wrap skipping frame 0: 0, 1, 0, 2, 0, 3, 0, 1, ...
//! - `Flags2` 0x0002 (random) picks each next frame at random on the
//!   [`Chance`](crate::chance::Chance) the caller gives: uniform over the counted frames but the
//!   one just made current, and but frame 0 as well under 0x0001. With no
//!   frame left to pick (two frames under both), where the original draws
//!   forever, the next frame stays.
//!
//! Then the sheet drawn shows the current frame modulo its own frame
//! count (`_SWSetCurrentFrameIndex` @0xa90d4-0xa90e9). Between advances
//! the sheet drawn keeps the frame it shows; a swap to another sheet
//! keeps it while it is in range, and shows frame 0 otherwise
//! (`_SWCloneFramesFromMaster` @0xac504-0xac532). A frozen slot steps its
//! frames and draws its random picks, while the frame shown stays, as
//! every `_SWSetCurrentFrameIndex` refuses a slot whose frames count none
//! (@0x2ee20-0x2ee30, @0xa90e7-0xa90ee).
//!
//! The function plays no sound (see the [`gate_sprite`](crate::gate_sprite)
//! module docs).

pub use nova_sim_state::shown::stellar_frames::{StellarAnimation, StellarFrames, is_animated};
