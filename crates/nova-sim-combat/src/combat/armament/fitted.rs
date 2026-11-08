//! A ship class's raw exit points, as its `shän` gives them, which the
//! new game's hardpoints are laid over ([`spots`](crate::combat::spots)).

use crate::catalog::{ShipExits, ShipId};

use super::Arsenal;

impl Arsenal {
    /// Ship type `ship`'s exit points raw from its `shän`; none for a
    /// class with no `shän`, or no record.
    #[must_use]
    pub fn class_exits(&self, ship: ShipId) -> Option<ShipExits> {
        self.hulls.get(&ship).and_then(|hull| hull.exits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::HullRecord;
    use nova_sim_catalog::fixture::hull;

    #[test]
    fn a_class_s_raw_exits_are_its_record_s() {
        let raw = ShipExits {
            frames: 36,
            ..ShipExits::default()
        };
        let arsenal = Arsenal::new(
            &[],
            vec![
                HullRecord {
                    exits: Some(raw),
                    ..hull(128)
                },
                HullRecord {
                    exits: None,
                    ..hull(129)
                },
            ],
        );
        assert_eq!(arsenal.class_exits(ShipId(128)), Some(raw));
        assert_eq!(arsenal.class_exits(ShipId(129)), None, "no shän");
        assert_eq!(arsenal.class_exits(ShipId(130)), None, "no record");
    }
}
