//! The player's system's background colour, which the view erases the
//! flight message's box in (`_DisplayComm` @0x902cf, through
//! `_SetSystemBackgroundBackColor` @0x6e06-0x6e46).

use super::{Chart, Session};
use crate::pilot::Pilot;

impl Session {
    /// The player's system's `BkgndColor`, raw `00RRGGBB`, as
    /// `_SetSystemBackgroundBackColor` reads it (@0x6e06-0x6e46): `None`
    /// when the system is not on the map, where the original, with no
    /// system, uses black (`BackColor` 33 @0x6e1c). The view narrows it
    /// to the display's RGB555.
    #[must_use]
    pub fn system_background(&self) -> Option<u32> {
        self.chart.system_background(&self.pilot)
    }
}

impl Chart {
    /// The `BkgndColor` of `pilot`'s system, raw: `None` off the map.
    fn system_background(&self, pilot: &Pilot) -> Option<u32> {
        self.galaxy.map().background(pilot.system)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StarSystem, SystemId};
    use crate::testkit::{FakePilotCatalog, catalog};

    /// The testkit catalog, system 130 of `BkgndColor` 0x00123456 and the
    /// others black.
    fn coloured() -> FakePilotCatalog {
        let base = catalog();
        FakePilotCatalog {
            star_map: base
                .star_map
                .iter()
                .map(|system| StarSystem {
                    background: if system.id == SystemId(130) {
                        0x0012_3456
                    } else {
                        0
                    },
                    ..system.clone()
                })
                .collect(),
            ..base
        }
    }

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "Ada").expect("starts")).expect("flies")
    }

    #[test]
    fn the_background_is_the_players_systems_raw() {
        let mut session = session(&coloured());
        assert_eq!(session.pilot.system, SystemId(130));
        assert_eq!(session.system_background(), Some(0x0012_3456));
        let other = session
            .chart
            .galaxy
            .map()
            .systems()
            .find(|&id| id != SystemId(130))
            .expect("another system");
        session.pilot.system = other;
        assert_eq!(session.system_background(), Some(0), "black");
    }

    #[test]
    fn a_system_off_the_map_has_none() {
        let mut session = session(&coloured());
        session.pilot.system = SystemId(9999);
        assert_eq!(session.system_background(), None);
    }
}
