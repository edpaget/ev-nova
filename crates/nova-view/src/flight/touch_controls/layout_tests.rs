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
use crate::view_shape::widened_size;

/// A window size in points and its scale factor: the inputs a fixture
/// derives its view and points per pixel from.
struct Window {
    name: &'static str,
    points: (f32, f32),
    scale: f32,
}

const WINDOWS: [Window; 6] = [
    Window {
        name: "desktop window",
        points: (1024.0, 768.0),
        scale: 1.0,
    },
    Window {
        name: "large desktop window",
        points: (1920.0, 1440.0),
        scale: 1.0,
    },
    Window {
        name: "full iPad",
        points: (1194.0, 834.0),
        scale: 2.0,
    },
    Window {
        name: "small iPad window",
        points: (700.0, 525.0),
        scale: 2.0,
    },
    Window {
        name: "iPad Split View half",
        points: (590.0, 834.0),
        scale: 2.0,
    },
    Window {
        name: "Slide Over",
        points: (320.0, 834.0),
        scale: 2.0,
    },
];

/// The fake safe area's cuts, in points: top, bottom, and each side.
const CUTS: (f32, f32, f32) = (24.0, 20.0, 32.0);

impl Window {
    /// Flight's view in this window: its widened shape
    /// ([`widened_size`] of the window's pixels).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn view(&self) -> (f32, f32) {
        let px = (
            (self.points.0 * self.scale).round() as u32,
            (self.points.1 * self.scale).round() as u32,
        );
        let (w, h) = widened_size(px);
        #[allow(clippy::cast_precision_loss)]
        (w as f32, h as f32)
    }

    /// Points per logical pixel: the window's points over the view, the
    /// smaller way (the view is letterboxed the other).
    fn points_per_pixel(&self) -> f32 {
        let (w, h) = self.view();
        (self.points.0 / w).min(self.points.1 / h)
    }

    /// The surroundings, told by touch, with the fake safe area cut from
    /// the canvas's edges.
    fn surroundings(&self) -> Surroundings {
        let ppp = self.points_per_pixel();
        let (w, h) = self.view();
        let (top, bottom, side) = CUTS;
        Surroundings {
            device: Device::Touch,
            safe_area: Bounds {
                min: Point::new(side / ppp, top / ppp),
                max: Point::new(w - side / ppp, h - bottom / ppp),
            },
            points_per_pixel: ppp,
        }
    }

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
