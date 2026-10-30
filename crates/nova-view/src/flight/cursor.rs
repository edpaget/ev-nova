//! The mouse cursor sprite in flight: `spïn` 650, drawn on the pointer,
//! animating while the pointer is over something the player can target
//! ([`Session::hover`](nova_sim::Session::hover)) and fading out while the
//! mouse rests.
//!
//! The original (`_HandleMouse` @0x37fba-0x38466, run once a game frame)
//! keeps the cursor's fade, 0 to 32 (`0x3b509c`):
//!
//! - a frame where the mouse moved, or its button is down
//!   (@0x3801c-0x3803b), sets it to 32 and stamps the time
//!   (@0x38053-0x38069); here a pointer move does
//!   ([`HoverCursor::pointer`]), and each step does while a button is
//!   down ([`HoverCursor::button`]). Ours is told of a button rather than
//!   reading `_Button` live, so a release lost outside the window ends the
//!   hold at [`HoverCursor::left`];
//! - at the end of each frame (@0x38427-0x3844d) it drops a level once a
//!   second (60 `_TickCount`s) has passed since that stamp, or on any frame
//!   the byte at `0x3b5550` is set, `_keyPressedThisFrame`, which
//!   `_PlayGame` clears each frame @0x45b3b and `_HandlePlayer` sets past
//!   each key block's gates, and on its own through a jump. Those gates are
//!   [`nova_sim::key_fade`]'s, which the view reads before a frame's steps
//!   (`FlightView::cursor_keyed`, `FlightView::step_cursor`) and at a key's
//!   press (`FlightView::key_fades_cursor`, [`HoverCursor::key`]).
//!
//! The cursor is hidden, and nothing else happens that frame, when the
//! sprite failed to load (`0x3b5128`), when the game is switched out
//! (`0x3b50e0`, which `_SwitchedOut` sets @0x4f388 and `_SwitchedIn`
//! clears), when the system arrow is shown (`0x3b516c`, `_HideTheCursor`'s
//! count, 0 outside the window) or when the fade is 0 (@0x38089-0x380b9).
//! Here the pointer leaving the window, or no pointer yet, hides it.
//!
//! Over something targetable the frame advances, wrapping, once 3
//! `_TickCount`s have passed since its last advance (@0x3833f-0x3837a);
//! otherwise it rests at frame 0 (@0x38293-0x382bf). The sprite is centred
//! on the pointer (@0x382c4-0x38323) at a translucency of 32 less the fade,
//! the branch for any depth but 16 bits (@0x383e3-0x38421); the 16-bit
//! branch (@0x3837f-0x383e1), which blends at min(28, fade), is left out:
//! the engine renders one way.
//!
//! Time is counted in `_TickCount`s, sixtieths of a second, two to a
//! flight step, as the original's 30 frames a second count them.

use std::num::NonZeroU16;

use super::boxes::{FADE_LEVELS, fade_tint};
use super::catalog::Looks;
use crate::geometry::Bounds;
use crate::{Blend, DrawCommand, DrawList, Point};

/// `_TickCount`s, sixtieths of a second, in one flight step.
pub const SIXTIETHS_PER_STEP: u64 = 2;
/// `_TickCount`s between two frames of the animation (@0x3833f).
pub const FRAME_SIXTIETHS: u64 = 3;
/// `_TickCount`s the mouse rests before the cursor starts to fade
/// (@0x38434).
pub const IDLE_SIXTIETHS: u64 = 60;

/// The cursor's state (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HoverCursor {
    /// Where the pointer is, in logical units, or `None` outside the
    /// window.
    at: Option<Point>,
    /// The fade, 0 (gone) to [`FADE_LEVELS`] (whole).
    fade: u8,
    /// When the mouse last moved or pressed.
    moved_at: u64,
    /// The frame shown.
    frame: u16,
    /// When the frame last advanced.
    stepped_at: u64,
    /// The time now, in `_TickCount`s.
    clock: u64,
    /// Whether a key that fades was pressed in flight since the last step.
    keyed: bool,
    /// Whether a mouse button is down.
    button: bool,
}

impl HoverCursor {
    /// The pointer moved to `at`, or pressed a button there: the cursor
    /// shows whole, and the mouse is no longer resting.
    pub fn pointer(&mut self, at: Point) {
        self.at = Some(at);
        self.fade = FADE_LEVELS;
        self.moved_at = self.clock;
    }

    /// A key that fades was pressed in flight: the next step fades the
    /// cursor a level, however short the press.
    pub fn key(&mut self) {
        self.keyed = true;
    }

    /// A mouse button went down or came up: while one is down, each step
    /// shows the cursor whole and counts as the mouse moving.
    pub fn button(&mut self, down: bool) {
        self.button = down;
    }

    /// The pointer left the window: the cursor is hidden until it comes
    /// back, and any button counts as let go, as its release won't come.
    pub fn left(&mut self) {
        self.at = None;
        self.button = false;
    }

    /// Where the pointer is while the cursor shows: `None` when it is
    /// hidden, and nothing is hovered.
    #[must_use]
    pub fn shown_at(&self) -> Option<Point> {
        self.at.filter(|_| self.fade > 0)
    }

    /// The fade, 0 to [`FADE_LEVELS`].
    #[must_use]
    pub fn fade(&self) -> u8 {
        self.fade
    }

    /// The frame shown.
    #[must_use]
    pub fn frame(&self) -> u16 {
        self.frame
    }

    /// One flight step: whether the pointer is over something targetable
    /// (`hovering`), how many frames the cursor's sheet holds (`None` when
    /// it could not be read) and whether this frame's pass set the byte
    /// (`keyed`), which fades it as a key pressed since the last step
    /// ([`HoverCursor::key`]) does. A held button's stamp runs first,
    /// whatever `frames` says, as `_HandleMouse`'s head comes before its
    /// return for a sprite that failed to load; with `None` the step does
    /// nothing more. See the module docs.
    pub fn step(&mut self, hovering: bool, frames: Option<NonZeroU16>, keyed: bool) {
        self.clock += SIXTIETHS_PER_STEP;
        let keyed = std::mem::take(&mut self.keyed) || keyed;
        if self.button {
            self.fade = FADE_LEVELS;
            self.moved_at = self.clock;
        }
        let Some(frames) = frames else {
            return;
        };
        if self.shown_at().is_none() {
            return;
        }
        if !hovering {
            self.frame = 0;
        } else if self.clock >= self.stepped_at + FRAME_SIXTIETHS {
            self.frame = (self.frame + 1) % frames.get();
            self.stepped_at = self.clock;
        }
        if keyed || self.clock > self.moved_at + IDLE_SIXTIETHS {
            self.fade = self.fade.saturating_sub(1);
        }
    }

    /// Draws the cursor on the pointer, at its fade, when it shows and its
    /// sheet was read, clipped to `play_area`. The cursor is a sprite of
    /// the target layer, and the sprite world ends 194 short of the
    /// window's right edge (`_SetupSpriteWorld`'s `subw $0xc2` @0x21b66),
    /// while `_HandleMouse` places it on the pointer wherever it is
    /// (`_SWSetSpriteLocation` @0x38323): over the status bar only the part
    /// still over the play area shows.
    pub fn draw(&self, list: &mut DrawList, looks: &Looks, play_area: Bounds) {
        let (Some(at), Some(sheet)) = (self.shown_at(), looks.cursor_sheet()) else {
            return;
        };
        let frame = self.frame % sheet.frames.get();
        list.clipped(
            play_area,
            DrawCommand::Sprite {
                image: sheet.image.frame(frame),
                center: at,
                tint: fade_tint(FADE_LEVELS - self.fade.min(FADE_LEVELS)),
                blend: Blend::Normal,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;
    use crate::ImageKey;
    use crate::flight::catalog::EffectSheet;

    const FOUR: Option<NonZeroU16> = NonZeroU16::new(4);

    fn at() -> Point {
        Point::new(100.0, 200.0)
    }

    /// A cursor over the pointer at [`at`], just moved.
    fn moved() -> HoverCursor {
        let mut cursor = HoverCursor::default();
        cursor.pointer(at());
        cursor
    }

    /// The frames shown over `steps` steps, hovering or not.
    fn frames(cursor: &mut HoverCursor, hovering: bool, steps: usize) -> Vec<u16> {
        (0..steps)
            .map(|_| {
                cursor.step(hovering, FOUR, false);
                cursor.frame()
            })
            .collect()
    }

    fn looks(sheet: Option<Result<EffectSheet, String>>) -> Looks {
        Looks {
            cursor: sheet,
            ..Looks::default()
        }
    }

    fn sheet() -> EffectSheet {
        EffectSheet {
            frame_width: 0,
            frame_height: 0,
            image: crate::SheetImage::Rled(651),
            frames: NonZeroU16::new(4).expect("non-zero"),
        }
    }

    /// A play area 800 wide, short of a status bar.
    const PLAY_AREA: Bounds = Bounds {
        min: Point::new(0.0, 0.0),
        max: Point::new(800.0, 600.0),
    };

    /// What the cursor draws, each command seen through the play area's
    /// clip, which every one must have.
    fn drawn(cursor: &HoverCursor, looks: &Looks) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        cursor.draw(&mut list, looks, PLAY_AREA);
        list.iter()
            .map(|command| match command {
                DrawCommand::Clipped { clip, command } => {
                    assert_eq!(*clip, PLAY_AREA, "clipped to the play area");
                    (**command).clone()
                }
                other => panic!("{other:?} not clipped"),
            })
            .collect()
    }

    #[test]
    fn the_frame_advances_every_three_sixtieths_while_hovering_and_wraps() {
        let mut cursor = moved();
        // Steps are two sixtieths apart: the frame advances on every
        // second, wrapping after the fourth frame.
        assert_eq!(
            frames(&mut cursor, true, 10),
            [0, 1, 1, 2, 2, 3, 3, 0, 0, 1]
        );
    }

    #[test]
    fn the_frame_rests_at_zero_off_a_ship() {
        let mut cursor = moved();
        assert_eq!(frames(&mut cursor, true, 4), [0, 1, 1, 2]);
        assert_eq!(frames(&mut cursor, false, 2), [0, 0]);
        assert_eq!(frames(&mut cursor, true, 1), [1], "due at once");
    }

    #[test]
    fn moving_the_pointer_shows_it_whole_and_it_fades_a_level_a_step_after_a_second_idle() {
        let mut cursor = HoverCursor::default();
        assert_eq!(cursor.shown_at(), None, "no pointer yet");
        cursor.pointer(at());
        assert_eq!((cursor.fade(), cursor.shown_at()), (32, Some(at())));
        for _ in 0..30 {
            cursor.step(false, FOUR, false);
        }
        assert_eq!(cursor.fade(), 32, "a second: not past it");
        cursor.step(false, FOUR, false);
        assert_eq!(cursor.fade(), 31);
        cursor.step(false, FOUR, false);
        assert_eq!(cursor.fade(), 30);
        cursor.pointer(at());
        assert_eq!(cursor.fade(), 32, "moved again");
        cursor.step(false, FOUR, false);
        assert_eq!(cursor.fade(), 32);
    }

    #[test]
    fn a_flight_key_fades_it_a_level_a_step_however_lately_it_moved() {
        let mut cursor = moved();
        cursor.step(false, FOUR, true);
        cursor.step(false, FOUR, true);
        assert_eq!(cursor.fade(), 30);
    }

    #[test]
    fn a_key_pressed_fades_it_a_level_at_the_next_step_alone() {
        let mut cursor = moved();
        cursor.key();
        cursor.step(false, FOUR, false);
        cursor.step(false, FOUR, false);
        assert_eq!(cursor.fade(), 31);
    }

    #[test]
    fn a_button_held_keeps_it_whole_however_long_the_mouse_rests() {
        let mut cursor = moved();
        cursor.button(true);
        for _ in 0..40 {
            cursor.step(false, FOUR, false);
            assert_eq!(cursor.fade(), 32);
        }
    }

    #[test]
    fn a_button_held_restamps_it_each_step_so_a_key_fades_it_only_one_level() {
        let mut cursor = moved();
        cursor.button(true);
        for _ in 0..3 {
            cursor.step(false, FOUR, true);
            assert_eq!(cursor.fade(), 31);
        }
    }

    #[test]
    fn a_button_held_brings_a_faded_out_cursor_back() {
        let mut cursor = moved();
        for _ in 0..32 {
            cursor.step(false, FOUR, true);
        }
        assert_eq!(cursor.shown_at(), None);
        cursor.button(true);
        cursor.step(false, FOUR, false);
        assert_eq!(cursor.fade(), 32);
    }

    #[test]
    fn with_no_sheet_a_button_held_still_stamps_the_mouse_as_moved() {
        let mut cursor = moved();
        cursor.button(true);
        for _ in 0..40 {
            cursor.step(false, None, false);
        }
        cursor.button(false);
        cursor.step(false, FOUR, false);
        assert_eq!(cursor.fade(), 32, "it moved a step ago");
    }

    #[test]
    fn released_it_goes_idle_a_second_after_the_last_step_the_button_was_down() {
        let mut cursor = moved();
        cursor.button(true);
        for _ in 0..40 {
            cursor.step(false, FOUR, false);
        }
        cursor.button(false);
        for _ in 0..30 {
            cursor.step(false, FOUR, false);
        }
        assert_eq!(cursor.fade(), 32, "a second: not past it");
        cursor.step(false, FOUR, false);
        assert_eq!(cursor.fade(), 31);
    }

    #[test]
    fn leaving_the_window_lets_go_of_the_button() {
        let mut cursor = moved();
        cursor.button(true);
        cursor.left();
        cursor.pointer(at());
        for _ in 0..32 {
            cursor.step(false, FOUR, true);
        }
        assert_eq!(cursor.fade(), 0, "no hold came back with the pointer");
    }

    #[test]
    fn a_faded_out_cursor_hovers_nothing_draws_nothing_and_stays_out() {
        let mut cursor = moved();
        for _ in 0..32 {
            cursor.step(true, FOUR, true);
        }
        assert_eq!(cursor.fade(), 0);
        assert_eq!(cursor.shown_at(), None);
        let frame = cursor.frame();
        cursor.step(true, FOUR, true);
        cursor.step(true, FOUR, true);
        assert_eq!((cursor.fade(), cursor.frame()), (0, frame), "held");
        assert_eq!(drawn(&cursor, &looks(Some(Ok(sheet())))), []);
    }

    #[test]
    fn a_cursor_whose_sheet_was_not_read_draws_nothing() {
        let cursor = moved();
        assert_eq!(drawn(&cursor, &looks(None)), []);
        assert_eq!(drawn(&cursor, &looks(Some(Err("no".to_owned())))), []);
    }

    #[test]
    fn stepping_without_a_sheet_does_nothing() {
        let mut cursor = moved();
        for _ in 0..40 {
            cursor.step(true, None, true);
        }
        assert_eq!((cursor.frame(), cursor.fade()), (0, 32));
    }

    #[test]
    fn it_draws_its_frame_on_the_pointer_at_its_fade() {
        let mut cursor = moved();
        cursor.step(true, FOUR, true);
        cursor.step(true, FOUR, true);
        assert_eq!((cursor.frame(), cursor.fade()), (1, 30));
        assert_eq!(
            drawn(&cursor, &looks(Some(Ok(sheet())))),
            [DrawCommand::Sprite {
                image: ImageKey::sprite(651, 1),
                center: at(),
                tint: Color::rgba(255, 255, 255, 239),
                blend: crate::Blend::Normal,
            }]
        );
        cursor.pointer(at());
        let [DrawCommand::Sprite { tint, .. }] = drawn(&cursor, &looks(Some(Ok(sheet()))))[..]
        else {
            panic!("one sprite");
        };
        assert_eq!(tint.a, 255, "whole");
    }

    #[test]
    fn the_frame_wraps_to_a_smaller_sheet_when_drawn() {
        let mut cursor = moved();
        frames(&mut cursor, true, 6);
        assert_eq!(cursor.frame(), 3);
        let two = Some(Ok(EffectSheet {
            frame_width: 0,
            frame_height: 0,
            image: crate::SheetImage::Rled(651),
            frames: NonZeroU16::new(2).expect("non-zero"),
        }));
        let [DrawCommand::Sprite { image, .. }] = drawn(&cursor, &looks(two))[..] else {
            panic!("one sprite");
        };
        assert_eq!(image, ImageKey::sprite(651, 1));
    }

    #[test]
    fn over_the_status_bar_it_is_still_drawn_cut_off_at_the_play_areas_edge() {
        let mut cursor = HoverCursor::default();
        let over_bar = Point::new(810.0, 200.0);
        cursor.pointer(over_bar);
        let mut list = DrawList::new();
        cursor.draw(&mut list, &looks(Some(Ok(sheet()))), PLAY_AREA);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Clipped {
                clip: PLAY_AREA,
                command: Box::new(DrawCommand::Sprite {
                    image: ImageKey::sprite(651, 0),
                    center: over_bar,
                    tint: Color::WHITE,
                    blend: crate::Blend::Normal,
                }),
            }]
        );
    }

    #[test]
    fn leaving_the_window_hides_it_until_the_pointer_comes_back() {
        let mut cursor = moved();
        cursor.left();
        assert_eq!(cursor.shown_at(), None);
        assert_eq!(drawn(&cursor, &looks(Some(Ok(sheet())))), []);
        cursor.pointer(Point::new(5.0, 6.0));
        assert_eq!(cursor.shown_at(), Some(Point::new(5.0, 6.0)));
    }
}
