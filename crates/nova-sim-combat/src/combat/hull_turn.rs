//! The angle a ship's hull stands at, which its weapon exit points and
//! fixed beams turn with, as the rulebook's
//! [`RuleKey::ContinuousHeadings`](crate::RuleKey::ContinuousHeadings)
//! says ([`HullTurn`], [`hull_angle`]).
//!
//! - By the engine ([`HullTurn::ByFrame`]), the hull stands at the angle
//!   of the rotation frame its sprite is drawn on: the frame
//!   `trunc(heading × FramesPer / 360)`, or the one its `shän` frame-angle
//!   table picks (`_HandleShipDisplay` @0x2b7a6-0x2b7d0, @0x2bba2-0x2bbe5;
//!   [`drawn_frame`](super::exits::drawn_frame)), at `trunc(frame × 360 / FramesPer)` whole degrees
//!   (`_ModifyShotStartPosition` @0x739e-0x73f5, `_HandleBeams`
//!   @0x303bb-0x3042e; [`frame_angle`](super::exits::frame_angle)). The angle holds still while the
//!   heading turns within a frame, then steps a frame's angle.
//! - By the other reading ([`HullTurn::Continuous`]), the new game's, the
//!   hull stands at the heading itself, so a model hull turns smoothly and
//!   its exits turn with it ([`ExitPoints::offset`](super::exits::ExitPoints::offset)
//!   turns them exactly rather than by the original's sine table).
//!
//! A sprite is drawn on its frame under either reading
//! ([`FrameSpec::rotation`](super::ship_frames::FrameSpec::rotation)), as
//! are its shield, glow and lights layers and its collision mask. None of
//! this plays a sound.

pub use nova_sim_state::shown::combat::hull_turn::{HullTurn, hull_angle};
