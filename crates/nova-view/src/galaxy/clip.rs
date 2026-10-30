//! The in-flight maps' clip to item 3: `_MapDialogUpdate` draws the map
//! into its work area and copies only item 3's rect to the window
//! (`CopyBits` @0x100a2-0x10123), so nothing the map draws shows outside
//! item 3.
//!
//! [`clip_opaque`] cuts a draw list to a rectangle and draws what is left
//! as it is, colours and blacks alike. The floating map's
//! `clip_and_fade` (`crate::flight::floating`) can't be used here: it
//! drops pictures and draws no black, as its frame ORs the map in.
//!
//! The trade-off, as the floating map's: a line that is not horizontal is
//! cut along its centre line, so a pen-2 or pen-3 line crossing item 3's
//! edge can show up to half its width past it.

use crate::draw::{DrawCommand, DrawList, fill_rect};
use crate::geometry::{Bounds, Point, intersection, lerp, span_in};

/// Draws `map` onto `out`, clipped to `area`:
///
/// - each horizontal line, the rect [`fill_rect`] fills, cut to its
///   overlap with `area`, and dropped when it misses;
/// - each other line cut where its centre line crosses `area`'s edges,
///   and dropped when it misses;
/// - each dot kept whole when inside, cut to the rect of its overlap when
///   partly inside, and dropped when outside;
/// - each text, sprite, picture (blended or not) and OR fill wrapped in
///   [`DrawCommand::Clipped`] with `area`, which the renderer crops;
/// - each command already clipped re-clipped to the two clips' overlap,
///   and dropped when they share none.
pub fn clip_opaque(map: &DrawList, area: Bounds, out: &mut DrawList) {
    for command in map {
        match command {
            &DrawCommand::Line {
                from,
                to,
                width,
                color,
            } => {
                // Exactly horizontal, as `fill_rect` draws a rect.
                #[allow(clippy::float_cmp)]
                let horizontal = from.y == to.y && from.x != to.x;
                if horizontal {
                    let half = width / 2.0;
                    let whole = Bounds {
                        min: Point::new(from.x.min(to.x), from.y - half),
                        max: Point::new(from.x.max(to.x), from.y + half),
                    };
                    if let Some(kept) = intersection(whole, area) {
                        if kept == whole {
                            out.push(command.clone());
                        } else {
                            fill_rect(out, kept, color);
                        }
                    }
                } else if let Some((t0, t1)) = span_in(from, to, area) {
                    out.line(lerp(from, to, t0), lerp(from, to, t1), width, color);
                }
            }
            &DrawCommand::Dot {
                center,
                size,
                color,
            } => {
                let half = size / 2.0;
                let whole = Bounds {
                    min: Point::new(center.x - half, center.y - half),
                    max: Point::new(center.x + half, center.y + half),
                };
                if let Some(kept) = intersection(whole, area) {
                    if kept == whole {
                        out.push(command.clone());
                    } else {
                        fill_rect(out, kept, color);
                    }
                }
            }
            DrawCommand::Clipped { clip, command } => {
                if let Some(both) = intersection(*clip, area) {
                    out.clipped(both, (**command).clone());
                }
            }
            DrawCommand::Text { .. }
            | DrawCommand::Sprite { .. }
            | DrawCommand::Colorized { .. }
            | DrawCommand::Picture { .. }
            | DrawCommand::StretchedPicture { .. }
            | DrawCommand::BlendedPicture { .. }
            | DrawCommand::TiledPicture { .. }
            | DrawCommand::OrFill { .. } => {
                out.clipped(area, command.clone());
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::{Color, Font, ImageKey};

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// Item 3 on the screen: (221, 136)-(679, 556).
    fn area() -> Bounds {
        Bounds {
            min: at(221.0, 136.0),
            max: at(679.0, 556.0),
        }
    }

    fn clipped(build: impl FnOnce(&mut DrawList)) -> Vec<DrawCommand> {
        let mut map = DrawList::new();
        build(&mut map);
        let mut out = DrawList::new();
        clip_opaque(&map, area(), &mut out);
        out.iter().cloned().collect()
    }

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: at(left, top),
            max: at(right, bottom),
        }
    }

    fn filled(area: Bounds, color: Color) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        fill_rect(&mut list, area, color);
        list.iter().cloned().collect()
    }

    #[test]
    fn a_rect_straddling_the_area_is_cut_to_the_overlap() {
        let drawn = clipped(|map| fill_rect(map, rect(200.0, 130.0, 240.0, 150.0), Color::WHITE));
        assert_eq!(
            drawn,
            filled(rect(221.0, 136.0, 240.0, 150.0), Color::WHITE)
        );
        let drawn = clipped(|map| fill_rect(map, rect(670.0, 550.0, 700.0, 560.0), Color::BLACK));
        assert_eq!(
            drawn,
            filled(rect(670.0, 550.0, 679.0, 556.0), Color::BLACK),
            "black is drawn too"
        );
    }

    #[test]
    fn a_rect_drawn_right_to_left_is_cut_the_same() {
        let drawn = clipped(|map| {
            map.line(at(240.0, 140.0), at(200.0, 140.0), 4.0, Color::WHITE);
        });
        assert_eq!(
            drawn,
            filled(rect(221.0, 138.0, 240.0, 142.0), Color::WHITE)
        );
    }

    #[test]
    fn a_diagonal_line_crossing_an_edge_is_cut_at_it() {
        let drawn = clipped(|map| {
            map.line(at(201.0, 200.0), at(241.0, 240.0), 2.0, Color::WHITE);
        });
        assert_eq!(
            drawn,
            [DrawCommand::Line {
                from: at(221.0, 220.0),
                to: at(241.0, 240.0),
                width: 2.0,
                color: Color::WHITE,
            }]
        );
        let drawn = clipped(|map| {
            map.line(at(300.0, 500.0), at(300.0, 600.0), 1.0, Color::WHITE);
        });
        assert_eq!(
            drawn,
            [DrawCommand::Line {
                from: at(300.0, 500.0),
                to: at(300.0, 556.0),
                width: 1.0,
                color: Color::WHITE,
            }],
            "a vertical line, along its centre"
        );
    }

    #[test]
    fn lines_rects_and_dots_wholly_outside_are_dropped() {
        let drawn = clipped(|map| {
            map.line(at(10.0, 10.0), at(100.0, 60.0), 2.0, Color::WHITE);
            fill_rect(map, rect(700.0, 200.0, 800.0, 210.0), Color::WHITE);
            fill_rect(map, rect(300.0, 556.0, 400.0, 600.0), Color::WHITE);
            map.dot(at(100.0, 100.0), 2.0, Color::WHITE);
        });
        assert_eq!(drawn, []);
    }

    #[test]
    fn text_sprites_and_pictures_are_clipped_by_the_renderer() {
        let text = DrawCommand::Text {
            text: "Sol".to_owned(),
            font: Font::Geneva,
            origin: at(670.0, 300.0),
            size: 9.0,
            wrap_width: None,
            color: Color::WHITE,
        };
        let sprite = DrawCommand::Sprite {
            image: ImageKey::icon(15000),
            center: at(225.0, 140.0),
            tint: Color::WHITE,
            blend: crate::draw::Blend::Normal,
        };
        let picture = DrawCommand::StretchedPicture {
            image: ImageKey::picture(9500),
            top_left: at(0.0, 0.0),
            width: 1000.0,
            height: 1000.0,
        };
        let plain = DrawCommand::Picture {
            image: ImageKey::picture(9501),
            top_left: at(5000.0, 0.0),
        };
        let nebula = DrawCommand::BlendedPicture {
            image: ImageKey::picture(9502),
            top_left: at(200.0, 100.0),
            width: 300.0,
            height: 300.0,
            blend: crate::draw::Blend::Max,
        };
        let drawn = clipped(|map| {
            for command in [&text, &sprite, &picture, &plain, &nebula] {
                map.push(command.clone());
            }
        });
        let wrapped: Vec<DrawCommand> = [text, sprite, picture, plain, nebula]
            .into_iter()
            .map(|command| DrawCommand::Clipped {
                clip: area(),
                command: Box::new(command),
            })
            .collect();
        assert_eq!(drawn, wrapped);
    }

    #[test]
    fn a_clipped_command_is_clipped_to_both() {
        let text = DrawCommand::Text {
            text: "Sol".to_owned(),
            font: Font::Geneva,
            origin: at(600.0, 300.0),
            size: 9.0,
            wrap_width: None,
            color: Color::WHITE,
        };
        let drawn = clipped(|map| {
            map.clipped(rect(600.0, 0.0, 800.0, 200.0), text.clone());
            map.clipped(rect(0.0, 0.0, 100.0, 100.0), text.clone());
        });
        assert_eq!(
            drawn,
            [DrawCommand::Clipped {
                clip: rect(600.0, 136.0, 679.0, 200.0),
                command: Box::new(text),
            }],
            "the second shares nothing with the area"
        );
    }

    #[test]
    fn a_dot_on_the_edge_is_cut() {
        let drawn = clipped(|map| {
            map.dot(at(221.0, 300.0), 4.0, Color::WHITE);
        });
        assert_eq!(
            drawn,
            filled(rect(221.0, 298.0, 223.0, 302.0), Color::WHITE)
        );
    }

    #[test]
    fn a_dot_over_the_right_edge_keeps_its_left_part() {
        let drawn = clipped(|map| {
            map.dot(at(678.0, 300.0), 4.0, Color::WHITE);
        });
        assert_eq!(
            drawn,
            filled(rect(676.0, 298.0, 679.0, 302.0), Color::WHITE)
        );
    }

    #[test]
    fn a_line_along_an_edge_is_kept_as_the_edges_are_in() {
        let drawn = clipped(|map| {
            map.line(at(221.0, 200.0), at(221.0, 300.0), 1.0, Color::WHITE);
            map.line(at(400.0, 600.0), at(400.0, 556.0), 1.0, Color::WHITE);
        });
        assert_eq!(
            drawn,
            [DrawCommand::Line {
                from: at(221.0, 200.0),
                to: at(221.0, 300.0),
                width: 1.0,
                color: Color::WHITE,
            }],
            "the second only touches the bottom edge at a point"
        );
    }

    #[test]
    fn commands_inside_pass_unchanged() {
        let mut map = DrawList::new();
        fill_rect(&mut map, rect(300.0, 300.0, 310.0, 302.0), Color::WHITE);
        map.line(at(300.0, 300.0), at(320.0, 340.0), 2.0, Color::WHITE);
        map.dot(at(400.0, 400.0), 2.0, Color::BLACK);
        let mut out = DrawList::new();
        clip_opaque(&map, area(), &mut out);
        assert_eq!(out, map);
    }
}
