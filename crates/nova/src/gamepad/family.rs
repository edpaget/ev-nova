//! Which [`ControllerFamily`] a pad is, from the ids its adapter reads:
//! the core's decision, so every adapter only forwards the ids
//! ([`PadIds`]) and nova-view names the family's buttons.
//!
//! The family is the vendor's: Microsoft's pads are Xbox, Sony's
//! `PlayStation` and Nintendo's Nintendo. Any other vendor, a third-party
//! pad in an Xbox mode included, or no ids at all, is generic, named by
//! position (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs's
//! findings).

use nova_view::ControllerFamily;

use super::PadIds;

/// Microsoft's USB vendor id.
const MICROSOFT: u16 = 0x045E;
/// Sony's.
const SONY: u16 = 0x054C;
/// Nintendo's.
const NINTENDO: u16 = 0x057E;

/// The family a pad with `ids` belongs to: by its vendor, generic for an
/// unknown one or none.
#[must_use]
pub const fn family_of(ids: Option<PadIds>) -> ControllerFamily {
    match ids {
        Some(PadIds {
            vendor: MICROSOFT, ..
        }) => ControllerFamily::Xbox,
        Some(PadIds { vendor: SONY, .. }) => ControllerFamily::PlayStation,
        Some(PadIds {
            vendor: NINTENDO, ..
        }) => ControllerFamily::Nintendo,
        _ => ControllerFamily::Generic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unnecessary_wraps, reason = "what family_of takes")]
    fn ids(vendor: u16, product: u16) -> Option<PadIds> {
        Some(PadIds { vendor, product })
    }

    #[test]
    fn a_known_id_of_each_family_maps_to_it() {
        for (vendor, product, family) in [
            (0x045E, 0x02EA, ControllerFamily::Xbox),
            (0x045E, 0x0B12, ControllerFamily::Xbox),
            (0x054C, 0x09CC, ControllerFamily::PlayStation),
            (0x054C, 0x0CE6, ControllerFamily::PlayStation),
            (0x057E, 0x2009, ControllerFamily::Nintendo),
        ] {
            assert_eq!(
                family_of(ids(vendor, product)),
                family,
                "{vendor:04X}:{product:04X}"
            );
        }
    }

    #[test]
    fn unknown_or_missing_ids_are_generic() {
        assert_eq!(family_of(ids(0x046D, 0xC21D)), ControllerFamily::Generic);
        assert_eq!(family_of(None), ControllerFamily::Generic);
    }
}
