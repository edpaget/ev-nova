//! The shared `FlightFrame` builders the component tests script their
//! flights with ([`FakeFlight::set_frame`] and the answers' frames), for
//! the states the tests keep spelling out: in flight in Sol, among NPCs
//! and with a target, landed, with a course or a jump under way, arrived
//! in Alpha Centauri; and the galaxy a flight's map answers, built with
//! no session ([`galaxy_of`]).

use super::*;
use nova_sim::flight_frame::{FrameShip, FrameTarget};
use nova_sim::story::{GalaxyOverlay, OverlaidGalaxy};

/// The frame of a flight in Sol (130), nav off, nothing plotted.
pub(super) fn in_sol() -> FlightFrame {
    FlightFrame {
        system: SystemId(130),
        ..FlightFrame::default()
    }
}

/// The frame of an intact flight in Sol, the player of ship 128 alone at
/// its centre.
pub(super) fn sol() -> FlightFrame {
    in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
}

/// A frame in Sol (130) holding `ships`, the player's first, each drawn
/// in the ships' layer in their order but the player's, drawn last, and
/// Sol's two stellars shown.
pub(super) fn in_sol_with(ships: Vec<FrameShip>) -> FlightFrame {
    let mut drawn: Vec<_> = ships
        .iter()
        .filter(|ship| ship.ship != ShipRef::Player)
        .map(|ship| (ship.ship, nova_sim::ShipLayer::Ships))
        .collect();
    drawn.push((ShipRef::Player, nova_sim::ShipLayer::Ships));
    FlightFrame {
        ships,
        drawn,
        stellars: vec![
            record::stellar(StellarId(128)),
            record::stellar(StellarId(129)),
        ],
        ..in_sol()
    }
}

/// The player's ship 128 at `at` facing `heading`.
pub(super) fn the_player_at(at: Vec2, heading: f32) -> FrameShip {
    record::player(128, at, heading)
}

/// NPC `id` of ship 129 at `at` facing `heading`.
pub(super) fn npc_129(id: u32, at: Vec2, heading: f32) -> FrameShip {
    record::npc(id, 129, at, heading)
}

/// A frame in Sol holding the player and `npcs`, with ship 129's traffic,
/// whose target cards are read.
pub(super) fn among(npcs: Vec<FrameShip>) -> FlightFrame {
    let mut ships = vec![the_player_at(Vec2::ZERO, 0.0)];
    ships.extend(npcs);
    FlightFrame {
        traffic_ships: vec![ShipId(129)],
        ..in_sol_with(ships)
    }
}

/// [`among`], with the target as `target` says.
pub(super) fn targeted(npcs: Vec<FrameShip>, target: FrameTarget) -> FlightFrame {
    FlightFrame {
        target: Some(target),
        ..among(npcs)
    }
}

/// A frame in system `system`, the player alone at the centre in ship
/// `class`, `landed` on Earth or not.
pub(super) fn in_system(system: i16, class: i16, landed: bool) -> FlightFrame {
    FlightFrame {
        system: SystemId(system),
        landed: landed.then_some(StellarId(128)),
        ships: vec![record::player(class, Vec2::ZERO, 0.0)],
        ..in_sol()
    }
}

/// [`sol`] docked at `stellar`, the player at `at`.
pub(super) fn docked_at(stellar: i16, at: Vec2) -> FlightFrame {
    FlightFrame {
        landed: Some(StellarId(stellar)),
        ships: vec![the_player_at(at, 0.0)],
        ..sol()
    }
}

/// The nav mode in Sol `mode`.
pub(super) fn sol_in(mode: NavMode) -> FlightFrame {
    FlightFrame {
        nav_mode: mode,
        ..in_sol()
    }
}

/// A flight in Sol with a course to Alpha Centauri (131) plotted, its
/// first hop selected.
pub(super) fn plotted_to_alpha() -> FlightFrame {
    FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        course_shown: vec![SystemId(131)],
        course_plotted: true,
        ..in_sol()
    }
}

/// A flight in Sol with a jump under way, Alpha Centauri (131)
/// selected in mode 3.
pub(super) fn jumping_to_alpha() -> FlightFrame {
    FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        jump_under_way: true,
        ..in_sol()
    }
}

/// [`sol`] with a jump under way, its own warp begun as `warping` says.
pub(super) fn under_way(warping: bool) -> FlightFrame {
    FlightFrame {
        jump_under_way: true,
        warping,
        ..sol()
    }
}

/// The frame Alpha Centauri (131) shows once arrived: the player at its
/// centre, Proxima (140) its one stellar, no jump under way.
pub(super) fn in_alpha() -> FlightFrame {
    FlightFrame {
        system: SystemId(131),
        stellars: vec![record::stellar(StellarId(140))],
        ..sol()
    }
}

/// The galaxy of `catalog`'s systems, as a flight's map answers it
/// (`MapSource::galaxy`), with `change` made to its overlay: built from
/// the catalog's star map with no session.
pub(super) fn galaxy_of(
    catalog: &FakeCatalog,
    change: impl FnOnce(&mut GalaxyOverlay),
) -> OverlaidGalaxy {
    let mut overlay = GalaxyOverlay::default();
    change(&mut overlay);
    OverlaidGalaxy::new(catalog.star_map(), overlay)
}

/// [`galaxy_of`] [`catalog`] with `change` made.
pub(super) fn galaxy_with(change: impl FnOnce(&mut GalaxyOverlay)) -> OverlaidGalaxy {
    galaxy_of(&catalog(), change)
}
