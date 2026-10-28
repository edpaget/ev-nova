//! Which update the in-flight maps show in item 3: `_MapDialogUpdate`'s
//! full update, or the partial one a drag draws
//! (`_partialMapDialogUpdate`).
//!
//! **The partial update.** `_ScrollMapDisplay(dh, dv)` (@0x1128d-0x11303)
//! does nothing when both are 0 (@0x11294-0x1129c); otherwise it sets the
//! flag (@0x112f3), runs `_MapDialogUpdate` and clears it again
//! (@0x112fb). Each poll of a drag calls it with the pointer's travel
//! since the last poll (`_DoSystemMap` @0x138b5-0x138cc), so a poll the
//! pointer moved on draws a partial update whether or not the truncated
//! scroll changed. A partial update draws item 3 black, the nebulae as
//! plain copies (`srcCopy` @0xfbae) and no borders (@0xf840), then
//! `_DrawMap`, and copies only item 3 to the window (@0x1004c-0x10123).
//! The window outside item 3 keeps what the last full update drew.
//!
//! **The release.** The full update runs on the release only when the
//! scroll differs from the drag's start (@0x13934-0x13948). A drag that
//! ends where it began leaves the partial frame in item 3: the pointer
//! brought back, or a move too small to change the truncated scroll.
//!
//! **The repairs.** The partial frame stays until something next redraws
//! item 3: a direct `_MapDialogUpdate`, or an update event (`_MapFilter`
//! @0x14605-0x14635) for an invalidation of item 3 or the whole dialog.
//! The map's open clears the flag (`_DoSystemMap` @0x12458) and draws the
//! full update (@0x125a5); and so do, each with its address:
//!
//! - a left press outside item 3, `_MapDialogUpdate` before
//!   `_TrackMapButtons` (@0x14558);
//! - a press in item 3 whose hit is accepted (@0x131af-0x13259; a
//!   Shift-click's change again @0x1358f);
//! - Clear Route, by button or C (@0x12cf0, @0x12d9f);
//! - Borders, by button or B (@0x12756, @0x127ae);
//! - a zoom step taken (@0x128d9, @0x12b0c); a refused one invalidates
//!   nothing (@0x12803-0x12816, and its zoom-in twin);
//! - Tab or `\`: the hypergate cycle's pick (@0x13e6f), and the course
//!   cycle always, a pick or none (@0x14032); a hypergate cycle with no
//!   gate to pick invalidates nothing (`jle` @0x13daf);
//! - Space (@0x14215) and each arrow (left and right @0x142fc, up
//!   @0x143aa, down @0x1445a);
//! - Find's close, which invalidates the whole dialog (@0x11e26-0x11e5d);
//! - the borders' fine pass, after its last system (@0x1479a).
//!
//! A press in item 3 that hits nothing starts a drag and redraws nothing.

/// Which update item 3 shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapUpdate {
    /// `_MapDialogUpdate`'s full update: the borders, then the nebulae
    /// copied with `addMax`.
    #[default]
    Full,
    /// The partial update a drag draws: no borders, and the nebulae as
    /// plain copies.
    Partial,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use nova_sim::story::{GalaxyOverlay, OverlaidGalaxy};

    use super::*;
    use crate::draw::Blend;
    use crate::galaxy::buttons::{MapButton, MapButtons};
    use crate::galaxy::catalog::{Galaxy, NebulaEntry, NebulaId, NebulaPicture, SystemId};
    use crate::galaxy::click::PressAnswer;
    use crate::galaxy::dialog::{DIALOG_ORIGIN, draw_frame};
    use crate::galaxy::map::GalaxyMap;
    use crate::galaxy::model::star_system;
    use crate::galaxy::testkit::{FakeCatalog, galaxy};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonStyle;
    use crate::{DrawCommand, DrawList, Input, Key, MouseButton, Point, Screen};

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// The course map in Alpha (128) at (0, 0), looking at it, with text
    /// metrics for Find.
    fn course_map() -> GalaxyMap {
        let mut map =
            GalaxyMap::course(&FakeCatalog::new(galaxy())).with_metrics(Rc::new(MonoMetrics));
        map.show_course(SystemId(128), &[]);
        map.recentre();
        map
    }

    fn drawn(map: &GalaxyMap) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        map.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn left(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(map: &mut GalaxyMap, at: Point) {
        map.input(&left(true, at));
        map.input(&left(false, at));
    }

    fn on(button: MapButton) -> Point {
        MapButtons::new(DIALOG_ORIGIN).rect(button).center()
    }

    /// Alpha, and the alternate Alpha, at item 3's centre.
    const ALPHA: Point = Point::new(450.0, 346.0);

    /// Empty space in item 3, far from every system.
    const EMPTY: Point = Point::new(300.0, 200.0);

    /// Drags `map` from [`EMPTY`] a pixel left, too little for the
    /// scroll to follow at 0.5625, and releases: the partial frame kept.
    fn jiggle(map: &mut GalaxyMap) {
        let scroll = map.place().expect("in flight").scroll();
        map.input(&left(true, EMPTY));
        map.input(&Input::PointerMoved(at(EMPTY.x - 1.0, EMPTY.y)));
        map.input(&left(false, at(EMPTY.x - 1.0, EMPTY.y)));
        assert_eq!(map.place().expect("in flight").scroll(), scroll);
        assert_eq!(map.update(), Some(MapUpdate::Partial), "kept");
    }

    /// The nebulae `list` draws, in order: each one's inner command.
    fn nebulae(list: &[DrawCommand]) -> Vec<&DrawCommand> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Clipped { command, .. }
                    if matches!(
                        **command,
                        DrawCommand::BlendedPicture { .. } | DrawCommand::StretchedPicture { .. }
                    ) =>
                {
                    Some(&**command)
                }
                _ => None,
            })
            .collect()
    }

    fn is_max(command: &DrawCommand) -> bool {
        matches!(
            command,
            DrawCommand::BlendedPicture {
                blend: Blend::Max,
                ..
            }
        )
    }

    fn is_plain(command: &DrawCommand) -> bool {
        matches!(command, DrawCommand::StretchedPicture { .. })
    }

    /// The frame `map` draws under item 3's contents: everything up to
    /// item 3's black.
    fn frame_len(map: &GalaxyMap) -> usize {
        let mut frame = DrawList::new();
        let place = map.place().expect("in flight");
        let state = crate::galaxy::buttons::ButtonState {
            can_zoom_out: place.can_zoom_out(),
            can_zoom_in: place.can_zoom_in(),
            clear_route: map.clear_route_enabled(),
            borders_shown: true,
        };
        draw_frame(
            &mut frame,
            &MapButtons::new(DIALOG_ORIGIN),
            &state,
            &ButtonStyle::STOCK,
            Some(&MonoMetrics),
        );
        frame.len()
    }

    #[test]
    fn the_full_frame_is_item_3_black_then_the_max_nebulae_then_draw_map() {
        let map = course_map();
        let list = drawn(&map);
        let frame = frame_len(&map);
        assert!(
            matches!(&list[frame], DrawCommand::Clipped { command, .. } if is_max(command)),
            "the nebula right after item 3's black: {:?}",
            list[frame]
        );
        assert_eq!(nebulae(&list).len(), 1);
        assert!(
            list[frame + 1..]
                .iter()
                .all(|command| !matches!(command, DrawCommand::Clipped { command, .. } if is_max(command))),
            "no nebula after _DrawMap's lines"
        );
        assert_eq!(map.update(), Some(MapUpdate::Full));
    }

    #[test]
    fn a_drag_draws_plain_copies_with_the_frame_as_it_was() {
        let mut map = course_map();
        let before = drawn(&map);
        let frame = frame_len(&map);
        map.input(&left(true, EMPTY));
        map.input(&Input::PointerMoved(at(EMPTY.x - 40.0, EMPTY.y)));
        assert_eq!(
            map.update(),
            Some(MapUpdate::Partial),
            "what borders skip on"
        );
        let during = drawn(&map);
        assert_eq!(nebulae(&during).len(), 1, "the nebula, drawn partially");
        assert!(nebulae(&during).iter().all(|command| is_plain(command)));
        assert_eq!(
            during[..frame],
            before[..frame],
            "outside item 3, as before"
        );
        map.input(&left(false, at(EMPTY.x - 40.0, EMPTY.y)));
        assert_eq!(map.update(), Some(MapUpdate::Full), "the scroll moved");
        let after = drawn(&map);
        assert_eq!(nebulae(&after).len(), 1, "the nebula, drawn in full");
        assert!(nebulae(&after).iter().all(|command| is_max(command)));
    }

    #[test]
    fn a_drag_ending_where_it_began_keeps_plain_copies_until_an_arrow() {
        let mut map = course_map();
        jiggle(&mut map);
        let kept = drawn(&map);
        assert_eq!(nebulae(&kept).len(), 1, "the nebula, drawn partially");
        assert!(nebulae(&kept).iter().all(|command| is_plain(command)));
        map.input(&press(Key::Left));
        let after = drawn(&map);
        assert_eq!(nebulae(&after).len(), 1, "the nebula, drawn in full");
        assert!(nebulae(&after).iter().all(|command| is_max(command)));
    }

    /// Something done to a map.
    type Repair = fn(&mut GalaxyMap);

    #[test]
    fn each_repair_the_map_makes_itself_restores_the_full_update() {
        let repairs: [(&str, Repair); 15] = [
            ("opened", GalaxyMap::opened),
            ("press outside item 3", |map| {
                click(map, at(700.0, 300.0));
            }),
            ("press on a button", |map| {
                click(map, on(MapButton::Done));
            }),
            ("accepted press", |map| {
                map.input(&left(true, ALPHA));
                map.take_press().expect("a hit");
                map.answer_press(Some(PressAnswer {
                    selected: Some(SystemId(129)),
                    gate_pick: None,
                    clear_route: None,
                }));
            }),
            ("Clear Route answered", GalaxyMap::route_cleared),
            ("Borders", |map| click(map, on(MapButton::Borders))),
            ("- key", |map| {
                map.input(&press(Key::Char('-')));
            }),
            ("= key", |map| {
                map.input(&press(Key::Char('=')));
            }),
            ("- button", |map| click(map, on(MapButton::ZoomOut))),
            ("+ button", |map| click(map, on(MapButton::ZoomIn))),
            ("Space", |map| {
                map.input(&press(Key::Space));
            }),
            ("arrows", |map| {
                for key in [Key::Right, Key::Up, Key::Down] {
                    map.input(&press(key));
                    assert_eq!(map.update(), Some(MapUpdate::Full), "{key:?}");
                }
            }),
            ("Find closed with nothing to find", |map| {
                map.input(&press(Key::Char('f')));
                assert!(map.finding());
                assert_eq!(map.update(), Some(MapUpdate::Partial), "open");
                map.input(&press(Key::Enter));
                assert!(!map.finding());
            }),
            ("Find searched", |map| {
                map.input(&press(Key::Char('f')));
                // The opening key's own text types nothing.
                map.input(&Input::Text('f'));
                map.input(&Input::Text('B'));
                map.input(&press(Key::Enter));
                assert_eq!(map.take_find().as_deref(), Some("B"));
            }),
            ("the owner's redraw", GalaxyMap::redraw),
        ];
        for (name, repair) in repairs {
            let mut map = course_map();
            jiggle(&mut map);
            repair(&mut map);
            assert_eq!(map.update(), Some(MapUpdate::Full), "{name}");
        }
    }

    #[test]
    fn a_refused_zoom_or_a_press_on_nothing_keeps_the_partial_update() {
        let mut map = course_map();
        map.input(&press(Key::Char('=')));
        jiggle(&mut map);
        map.input(&press(Key::Char('=')));
        assert_eq!(map.update(), Some(MapUpdate::Partial), "= at the limit");
        // A click on + at the limit is still a press outside item 3, which
        // redraws before `_TrackMapButtons` (@0x14558).
        map.input(&left(true, EMPTY));
        assert_eq!(map.update(), Some(MapUpdate::Partial), "a press on nothing");
        map.input(&left(false, EMPTY));
        assert_eq!(map.update(), Some(MapUpdate::Partial), "and its release");
        let mut map = course_map();
        jiggle(&mut map);
        map.input(&left(true, ALPHA));
        map.take_press().expect("a hit");
        map.answer_press(None);
        assert_eq!(
            map.update(),
            Some(MapUpdate::Partial),
            "a press accepted nowhere"
        );
    }

    // The viewer and the course map's nebulae.

    /// Nebula `id` at (`left`, 0), 40 × 40, with one 40 × 40 picture.
    fn nebula(id: i16, left: i16) -> NebulaEntry {
        NebulaEntry {
            id: NebulaId(id),
            name: format!("Nebula {id}"),
            left,
            top: 0,
            width: 40,
            height: 40,
            pictures: vec![NebulaPicture {
                id: 9500 + (id - 128) * 7,
                slot: 0,
                width: 40,
                height: 40,
            }],
        }
    }

    /// Alpha's galaxy with nebulae 128, 129 and 130 side by side.
    fn three_nebulae() -> Galaxy {
        Galaxy {
            nebulae: vec![nebula(128, -60), nebula(129, -20), nebula(130, 20)],
            ..galaxy()
        }
    }

    /// The session's galaxy over `galaxy`'s systems: 128 inactive and
    /// explored, 129 active and unexplored, 130 active and explored.
    fn session(galaxy: &Galaxy) -> nova_sim::galaxy_map::GalaxyShown {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_nebula_active(NebulaId(128), Some(false));
        overlay.set_nebula_explored(NebulaId(128), true);
        overlay.set_nebula_explored(NebulaId(130), true);
        nova_sim::galaxy_map::GalaxyShown::of(&OverlaidGalaxy::new(
            galaxy.systems.iter().map(star_system).collect(),
            overlay,
        ))
    }

    fn pictures(list: &[DrawCommand]) -> Vec<i16> {
        nebulae(list)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::BlendedPicture { image, .. }
                | DrawCommand::StretchedPicture { image, .. } => Some(image.id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_viewer_draws_an_inactive_and_an_unexplored_nebula() {
        let galaxy = three_nebulae();
        let mut viewer = GalaxyMap::new(&FakeCatalog::new(galaxy.clone()));
        viewer.show_galaxy(&session(&galaxy));
        assert_eq!(pictures(&drawn(&viewer)), [9500, 9507, 9514]);
        let mut course = GalaxyMap::course(&FakeCatalog::new(galaxy.clone()));
        course.show_course(SystemId(128), &[]);
        course.recentre();
        course.show_galaxy(&session(&galaxy));
        assert_eq!(
            pictures(&drawn(&course)),
            [9514],
            "only the active, explored one"
        );
    }
}
