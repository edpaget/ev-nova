//! What a pad's buttons and axes are called on each [`ControllerFamily`]:
//! the labels printed on its pads, as words, never a console maker's
//! artwork. A position a family prints nothing distinctive on is named
//! for where it is (the d-pad, the sticks), and the generic family names
//! every button by its position. Nintendo's Minus and Plus are words, so
//! no name holds the `+` that joins a layer to its button (`View+A`).
//!
//! rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs's findings
//! record the table.

use super::{ControllerFamily, PadAxis, PadButton};

/// What `button` is called on a pad of `family`.
///
/// It matches [`ControllerFamily`] exhaustively, as its owner: a new
/// family must name every button.
#[must_use]
pub const fn pad_button_name(family: ControllerFamily, button: PadButton) -> &'static str {
    use ControllerFamily as F;
    use PadButton as P;
    match (button, family) {
        (P::South, F::Xbox) | (P::East, F::Nintendo) => "A",
        (P::East, F::Xbox) | (P::South, F::Nintendo) => "B",
        (P::West, F::Xbox) | (P::North, F::Nintendo) => "X",
        (P::North, F::Xbox) | (P::West, F::Nintendo) => "Y",
        (P::South, F::PlayStation) => "Cross",
        (P::East, F::PlayStation) => "Circle",
        (P::West, F::PlayStation) => "Square",
        (P::North, F::PlayStation) => "Triangle",
        (P::South, F::Generic) => "South",
        (P::East, F::Generic) => "East",
        (P::West, F::Generic) => "West",
        (P::North, F::Generic) => "North",
        (P::LeftShoulder, F::Xbox) => "LB",
        (P::RightShoulder, F::Xbox) => "RB",
        (P::LeftShoulder, F::PlayStation) => "L1",
        (P::RightShoulder, F::PlayStation) => "R1",
        (P::LeftShoulder, F::Nintendo) => "L",
        (P::RightShoulder, F::Nintendo) => "R",
        (P::LeftShoulder, F::Generic) => "L Shoulder",
        (P::RightShoulder, F::Generic) => "R Shoulder",
        (P::LeftTrigger, F::Xbox) => "LT",
        (P::RightTrigger, F::Xbox) => "RT",
        (P::LeftTrigger, F::PlayStation) => "L2",
        (P::RightTrigger, F::PlayStation) => "R2",
        (P::LeftTrigger, F::Nintendo) => "ZL",
        (P::RightTrigger, F::Nintendo) => "ZR",
        (P::LeftTrigger, F::Generic) => "L Trigger",
        (P::RightTrigger, F::Generic) => "R Trigger",
        (P::Select, F::Xbox) => "View",
        (P::Start, F::Xbox) => "Menu",
        (P::Select, F::PlayStation) => "Share",
        (P::Start, F::PlayStation) => "Options",
        (P::Select, F::Nintendo) => "Minus",
        (P::Start, F::Nintendo) => "Plus",
        (P::Select, F::Generic) => "Select",
        (P::Start, F::Generic) => "Start",
        (P::LeftStick, F::Xbox) => "LS",
        (P::RightStick, F::Xbox) => "RS",
        (P::LeftStick, F::PlayStation) => "L3",
        (P::RightStick, F::PlayStation) => "R3",
        (P::LeftStick, F::Nintendo | F::Generic) => "L Stick Click",
        (P::RightStick, F::Nintendo | F::Generic) => "R Stick Click",
        (P::DPadUp, _) => "D-Up",
        (P::DPadDown, _) => "D-Down",
        (P::DPadLeft, _) => "D-Left",
        (P::DPadRight, _) => "D-Right",
        (P::LeftStickUp, _) => "L Stick Up",
        (P::LeftStickDown, _) => "L Stick Down",
        (P::LeftStickLeft, _) => "L Stick Left",
        (P::LeftStickRight, _) => "L Stick Right",
        (P::RightStickUp, _) => "R Stick Up",
        (P::RightStickDown, _) => "R Stick Down",
        (P::RightStickLeft, _) => "R Stick Left",
        (P::RightStickRight, _) => "R Stick Right",
    }
}

/// What `axis` is called on a pad of `family`: every family's the same.
#[must_use]
pub const fn pad_axis_name(family: ControllerFamily, axis: PadAxis) -> &'static str {
    match (family, axis) {
        (_, PadAxis::LeftX) => "L Stick X",
        (_, PadAxis::LeftY) => "L Stick Y",
        (_, PadAxis::RightX) => "R Stick X",
        (_, PadAxis::RightY) => "R Stick Y",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use ControllerFamily as F;
    use PadButton as P;

    #[test]
    fn each_family_names_south_and_east_its_own_way() {
        for (family, south, east) in [
            (F::Xbox, "A", "B"),
            (F::PlayStation, "Cross", "Circle"),
            (F::Nintendo, "B", "A"),
            (F::Generic, "South", "East"),
        ] {
            assert_eq!(pad_button_name(family, P::South), south, "{family:?}");
            assert_eq!(pad_button_name(family, P::East), east, "{family:?}");
        }
    }

    #[test]
    fn every_button_and_axis_has_a_name_in_every_family() {
        for family in ControllerFamily::ALL {
            let names: BTreeSet<&str> = PadButton::ALL
                .iter()
                .map(|&button| pad_button_name(family, button))
                .collect();
            assert_eq!(names.len(), PadButton::ALL.len(), "{family:?}: one each");
            assert!(!names.contains(""), "{family:?}");
            for name in &names {
                assert!(!name.contains('+'), "{family:?} {name}: + joins a layer");
            }
            let axes: BTreeSet<&str> = PadAxis::ALL
                .iter()
                .map(|&axis| pad_axis_name(family, axis))
                .collect();
            assert_eq!(axes.len(), PadAxis::ALL.len(), "{family:?}");
        }
    }

    /// The findings' name table, row by row: each position with its
    /// Xbox, `PlayStation`, Nintendo and generic names.
    const TABLE: &[(PadButton, [&str; 4])] = &[
        (P::South, ["A", "Cross", "B", "South"]),
        (P::East, ["B", "Circle", "A", "East"]),
        (P::West, ["X", "Square", "Y", "West"]),
        (P::North, ["Y", "Triangle", "X", "North"]),
        (P::LeftShoulder, ["LB", "L1", "L", "L Shoulder"]),
        (P::RightShoulder, ["RB", "R1", "R", "R Shoulder"]),
        (P::LeftTrigger, ["LT", "L2", "ZL", "L Trigger"]),
        (P::RightTrigger, ["RT", "R2", "ZR", "R Trigger"]),
        (P::Select, ["View", "Share", "Minus", "Select"]),
        (P::Start, ["Menu", "Options", "Plus", "Start"]),
        (P::LeftStick, ["LS", "L3", "L Stick Click", "L Stick Click"]),
        (
            P::RightStick,
            ["RS", "R3", "R Stick Click", "R Stick Click"],
        ),
    ];

    /// The names every family shares.
    const SHARED: &[(PadButton, &str)] = &[
        (P::DPadUp, "D-Up"),
        (P::DPadDown, "D-Down"),
        (P::DPadLeft, "D-Left"),
        (P::DPadRight, "D-Right"),
        (P::LeftStickUp, "L Stick Up"),
        (P::LeftStickDown, "L Stick Down"),
        (P::LeftStickLeft, "L Stick Left"),
        (P::LeftStickRight, "L Stick Right"),
        (P::RightStickUp, "R Stick Up"),
        (P::RightStickDown, "R Stick Down"),
        (P::RightStickLeft, "R Stick Left"),
        (P::RightStickRight, "R Stick Right"),
    ];

    #[test]
    fn the_name_table_is_pinned() {
        let families = [F::Xbox, F::PlayStation, F::Nintendo, F::Generic];
        assert_eq!(TABLE.len() + SHARED.len(), PadButton::ALL.len());
        for &(button, names) in TABLE {
            for (family, name) in families.into_iter().zip(names) {
                assert_eq!(
                    pad_button_name(family, button),
                    name,
                    "{family:?} {button:?}"
                );
            }
        }
        for &(button, name) in SHARED {
            for family in families {
                assert_eq!(
                    pad_button_name(family, button),
                    name,
                    "{family:?} {button:?}"
                );
            }
        }
        for family in families {
            assert_eq!(
                PadAxis::ALL.map(|axis| pad_axis_name(family, axis)),
                ["L Stick X", "L Stick Y", "R Stick X", "R Stick Y"],
                "{family:?}"
            );
        }
        assert_eq!(ControllerFamily::ALL, families);
        assert_eq!(ControllerFamily::default(), F::Generic);
    }

    #[test]
    fn generic_names_are_positions() {
        assert_eq!(
            [P::South, P::East, P::West, P::North]
                .map(|button| pad_button_name(F::Generic, button)),
            ["South", "East", "West", "North"]
        );
    }
}
