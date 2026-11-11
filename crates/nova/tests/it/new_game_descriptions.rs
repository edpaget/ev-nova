//! The new game's descriptions, names and About text through the app,
//! over the checked-in fixture (`nova_data::new_game::fixture::folder`)
//! with no interface file, wired as `main` wires it
//! (`new_game_port::app_over`), so its dialogs are the placeholders:
//!
//! - **Shipyard.** At Hearth Dock the cutter's cell shows its body in the
//!   description item, and the grid's cells name the cutter and the racked
//!   hauler.
//! - **Outfitter.** For a pilot owning the turret ring, so the mass
//!   driver, a turret, is listed, the mass driver's cell and the cargo
//!   racks' cell each show their body. (Not the gun mount: it is a
//!   lawless kit, and Hearth Dock's scan denies the landing.)
//! - **Hail.** The comm dialog's class line names the hailed hull.
//! - **About.** Over the fixture the menu bar's About is offered in flight
//!   and shows `game.md`'s body. Over a copy without `game.md`, pressing
//!   I or A on the main menu, where each is About, opens nothing and
//!   warns of nothing (each [`Harness::frame`] asserts it), and in flight
//!   the menu bar's About is greyed and picking it shows nothing.
//!
//! The mapping is pinned in nova-data (`new_game::descriptions`, its
//! fixture tests), the screens in nova-view, and About's greying in
//! `nova::app::screen`; this pins that they meet.

use std::fs;
use std::path::Path;

use nova::app::{MenuPick, Showing, WindowEvent};
use nova_data::new_game::fixture::folder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{OutfitId, Service, ShipId};
use nova_view::draw::{DrawCommand, DrawList};

use nova_view::spaceport::layout::service_item;
use nova_view::ui::comm::CLASS_LABEL;
use nova_view::{MenuCommand, Screen};
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{land_at, new_pilot_flying, opened_from};
use crate::new_game_port::{app_over, click_port_item, on_the_dock_job, resumed_after, spaceport};
use crate::new_game_saved_ids::copy;
use crate::new_game_travel::{fixture, nova_of};
use crate::pilots::{Harness, texts};

/// The texts `screen` draws, each with where it starts.
fn drawn_texts(screen: &dyn Screen) -> Vec<(String, nova_view::geometry::Point)> {
    let mut list = DrawList::new();
    screen.draw(&mut list);
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
            _ => None,
        })
        .collect()
}

/// Whether the next frame draws `text`, a run's words joined as they wrap.
fn frame_draws(game: &mut Harness, text: &str) -> bool {
    texts(&game.frame()).join(" ").contains(text)
}

/// A pilot on the dock job landed at Hearth Dock, owning `kits` (by
/// their bible ids), with `service` open.
fn at_hearth_dock(kits: &[&str], service: Service) -> Harness {
    let owned: Vec<OutfitId> = (kits.iter())
        .map(|kit| OutfitId(nova_of(&fixture().ids.kits, kit)))
        .collect();
    let (mut game, _store) = resumed_after(|pilot| {
        on_the_dock_job(pilot);
        for kit in owned {
            pilot.outfits.insert(kit, 1);
        }
    });
    land_at(&mut game, "hearth-dock");
    click_port_item(&mut game, service_item(service));
    game.frame();
    game
}

#[test]
fn the_shipyard_shows_the_cutter_s_body_and_names_its_hulls_in_their_cells() {
    let mut game = at_hearth_dock(&[], Service::Shipyard);
    let ship = |id: &str| ShipId(nova_of(&fixture().ids.ships, id));
    let shipyard = spaceport(&game).open_shipyard().expect("the shipyard");
    assert_eq!(shipyard.problem(), None, "laid out");
    let cell_of = |id: ShipId| {
        let index = (shipyard.shipyard().rows.iter())
            .position(|row| row.id == id)
            .unwrap_or_else(|| panic!("{id:?} listed"));
        shipyard.cell_bounds(index).expect("a cell")
    };
    let drawn = drawn_texts(shipyard);
    for (id, name) in [("cutter", "Cutter"), ("racked-hauler", "Racked hauler")] {
        let cell = cell_of(ship(id));
        let in_cell: Vec<&str> = (drawn.iter())
            .filter(|(_, at)| cell.contains(*at))
            .map(|(text, _)| text.as_str())
            .collect();
        assert_eq!(in_cell.join(" "), name, "{id}'s cell: {drawn:?}");
    }
    let cutter = cell_of(ship("cutter")).center();
    game.click(cutter);
    assert!(frame_draws(&mut game, "A patrol corvette."));
}

/// Clicks the open outfitter's cell for `outfit`.
fn click_outfit(game: &mut Harness, outfit: OutfitId) {
    let outfitter = spaceport(game).open_outfitter().expect("the outfitter");
    assert_eq!(outfitter.problem(), None, "laid out");
    let index = (outfitter.outfitter().rows.iter())
        .position(|row| row.id == outfit)
        .unwrap_or_else(|| panic!("{outfit:?} listed"));
    let cell = outfitter.cell_bounds(index).expect("a cell").center();
    game.click(cell);
}

#[test]
fn the_outfitter_shows_the_mass_driver_s_and_the_cargo_racks_bodies() {
    let mut game = at_hearth_dock(&["turret-ring"], Service::Outfitter);
    let ids = fixture().ids;
    click_outfit(&mut game, OutfitId(nova_of(&ids.outfits, "mass-driver")));
    assert!(frame_draws(&mut game, "A slug thrower on a turret ring."));
    click_outfit(&mut game, OutfitId(nova_of(&ids.kits, "cargo-racks")));
    assert!(frame_draws(&mut game, "Extra racks bolted into the hold."));
}

/// A new pilot, Ada, flying in the start system of the folder `dir`.
fn flying_over(dir: &Path) -> Harness {
    new_pilot_flying(app_over(opened_from(dir), &MemoryPilots::new()))
}

#[test]
fn a_hail_s_class_line_names_the_hailed_hull() {
    let mut game = flying_over(&folder());
    game.press(KeyCode::Tab);
    game.press(KeyCode::KeyY);
    assert_eq!(game.showing(), Showing::Comm);
    let drawn = drawn_texts(game.app.screen().comm().expect("the comm dialog"));
    let class = (drawn.iter())
        .position(|(text, _)| text == CLASS_LABEL)
        .and_then(|label| drawn.get(label + 1))
        .map(|(text, _)| text.as_str());
    // Hearth's traffic flies the one civilian hull.
    assert_eq!(class, Some("Hauler"), "{drawn:?}");
}

fn about() -> MenuPick {
    MenuPick::Command(MenuCommand::About)
}

#[test]
fn about_shows_game_md_s_body_in_flight() {
    let mut game = flying_over(&folder());
    assert!(game.app.menu_enabled(about()), "offered");
    game.handle(WindowEvent::Menu(MenuCommand::About));
    assert_eq!(game.showing(), Showing::About);
    assert!(frame_draws(&mut game, "A small galaxy for ev-nova's tests"));
}

#[test]
fn over_data_with_no_game_md_about_is_greyed_and_shows_nothing_or_warns() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let copied = dir.path().join("new-game");
    copy(&folder(), &copied);
    fs::remove_file(copied.join("game.md")).expect("removes game.md");
    // On the main menu I is About (`MainScreenCommand::About`) and so is
    // A (`MainMenuCommand::About`, after its button's click); in flight I
    // is mission info, so the keys are pressed here.
    let mut game = app_over(opened_from(&copied), &MemoryPilots::new());
    game.frame();
    assert_eq!(game.showing(), Showing::MainMenu);
    for key in [KeyCode::KeyI, KeyCode::KeyA] {
        game.press(key);
        for _ in 0..30 {
            game.frame();
        }
        assert_eq!(game.showing(), Showing::MainMenu, "{key:?}");
    }
    let mut game = new_pilot_flying(game);
    assert!(!game.app.menu_enabled(about()), "greyed");
    game.handle(WindowEvent::Menu(MenuCommand::About));
    game.frame();
    assert_eq!(game.showing(), Showing::Flight, "picked");
}
