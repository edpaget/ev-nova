//! The galaxy map's two info panels, laid out as `_MapDialogUpdate`
//! draws them (@0x10136-0x111bf): what they say is the session's
//! ([`MapPanel`], `nova_sim`'s [`map_panel`](nova_sim::map_panel)
//! decides it); this module puts each line at its DITL 2000 offset.
//!
//! Every line is a baseline in Geneva 9 (`_smallTextFont`,
//! `_smallTextSize`), set through the HUD's `Pen`. Labels are
//! [`LABEL`] (`_rgbLightGrey`), values white (`ForeColor` 30), the date
//! [`DATE`] (`_rgbDarkGrey`), and each item's top or bottom rule black.
//!
//! **The bottom strip**, [`INFO_STRIP`] (item 2):
//!
//! - a black rule from (item 6's left + 1, top) to (item 6's right − 2,
//!   top) (@0x101e5-0x1020e);
//! - the hypergate prompt at (left + right / 2 − width / 2, top + 24),
//!   each half truncated toward zero (@0x10213-0x102cd): left + right / 2,
//!   not (left + right) / 2, so it sits 4 px right of centre, as the
//!   engine reads it;
//! - or "Ports:" at (left + 10, top + 12) and the list from (left + 45,
//!   top + 12), ", " after each name but the last. Before a name of width
//!   w, when left + w + 45 + x > item 6's right − 20 the pen moves to
//!   (left + 45, top + 24) and x becomes 0, otherwise x grows by w; each
//!   ", " adds its width (@0x1044b-0x10497). So a wrapped name's own width
//!   is never counted, and every wrap goes back to the start of the
//!   second line, as the engine reads it; no stock system wraps;
//! - "Navigation Hazards:" at (left + 10, top + 36), its value at
//!   (left + 110, top + 36) (@0x10586-0x109aa);
//! - the date with its right edge at right − 5, on top + 36
//!   (@0x109af-0x10a6f).
//!
//! **The right column**, [`INFO_COLUMN`] (item 6):
//!
//! - a black rule from (left + 1, bottom − 1) to (right − 2, bottom − 1)
//!   (@0x10aed-0x10b1a);
//! - the title at (left + 10, top + 12), the name at (left + 15,
//!   top + 26);
//! - "Uninhabited System" at (left + 10, top + 75); or the labels
//!   Government, Legal Status, Goods Traded and Services at (left + 10,
//!   top + 75 / 119 / 155 / 250), their values at (left + 15, top + 88 /
//!   132 / 168 / 263), a list's rows 12 apart, and "None Available", or
//!   the `<Unknown>` of a system with no service, at top + 262.
//!
//! Without text metrics nothing is measured: the Ports list is one line,
//! the prompt starts at left + right / 2 and the date at right − 5.
//!
//! Until the map is laid out as DITL 2000 (galaxy-map-fidelity phase 5),
//! the in-flight maps draw both items from [`PANEL_ORIGIN`], each over a
//! [`PANEL`] fill.

use nova_sim::map_panel::{ColumnBody, Listing, MapPanel, PanelWords, Ports, Services, Strip};

use super::map::{DARK_GREY, LIGHT_GREY, PANEL};
use super::view::Bounds;
use crate::draw::fill_rect;
use crate::flight::pen::Pen;
use crate::flight::radar::IRect;
use crate::text::TextMetrics;
use crate::{Color, DrawList, Font, Point};

/// DITL 2000 item 2, the bottom strip, in dialog coordinates.
pub const INFO_STRIP: IRect = IRect::new(8, 436, 594, 478);
/// DITL 2000 item 6, the right column, in dialog coordinates.
pub const INFO_COLUMN: IRect = IRect::new(474, 8, 594, 437);
/// Where the in-flight maps put the dialog's top-left corner until the
/// map is laid out as the dialog (galaxy-map-fidelity phase 5): the strip
/// at (430, 616)-(1016, 658) under the map, the column at
/// (896, 188)-(1016, 617) over its right edge, meeting at y 616 as in the
/// dialog.
pub const PANEL_ORIGIN: Point = Point::new(422.0, 180.0);

/// The labels' and the prompt's colour, `_rgbLightGrey`.
pub const LABEL: Color = LIGHT_GREY;
/// The date's colour, `_rgbDarkGrey`.
pub const DATE: Color = DARK_GREY;
/// The font every line is set in, `_smallTextFont`.
pub const PANEL_FONT: Font = Font::Geneva;
/// The size every line is set at, `_smallTextSize`.
pub const PANEL_SIZE: f32 = 9.0;
/// How far apart a list's rows are.
const ROW: f32 = 12.0;

/// `rect` moved by `origin`, as screen bounds.
#[allow(clippy::cast_precision_loss)] // Dialog pixels.
#[must_use]
pub fn placed(rect: IRect, origin: Point) -> Bounds {
    Bounds {
        min: Point::new(origin.x + rect.left as f32, origin.y + rect.top as f32),
        max: Point::new(origin.x + rect.right as f32, origin.y + rect.bottom as f32),
    }
}

/// Draws `panel`'s two items with the dialog's top-left corner at
/// `origin`, each over a [`PANEL`] fill, measuring with `metrics`, if any
/// (see the module docs).
pub fn draw_panel(
    list: &mut DrawList,
    panel: &MapPanel,
    origin: Point,
    metrics: Option<&dyn TextMetrics>,
) {
    let strip = placed(INFO_STRIP, origin);
    let column = placed(INFO_COLUMN, origin);
    fill_rect(list, strip, PANEL);
    fill_rect(list, column, PANEL);
    let mut pen = Pen {
        list,
        font: PANEL_FONT,
        metrics,
    };
    draw_strip(&mut pen, &panel.strip, &panel.words, strip, column);
    draw_column(
        &mut pen,
        &panel.column.title,
        &panel.column.body,
        &panel.words,
        column,
    );
}

/// A one-pixel black rule from `from` to `to`, as a pen-1 `LineTo` covers
/// it.
fn rule(list: &mut DrawList, from: Point, to: Point) {
    let area = Bounds {
        min: from,
        max: Point::new(to.x + 1.0, to.y + 1.0),
    };
    fill_rect(list, area, Color::BLACK);
}

/// The bottom strip in `strip`, item 6 at `column` (see the module docs).
#[allow(clippy::cast_precision_loss)] // Dialog pixels.
fn draw_strip(pen: &mut Pen, panel: &Strip, words: &PanelWords, strip: Bounds, column: Bounds) {
    let (left, top, right) = (strip.min.x, strip.min.y, strip.max.x);
    rule(
        pen.list,
        Point::new(column.min.x + 1.0, top),
        Point::new(column.max.x - 2.0, top),
    );
    let date = match panel {
        Strip::HypergatePrompt { prompt, date } => {
            // left + right / 2 in the dialog's coordinates, truncated as
            // the original's integer halving is, then onto the screen.
            let dialog = strip.min.x - INFO_STRIP.left as f32;
            let centre = dialog + (INFO_STRIP.left + INFO_STRIP.right / 2) as f32;
            let x = centre - (pen.width(prompt, PANEL_SIZE) / 2.0).trunc();
            pen.at(prompt, x, top + 24.0, PANEL_SIZE, LABEL);
            date
        }
        Strip::Info {
            ports,
            hazards,
            date,
        } => {
            pen.at(&words.ports, left + 10.0, top + 12.0, PANEL_SIZE, LABEL);
            draw_ports(pen, ports, words, strip, column);
            pen.at(&words.hazards, left + 10.0, top + 36.0, PANEL_SIZE, LABEL);
            pen.at(hazards, left + 110.0, top + 36.0, PANEL_SIZE, Color::WHITE);
            date
        }
    };
    let x = right - pen.width(date, PANEL_SIZE) - 5.0;
    pen.at(date, x, top + 36.0, PANEL_SIZE, DATE);
}

/// The Ports list (see the module docs' wrap).
fn draw_ports(pen: &mut Pen, ports: &Ports, words: &PanelWords, strip: Bounds, column: Bounds) {
    let (left, top) = (strip.min.x, strip.min.y);
    let start = left + 45.0;
    let names = match ports {
        Ports::Listed(names) => names,
        Ports::None => {
            pen.at(&words.none, start, top + 12.0, PANEL_SIZE, Color::WHITE);
            return;
        }
        Ports::Unknown => {
            pen.at(&words.unknown, start, top + 12.0, PANEL_SIZE, Color::WHITE);
            return;
        }
    };
    if pen.metrics.is_none() {
        pen.at(
            &names.join(", "),
            start,
            top + 12.0,
            PANEL_SIZE,
            Color::WHITE,
        );
        return;
    }
    let limit = column.max.x - 20.0;
    let (mut x_pen, mut baseline, mut run) = (start, top + 12.0, 0.0);
    for (index, name) in names.iter().enumerate() {
        let width = pen.width(name, PANEL_SIZE);
        if left + width + 45.0 + run > limit {
            x_pen = start;
            baseline = top + 24.0;
            run = 0.0;
        } else {
            run += width;
        }
        x_pen = pen.at(name, x_pen, baseline, PANEL_SIZE, Color::WHITE);
        if index + 1 < names.len() {
            x_pen = pen.at(", ", x_pen, baseline, PANEL_SIZE, Color::WHITE);
            run += pen.width(", ", PANEL_SIZE);
        }
    }
}

/// The right column in `column` (see the module docs).
fn draw_column(pen: &mut Pen, title: &str, body: &ColumnBody, words: &PanelWords, column: Bounds) {
    let (left, top, right, bottom) = (column.min.x, column.min.y, column.max.x, column.max.y);
    rule(
        pen.list,
        Point::new(left + 1.0, bottom - 1.0),
        Point::new(right - 2.0, bottom - 1.0),
    );
    pen.at(title, left + 10.0, top + 12.0, PANEL_SIZE, LABEL);
    let label = |pen: &mut Pen, text: &str, down: f32| {
        pen.at(text, left + 10.0, top + down, PANEL_SIZE, LABEL);
    };
    let value = |pen: &mut Pen, text: &str, down: f32| {
        pen.at(text, left + 15.0, top + down, PANEL_SIZE, Color::WHITE);
    };
    let (name, government, legal, goods, services) = match body {
        ColumnBody::Unknown => {
            value(pen, &words.unknown, 26.0);
            return;
        }
        ColumnBody::Uninhabited { name } => {
            value(pen, name, 26.0);
            pen.at(
                &words.uninhabited,
                left + 10.0,
                top + 75.0,
                PANEL_SIZE,
                Color::WHITE,
            );
            return;
        }
        ColumnBody::Inhabited {
            name,
            government,
            legal,
            goods,
            services,
        } => (name, government, legal, goods, services),
    };
    value(pen, name, 26.0);
    label(pen, &words.government, 75.0);
    value(pen, government, 88.0);
    label(pen, &words.legal_status, 119.0);
    value(pen, legal, 132.0);
    label(pen, &words.goods_traded, 155.0);
    match goods {
        Listing::Items(items) => rows(pen, items, value, 168.0),
        Listing::None => value(pen, &words.none, 168.0),
        Listing::Unknown => value(pen, &words.unknown, 168.0),
    }
    label(pen, &words.services, 250.0);
    match services {
        Services::Items(items) => rows(pen, items, value, 263.0),
        Services::NoneAvailable => value(pen, &words.none_available, 262.0),
        Services::Unknown { any: true } => value(pen, &words.unknown, 263.0),
        Services::Unknown { any: false } => value(pen, &words.unknown, 262.0),
    }
}

/// Each of `items` set by `set`, [`ROW`] apart from `first`.
#[allow(clippy::cast_precision_loss)] // A handful of rows.
fn rows(pen: &mut Pen, items: &[String], set: impl Fn(&mut Pen, &str, f32), first: f32) {
    for (index, item) in items.iter().enumerate() {
        set(pen, item, first + ROW * index as f32);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::map_panel::Column;

    use super::*;
    use crate::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    /// The stock words.
    fn words() -> PanelWords {
        let mut list = vec![String::new(); 340];
        let mut set = |n: usize, text: &str| list[n - 1] = text.to_owned();
        set(308, " - Please select a hypergate destination - ");
        set(309, "Ports:");
        set(310, "<Unknown>");
        set(311, "Navigation Hazards:");
        set(325, "Government:");
        set(326, "Legal Status:");
        set(327, "Goods Traded:");
        set(328, "Services:");
        set(332, "None Available");
        set(334, "Uninhabited System");
        set(335, "None");
        PanelWords::from_list(&list)
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|&item| item.to_owned()).collect()
    }

    fn info(ports: Ports, body: ColumnBody) -> MapPanel {
        MapPanel {
            strip: Strip::Info {
                ports,
                hazards: "Sparse asteroid field".to_owned(),
                date: "Jun 23, 1177".to_owned(),
            },
            column: Column {
                title: "Selected System:".to_owned(),
                body,
            },
            words: words(),
        }
    }

    fn inhabited(goods: Listing, services: Services) -> ColumnBody {
        ColumnBody::Inhabited {
            name: "Sol".to_owned(),
            government: "Federation".to_owned(),
            legal: "Clean".to_owned(),
            goods,
            services,
        }
    }

    /// Each text drawn: what, its baseline's start and its colour.
    fn texts(
        panel: &MapPanel,
        metrics: Option<&dyn TextMetrics>,
    ) -> Vec<(String, f32, f32, Color)> {
        let mut list = DrawList::new();
        draw_panel(&mut list, panel, Point::new(0.0, 0.0), metrics);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    font: Font::Geneva,
                    origin,
                    size,
                    color,
                    ..
                } if *size == PANEL_SIZE => {
                    Some((text.clone(), origin.x, origin.y + PANEL_SIZE, *color))
                }
                _ => None,
            })
            .collect()
    }

    /// Where `text` was drawn, and in what colour.
    fn found(drawn: &[(String, f32, f32, Color)], text: &str) -> (f32, f32, Color) {
        let (_, x, y, color) = drawn
            .iter()
            .find(|(drawn, ..)| drawn == text)
            .unwrap_or_else(|| panic!("{text:?} not drawn in {drawn:?}"));
        (*x, *y, *color)
    }

    const W: Color = Color::WHITE;

    #[test]
    fn every_label_and_value_lands_at_its_offset_and_colour() {
        let panel = info(
            Ports::Listed(strings(&["Earth"])),
            inhabited(
                Listing::Items(strings(&["Food", "Industrial"])),
                Services::Items(strings(&["Trading", "Shipyard"])),
            ),
        );
        let drawn = texts(&panel, Some(&MonoMetrics));
        // The strip: left 8, top 436, right 594.
        assert_eq!(found(&drawn, "Ports:"), (18.0, 448.0, LABEL));
        assert_eq!(found(&drawn, "Earth"), (53.0, 448.0, W));
        assert_eq!(found(&drawn, "Navigation Hazards:"), (18.0, 472.0, LABEL));
        assert_eq!(found(&drawn, "Sparse asteroid field"), (118.0, 472.0, W));
        // The column: left 474, top 8.
        assert_eq!(found(&drawn, "Selected System:"), (484.0, 20.0, LABEL));
        assert_eq!(found(&drawn, "Sol"), (489.0, 34.0, W));
        assert_eq!(found(&drawn, "Government:"), (484.0, 83.0, LABEL));
        assert_eq!(found(&drawn, "Federation"), (489.0, 96.0, W));
        assert_eq!(found(&drawn, "Legal Status:"), (484.0, 127.0, LABEL));
        assert_eq!(found(&drawn, "Clean"), (489.0, 140.0, W));
        assert_eq!(found(&drawn, "Goods Traded:"), (484.0, 163.0, LABEL));
        assert_eq!(found(&drawn, "Food"), (489.0, 176.0, W));
        assert_eq!(found(&drawn, "Industrial"), (489.0, 188.0, W), "12 below");
        assert_eq!(found(&drawn, "Services:"), (484.0, 258.0, LABEL));
        assert_eq!(found(&drawn, "Trading"), (489.0, 271.0, W));
        assert_eq!(found(&drawn, "Shipyard"), (489.0, 283.0, W));
    }

    #[test]
    fn both_items_are_filled_and_ruled_black() {
        let mut list = DrawList::new();
        let panel = info(Ports::None, ColumnBody::Unknown);
        draw_panel(&mut list, &panel, PANEL_ORIGIN, None);
        let fills: Vec<(Point, Point, f32, Color)> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } => Some((*from, *to, *width, *color)),
                _ => None,
            })
            .collect();
        // The strip at (430, 616)-(1016, 658), the column at
        // (896, 188)-(1016, 617).
        assert!(fills.contains(&(
            Point::new(430.0, 637.0),
            Point::new(1016.0, 637.0),
            42.0,
            PANEL
        )));
        assert!(fills.contains(&(
            Point::new(896.0, 402.5),
            Point::new(1016.0, 402.5),
            429.0,
            PANEL
        )));
        // The rules: (897, 616)-(1014, 616), the strip's and the column's.
        let rule = (
            Point::new(897.0, 616.5),
            Point::new(1015.0, 616.5),
            1.0,
            Color::BLACK,
        );
        assert_eq!(fills.iter().filter(|fill| **fill == rule).count(), 2);
    }

    #[test]
    fn the_unknown_none_and_uninhabited_lines_land_where_their_values_do() {
        let drawn = texts(&info(Ports::Unknown, ColumnBody::Unknown), None);
        let unknowns: Vec<(f32, f32)> = drawn
            .iter()
            .filter(|(text, ..)| text == "<Unknown>")
            .map(|(_, x, y, _)| (*x, *y))
            .collect();
        assert_eq!(unknowns, [(53.0, 448.0), (489.0, 34.0)]);
        assert!(!drawn.iter().any(|(text, ..)| text == "Government:"));
        let drawn = texts(
            &info(
                Ports::None,
                ColumnBody::Uninhabited {
                    name: "Nowhere".to_owned(),
                },
            ),
            None,
        );
        assert_eq!(found(&drawn, "None"), (53.0, 448.0, W));
        assert_eq!(found(&drawn, "Nowhere"), (489.0, 34.0, W));
        assert_eq!(found(&drawn, "Uninhabited System"), (484.0, 83.0, W));
        assert!(!drawn.iter().any(|(text, ..)| text == "Government:"));
    }

    #[test]
    fn the_goods_and_services_none_and_unknown_rows() {
        let drawn = texts(
            &info(
                Ports::None,
                inhabited(Listing::None, Services::NoneAvailable),
            ),
            None,
        );
        assert_eq!(found(&drawn, "None Available"), (489.0, 270.0, W), "262");
        let goods_none: Vec<(f32, f32)> = drawn
            .iter()
            .filter(|(text, x, ..)| text == "None" && *x == 489.0)
            .map(|(_, x, y, _)| (*x, *y))
            .collect();
        assert_eq!(goods_none, [(489.0, 176.0)]);
        for (any, down) in [(true, 263.0), (false, 262.0)] {
            let drawn = texts(
                &info(
                    Ports::None,
                    inhabited(Listing::Unknown, Services::Unknown { any }),
                ),
                None,
            );
            let unknowns: Vec<f32> = drawn
                .iter()
                .filter(|(text, ..)| text == "<Unknown>")
                .map(|(_, _, y, _)| *y)
                .collect();
            assert_eq!(unknowns, [176.0, 8.0 + down], "{any}");
        }
    }

    #[test]
    fn the_ports_wrap_to_the_second_lines_start_without_counting_the_wrapped_name() {
        // MonoMetrics: 4.5 a character at Geneva 9. The limit is 574:
        // left 8 + w + 45 + x must stay within it, so x + w ≤ 521.
        let long = "a".repeat(100); // 450 wide.
        let names = strings(&[&long, "bbbbbbbbbbbbbbbbbbbb", "cc", &long, "dd"]);
        let drawn = texts(
            &info(Ports::Listed(names), ColumnBody::Unknown),
            Some(&MonoMetrics),
        );
        let at = |text: &str| {
            let (x, y, _) = found(&drawn, text);
            (x, y)
        };
        // The first fits: x = 450, then ", " makes it 459.
        assert_eq!(at(&long), (53.0, 448.0));
        // 459 + 90 > 521: "b…" wraps to line 2's start, x = 0, its own
        // width not counted; then ", " makes x 9.
        assert_eq!(at("bbbbbbbbbbbbbbbbbbbb"), (53.0, 460.0));
        // 9 + 9 ≤ 521: "cc" follows on, after "b…" and its ", ", at
        // 53 + 90 + 9.
        assert_eq!(at("cc"), (152.0, 460.0));
        // x = 9 + 9 + 9 = 27; 27 + 450 ≤ 521: the second long name follows
        // on, at 152 + 9 + 9 = 170; x 477 + 9 = 486.
        let longs: Vec<(f32, f32)> = drawn
            .iter()
            .filter(|(text, ..)| *text == long)
            .map(|(_, x, y, _)| (*x, *y))
            .collect();
        assert_eq!(longs, [(53.0, 448.0), (170.0, 460.0)]);
        // 486 + 9 ≤ 521 too: "dd" follows on the second line.
        assert_eq!(at("dd"), (170.0 + 450.0 + 9.0, 460.0));
    }

    #[test]
    fn a_second_wrap_returns_to_the_second_lines_start() {
        let long = "a".repeat(100);
        let other = "e".repeat(100);
        let third = "f".repeat(100);
        let names = strings(&[&long, &other, &third]);
        let drawn = texts(
            &info(Ports::Listed(names), ColumnBody::Unknown),
            Some(&MonoMetrics),
        );
        assert_eq!(found(&drawn, &long).0, 53.0);
        // 459 + 450 > 521: wraps, x 0, then ", " 9.
        assert_eq!(
            (found(&drawn, &other).0, found(&drawn, &other).1),
            (53.0, 460.0)
        );
        // 9 + 450 ≤ 521: follows on, x 459 + 9 = 468.
        assert_eq!(found(&drawn, &third).0, 53.0 + 450.0 + 9.0);
        let commas = drawn.iter().filter(|(text, ..)| text == ", ").count();
        assert_eq!(commas, 2, "after each but the last");
        // A fourth long one would wrap again, to line 2's start.
        let fourth = "g".repeat(100);
        let names = strings(&[&long, &other, &third, &fourth]);
        let drawn = texts(
            &info(Ports::Listed(names), ColumnBody::Unknown),
            Some(&MonoMetrics),
        );
        assert_eq!(
            (found(&drawn, &fourth).0, found(&drawn, &fourth).1),
            (53.0, 460.0)
        );
    }

    #[test]
    fn without_metrics_the_ports_are_one_line() {
        let names = strings(&["Earth", "Mars"]);
        let drawn = texts(&info(Ports::Listed(names), ColumnBody::Unknown), None);
        assert_eq!(found(&drawn, "Earth, Mars"), (53.0, 448.0, W));
    }

    fn prompt() -> MapPanel {
        MapPanel {
            strip: Strip::HypergatePrompt {
                prompt: "abcd".to_owned(),
                date: "Jun 23, 1177".to_owned(),
            },
            column: Column {
                title: "Current System:".to_owned(),
                body: ColumnBody::Unknown,
            },
            words: words(),
        }
    }

    #[test]
    fn the_prompt_sits_at_left_plus_half_the_right_less_half_its_width() {
        // 8 + 594 / 2 = 305; "abcd" is 18 wide: 305 − 9 = 296.
        let drawn = texts(&prompt(), Some(&MonoMetrics));
        assert_eq!(found(&drawn, "abcd"), (296.0, 460.0, LABEL));
        assert!(!drawn.iter().any(|(text, ..)| text == "Ports:"));
        // "abc" is 13.5 wide: half 6.75, truncated to 6.
        let MapPanel { column, words, .. } = prompt();
        let panel = MapPanel {
            strip: Strip::HypergatePrompt {
                prompt: "abc".to_owned(),
                date: String::new(),
            },
            column,
            words,
        };
        assert_eq!(found(&texts(&panel, Some(&MonoMetrics)), "abc").0, 299.0);
        // On screen the halving is of the dialog's right edge, not the
        // screen's: from the origin, 422 + 305 − 9.
        let mut list = DrawList::new();
        draw_panel(&mut list, &prompt(), PANEL_ORIGIN, Some(&MonoMetrics));
        let x = list.iter().find_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if text == "abcd" => Some(origin.x),
            _ => None,
        });
        assert_eq!(x, Some(718.0));
    }

    #[test]
    fn the_dates_right_edge_is_5_in_from_the_strips() {
        // "Jun 23, 1177" is 12 characters, 54 wide: 594 − 54 − 5.
        let drawn = texts(&prompt(), Some(&MonoMetrics));
        assert_eq!(found(&drawn, "Jun 23, 1177"), (535.0, 472.0, DATE));
        let drawn = texts(&info(Ports::None, ColumnBody::Unknown), Some(&MonoMetrics));
        assert_eq!(found(&drawn, "Jun 23, 1177"), (535.0, 472.0, DATE));
        let drawn = texts(&prompt(), None);
        assert_eq!(
            found(&drawn, "Jun 23, 1177"),
            (589.0, 472.0, DATE),
            "unmeasured"
        );
        assert_eq!(found(&drawn, "abcd").0, 305.0, "unmeasured");
    }
}
