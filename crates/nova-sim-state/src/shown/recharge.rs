//! Why the spaceport can't recharge the ship ([`RechargeRefusal`]) and
//! whether a stellar sells fuel at all ([`sells_fuel`]), from
//! `nova_sim_port::recharge`, which re-exports them and keeps the fill
//! and its settling.

use crate::landing::StellarFlags;

/// Why the spaceport won't recharge the ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RechargeRefusal {
    /// No fuel is sold: the ship is not landed, or the stellar sells none.
    NoFuel,
}

/// Whether a stellar with these `spöb` flags sells fuel: any but an
/// uninhabited one.
#[must_use]
pub fn sells_fuel(flags: u32) -> bool {
    flags & StellarFlags::UNINHABITED == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stellar_but_an_uninhabited_one_sells_fuel() {
        for flags in [0x01, 0x57, 0x4242_2157] {
            assert!(sells_fuel(flags), "{flags:#x}");
        }
        for flags in [0x20, 0x21, 0x4242_2177, u32::MAX] {
            assert!(!sells_fuel(flags), "{flags:#x}");
        }
    }
}
