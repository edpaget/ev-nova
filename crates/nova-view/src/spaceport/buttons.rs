//! The port's buttons as `_DrawPlanetButtons` draws them (@0x2968d in
//! `EV Nova.app`'s i386 executable).
//!
//! It draws the same seven buttons at every spaceport, never hiding one
//! (@0x29873-0x2995e): Leave (item 12), Recharge (4), the Trade Center
//! (7), the Outfitter (8), the Shipyard (9), the Mission BBS (10) and the
//! Bar (11), in that order, each labelled from `STR#` 150
//! (@0x2982a-0x29852). One the stellar doesn't offer is drawn disabled
//! (state -2, @0x2993b), which is [`ButtonSkin::disabled`] with the
//! style's grey label, and `_TrackPlanetButtons` (@0x2996f) never takes a
//! press on it (@0x29abe-0x29bca), which [`Dialog`] matches by never
//! reporting a greyed item. Leave is always enabled (@0x297b4). Recharge
//! is enabled where the stellar sells fuel, the cash is above 0 and the
//! tank isn't full (@0x297a3-0x297f4); the view is told the last two
//! (`SpaceportView::set_recharge_enabled`). Each service is enabled when
//! the stellar offers it (`nova_sim::services`). Neither routine plays a
//! sound.
//!
//! [`ButtonSkin::disabled`]: crate::ui::button::ButtonSkin
//! [`Dialog`]: crate::ui::dialog::Dialog

use nova_sim::Service;

use super::layout::{LEAVE_ITEM, LEAVE_LABEL, RECHARGE_ITEM, RECHARGE_LABEL, label, service_item};
use crate::ui::dialog::Role;

/// The services' buttons, in the order `_DrawPlanetButtons` draws them
/// after Leave and Recharge (@0x296d8-0x2979e).
const SERVICES: [Service; 5] = [
    Service::TradeCenter,
    Service::Outfitter,
    Service::Shipyard,
    Service::MissionBbs,
    Service::Bar,
];

/// The seven buttons' items and roles, in `_DrawPlanetButtons`' order:
/// Leave, Recharge, then the five services, each a button with its label.
#[must_use]
pub fn roles() -> Vec<(usize, Role)> {
    [(LEAVE_ITEM, LEAVE_LABEL), (RECHARGE_ITEM, RECHARGE_LABEL)]
        .into_iter()
        .chain(SERVICES.map(|service| (service_item(service), label(service))))
        .map(|(item, text)| (item, Role::Button(text.to_owned())))
        .collect()
}

/// The buttons to draw disabled: each service not in `offered`, and
/// Recharge unless `recharge` says it is enabled. Leave never is.
#[must_use]
pub fn greyed(offered: &[Service], recharge: bool) -> Vec<usize> {
    (!recharge)
        .then_some(RECHARGE_ITEM)
        .into_iter()
        .chain(
            SERVICES
                .into_iter()
                .filter(|service| !offered.contains(service))
                .map(service_item),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(text: &str) -> Role {
        Role::Button(text.to_owned())
    }

    #[test]
    fn every_button_is_drawn_with_its_label_in_draw_planet_buttons_order() {
        assert_eq!(
            roles(),
            [
                (12, button("Leave")),
                (4, button("Recharge")),
                (7, button("Trade Center")),
                (8, button("Outfitter")),
                (9, button("Shipyard")),
                (10, button("Mission BBS")),
                (11, button("Bar")),
            ]
        );
    }

    #[test]
    fn the_services_not_offered_and_a_disabled_recharge_are_greyed() {
        use Service::{Bar, MissionBbs, Outfitter, Shipyard, TradeCenter};
        assert_eq!(greyed(&[TradeCenter], true), [8, 9, 10, 11]);
        assert_eq!(greyed(&[TradeCenter], false), [4, 8, 9, 10, 11]);
        assert_eq!(greyed(&[] as &[Service], false), [4, 7, 8, 9, 10, 11]);
        assert_eq!(
            greyed(&[TradeCenter, Outfitter, Shipyard, Bar, MissionBbs], true),
            Vec::<usize>::new()
        );
        assert_eq!(greyed(&[Bar, MissionBbs], true), [7, 8, 9]);
    }
}

/// The buttons as the spaceport draws and tracks them.
#[cfg(test)]
mod view_tests {
    use nova_sim::landing::StellarFlags;

    use super::*;
    use crate::color::Color;
    use crate::draw::DrawCommand;
    use crate::geometry::Point;
    use crate::input::{Input, Key, MouseButton};
    use crate::screen::Screen;
    use crate::spaceport::SpaceportView;
    use crate::spaceport::view::tests::{click_item, drawn, flagged, item, key, view_of};
    use crate::ui::button::{ButtonSkin, ButtonStyle};

    const LAND: u32 = StellarFlags::CAN_LAND;

    /// The left cap `view` draws for button `number`, and its label's
    /// colour.
    fn looks(view: &SpaceportView, number: usize) -> (Option<crate::image::ImageKey>, Color) {
        let bounds = item(view, number);
        let commands = drawn(view);
        let cap = commands.iter().find_map(|command| match command {
            DrawCommand::StretchedPicture {
                image, top_left, ..
            } if *top_left == bounds.min => Some(*image),
            _ => None,
        });
        let text = roles()
            .into_iter()
            .find_map(|(each, role)| match role {
                crate::ui::dialog::Role::Button(text) if each == number => Some(text),
                _ => None,
            })
            .expect("a button");
        let color = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text: t, color, .. } if *t == text => Some(*color),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{text} drawn"));
        (cap, color)
    }

    fn enabled(view: &SpaceportView, number: usize) {
        assert_eq!(
            looks(view, number),
            (Some(ButtonSkin::NOVA.normal.left), Color::BLACK),
            "{number} enabled"
        );
    }

    fn disabled(view: &SpaceportView, number: usize) {
        assert_eq!(
            looks(view, number),
            (
                Some(ButtonSkin::NOVA.disabled.left),
                ButtonStyle::STOCK.grey
            ),
            "{number} disabled"
        );
    }

    /// Nothing opened, asked for or said, and the description kept.
    fn untouched(view: &mut SpaceportView, what: &str) {
        assert!(view.open_service().is_none(), "{what}");
        assert!(view.open_outfitter().is_none(), "{what}");
        assert!(view.open_shipyard().is_none(), "{what}");
        assert!(view.open_bar().is_none(), "{what}");
        assert!(view.open_trade().is_none(), "{what}");
        assert!(!view.take_recharge(), "{what}");
        assert!(!view.take_mission_computer_request(), "{what}");
        assert!(!view.left(), "{what}");
        assert_eq!(view.take_sounds(), [], "{what}: silent");
        let lines = view
            .dialog()
            .expect("laid out")
            .scroll_text()
            .expect("the box")
            .lines()
            .to_vec();
        assert_eq!(lines, ["Blue and green.", "Home."], "{what}");
    }

    #[test]
    fn a_trade_center_alone_draws_every_button_greying_the_services_it_lacks() {
        let view = view_of(&flagged(LAND | StellarFlags::TRADE_CENTER));
        for number in [12, 4, 7, 10] {
            enabled(&view, number);
        }
        for number in [8, 9, 11] {
            disabled(&view, number);
        }
    }

    #[test]
    fn a_click_or_a_letter_on_a_greyed_button_does_nothing_and_is_silent() {
        let bare = || view_of(&flagged(LAND | StellarFlags::TRADE_CENTER));
        for number in [8, 9, 11] {
            let mut view = bare();
            click_item(&mut view, number);
            untouched(&mut view, &format!("item {number}"));
        }
        for letter in ['o', 's', 'b'] {
            let mut view = bare();
            view.input(&key(Key::Char(letter)));
            untouched(&mut view, &format!("{letter}"));
        }
    }

    #[test]
    fn an_uninhabited_stellar_with_no_services_greys_all_six_and_leave_works() {
        let mut view = view_of(&flagged(LAND | StellarFlags::UNINHABITED));
        assert_eq!(view.offered(), []);
        enabled(&view, 12);
        for number in [4, 7, 8, 9, 10, 11] {
            disabled(&view, number);
            click_item(&mut view, number);
            untouched(&mut view, &format!("item {number}"));
        }
        click_item(&mut view, 12);
        assert!(view.left());
    }

    #[test]
    fn an_uninhabited_stellar_opens_its_flagged_services() {
        let flags = LAND | StellarFlags::UNINHABITED | StellarFlags::TRADE_CENTER;
        let mut view = view_of(&flagged(flags));
        enabled(&view, 7);
        disabled(&view, 10);
        disabled(&view, 4);
        click_item(&mut view, 7);
        assert!(view.open_service().is_some(), "the trade center");
    }

    #[test]
    fn a_disabled_recharge_takes_no_click_but_r_and_f_still_ask() {
        let mut view = view_of(&flagged(LAND | StellarFlags::TRADE_CENTER));
        enabled(&view, 4);
        view.set_recharge_enabled(false);
        disabled(&view, 4);
        click_item(&mut view, 4);
        untouched(&mut view, "a click");
        for letter in ['r', 'f'] {
            view.input(&key(Key::Char(letter)));
            assert!(view.take_recharge(), "{letter}");
            assert_eq!(view.take_sounds(), [], "{letter}");
        }
        view.set_recharge_enabled(true);
        enabled(&view, 4);
        click_item(&mut view, 4);
        assert!(view.take_recharge(), "enabled again");
    }

    #[test]
    fn where_no_fuel_is_sold_recharge_stays_disabled() {
        let mut view = view_of(&flagged(LAND | StellarFlags::UNINHABITED));
        view.set_recharge_enabled(true);
        disabled(&view, 4);
        click_item(&mut view, 4);
        view.input(&key(Key::Char('r')));
        assert!(!view.take_recharge());
    }

    fn press(view: &mut SpaceportView, pressed: bool, at: Point) {
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }

    /// `_TrackPlanetButtons` (@0x29bd0-0x29d56) highlights only the button
    /// first pressed, and reports it only when released on it.
    #[test]
    fn only_the_button_first_pressed_and_released_on_activates() {
        let everything = || view_of(&flagged(0xFF & !StellarFlags::UNINHABITED));
        let mut view = everything();
        let (trade, outfitter) = (item(&view, 7).center(), item(&view, 8).center());
        press(&mut view, true, trade);
        view.input(&Input::PointerMoved(outfitter));
        press(&mut view, false, outfitter);
        assert!(view.open_service().is_none(), "slid onto the outfitter");
        assert!(view.open_outfitter().is_none());
        assert!(!view.take_outfitter_opened());
        let mut view = everything();
        press(&mut view, true, trade);
        view.input(&Input::PointerMoved(Point::new(0.0, 0.0)));
        view.input(&Input::PointerMoved(trade));
        press(&mut view, false, trade);
        assert!(view.open_service().is_some(), "back on the trade center");
    }

    fn click(view: &mut SpaceportView, at: Point) {
        press(view, true, at);
        press(view, false, at);
    }

    /// The points on button `number`'s right and bottom edges, through its
    /// centre, and the points a pixel inside them.
    fn edges(view: &SpaceportView, number: usize) -> ([Point; 2], [Point; 2]) {
        let r = item(view, number);
        let c = r.center();
        (
            [Point::new(r.max.x, c.y), Point::new(c.x, r.max.y)],
            [
                Point::new(r.max.x - 1.0, c.y),
                Point::new(c.x, r.max.y - 1.0),
            ],
        )
    }

    /// `_TrackPlanetButtons` tests the press with `PtInRect` (@0x29ac8): a
    /// button's right and bottom edges are off it.
    #[test]
    fn a_press_on_a_buttons_right_or_bottom_edge_presses_nothing() {
        let fresh = || view_of(&flagged(LAND | StellarFlags::TRADE_CENTER));
        let view = fresh();
        for number in [12, 7] {
            let (on_edge, _) = edges(&view, number);
            for at in on_edge {
                for (other, _) in roles() {
                    assert!(
                        !item(&view, other).contains_half_open(at),
                        "{number}'s edge {at:?} is on item {other}"
                    );
                }
            }
        }
        let (on_edge, inside) = edges(&view, 12);
        for at in on_edge {
            let mut view = fresh();
            click(&mut view, at);
            assert!(!view.left(), "Leave's edge {at:?}");
        }
        for at in inside {
            let mut view = fresh();
            click(&mut view, at);
            assert!(view.left(), "a pixel inside Leave, {at:?}");
        }
        let (on_edge, inside) = edges(&view, 7);
        for at in on_edge {
            let mut view = fresh();
            click(&mut view, at);
            assert!(
                view.open_service().is_none(),
                "the trade center's edge {at:?}"
            );
        }
        for at in inside {
            let mut view = fresh();
            click(&mut view, at);
            assert!(
                view.open_service().is_some(),
                "a pixel inside the trade center, {at:?}"
            );
        }
    }

    /// ... and so is the pointer while held (@0x29c08-0x29d0f).
    #[test]
    fn dragging_onto_a_buttons_right_or_bottom_edge_counts_as_off_it() {
        let fresh = || view_of(&flagged(LAND | StellarFlags::TRADE_CENTER));
        let view = fresh();
        let centre = item(&view, 7).center();
        let (on_edge, inside) = edges(&view, 7);
        for (edge, near) in on_edge.into_iter().zip(inside) {
            let mut view = fresh();
            press(&mut view, true, centre);
            view.input(&Input::PointerMoved(edge));
            press(&mut view, false, edge);
            assert!(view.open_service().is_none(), "released on {edge:?}");
            let mut view = fresh();
            press(&mut view, true, centre);
            view.input(&Input::PointerMoved(near));
            press(&mut view, false, near);
            assert!(view.open_service().is_some(), "released on {near:?}");
        }
    }
}
