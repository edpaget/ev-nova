//! The layer a ship is drawn in, [`ShipLayer`], from
//! `nova_sim_combat::ship_layer`, which re-exports it and keeps the order
//! the ships enter each layer.

/// The layer a ship is drawn in, bottom to top (see
/// `nova_sim_combat::ship_layer`'s module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipLayer {
    /// A disabled or breaking-up ship: `0x3b53a0`, the bottom ship layer.
    Disabled,
    /// A ship the player leads: `0x3b52a8`, the escorts' layer.
    Led,
    /// Every other ship, and the player: `0x3b541c`, the top ship layer.
    Ships,
}

impl ShipLayer {
    /// The layer of a ship `disabled` or not, `led` by the player or not:
    /// being disabled wins, as @0x2bd24 asks it first.
    #[must_use]
    pub fn of(disabled: bool, led: bool) -> Self {
        if disabled {
            Self::Disabled
        } else if led {
            Self::Led
        } else {
            Self::Ships
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disabled_ship_is_in_the_bottom_layer_even_when_led() {
        assert_eq!(ShipLayer::of(true, true), ShipLayer::Disabled);
        assert_eq!(ShipLayer::of(true, false), ShipLayer::Disabled);
        assert_eq!(ShipLayer::of(false, true), ShipLayer::Led);
        assert_eq!(ShipLayer::of(false, false), ShipLayer::Ships);
        assert!(ShipLayer::Disabled < ShipLayer::Led);
        assert!(ShipLayer::Led < ShipLayer::Ships);
    }
}
