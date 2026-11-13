//! Why a hypergate or wormhole took the ship nowhere ([`GateRefusal`]),
//! and the hypergate map's next pick ([`next_gate_pick`]), from
//! `nova_sim_ai::gate`, which re-exports them and keeps the gates' links,
//! exits and emerging.

use nova_sim_catalog::catalog::SystemId;

/// Why entering a hypergate or wormhole took the ship nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateRefusal {
    /// No entry is pending: the land key has not just been pressed over a
    /// gate of that kind, cleared, or a tick has passed since.
    NotAtGate,
    /// The hypergate has no links (`_StellarNumHyperLinks` is 0): the
    /// original says nothing.
    NoLinks,
    /// No system was picked on the map, or none the hypergate's links lead
    /// to (`STR#` 2002 #50, "Hypergate jump cancelled.").
    Cancelled,
    /// The wormhole leads nowhere (`STR#` 2002 #84 and #86, "the radiation
    /// levels are too extreme").
    NoExit,
}

/// The hypergate map's next pick from `selected`, the map's selection, on
/// Tab or `\`, as `_MapFilter` steps through the `cycle`
/// (`nova_sim_ai::gate::hypergate_cycle`, @0x13db5-0x13f0a): from the first entry equal
/// to `selected` (`None` matching a `None` entry, as −1 matches −1), the
/// one before it with Shift (`back`), the first wrapping to the last, and
/// otherwise the one after it, the last wrapping to the first; from a
/// selection not in the cycle, the first entry either way. So a
/// destination listed twice keeps a plain press on its second copy. With
/// an empty cycle, `None`: nothing changes (@0x13dac).
#[must_use]
pub fn next_gate_pick(
    cycle: &[Option<SystemId>],
    selected: Option<SystemId>,
    back: bool,
) -> Option<Option<SystemId>> {
    let last = cycle.len().checked_sub(1)?;
    let next = match cycle.iter().position(|&entry| entry == selected) {
        None => 0,
        Some(at) if back => at.checked_sub(1).unwrap_or(last),
        Some(at) => (at + 1) % cycle.len(),
    };
    Some(cycle[next])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_gate_pick_steps_on_and_shift_steps_back_each_wrapping() {
        let (a, b) = (Some(SystemId(131)), Some(SystemId(132)));
        let cycle = [a, None, b];
        assert_eq!(next_gate_pick(&cycle, a, false), Some(None));
        assert_eq!(
            next_gate_pick(&cycle, None, false),
            Some(b),
            "-1 is found too"
        );
        assert_eq!(next_gate_pick(&cycle, b, false), Some(a), "wraps");
        assert_eq!(next_gate_pick(&cycle, b, true), Some(None));
        assert_eq!(next_gate_pick(&cycle, a, true), Some(b), "wraps back");
    }

    #[test]
    fn the_next_gate_pick_from_a_selection_not_listed_is_the_first_either_way() {
        let cycle = [Some(SystemId(131)), Some(SystemId(132))];
        for back in [false, true] {
            assert_eq!(
                next_gate_pick(&cycle, Some(SystemId(130)), back),
                Some(Some(SystemId(131))),
                "{back}"
            );
            assert_eq!(
                next_gate_pick(&cycle, None, back),
                Some(Some(SystemId(131))),
                "{back}"
            );
        }
    }

    #[test]
    fn the_next_gate_pick_steps_on_from_the_first_copy_of_a_repeat() {
        let (a, b) = (Some(SystemId(131)), Some(SystemId(132)));
        assert_eq!(
            next_gate_pick(&[a, a, b], a, false),
            Some(a),
            "stuck on 131"
        );
        assert_eq!(next_gate_pick(&[b, a, a], a, true), Some(b));
    }

    #[test]
    fn no_next_gate_pick_from_an_empty_cycle() {
        for back in [false, true] {
            assert_eq!(next_gate_pick(&[], Some(SystemId(131)), back), None);
        }
    }
}
