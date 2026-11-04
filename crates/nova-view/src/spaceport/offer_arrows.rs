//! The offer dialog's scroll arrows: items 9 and 10 of `DLOG` 1016 and
//! 1020, drawn by `_DrawSingleMissionButtons` @0x27db8 and held or tracked
//! by `_SingleMissionFilter` @0x98d9a.
//!
//! They are the text dialog's arrows in other routines, and nothing in
//! those routines differs, so [`Dialog`](crate::ui::Dialog)'s hold, its
//! tracked click and [`ScrollArrow::button`](crate::ui::scroll_arrows::ScrollArrow)
//! are their port too (see [`scroll_arrows`](crate::ui::scroll_arrows)):
//!
//! - Item 9 is up and item 10 down, tested in that order, each held only
//!   while its flag is set (@0x9900d-0x99162). The hold steps 1 px at
//!   once and then each Mac tick while `StillDown`, which doesn't read the
//!   pointer's position (@0x99060-0x991e6).
//! - Their states are `_DrawTextButtons`': both flags clear draws neither,
//!   one set draws its arrow enabled and the other disabled
//!   (@0x27de9-0x27ea5), and the one held draws pressed (@0x280a4-0x28177).
//! - Any other press on them is tracked by `_TrackSingleMissionButtons`
//!   @0x28188, which the filter turns into no item (@0x9922f-0x9925a).
//! - Up and Down give items 9 and 10 while their flags are set
//!   (@0x98efd, @0x98f54), and `_OfferOneMission` steps those 10 px, gated
//!   on the flags again (@0xa3413-0xa3490).
//!
//! The text scrolls by pixels, a partial line cut at the box
//! ([`ScrollText::pixel_scrolled`](crate::ui::ScrollText::pixel_scrolled)).
//! Its flags start as 3003's do: up clear, and down set exactly when the
//! text can move (@0xa2bf0-0xa2c55).
//!
//! The arrows stay where the template has them on a shrunk 1016
//! (`_OfferOneMission` never moves them, @0xa2645-0xa2816), so a dialog
//! shrunk far enough leaves them outside it, hidden as any item wholly
//! outside is. One shrunk less, with them partly inside, never draws them
//! either: a shrunk box is taller than its text, so both flags start clear
//! and the arrows' state is −2. Nothing plays a sound.
//!
//! The held loops read no events, nor does the tracker, so keys pressed
//! meanwhile wait for the release ([`OfferDialog`](super::OfferDialog)'s
//! `Screen::input`, and the `held_keys` module).

use crate::ui::dialog::Role;
use crate::ui::scroll_arrows::ScrollArrow;

/// The scroll-up arrow's item ([`ScrollArrow::Up`]), `GetDialogItem` 9
/// (@0x99030).
pub const SCROLL_UP_ITEM: usize = 9;

/// The scroll-down arrow's item ([`ScrollArrow::Down`]), `GetDialogItem`
/// 10 (@0x99138).
pub const SCROLL_DOWN_ITEM: usize = 10;

/// The arrows' roles, in both variants.
pub(super) fn arrow_roles() -> [(usize, Role); 2] {
    [
        (SCROLL_UP_ITEM, Role::ScrollArrow(ScrollArrow::Up)),
        (SCROLL_DOWN_ITEM, Role::ScrollArrow(ScrollArrow::Down)),
    ]
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;

    use nova_sim::mission::OfferLabels;
    use nova_sim::{MissionId, OfferPrompt};

    use super::*;
    use crate::draw::{DrawCommand, DrawList};
    use crate::font::Font;
    use crate::geometry::Point;
    use crate::input::{Input, Key, MouseButton};
    use crate::screen::Screen;
    use crate::spaceport::offer::{OfferDialog, TEXT_ITEM, fixture};
    use crate::text::TextMetrics;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::{Button, ButtonSkin, ButtonStyle};
    use crate::ui::desc::{BODY_SIZE, PAGE_UNITS};
    use crate::ui::dialog::DialogTemplate;
    use crate::ui::scroll_text::ARROW_UNITS;

    /// One Mac tick.
    const TICK: Duration = Duration::from_nanos(16_666_666);

    /// A `PICT` above 127, which shows the offer in 1020.
    const GRAPHIC: i16 = 5005;

    /// `lines` lines of text, each a line high.
    fn lines(lines: usize) -> String {
        (0..lines)
            .map(|n| format!("l{n}"))
            .collect::<Vec<_>>()
            .join("\r")
    }

    fn offer_of(template: DialogTemplate, text: &str, graphic: Option<i16>) -> OfferDialog {
        OfferDialog::new(
            Ok((template, Rc::new(MonoMetrics))),
            OfferPrompt {
                mission: MissionId(141),
                text: text.to_owned(),
                graphic,
                labels: OfferLabels {
                    accept: "Yes".to_owned(),
                    refuse: Some("No".to_owned()),
                },
                movie: None,
            },
            ButtonStyle::STOCK,
        )
    }

    /// Both variants, `text` in each: stock 1016's shape and stock 1020's.
    fn both(text: &str) -> [OfferDialog; 2] {
        [
            offer_of(fixture::stock_1016(), text, None),
            offer_of(fixture::stock_1020(), text, Some(GRAPHIC)),
        ]
    }

    /// Forty lines, 480 high: past either box.
    fn long() -> [OfferDialog; 2] {
        both(&lines(40))
    }

    fn offset(offer: &OfferDialog) -> f32 {
        offer
            .dialog()
            .and_then(|dialog| dialog.scroll_text())
            .expect("the text")
            .offset()
    }

    fn item_at(offer: &OfferDialog, item: usize) -> Point {
        offer
            .dialog()
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("the item")
            .center()
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn left(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn drawn(offer: &OfferDialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        offer.draw(&mut list);
        list.iter().cloned().collect()
    }

    /// `item` drawn as `_DrawButton` draws an arrow.
    fn arrow(offer: &OfferDialog, item: usize, enabled: bool, pressed: bool) -> Vec<DrawCommand> {
        let label = if item == SCROLL_UP_ITEM { "^" } else { "&" };
        let button = Button {
            rect: offer
                .dialog()
                .and_then(|dialog| dialog.item_bounds(item))
                .expect("the arrow"),
            label: label.to_owned(),
            enabled,
        };
        let mut list = DrawList::new();
        button.draw(
            pressed,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut list,
        );
        list.iter().cloned().collect()
    }

    /// Whether `part` is drawn, in one run, in `whole`.
    fn draws(whole: &[DrawCommand], part: &[DrawCommand]) -> bool {
        !part.is_empty() && whole.windows(part.len()).any(|run| run == part)
    }

    /// The arrows' looks, (up enabled, up pressed) and (down enabled, down
    /// pressed), checked against what the offer draws.
    fn assert_arrows(offer: &OfferDialog, up: (bool, bool), down: (bool, bool)) {
        let commands = drawn(offer);
        let both = [
            arrow(offer, SCROLL_UP_ITEM, up.0, up.1),
            arrow(offer, SCROLL_DOWN_ITEM, down.0, down.1),
        ]
        .concat();
        assert!(draws(&commands, &both), "{up:?} {down:?}: {commands:#?}");
    }

    fn draws_no_arrow(offer: &OfferDialog) -> bool {
        let commands = drawn(offer);
        [SCROLL_UP_ITEM, SCROLL_DOWN_ITEM].iter().all(|&item| {
            [(true, false), (false, false), (true, true), (false, true)]
                .iter()
                .all(|&(enabled, pressed)| !draws(&commands, &arrow(offer, item, enabled, pressed)))
        })
    }

    #[test]
    fn down_and_up_scroll_ten_pixels_a_press_or_repeat_in_both_variants() {
        for mut offer in long() {
            offer.input(&key(Key::ArrowDown, false));
            assert_eq!(offset(&offer), ARROW_UNITS);
            offer.input(&key(Key::ArrowDown, true));
            assert_eq!(offset(&offer), 2.0 * ARROW_UNITS, "a repeat");
            offer.input(&key(Key::ArrowUp, false));
            assert_eq!(offset(&offer), ARROW_UNITS);
        }
    }

    /// `_SingleMissionFilter` gives item 9 only while up is set, and
    /// `_OfferOneMission` steps it only then (@0x98ef0-0x98efd,
    /// @0xa3413-0xa3422).
    #[test]
    fn up_at_the_top_and_down_on_a_text_that_fits_move_nothing() {
        for mut offer in long() {
            offer.input(&key(Key::ArrowUp, false));
            assert_eq!(offset(&offer), 0.0);
        }
        for mut offer in both("short") {
            offer.input(&key(Key::ArrowDown, false));
            assert_eq!(offset(&offer), 0.0);
        }
    }

    #[test]
    fn a_line_partly_scrolled_off_is_cut_at_the_box() {
        for mut offer in long() {
            offer.input(&key(Key::ArrowDown, false));
            let box_bounds = offer
                .dialog()
                .and_then(|dialog| dialog.item_bounds(TEXT_ITEM))
                .expect("the text box");
            let commands = drawn(&offer);
            let first = commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Clipped { clip, command } => match &**command {
                        DrawCommand::Text { text, origin, .. } if text == "l0" => {
                            Some((*clip, origin.y))
                        }
                        _ => None,
                    },
                    _ => None,
                })
                .expect("the first line, cut");
            assert_eq!(first, (box_bounds, box_bounds.min.y - ARROW_UNITS));
        }
    }

    #[test]
    fn home_end_page_up_and_page_down_scroll_as_the_text_dialogs_do() {
        for mut offer in both(&lines(100)) {
            offer.input(&key(Key::PageDown, false));
            assert_eq!(offset(&offer), PAGE_UNITS);
            offer.input(&key(Key::PageUp, false));
            assert_eq!(offset(&offer), 0.0);
            offer.input(&key(Key::End, false));
            let text = offer.dialog().and_then(|d| d.scroll_text()).expect("text");
            assert_eq!(text.offset(), text.max_offset());
            assert!(!text.can_scroll_down(), "End clears it (@0x98f66-0x98fa8)");
            offer.input(&key(Key::Home, false));
            assert_eq!(offset(&offer), 0.0);
        }
    }

    /// The user's standing decision keeps dialog keys press-only, though
    /// `_SingleMissionFilter` reads auto-key too (@0x98e31-0x98e44).
    #[test]
    fn the_page_keys_act_on_the_press_alone() {
        for mut offer in both(&lines(100)) {
            offer.input(&key(Key::PageDown, true));
            assert_eq!(offset(&offer), 0.0);
        }
    }

    #[test]
    fn at_the_top_up_draws_disabled_and_down_enabled() {
        for offer in long() {
            assert_arrows(&offer, (false, false), (true, false));
        }
    }

    #[test]
    fn once_moved_both_arrows_draw_enabled() {
        for mut offer in long() {
            offer.input(&key(Key::ArrowDown, false));
            assert_arrows(&offer, (true, false), (true, false));
        }
    }

    #[test]
    fn a_text_that_fits_draws_neither_arrow() {
        for offer in both(&lines(3)) {
            assert!(draws_no_arrow(&offer));
        }
    }

    #[test]
    fn a_held_down_arrow_steps_a_pixel_at_once_then_one_each_tick_drawn_pressed() {
        for mut offer in long() {
            let at = item_at(&offer, SCROLL_DOWN_ITEM);
            offer.input(&left(true, at));
            assert_eq!(offset(&offer), 1.0);
            offer.tick(TICK);
            offer.tick(TICK);
            assert_eq!(offset(&offer), 3.0);
            assert_arrows(&offer, (true, false), (true, true));
            offer.input(&left(false, at));
            offer.tick(TICK * 5);
            assert_eq!(offset(&offer), 3.0, "let go");
            assert_arrows(&offer, (true, false), (true, false));
        }
    }

    #[test]
    fn a_held_up_arrow_steps_back() {
        for mut offer in long() {
            offer.input(&key(Key::ArrowDown, false));
            let at = item_at(&offer, SCROLL_UP_ITEM);
            offer.input(&left(true, at));
            offer.tick(TICK);
            assert_eq!(offset(&offer), ARROW_UNITS - 2.0);
            assert_arrows(&offer, (true, true), (true, false));
        }
    }

    /// `_TrackSingleMissionButtons` takes it, and the filter gives no item
    /// for an arrow (@0x9922f-0x9925a).
    #[test]
    fn a_click_on_the_disabled_up_arrow_does_nothing() {
        for mut offer in long() {
            let at = item_at(&offer, SCROLL_UP_ITEM);
            offer.input(&left(true, at));
            offer.tick(TICK * 5);
            offer.input(&left(false, at));
            assert_eq!(offset(&offer), 0.0);
            assert!(!offer.answered());
        }
    }

    /// A shrunk box is `max(h, 48) + 16` high, taller than its text, so
    /// both flags start clear and `_DrawSingleMissionButtons` gives the
    /// arrows −2 (@0x27e05-0x27e0b), whether they are inside the dialog
    /// (`d <= 7`), cut by its edge (`7 < d < 30`) or outside it.
    #[test]
    fn a_shrunk_1016_draws_no_arrow_however_far_it_shrinks() {
        let line = MonoMetrics.line_height(Font::Geneva, BODY_SIZE);
        for count in [1, 12, 18, 19, 20, 21, 22] {
            let offer = offer_of(fixture::stock_1016(), &lines(count), None);
            let height = offer.dialog().expect("laid").bounds().height();
            assert_ne!(height, 317.0, "{count} lines shrink or grow it");
            assert!(draws_no_arrow(&offer), "{count} lines, {line} each");
        }
    }

    // Keys wait for the release (`ui::held_keys`): `_SingleMissionFilter`'s
    // held loops and `_TrackSingleMissionButtons` read no events
    // (@0x99060-0x991e6, @0x28188).

    /// A long offer in 1016, a hundred lines, with a press held on its
    /// down arrow.
    fn held() -> (OfferDialog, Point) {
        let mut offer = offer_of(fixture::stock_1016(), &lines(100), None);
        let at = item_at(&offer, SCROLL_DOWN_ITEM);
        offer.input(&left(true, at));
        (offer, at)
    }

    #[test]
    fn a_labels_letter_during_a_hold_chooses_at_the_release() {
        use crate::spaceport::offer::OfferChoice;
        for (letter, choice) in [('y', OfferChoice::Accept), ('n', OfferChoice::Refuse)] {
            let (mut offer, at) = held();
            offer.input(&key(
                crate::input::letter_key(letter).expect("a letter key"),
                false,
            ));
            assert_eq!(offer.take_choice(), None, "it waits");
            offer.input(&left(false, at));
            assert_eq!(offer.take_choice(), Some(choice));
        }
    }

    #[test]
    fn return_during_a_click_on_a_button_waits_for_its_release() {
        use crate::spaceport::offer::{OfferChoice, REFUSE_ITEM};
        let mut offer = offer_of(fixture::stock_1016(), &lines(40), None);
        let refuse = item_at(&offer, REFUSE_ITEM);
        offer.input(&left(true, refuse));
        offer.input(&key(Key::Enter, false));
        assert!(!offer.answered());
        offer.input(&left(false, refuse));
        assert_eq!(
            offer.take_choice(),
            Some(OfferChoice::Refuse),
            "the click first"
        );
    }

    #[test]
    fn m_i_and_p_during_a_hold_ask_at_the_release() {
        let map = crate::flight::view::MAP_KEY;
        let info = crate::ui::mission_info::MISSION_INFO_KEY;
        let player = crate::ui::player_info::PLAYER_INFO_KEY;
        let (mut offer, at) = held();
        offer.input(&key(map, false));
        assert!(!offer.take_map_request(), "it waits");
        offer.input(&left(false, at));
        assert!(offer.take_map_request());
        let (mut offer, at) = held();
        offer.input(&key(info, false));
        offer.input(&left(false, at));
        assert!(offer.take_info_request());
        let (mut offer, at) = held();
        offer.input(&key(player, false));
        offer.input(&left(false, at));
        assert!(offer.take_player_info_request());
    }

    #[test]
    fn the_page_keys_wait_for_the_release() {
        let (mut offer, at) = held();
        offer.input(&key(Key::PageDown, false));
        assert_eq!(offset(&offer), 1.0);
        offer.input(&left(false, at));
        assert_eq!(offset(&offer), 1.0 + PAGE_UNITS);
    }

    #[test]
    fn a_cancelled_hold_chooses_on_the_next_tick() {
        use crate::spaceport::offer::OfferChoice;
        let (mut offer, _) = held();
        offer.input(&key(Key::Enter, false));
        offer.cancel_pointer();
        assert!(!offer.answered());
        offer.tick(Duration::ZERO);
        assert_eq!(offer.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn letting_go_of_the_keys_during_a_hold_is_a_cancel() {
        let (mut offer, _) = held();
        offer.input(&key(Key::Enter, false));
        offer.release_keys();
        offer.tick(TICK * 3);
        assert_eq!(offset(&offer), 1.0, "the hold is over");
        assert!(offer.answered());
    }

    /// The answer's `FlushEvents` calls pass event codes where it takes
    /// masks, so they flush only mouse events (@0xa2da2-0xa2dde): a key
    /// queued after the answering key is left for what shows next.
    #[test]
    fn a_key_queued_after_the_answering_key_is_left_for_what_shows_next() {
        let (mut offer, at) = held();
        for k in [Key::Enter, Key::KeyB] {
            offer.input(&key(k, false));
        }
        offer.input(&left(false, at));
        assert!(offer.answered());
        assert_eq!(offer.take_unclaimed(), [key(Key::KeyB, false)]);
        assert_eq!(offer.take_unclaimed(), [], "once");
    }

    /// A press that ends a hold (a release the engine never saw) replays
    /// after the keys: an accept's flush drops it, as its codes read as
    /// masks flush mouse events, and a refusal, which flushes nothing,
    /// leaves it.
    #[test]
    fn a_press_left_after_an_accept_goes_with_its_flush_and_after_a_refusal_stays() {
        for (letter, kept) in [('y', false), ('n', true)] {
            let (mut offer, at) = held();
            for k in [
                crate::input::letter_key(letter).expect("a letter key"),
                Key::KeyB,
            ] {
                offer.input(&key(k, false));
            }
            offer.input(&left(true, at));
            assert!(offer.answered());
            let mut left_over = vec![key(Key::KeyB, false)];
            if kept {
                left_over.push(left(true, at));
            }
            assert_eq!(offer.take_unclaimed(), left_over, "{letter}");
        }
    }

    /// `_DoSystemMap`, `_DoPlayerInfoDialog` and `_DoMissionInfoDialog`
    /// flush the queue as they open (@0x1257b, @0x5a544, @0x9e287).
    #[test]
    fn a_key_queued_after_m_i_or_p_is_dropped() {
        for asking in [
            crate::flight::view::MAP_KEY,
            crate::ui::mission_info::MISSION_INFO_KEY,
            crate::ui::player_info::PLAYER_INFO_KEY,
        ] {
            let (mut offer, at) = held();
            for k in [asking, Key::Enter] {
                offer.input(&key(k, false));
            }
            offer.input(&left(false, at));
            assert!(
                !offer.answered(),
                "{asking:?}: the Return went with the flush"
            );
            assert_eq!(offer.take_unclaimed(), []);
        }
    }

    /// With no active mission that isn't hidden, `_OfferOneMission`'s I
    /// arm does nothing (the count @0xa32c1-0xa32ee): no screen opens, so
    /// nothing flushes, and the offer takes the next event.
    #[test]
    fn i_then_return_with_no_mission_listed_accepts_at_the_release() {
        use crate::spaceport::offer::OfferChoice;
        let (mut offer, at) = held();
        offer.set_missions_listed(false);
        for k in [crate::ui::mission_info::MISSION_INFO_KEY, Key::Enter] {
            offer.input(&key(k, false));
        }
        offer.input(&left(false, at));
        assert!(!offer.take_info_request(), "nothing to open");
        assert_eq!(offer.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn the_venue_hears_the_choice_after_the_release_and_after_a_ticks_replay() {
        use crate::spaceport::offer::{OfferChoice, VenueOffers};
        for cancelled in [false, true] {
            let mut offers = VenueOffers::asking(crate::spaceport::offer::OfferTemplates {
                plain: Ok(fixture::stock_1016()),
                picture: Err("none".to_owned()),
            });
            let mut prompt = fixture::prompt();
            prompt.text = lines(40);
            offers.open(prompt, Rc::new(MonoMetrics), ButtonStyle::STOCK);
            let at = item_at(offers.dialog().expect("open"), SCROLL_DOWN_ITEM);
            offers.input(&left(true, at), &mut Vec::new());
            offers.input(&key(Key::Enter, false), &mut Vec::new());
            assert_eq!(offers.take_choice(), None);
            if cancelled {
                offers.cancel_pointer();
                offers.tick(Duration::ZERO, false);
            } else {
                offers.input(&left(false, at), &mut Vec::new());
            }
            assert_eq!(
                offers.take_choice(),
                Some((MissionId(141), OfferChoice::Accept)),
                "cancelled {cancelled}"
            );
        }
    }

    #[test]
    fn the_venues_offer_ticks_its_dialog_while_nothing_covers_it() {
        use crate::spaceport::offer::VenueOffers;
        let mut offers = VenueOffers::asking(crate::spaceport::offer::OfferTemplates {
            plain: Ok(fixture::stock_1016()),
            picture: Err("none".to_owned()),
        });
        let mut prompt = fixture::prompt();
        prompt.text = lines(40);
        offers.open(prompt, Rc::new(MonoMetrics), ButtonStyle::STOCK);
        let at = item_at(offers.dialog().expect("open"), SCROLL_DOWN_ITEM);
        offers.input(&left(true, at), &mut Vec::new());
        offers.tick(TICK, false);
        assert_eq!(offset(offers.dialog().expect("open")), 2.0);
    }
}
