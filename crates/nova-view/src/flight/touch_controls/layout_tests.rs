//! The layout's rules, at every fixture size: sizes in points, inside the
//! safe area and the play area, no overlaps, the tiers where the module
//! docs say, and every flight command the keyboard reaches reached.

#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use super::*;
use crate::geometry::Point;
use crate::input::{Device, Surroundings};
use crate::keymap::KeyMap;
use crate::keymap::command::{FlightCommand, Kind};
use crate::keymap::contexts::FLIGHT;
use crate::ui::touch_aids::testkit::{WINDOWS, Window};

/// Flight's region in a fixture window.
trait Regions {
    fn region(&self) -> Region;
}

impl Regions for Window {
    fn region(&self) -> Region {
        Region::for_flight(&self.surroundings(), self.view())
    }
}

/// Every layout of `region`: the menu shut, then open on each of its
/// pages, with the escort menu shut and shown.
fn every_layout(region: &Region) -> Vec<(MenuView, Layout)> {
    let mut all = Vec::new();
    for escort_menu in [false, true] {
        let shut = MenuView {
            open: false,
            page: 0,
            escort_menu,
        };
        let pages = layout(region, shut).pages;
        all.push((shut, layout(region, shut)));
        for page in 0..pages {
            let open = MenuView {
                open: true,
                page,
                escort_menu,
            };
            all.push((open, layout(region, open)));
        }
    }
    all
}

/// A control's size in points along each axis.
fn points(placed: &Placed, region: &Region) -> (f32, f32) {
    (
        placed.rect.width() * region.points_per_pixel,
        placed.rect.height() * region.points_per_pixel,
    )
}

/// Tolerance for sizes in points after the trip through logical units.
const EPS: f32 = 1e-3;

fn find(layout: &Layout, id: ControlId) -> Option<&Placed> {
    layout.placed.iter().find(|placed| placed.id == id)
}

fn command(id: impl Into<Command>) -> ControlId {
    ControlId::Command(id.into())
}

#[test]
fn every_control_is_at_least_44_points_across_at_each_size() {
    for window in &WINDOWS {
        let region = window.region();
        for (view, layout) in every_layout(&region) {
            assert!(!layout.placed.is_empty(), "{}: {view:?}", window.name);
            for placed in &layout.placed {
                let (w, h) = points(placed, &region);
                assert!(
                    w >= MIN_PT - EPS && h >= MIN_PT - EPS,
                    "{}: {view:?} {:?} is {w} × {h} pt",
                    window.name,
                    placed.id
                );
            }
        }
    }
}

#[test]
fn fire_is_64_to_88_points_at_each_size() {
    for window in &WINDOWS {
        let region = window.region();
        let shut = layout(&region, MenuView::default());
        for fire in [FlightCommand::FirePrimary, FlightCommand::FireSecondary] {
            if let Some(placed) = find(&shut, command(fire)) {
                let (w, h) = points(placed, &region);
                for side in [w, h] {
                    assert!(
                        (64.0 - EPS..=88.0 + EPS).contains(&side),
                        "{}: {fire:?} is {w} × {h} pt",
                        window.name
                    );
                }
            }
        }
        let open_pages = shut.pages;
        for page in 0..open_pages {
            let open = layout(
                &region,
                MenuView {
                    open: true,
                    page,
                    escort_menu: false,
                },
            );
            assert!(
                find(&open, command(FlightCommand::FirePrimary)).is_none(),
                "fire is never a menu cell's size"
            );
        }
    }
    let desktop = WINDOWS[0].region();
    let shut = layout(&desktop, MenuView::default());
    let primary = find(&shut, command(FlightCommand::FirePrimary)).expect("placed");
    assert_eq!(
        points(primary, &desktop),
        (PRIMARY_FIRE_PT, PRIMARY_FIRE_PT)
    );
    let secondary = find(&shut, command(FlightCommand::FireSecondary)).expect("placed");
    assert_eq!(
        points(secondary, &desktop),
        (SECONDARY_FIRE_PT, SECONDARY_FIRE_PT)
    );
}

#[test]
fn every_control_lies_inside_the_safe_area_and_the_play_area() {
    for window in &WINDOWS {
        let told = window.surroundings();
        let (w, h) = window.view();
        let play = Bounds::at(Point::new(0.0, 0.0), w - STATUS_BAR_WIDTH, h);
        let region = window.region();
        for (view, layout) in every_layout(&region) {
            let rects = layout.placed.iter().map(|p| (p.id, p.rect));
            for (id, rect) in rects.chain(layout.panel.map(|p| (ControlId::More, p))) {
                for corner in [rect.min, rect.max] {
                    assert!(
                        told.safe_area.contains(corner) && play.contains(corner),
                        "{}: {view:?} {id:?} at {rect:?}",
                        window.name
                    );
                }
            }
        }
    }
}

#[test]
fn no_two_controls_overlap() {
    for window in &WINDOWS {
        let region = window.region();
        for (view, layout) in every_layout(&region) {
            for (i, a) in layout.placed.iter().enumerate() {
                for b in &layout.placed[i + 1..] {
                    let overlap = crate::geometry::intersection(a.rect, b.rect);
                    assert!(
                        overlap.is_none(),
                        "{}: {view:?} {:?} and {:?}",
                        window.name,
                        a.id,
                        b.id
                    );
                }
            }
        }
    }
}

#[test]
fn at_full_size_the_tiers_sit_where_the_module_docs_say() {
    let region = WINDOWS[0].region();
    let r = region.rect;
    let shut = layout(&region, MenuView::default());
    assert!(shut.overflow.is_empty(), "{:?}", shut.overflow);
    assert_eq!(shut.pages, 1);
    let primary = find(&shut, command(FlightCommand::FirePrimary)).expect("primary");
    assert_eq!(primary.rect.max, r.max, "in the corner");
    assert!(primary.round);
    let secondary = find(&shut, command(FlightCommand::FireSecondary)).expect("secondary");
    assert!(secondary.rect.max.y < primary.rect.min.y, "above");
    assert_eq!(secondary.rect.center().x, primary.rect.center().x);
    let afterburner = find(&shut, command(FlightCommand::Afterburner)).expect("afterburner");
    assert!(afterburner.rect.max.x < primary.rect.min.x, "left of it");
    assert_eq!(afterburner.rect.max.y, primary.rect.max.y);
    let stick = find(&shut, ControlId::Stick).expect("the stick zone");
    assert_eq!(stick.rect.min.x, r.min.x);
    assert_eq!(stick.rect.max.y, r.max.y);
    let mut top: Vec<&Placed> = shut
        .placed
        .iter()
        .filter(|placed| placed.rect.min.y == r.min.y)
        .collect();
    top.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    let top: Vec<ControlId> = top.into_iter().map(|placed| placed.id).collect();
    let mut along: Vec<ControlId> = EDGE.iter().map(|&c| ControlId::Command(c)).collect();
    along.push(ControlId::More);
    assert_eq!(top, along, "the edge row, then More");
    let more = find(&shut, ControlId::More).expect("More");
    assert_eq!(more.rect.max.x, r.max.x, "More at the top right");
}

#[test]
fn escort_groups_are_placed_only_while_the_escort_menu_shows() {
    for window in &WINDOWS {
        let region = window.region();
        let groups = |escort_menu: bool, open: bool| {
            let view = MenuView {
                open,
                page: 0,
                escort_menu,
            };
            let all = layout(&region, view);
            let pages = all.pages;
            (0..pages.max(1))
                .flat_map(|page| layout(&region, MenuView { page, ..view }).placed)
                .chain(all.placed.clone())
                .filter(|placed| GROUPS.iter().any(|&g| placed.id == ControlId::Command(g)))
                .count()
                + all
                    .overflow
                    .iter()
                    .filter(|command| GROUPS.contains(command))
                    .count()
        };
        assert_eq!(groups(false, false), 0, "{}", window.name);
        assert_eq!(groups(false, true), 0, "{}", window.name);
        assert!(groups(true, false) >= GROUPS.len(), "{}", window.name);
    }
    let desktop = WINDOWS[0].region();
    let shown = layout(
        &desktop,
        MenuView {
            escort_menu: true,
            ..MenuView::default()
        },
    );
    for group in GROUPS {
        let placed = find(&shown, ControlId::Command(group)).expect("a group");
        assert!(
            placed.rect.min.x
                >= crate::flight::escorts::MENU_AT.x + crate::flight::escorts::MENU_WIDTH,
            "beside the escort menu's list"
        );
    }
}

/// Every command the overlay reaches in `region`: the stick's tier while
/// its zone is placed, each placed control, each menu cell on any page,
/// and the escort groups with the escort menu shown.
fn reached(region: &Region) -> BTreeSet<Command> {
    let mut reached = BTreeSet::new();
    for (_, layout) in every_layout(region) {
        for placed in &layout.placed {
            match placed.id {
                ControlId::Command(command) => {
                    reached.insert(command);
                }
                ControlId::Stick => reached.extend(STICK),
                _ => {}
            }
        }
    }
    reached
}

#[test]
fn every_flight_command_the_keyboard_reaches_is_reached_by_the_overlay() {
    let keys = KeyMap::default();
    let wanted: Vec<Command> = FLIGHT
        .entries
        .iter()
        .flat_map(|entry| entry.context.commands())
        .filter(|&command| command.kind() != Kind::Axis)
        .filter(|&command| {
            keys.combo(command).is_some()
                || KeyMap::TWINS.iter().any(|twin| twin.command == command)
        })
        .collect();
    assert!(wanted.len() > 50, "{}", wanted.len());
    for window in &WINDOWS {
        let reached = reached(&window.region());
        for command in &wanted {
            assert!(reached.contains(command), "{}: {command:?}", window.name);
        }
    }
}

#[test]
fn eject_and_self_destruct_are_only_in_the_menu_and_last() {
    for window in &WINDOWS {
        let region = window.region();
        let shut = layout(&region, MenuView::default());
        let menu = shut.menu.clone();
        assert_eq!(
            menu[menu.len() - 2..],
            [
                Command::from(FlightCommand::Eject),
                Command::from(FlightCommand::SelfDestruct)
            ],
            "{}",
            window.name
        );
        for dangerous in [FlightCommand::Eject, FlightCommand::SelfDestruct] {
            assert!(find(&shut, command(dangerous)).is_none(), "{}", window.name);
        }
    }
}

#[test]
fn controls_that_do_not_fit_join_the_front_of_the_menu() {
    let slide_over = WINDOWS[5].region();
    let shut = layout(&slide_over, MenuView::default());
    assert!(!shut.overflow.is_empty());
    assert!(shut.overflow.contains(&FlightCommand::FireSecondary.into()));
    assert!(shut.overflow.contains(&FlightCommand::Afterburner.into()));
    assert!(shut.overflow.iter().any(|command| EDGE.contains(command)));
    assert_eq!(shut.menu[..shut.overflow.len()], shut.overflow[..]);
    for overflowed in &shut.overflow {
        assert!(
            find(&shut, ControlId::Command(*overflowed)).is_none(),
            "{overflowed:?}"
        );
    }
    let desktop = layout(&WINDOWS[0].region(), MenuView::default());
    assert_eq!(desktop.menu, MENU);
}

#[test]
fn the_menu_pages_when_its_cells_do_not_fit_and_every_page_has_its_arrows() {
    let slide_over = WINDOWS[5].region();
    let pages = layout(&slide_over, MenuView::default()).pages;
    assert!(pages > 1, "{pages}");
    let mut seen = Vec::new();
    for page in 0..pages {
        let open = layout(
            &slide_over,
            MenuView {
                open: true,
                page,
                escort_menu: false,
            },
        );
        assert!(find(&open, ControlId::PagePrev).is_some(), "page {page}");
        assert!(find(&open, ControlId::PageNext).is_some(), "page {page}");
        let panel = open.panel.expect("a panel");
        for placed in &open.placed {
            if let ControlId::Command(command) = placed.id
                && panel.contains(placed.rect.center())
            {
                seen.push(command);
            }
        }
    }
    let shut = layout(&slide_over, MenuView::default());
    assert_eq!(seen, shut.menu, "every cell once, in order");
    let desktop = WINDOWS[0].region();
    let open = layout(
        &desktop,
        MenuView {
            open: true,
            page: 0,
            escort_menu: false,
        },
    );
    assert_eq!(open.pages, 1);
    assert!(find(&open, ControlId::PagePrev).is_none());
    assert!(
        find(&open, ControlId::Stick).is_none(),
        "the panel covers it"
    );
    assert!(find(&open, command(FlightCommand::FirePrimary)).is_none());
    assert!(find(&open, ControlId::More).is_some());
}

#[test]
fn a_region_too_small_for_more_places_nothing() {
    let tiny = Region {
        rect: Bounds::at(Point::new(10.0, 10.0), 40.0, 400.0),
        points_per_pixel: 1.0,
    };
    let none = layout(&tiny, MenuView::default());
    assert!(none.placed.is_empty());
    assert_eq!(none.panel, None);
    let squashed = Region {
        rect: Bounds::at(Point::new(10.0, 10.0), 400.0, 60.0),
        points_per_pixel: 1.0,
    };
    assert!(layout(&squashed, MenuView::default()).placed.is_empty());
    let nowhere = Region::for_flight(
        &Surroundings {
            safe_area: Bounds::at(Point::new(900.0, 0.0), 100.0, 768.0),
            ..Surroundings::of(Device::Touch)
        },
        (1024.0, 768.0),
    );
    assert!(layout(&nowhere, MenuView::default()).placed.is_empty());
}

#[test]
fn the_region_is_the_safe_area_within_the_play_area_inset_by_the_margin() {
    let told = Surroundings {
        safe_area: Bounds {
            min: Point::new(40.0, 30.0),
            max: Point::new(1000.0, 700.0),
        },
        points_per_pixel: 2.0,
        ..Surroundings::of(Device::Touch)
    };
    let region = Region::for_flight(&told, (1024.0, 768.0));
    assert_eq!(region.points_per_pixel, 2.0);
    let margin = MARGIN_PT / 2.0;
    assert_eq!(
        region.rect,
        Bounds {
            min: Point::new(40.0 + margin, 30.0 + margin),
            max: Point::new(1024.0 - STATUS_BAR_WIDTH - margin, 700.0 - margin),
        }
    );
}

#[test]
fn hit_finds_the_control_under_a_point() {
    let region = WINDOWS[0].region();
    let shut = layout(&region, MenuView::default());
    for placed in &shut.placed {
        assert_eq!(shut.hit(placed.rect.center()), Some(placed.id));
    }
    assert_eq!(shut.hit(Point::new(-5.0, -5.0)), None);
}

#[test]
fn the_grid_counts_the_cells_a_panel_holds_exactly() {
    let region = Region {
        rect: Bounds::at(Point::new(0.0, 0.0), 1000.0, 1000.0),
        points_per_pixel: 1.0,
    };
    // Three cells and two gaps across, four and three down.
    let panel = Bounds::at(Point::new(0.0, 0.0), 304.0, 200.0);
    let grid = Grid::fit(&region, panel).expect("a grid");
    assert_eq!((grid.cols, grid.rows), (3, 4));
    let short = Bounds::at(Point::new(0.0, 0.0), 303.0, 199.0);
    let grid_short = Grid::fit(&region, short).expect("a grid");
    assert_eq!((grid_short.cols, grid_short.rows), (2, 3));
    assert_eq!(grid.per_page(12), 12, "all fit on one page");
    assert_eq!(grid.per_page(10), 12);
    assert_eq!(grid.per_page(13), 9, "a row for the arrows");
    let wide = Grid::fit(
        &Region {
            points_per_pixel: 2.0,
            ..region
        },
        panel,
    )
    .expect("a grid");
    // Cells of 48 × 22 with gaps of 4: (304 + 4) / 52 and (200 + 4) / 26.
    assert_eq!(
        (wide.cols, wide.rows),
        (5, 7),
        "points per pixel halves a cell"
    );
}

#[test]
fn a_panel_narrower_than_both_page_arrows_holds_no_grid() {
    let region = Region {
        rect: Bounds::at(Point::new(0.0, 0.0), 1000.0, 1000.0),
        points_per_pixel: 1.0,
    };
    let arrows = 2.0 * ARROW_PT + GAP_PT;
    assert!(arrows > CELL_PT.0, "a cell fits where the arrows don't");
    let narrow = Bounds::at(Point::new(0.0, 0.0), arrows - 1.0, 200.0);
    assert!(Grid::fit(&region, narrow).is_none());
    let enough = Bounds::at(Point::new(0.0, 0.0), arrows, 200.0);
    assert!(Grid::fit(&region, enough).is_some());
    let one_row = Bounds::at(Point::new(0.0, 0.0), 400.0, CELL_PT.1 + GAP_PT);
    assert!(
        Grid::fit(&region, one_row).is_none(),
        "no row left for the arrows"
    );
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn the_stick_zone_is_the_left_two_fifths_and_the_bottom_three_fifths_below_the_edge_row() {
    for window in &WINDOWS {
        let region = window.region();
        let r = region.rect;
        let u = |pt: f32| pt / region.points_per_pixel;
        let shut = layout(&region, MenuView::default());
        let Some(zone) = find(&shut, ControlId::Stick) else {
            continue;
        };
        let ring = 2.0 * u(STICK_RADIUS_PT);
        let width = (0.4 * r.width()).max(ring);
        let height = (0.6 * (r.height() - u(EDGE_PT + GAP_PT))).max(ring);
        assert!(near(zone.rect.width(), width), "{}", window.name);
        assert!(near(zone.rect.height(), height), "{}", window.name);
        assert_eq!(zone.rect.min.x, r.min.x, "{}", window.name);
        assert!(near(zone.rect.max.y, r.max.y), "{}", window.name);
    }
    let desktop = WINDOWS[0].region();
    let zone = find(&layout(&desktop, MenuView::default()), ControlId::Stick)
        .expect("placed")
        .rect;
    assert!(
        zone.width() > 2.0 * STICK_RADIUS_PT,
        "two fifths, past the ring"
    );
    let slide_over = WINDOWS[5].region();
    let zone = find(&layout(&slide_over, MenuView::default()), ControlId::Stick)
        .expect("placed")
        .rect;
    let ring = 2.0 * STICK_RADIUS_PT / slide_over.points_per_pixel;
    assert!(
        near(zone.width(), ring) && near(zone.height(), ring),
        "the ring at least"
    );
}

#[test]
fn edge_buttons_step_a_button_and_a_gap_along_a_row_and_a_row_down() {
    for window in &WINDOWS[..4] {
        let region = window.region();
        let u = |pt: f32| pt / region.points_per_pixel;
        let shut = layout(&region, MenuView::default());
        assert!(
            EDGE.iter()
                .all(|&c| find(&shut, ControlId::Command(c)).is_some()),
            "{}: every edge command placed, a second row if need be",
            window.name
        );
        let land = find(&shut, ControlId::Command(EDGE[0])).expect("Land").rect;
        let jump = find(&shut, ControlId::Command(EDGE[1])).expect("Jump").rect;
        assert_eq!(land.min, region.rect.min, "{}", window.name);
        assert!(
            near(jump.min.x - land.min.x, u(EDGE_WIDTH_PT + GAP_PT)),
            "{}",
            window.name
        );
        for &c in &EDGE {
            let rect = find(&shut, ControlId::Command(c)).expect("placed").rect;
            let rows = (rect.min.y - land.min.y) / u(EDGE_PT + GAP_PT);
            assert!(near(rows, rows.round()), "{}: {c:?} on a row", window.name);
        }
    }
    let small = WINDOWS[3].region();
    let shut = layout(&small, MenuView::default());
    let second_row = EDGE
        .iter()
        .filter(|&&c| {
            find(&shut, ControlId::Command(c))
                .expect("placed")
                .rect
                .min
                .y
                > small.rect.min.y
        })
        .count();
    assert!(second_row >= 3, "{second_row} on the second row");
}

#[test]
fn the_escort_groups_start_a_gap_past_the_escort_menu_and_below_the_edge_row() {
    let desktop = WINDOWS[0].region();
    let shown = MenuView {
        escort_menu: true,
        ..MenuView::default()
    };
    let shut = layout(&desktop, shown);
    let rect = |layout: &Layout, n: usize| {
        find(layout, ControlId::Command(GROUPS[n]))
            .expect("a group")
            .rect
    };
    let edge_bottom = find(&shut, ControlId::Command(EDGE[0]))
        .expect("Land")
        .rect
        .max
        .y;
    let first = rect(&shut, 0);
    assert_eq!(
        first.min,
        Point::new(
            crate::flight::escorts::MENU_AT.x + crate::flight::escorts::MENU_WIDTH + GAP_PT,
            edge_bottom + GAP_PT
        )
    );
    assert!(near(rect(&shut, 1).min.y - first.min.y, EDGE_PT + GAP_PT));
    assert_eq!(rect(&shut, 1).min.x, first.min.x, "one column");
    let split = WINDOWS[4].region();
    let narrow = layout(&split, shown);
    let u = |pt: f32| pt / split.points_per_pixel;
    let (a, b) = (rect(&narrow, 0), rect(&narrow, 1));
    assert!(
        near(b.min.x - a.min.x, u(EDGE_WIDTH_PT + GAP_PT)),
        "the next column"
    );
    assert_eq!(b.min.y, a.min.y, "its top");
}

#[test]
fn the_open_menu_keeps_more_and_the_first_edge_row_where_they_were() {
    let desktop = WINDOWS[0].region();
    let shut = layout(&desktop, MenuView::default());
    let open = layout(
        &desktop,
        MenuView {
            open: true,
            ..MenuView::default()
        },
    );
    for id in EDGE
        .iter()
        .map(|&c| ControlId::Command(c))
        .chain([ControlId::More])
    {
        assert_eq!(find(&open, id), find(&shut, id), "{id:?}");
    }
}

#[test]
fn the_page_arrows_sit_at_the_panels_two_ends() {
    let slide_over = WINDOWS[5].region();
    let open = layout(
        &slide_over,
        MenuView {
            open: true,
            ..MenuView::default()
        },
    );
    let panel = open.panel.expect("open");
    let prev = find(&open, ControlId::PagePrev).expect("back").rect;
    let next = find(&open, ControlId::PageNext).expect("on").rect;
    assert_eq!(prev.min.x, panel.min.x);
    assert!(near(next.max.x, panel.max.x));
    assert!(near(next.width(), ARROW_PT / slide_over.points_per_pixel));
}

#[test]
fn a_page_past_the_last_shows_the_last() {
    let slide_over = WINDOWS[5].region();
    let pages = layout(&slide_over, MenuView::default()).pages;
    let at = |page: usize| {
        layout(
            &slide_over,
            MenuView {
                open: true,
                page,
                escort_menu: false,
            },
        )
    };
    assert_eq!(at(pages + 5), at(pages - 1));
    assert_eq!(at(pages), at(pages - 1));
}
