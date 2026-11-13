//! A left press on an in-flight map ([`MapPress`]) and what an accepted
//! click did ([`MapClicked`]), from `nova_sim::session::map_click`, which
//! re-exports them and keeps the click's rules.

use nova_sim_catalog::catalog::SystemId;

/// A left press in an in-flight map's item 3, as `_DoSystemMap` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapPress<'a> {
    /// The visible systems whose hit box holds the press, by ascending ID.
    pub hits: &'a [SystemId],
    /// Whether Shift was held at the press (`_KeyCheck(0x38)`, @0x13071,
    /// @0x1325e).
    pub shift: bool,
    /// On the hypergate map, the systems the gate offers; `None` on the
    /// course map.
    pub hypergate: Option<&'a [SystemId]>,
    /// The mission systems taken as the map opened (`_missionSyst`,
    /// `nova_sim::Session::map_mission_systems`).
    pub missions: &'a [SystemId],
    /// The green mission system the map's opener marked
    /// (`_greenMissionSyst`, [`MapMarks::green`](crate::galaxy_map::MapMarks::green)),
    /// accepted raw as a mission system is (@0x12f82-0x12f8a); `None` on
    /// every map but a mission dialog's.
    pub green: Option<SystemId>,
}

/// What an accepted click did (`nova_sim::Session::map_click`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapClicked {
    /// The map's new selection (`_selSystem`); `None` leaves it as it is.
    pub selected: Option<SystemId>,
    /// On the hypergate map, its new pick, `Some(None)` for none; `None`
    /// off it.
    pub gate_pick: Option<Option<SystemId>>,
    /// What the click writes to `showClearRouteButton`; `None` leaves it.
    /// By the engine's reading only a hop written sets it, and no click
    /// clears it; by the other, the click writes it outright, on while
    /// the plotted course has a hop.
    pub clear_route: Option<bool>,
    /// Whether the click touched the course, which the map shows again.
    pub course_changed: bool,
    /// Whether the nav area redraws at once (`_DrawStatus`).
    pub nav_redraw: bool,
}
