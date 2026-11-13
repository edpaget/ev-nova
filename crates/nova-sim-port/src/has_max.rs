//! Whether the player holds the most of an outfit it may: `_HasMaxOfItem`
//! (@0x4512-0x4894 in the `EV Nova` executable), ported whole, once, in
//! `nova_sim_state::shown::has_max`, re-exported here at its old path. It
//! serves the outfitter's buy check and info box (see
//! [`outfitter`](crate::outfitter)), `H`'s hold (see
//! [`ship_change`](crate::ship_change)), a boarding grant's pick (see
//! [`grant`](crate::grant)) and a boarding's Ammo take
//! (`nova_sim::Session::plunder`).

pub use nova_sim_state::shown::has_max::*;
