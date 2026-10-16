//! A `dësc`'s QuickTime movie, played as `_PlayQTMovie` (@0x55957 in the
//! `EV Nova` executable) plays it: the rules are [`MovieStage`]'s, and the
//! frames come through the [`MoviePlayer`] port.
//!
//! What `_PlayQTMovie` does, once the movie has opened:
//!
//! - **Cinematic** (`Flags` 0x0004): the screen fades out to black
//!   (`_FadeScreenOut` @0x55a6e), the cursor is hidden (@0x55a73), a black
//!   window over the whole main window is opened (@0x55b2d-0x55c32), and
//!   the screen fades back in (`_FadeScreenIn` @0x55dce) before the movie
//!   starts. As it closes, the screen fades out again (@0x55eb8), the
//!   cursor shows (@0x55ebd), the window goes (@0x55ec5-0x55ecd) and the
//!   screen fades in on what is beneath (@0x55edb). Each fade takes 1.0 s
//!   and is synchronous, so the movie does not run during one.
//! - **Plain**: no fade, and the window is the movie's own box, painted
//!   black, centred on the main window (@0x55b43, `_CenterWindow`
//!   @0x55ba2).
//! - **Double size** (`Flags` 0x0002): the box, moved to the origin, is
//!   made (2w + 1) x (2h + 1) (@0x55a8b-0x55b1f).
//! - The box is scaled down to fit the window's width, then its height,
//!   keeping its shape, each side truncated (@0x55cb8-0x55d5d), and centred
//!   in it by `_CenterRect` (@0x55d76, @0xc93dd), each half truncated.
//! - **The loop** (@0x55de0-0x55e9f) ends as the movie ends, or when the
//!   mouse button, Space, Return or keypad Enter is down (`Button`,
//!   `KeyCheck` 0x31, 0x24 and 0x4c): the movie stops and the queued mouse
//!   and key-down events are flushed (`FlushEvents(0xf)`). It reads no
//!   events itself (`WaitNextEvent` with mask 0), so what else is pressed
//!   stays queued: after a movie that ends on its own, it reaches what
//!   comes next ([`MovieStage::take_replay`]). The keys are read as held,
//!   so one still down from before the movie (the Return that closed a
//!   text) skips it at once. The engine sees a key or button only as it
//!   goes down or up, so one held from before skips the movie as it is let
//!   go: later than the original by however long it is still held.
//!
//! It plays no sound. Its early returns (the preference, a file that is
//! not found or does not open, @0x5597c-0x55a5f) come before any fade, so
//! a movie that cannot start ends at once with nothing drawn.
//!
//! Opening and decoding the file, and finding it (by its bare name in the
//! Plug-Ins folder, then the Nova Files folder, @0x559d2 and @0x559ff), are
//! the [`MoviePlayer`]'s: the engine has no QuickTime decoder, and wires a
//! player that plays nothing.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use nova_sim::DescMovie;

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::fade::{DEATH_FADE_FOR, ScreenFade};
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key, MouseButton};
use crate::system::camera::VIEW_SIZE;

/// A movie file's frames: opened, decoded and drawn (`NewMovieFromDataRef`,
/// `MoviesTask`). It decides nothing of how the original plays a movie.
pub trait MoviePlayer {
    /// Opens and starts `file`, a bare file name: its frames' natural size
    /// in pixels, or `None` when it cannot play (no such file, or nothing
    /// to decode it).
    fn start(&self, file: &str) -> Option<(u32, u32)>;
    /// Moves the movie on by `dt`: whether it is still playing
    /// (`IsMovieDone`).
    fn advance(&self, dt: Duration) -> bool;
    /// Stops the movie and lets it go (`StopMovie`, `DisposeMovie`).
    fn stop(&self);
    /// Draws the current frame stretched into `rect`.
    fn draw_frame(&self, rect: Bounds, list: &mut DrawList);
}

/// The player movies are played through, and whether they play at all:
/// the "QuickTime Movies" preference, a set `*0x3b501c + 7` turning them
/// off (`_PlayQTMovie` @0x5597c, and each call site). Clones share the
/// preference, so turning it on or off reaches every screen holding one.
/// [`Movies::none`] plays nothing.
#[derive(Clone)]
pub struct Movies {
    player: Option<Rc<dyn MoviePlayer>>,
    on: Rc<Cell<bool>>,
}

impl std::fmt::Debug for Movies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Movies")
            .field("player", &self.player.is_some())
            .field("on", &self.on.get())
            .finish()
    }
}

impl Default for Movies {
    fn default() -> Self {
        Self::none()
    }
}

impl Movies {
    /// Movies played through `player`, while `on`.
    #[must_use]
    pub fn new(player: Rc<dyn MoviePlayer>, on: bool) -> Self {
        Self {
            player: Some(player),
            on: Rc::new(Cell::new(on)),
        }
    }

    /// No movie plays.
    #[must_use]
    pub fn none() -> Self {
        Self {
            player: None,
            on: Rc::new(Cell::new(false)),
        }
    }

    /// Turns movies on or off, for every clone.
    pub fn set_on(&self, on: bool) {
        self.on.set(on);
    }

    /// Whether movies play.
    #[must_use]
    pub fn on(&self) -> bool {
        self.on.get() && self.player.is_some()
    }

    /// A stage playing `movie`, when movies are on.
    #[must_use]
    pub fn stage(&self, movie: &DescMovie) -> Option<MovieStage> {
        let player = self.player.as_ref().filter(|_| self.on.get())?;
        Some(MovieStage::new(movie.clone(), Rc::clone(player)))
    }
}

/// How long each of a cinematic movie's fades takes: the 1.0 s of
/// `_FadeScreenOut` and `_FadeScreenIn` (@0x5399, @0x542d).
pub const MOVIE_FADE_FOR: Duration = DEATH_FADE_FOR;

/// The main window a movie is placed in: the whole screen.
const MAIN_WINDOW: (i32, i32) = (VIEW_SIZE.0 as i32, VIEW_SIZE.1 as i32);

/// Where a movie stands.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// Not started yet.
    Waiting,
    /// Cinematic: the screen fading out to black.
    FadingOut(ScreenFade),
    /// Cinematic: the screen fading in on the black window.
    FadingIn(ScreenFade),
    /// The movie running.
    Playing,
    /// Cinematic: the stopped movie fading out to black.
    ClosingOut(ScreenFade),
    /// Cinematic: the screen fading in on what is beneath.
    ClosingIn(ScreenFade),
    /// Over.
    Ended,
}

/// A `dësc`'s movie played over whatever is beneath, as `_PlayQTMovie`
/// plays it (see the [module docs](self)), on display time. It starts as
/// [`MovieStage::begin`] is called, and is [`ended`](MovieStage::ended)
/// once it is over.
#[derive(Clone)]
pub struct MovieStage {
    movie: DescMovie,
    player: Rc<dyn MoviePlayer>,
    phase: Phase,
    /// Where the window is, black.
    window: Bounds,
    /// Where the frames are drawn.
    frame: Bounds,
    /// The input that stays queued for what comes next.
    replay: Vec<Input>,
}

impl std::fmt::Debug for MovieStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MovieStage")
            .field("movie", &self.movie)
            .field("phase", &self.phase)
            .finish_non_exhaustive()
    }
}

/// `inner`, `size` wide and high, centred in `outer` as `_CenterRect`
/// centres it: each offset the outer half-size less the inner one, both
/// halves truncated.
fn centred(outer: (i32, i32, i32, i32), size: (i32, i32)) -> (i32, i32, i32, i32) {
    let (left, top, width, height) = outer;
    let x = left + (width / 2 - size.0 / 2);
    let y = top + (height / 2 - size.1 / 2);
    (x, y, size.0, size.1)
}

/// `size` scaled down to fit `room`: to its width, keeping its shape, when
/// it is wider, then to its height when it is taller, each side truncated
/// (@0x55cb8-0x55d5d).
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn fitted(size: (i32, i32), room: (i32, i32)) -> (i32, i32) {
    let scale =
        |size: (i32, i32), by: f32| ((size.0 as f32 * by) as i32, (size.1 as f32 * by) as i32);
    let mut size = size;
    if room.0 < size.0 {
        size = scale(size, room.0 as f32 / size.0 as f32);
    }
    if room.1 < size.1 {
        size = scale(size, room.1 as f32 / size.1 as f32);
    }
    size
}

#[allow(clippy::cast_precision_loss)]
fn bounds((x, y, width, height): (i32, i32, i32, i32)) -> Bounds {
    Bounds::at(Point::new(x as f32, y as f32), width as f32, height as f32)
}

/// The full screen, which the cinematic window and its fades cover.
fn screen() -> Bounds {
    bounds((0, 0, MAIN_WINDOW.0, MAIN_WINDOW.1))
}

impl MovieStage {
    /// `movie`, played through `player` once it begins.
    #[must_use]
    pub fn new(movie: DescMovie, player: Rc<dyn MoviePlayer>) -> Self {
        Self {
            movie,
            player,
            phase: Phase::Waiting,
            window: Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0),
            frame: Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0),
            replay: Vec::new(),
        }
    }

    /// The movie played.
    #[must_use]
    pub fn movie(&self) -> &DescMovie {
        &self.movie
    }

    /// Starts the movie, once: one the player cannot start ends at once,
    /// with nothing drawn; a cinematic one fades the screen out first.
    pub fn begin(&mut self) {
        if self.phase != Phase::Waiting {
            return;
        }
        let Some((width, height)) = self.player.start(&self.movie.file) else {
            self.phase = Phase::Ended;
            return;
        };
        let natural = (
            i32::try_from(width).unwrap_or(i32::MAX / 4),
            i32::try_from(height).unwrap_or(i32::MAX / 4),
        );
        let size = if self.movie.double_size {
            (2 * natural.0 + 1, 2 * natural.1 + 1)
        } else {
            natural
        };
        let main = (0, 0, MAIN_WINDOW.0, MAIN_WINDOW.1);
        let window = if self.movie.cinematic {
            main
        } else {
            centred(main, size)
        };
        let frame = centred(window, fitted(size, (window.2, window.3)));
        self.window = bounds(window);
        self.frame = bounds(frame);
        self.phase = if self.movie.cinematic {
            Phase::FadingOut(ScreenFade::to_black(MOVIE_FADE_FOR))
        } else {
            Phase::Playing
        };
    }

    /// Whether it has begun.
    #[must_use]
    pub fn begun(&self) -> bool {
        self.phase != Phase::Waiting
    }

    /// Whether it is over.
    #[must_use]
    pub fn ended(&self) -> bool {
        self.phase == Phase::Ended
    }

    /// Whether the cursor is hidden: from the end of a cinematic movie's
    /// first fade to the end of its third (@0x55a73, @0x55ebd).
    #[must_use]
    pub fn hides_cursor(&self) -> bool {
        matches!(
            self.phase,
            Phase::FadingIn(_) | Phase::Playing | Phase::ClosingOut(_)
        ) && self.movie.cinematic
    }

    /// The input it left queued, taken once: what was pressed while it
    /// played, other than the keys and button that skip it, and what came
    /// after a skip's flush.
    pub fn take_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.replay)
    }

    /// Stops the movie and closes it: a cinematic one fades out and back
    /// in, a plain one ends at once.
    fn close(&mut self) {
        self.player.stop();
        self.phase = if self.movie.cinematic {
            Phase::ClosingOut(ScreenFade::to_black(MOVIE_FADE_FOR))
        } else {
            Phase::Ended
        };
    }

    /// Moves it on by `dt`: a fade runs, or the movie does, closing as it
    /// ends.
    pub fn tick(&mut self, dt: Duration) {
        match &mut self.phase {
            Phase::Waiting | Phase::Ended => {}
            Phase::FadingOut(fade) => {
                fade.advance(dt);
                if fade.done() {
                    self.phase = Phase::FadingIn(ScreenFade::from_black(MOVIE_FADE_FOR));
                }
            }
            Phase::FadingIn(fade) => {
                fade.advance(dt);
                if fade.done() {
                    self.phase = Phase::Playing;
                }
            }
            Phase::Playing => {
                if !self.player.advance(dt) {
                    self.close();
                }
            }
            Phase::ClosingOut(fade) => {
                fade.advance(dt);
                if fade.done() {
                    self.phase = Phase::ClosingIn(ScreenFade::from_black(MOVIE_FADE_FOR));
                }
            }
            Phase::ClosingIn(fade) => {
                fade.advance(dt);
                if fade.done() {
                    self.phase = Phase::Ended;
                }
            }
        }
    }

    /// Takes `input`. While the movie runs, the mouse button or Space or
    /// Return going down or up skips it, the queued input flushed; other
    /// keys and clicks stay queued for what comes next, as they do during
    /// the fades. A pointer's moves go nowhere.
    pub fn input(&mut self, input: &Input) {
        if self.phase == Phase::Ended {
            return;
        }
        let skips = matches!(
            input,
            Input::Key {
                key: Key::Space | Key::Enter,
                ..
            } | Input::PointerButton {
                button: MouseButton::Left,
                ..
            }
        );
        if self.phase == Phase::Playing && skips {
            self.replay.clear();
            self.close();
            return;
        }
        if matches!(
            input,
            Input::Key { .. } | Input::Text(_) | Input::PointerButton { .. }
        ) {
            self.replay.push(*input);
        }
    }

    /// What it shows over what is beneath: the fade out over it; the
    /// black window, and the frame in it, while the movie runs or stops;
    /// the fades over them, and the last fade over what is beneath again.
    pub fn draw(&self, list: &mut DrawList) {
        match self.phase {
            Phase::Waiting | Phase::Ended => {}
            Phase::FadingOut(fade) | Phase::ClosingIn(fade) => fade.draw(list),
            Phase::FadingIn(fade) => {
                fill_rect(list, screen(), Color::BLACK);
                fade.draw(list);
            }
            Phase::Playing => self.draw_movie(list),
            Phase::ClosingOut(fade) => {
                self.draw_movie(list);
                fade.draw(list);
            }
        }
    }

    /// The black window, and the frame in it.
    fn draw_movie(&self, list: &mut DrawList) {
        fill_rect(list, self.window, Color::BLACK);
        self.player.draw_frame(self.frame, list);
    }
}

#[cfg(test)]
pub(crate) mod fixture {
    //! A movie player for tests.

    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use super::MoviePlayer;
    use crate::color::Color;
    use crate::draw::{DrawList, fill_rect};
    use crate::geometry::Bounds;

    /// What a [`MockMoviePlayer`] was asked.
    #[derive(Clone, Debug, PartialEq)]
    pub(crate) enum Call {
        /// `start(file)`.
        Start(String),
        /// `advance`.
        Advance,
        /// `stop`.
        Stop,
        /// `draw_frame(rect)`.
        Draw(Bounds),
    }

    /// The colour a [`MockMoviePlayer`] draws its frame in.
    pub(crate) const FRAME: Color = Color::rgba(1, 2, 3, 255);

    /// A player whose movies are `size` and play for `advances` advances,
    /// or, with no size, cannot start; it logs every call.
    pub(crate) struct MockMoviePlayer {
        size: Option<(u32, u32)>,
        advances: u32,
        advanced: RefCell<u32>,
        pub(crate) log: RefCell<Vec<Call>>,
    }

    impl MockMoviePlayer {
        /// A player whose movies are `size` and end on their `advances`th
        /// advance.
        pub(crate) fn playing(size: (u32, u32), advances: u32) -> Rc<Self> {
            Rc::new(Self {
                size: Some(size),
                advances,
                advanced: RefCell::new(0),
                log: RefCell::new(Vec::new()),
            })
        }

        /// A player that cannot start any movie.
        pub(crate) fn refusing() -> Rc<Self> {
            Rc::new(Self {
                size: None,
                advances: 0,
                advanced: RefCell::new(0),
                log: RefCell::new(Vec::new()),
            })
        }

        /// The calls, `Draw`s left out.
        pub(crate) fn calls(&self) -> Vec<Call> {
            self.log
                .borrow()
                .iter()
                .filter(|call| !matches!(call, Call::Draw(_)))
                .cloned()
                .collect()
        }

        /// The rects of the frames drawn.
        pub(crate) fn drawn(&self) -> Vec<Bounds> {
            self.log
                .borrow()
                .iter()
                .filter_map(|call| match call {
                    Call::Draw(rect) => Some(*rect),
                    _ => None,
                })
                .collect()
        }
    }

    impl MoviePlayer for MockMoviePlayer {
        fn start(&self, file: &str) -> Option<(u32, u32)> {
            self.log.borrow_mut().push(Call::Start(file.to_owned()));
            *self.advanced.borrow_mut() = 0;
            self.size
        }

        fn advance(&self, _dt: Duration) -> bool {
            self.log.borrow_mut().push(Call::Advance);
            let mut advanced = self.advanced.borrow_mut();
            *advanced += 1;
            *advanced < self.advances
        }

        fn stop(&self) {
            self.log.borrow_mut().push(Call::Stop);
        }

        fn draw_frame(&self, rect: Bounds, list: &mut DrawList) {
            self.log.borrow_mut().push(Call::Draw(rect));
            fill_rect(list, rect, FRAME);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::fixture::{Call, FRAME, MockMoviePlayer};
    use super::*;
    use crate::draw::DrawCommand;

    const TICK: Duration = Duration::from_millis(100);

    fn movie(flags: u16) -> DescMovie {
        DescMovie::from_desc("Race.mov", flags).expect("named")
    }

    fn staged(player: &Rc<MockMoviePlayer>, flags: u16) -> MovieStage {
        let shared: Rc<dyn MoviePlayer> = Rc::clone(player) as Rc<dyn MoviePlayer>;
        MovieStage::new(movie(flags), shared)
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Bounds {
        Bounds::at(Point::new(x, y), width, height)
    }

    /// The filled rects drawn, each with its colour.
    fn fills(stage: &MovieStage) -> Vec<(Bounds, Color)> {
        let mut list = DrawList::new();
        stage.draw(&mut list);
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } => Some((
                    Bounds::at(
                        Point::new(from.x, from.y - width / 2.0),
                        to.x - from.x,
                        width,
                    ),
                    color,
                )),
                _ => None,
            })
            .collect()
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    fn click(pressed: bool) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(10.0, 10.0),
        }
    }

    /// Ticks `stage` by `total` in [`TICK`] steps.
    fn run(stage: &mut MovieStage, total: Duration) {
        let mut left = total;
        while !left.is_zero() {
            let step = left.min(TICK);
            stage.tick(step);
            left -= step;
        }
    }

    #[test]
    fn movies_make_stages_only_while_on_and_clones_share_the_switch() {
        let player = MockMoviePlayer::playing((160, 120), 3);
        let movies = Movies::new(Rc::clone(&player) as Rc<dyn MoviePlayer>, true);
        assert!(movies.on());
        let stage = movies.stage(&movie(0x0001)).expect("on");
        assert_eq!(stage.movie(), &movie(0x0001));
        let other = movies.clone();
        other.set_on(false);
        assert!(!movies.on(), "shared");
        assert!(movies.stage(&movie(0)).is_none());
        movies.set_on(true);
        assert!(other.stage(&movie(0)).is_some());
        assert!(player.calls().is_empty(), "nothing started");
        let none = Movies::default();
        none.set_on(true);
        assert!(!none.on(), "no player");
        assert!(none.stage(&movie(0)).is_none());
        assert_eq!(format!("{none:?}"), "Movies { player: false, on: true }");
    }

    #[test]
    fn it_starts_its_file_once_as_it_begins() {
        let player = MockMoviePlayer::playing((160, 120), 3);
        let mut stage = staged(&player, 0);
        assert!(player.calls().is_empty(), "not before it begins");
        assert!(!stage.begun());
        assert!(fills(&stage).is_empty(), "nothing drawn before it begins");
        stage.begin();
        stage.begin();
        assert!(stage.begun());
        assert_eq!(player.calls(), [Call::Start("Race.mov".to_owned())]);
    }

    #[test]
    fn a_movie_that_cannot_start_ends_at_once_and_draws_nothing() {
        let player = MockMoviePlayer::refusing();
        let mut stage = staged(&player, 0x0004);
        stage.begin();
        assert!(stage.ended());
        assert!(fills(&stage).is_empty());
        assert!(!stage.hides_cursor(), "no fade, no hidden cursor");
    }

    #[test]
    fn a_plain_movie_ends_as_its_movie_does_and_stops_it() {
        let player = MockMoviePlayer::playing((160, 120), 3);
        let mut stage = staged(&player, 0);
        stage.begin();
        stage.tick(TICK);
        stage.tick(TICK);
        assert!(!stage.ended());
        stage.tick(TICK);
        assert!(stage.ended(), "on the advance that finds it done");
        assert_eq!(
            player.calls(),
            [
                Call::Start("Race.mov".to_owned()),
                Call::Advance,
                Call::Advance,
                Call::Advance,
                Call::Stop,
            ]
        );
    }

    #[test]
    fn a_plain_movie_does_not_fade_and_draws_its_black_window_centred_under_its_frame() {
        let player = MockMoviePlayer::playing((160, 120), 3);
        let mut stage = staged(&player, 0);
        stage.begin();
        assert!(!stage.hides_cursor());
        let box_at = rect(432.0, 324.0, 160.0, 120.0);
        assert_eq!(
            fills(&stage),
            [(box_at, Color::BLACK), (box_at, FRAME)],
            "the movie's own box, centred on the screen (_CenterWindow @0x55ba2)"
        );
        assert_eq!(player.drawn(), [box_at]);
    }

    #[test]
    fn a_double_size_movies_frame_is_twice_its_size_plus_one_and_centred() {
        let player = MockMoviePlayer::playing((160, 120), 3);
        let mut stage = staged(&player, 0x0002);
        stage.begin();
        let doubled = rect(352.0, 264.0, 321.0, 241.0);
        assert_eq!(
            fills(&stage),
            [(doubled, Color::BLACK), (doubled, FRAME)],
            "(2w + 1) x (2h + 1) (@0x55ad8-0x55adc), the halves truncated"
        );
    }

    #[test]
    fn a_movie_wider_or_taller_than_the_screen_is_scaled_down_to_fit_keeping_its_shape() {
        let player = MockMoviePlayer::playing((2048, 768), 3);
        let mut stage = staged(&player, 0x0004);
        stage.begin();
        run(&mut stage, MOVIE_FADE_FOR * 2);
        assert_eq!(
            player.drawn().last().copied(),
            None,
            "nothing drawn during the fades"
        );
        let _ = fills(&stage);
        assert_eq!(
            player.drawn(),
            [rect(0.0, 192.0, 1024.0, 384.0)],
            "to the width, then centred"
        );
        let player = MockMoviePlayer::playing((300, 1536), 3);
        let mut stage = staged(&player, 0x0004);
        stage.begin();
        run(&mut stage, MOVIE_FADE_FOR * 2);
        let _ = fills(&stage);
        assert_eq!(
            player.drawn(),
            [rect(437.0, 0.0, 150.0, 768.0)],
            "to the height"
        );
    }

    #[test]
    fn a_movie_of_a_size_too_large_to_count_still_fits_the_screen() {
        for (size, wide) in [((u32::MAX, 100), true), ((100, u32::MAX), false)] {
            let player = MockMoviePlayer::playing(size, 3);
            let mut stage = staged(&player, 0x0004);
            stage.begin();
            run(&mut stage, MOVIE_FADE_FOR * 2);
            let _ = fills(&stage);
            let frame = player.drawn()[0];
            if wide {
                assert!(frame.width() > 1000.0, "{frame:?}");
            } else {
                assert!(frame.height() > 700.0, "{frame:?}");
            }
        }
    }

    #[test]
    fn it_debugs_as_its_movie_and_where_it_stands() {
        let player = MockMoviePlayer::playing((160, 120), 3);
        let debugged = format!("{:?}", staged(&player, 0));
        assert!(
            debugged.starts_with("MovieStage { movie: DescMovie {"),
            "{debugged}"
        );
        assert!(debugged.contains("phase: Waiting"), "{debugged}");
    }

    #[test]
    fn a_plain_movie_larger_than_the_screen_has_a_window_as_large() {
        let player = MockMoviePlayer::playing((2000, 100), 3);
        let mut stage = staged(&player, 0);
        stage.begin();
        assert_eq!(player.drawn(), Vec::<Bounds>::new(), "nothing drawn yet");
        let _ = fills(&stage);
        assert_eq!(
            player.drawn(),
            [rect(-488.0, 334.0, 2000.0, 100.0)],
            "its window is its box, so it never shrinks"
        );
    }

    #[test]
    fn a_cinematic_movie_fades_out_to_a_black_screen_then_in_before_it_runs() {
        let player = MockMoviePlayer::playing((160, 120), 3);
        let mut stage = staged(&player, 0x0004);
        stage.begin();
        assert!(!stage.hides_cursor(), "not until the first fade is over");
        assert_eq!(fills(&stage), [], "the fade starts clear");
        run(&mut stage, Duration::from_millis(500));
        assert_eq!(
            fills(&stage),
            [(screen(), Color::rgba(0, 0, 0, 128))],
            "half way to black over what is beneath"
        );
        run(&mut stage, Duration::from_millis(500));
        assert!(stage.hides_cursor(), "_HideTheCursor @0x55a73");
        assert_eq!(
            fills(&stage),
            [(screen(), Color::BLACK), (screen(), Color::BLACK)],
            "the black window, under the fade still black"
        );
        run(&mut stage, MOVIE_FADE_FOR);
        assert_eq!(
            player.calls(),
            [Call::Start("Race.mov".to_owned())],
            "the movie does not run during the fades"
        );
        let frame = rect(432.0, 324.0, 160.0, 120.0);
        assert_eq!(
            fills(&stage),
            [(screen(), Color::BLACK), (frame, FRAME)],
            "the frame centred in the black window over the whole screen"
        );
        stage.tick(TICK);
        assert_eq!(player.calls().last(), Some(&Call::Advance));
    }

    #[test]
    fn a_cinematic_movie_fades_out_and_in_again_as_it_closes() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut stage = staged(&player, 0x0004);
        stage.begin();
        run(&mut stage, MOVIE_FADE_FOR * 2);
        stage.tick(TICK);
        assert_eq!(player.calls().last(), Some(&Call::Stop), "stopped first");
        assert!(!stage.ended());
        assert!(stage.hides_cursor());
        run(&mut stage, Duration::from_millis(500));
        let frame = rect(432.0, 324.0, 160.0, 120.0);
        assert_eq!(
            fills(&stage),
            [
                (screen(), Color::BLACK),
                (frame, FRAME),
                (screen(), Color::rgba(0, 0, 0, 128))
            ],
            "the stopped frame fading to black"
        );
        run(&mut stage, Duration::from_millis(500));
        assert!(!stage.hides_cursor(), "_ShowTheCursor @0x55ebd");
        assert_eq!(
            fills(&stage),
            [(screen(), Color::BLACK)],
            "the window gone, black over what is beneath"
        );
        run(&mut stage, Duration::from_millis(900));
        assert!(!stage.ended());
        run(&mut stage, Duration::from_millis(100));
        assert!(stage.ended(), "after the last fade back in");
        assert_eq!(fills(&stage), []);
        assert_eq!(
            player
                .calls()
                .iter()
                .filter(|call| **call == Call::Advance)
                .count(),
            1,
            "the stopped movie never advances again"
        );
    }

    #[test]
    fn each_fade_takes_a_second() {
        assert_eq!(MOVIE_FADE_FOR, Duration::from_secs(1));
    }

    #[test]
    fn a_click_space_or_return_skips_the_running_movie() {
        for skip in [click(true), press(Key::Space), press(Key::Enter)] {
            let player = MockMoviePlayer::playing((160, 120), 30);
            let mut stage = staged(&player, 0);
            stage.begin();
            stage.tick(TICK);
            stage.input(&skip);
            assert!(stage.ended(), "{skip:?}");
            assert_eq!(player.calls().last(), Some(&Call::Stop), "{skip:?}");
        }
    }

    #[test]
    fn a_skip_key_or_button_let_go_skips_it_as_one_held_from_before() {
        for held in [click(false), release(Key::Space), release(Key::Enter)] {
            let player = MockMoviePlayer::playing((160, 120), 30);
            let mut stage = staged(&player, 0);
            stage.begin();
            stage.input(&held);
            assert!(stage.ended(), "{held:?}");
        }
    }

    #[test]
    fn a_skip_flushes_what_was_queued() {
        let player = MockMoviePlayer::playing((160, 120), 30);
        let mut stage = staged(&player, 0x0004);
        stage.begin();
        stage.input(&press(Key::Escape));
        run(&mut stage, MOVIE_FADE_FOR * 2);
        stage.input(&press(Key::Char('d')));
        stage.input(&press(Key::Space));
        stage.input(&press(Key::Char('x')));
        assert_eq!(
            stage.take_replay(),
            [press(Key::Char('x'))],
            "FlushEvents(0xf) @0x55e9f; what comes after waits for what is next"
        );
    }

    #[test]
    fn other_input_waits_for_what_comes_after_a_movie_that_ends_on_its_own() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut stage = staged(&player, 0);
        stage.begin();
        stage.input(&press(Key::Escape));
        stage.input(&Input::Text('e'));
        stage.input(&Input::PointerMoved(Point::new(3.0, 4.0)));
        stage.input(&Input::PointerButton {
            button: MouseButton::Right,
            pressed: true,
            at: Point::new(3.0, 4.0),
        });
        stage.input(&release(Key::Escape));
        stage.tick(TICK);
        stage.tick(TICK);
        assert!(stage.ended(), "the keys did not skip it");
        assert_eq!(
            stage.take_replay(),
            [
                press(Key::Escape),
                Input::Text('e'),
                Input::PointerButton {
                    button: MouseButton::Right,
                    pressed: true,
                    at: Point::new(3.0, 4.0),
                },
                release(Key::Escape),
            ],
            "the pointer's move goes nowhere"
        );
        assert!(stage.take_replay().is_empty(), "taken once");
    }

    #[test]
    fn a_skip_key_during_the_fades_waits_and_does_not_skip() {
        let player = MockMoviePlayer::playing((160, 120), 30);
        let mut stage = staged(&player, 0x0004);
        stage.begin();
        stage.input(&press(Key::Space));
        assert!(!stage.ended());
        assert!(matches!(stage.phase, Phase::FadingOut(_)));
        assert_eq!(stage.take_replay(), [press(Key::Space)]);
    }

    #[test]
    fn an_ended_movie_takes_no_input() {
        let player = MockMoviePlayer::refusing();
        let mut stage = staged(&player, 0);
        stage.begin();
        stage.input(&press(Key::Char('a')));
        assert!(stage.take_replay().is_empty());
        stage.tick(TICK);
        assert!(stage.ended());
    }
}
