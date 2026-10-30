//! The [`MoviePlayer`] over decoded frames: it finds the file, opens it,
//! and shows each frame on its [`MovieScreen`] once the movie's clock
//! reaches it. It decides only when frames show and when the movie ends;
//! how a movie is played (fades, skipping, the window) is
//! [`nova_view::movie::MovieStage`]'s.
//!
//! It is generic over the [`Open`] and [`Folders`] ports, so its timing
//! is tested on hand-written ones; the game's is
//! [`FfmpegPlayer`](crate::FfmpegPlayer).

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use nova_view::geometry::Bounds;
use nova_view::movie::MoviePlayer;
use nova_view::{DrawList, MovieScreen};

use crate::lookup::{self, Folders};
use crate::source::{Frame, FrameSource, Open};

/// Plays movies found by bare name in `dirs`, in order.
pub struct Player<O: Open, F: Folders> {
    open: O,
    folders: F,
    dirs: Vec<PathBuf>,
    state: RefCell<State<O::Source>>,
}

/// What the player shows and plays. Behind a `RefCell`, as the port takes
/// `&self`.
struct State<S> {
    screen: MovieScreen,
    playing: Option<Playing<S>>,
}

/// A started movie.
struct Playing<S> {
    /// The frames still to decode; `None` once they ran out or failed.
    source: Option<S>,
    /// The movie's length, when it says.
    duration: Option<Duration>,
    /// How far it has played.
    clock: Duration,
    /// The next frame, decoded but not yet due.
    next: Option<Frame>,
    /// When the last frame shown stops showing.
    last_end: Duration,
    /// Whether decoding failed: the movie then ends as its last good
    /// frame does.
    failed: bool,
    /// Whether it has ended: once it has, it stays ended.
    ended: bool,
}

impl<S: FrameSource> Playing<S> {
    /// Decodes the next frame into `next`, if there is none there and the
    /// source has more. A decode error stops the decoding.
    fn fill(&mut self) {
        if self.next.is_some() {
            return;
        }
        let Some(source) = self.source.as_mut() else {
            return;
        };
        match source.next_frame() {
            Ok(Some(frame)) => self.next = Some(frame),
            Ok(None) => self.source = None,
            Err(_) => {
                self.source = None;
                self.failed = true;
            }
        }
    }

    /// The last frame due by the clock, the frames before it skipped.
    fn due(&mut self) -> Option<Frame> {
        let mut due = None;
        loop {
            self.fill();
            match self.next.take() {
                Some(frame) if frame.at <= self.clock => {
                    self.last_end = frame.at + frame.duration.unwrap_or_default();
                    due = Some(frame);
                }
                next => {
                    self.next = next;
                    return due;
                }
            }
        }
    }

    /// When it ends: as its last good frame does once decoding failed,
    /// else its duration, else the end of its last frame once every frame
    /// is decoded, else not yet known. The source is let go only once
    /// `next` is empty, so with no source every frame has been shown.
    fn end(&self) -> Option<Duration> {
        if self.failed {
            return Some(self.last_end);
        }
        self.duration
            .or_else(|| self.source.is_none().then_some(self.last_end))
    }
}

impl<O: Open, F: Folders> Player<O, F> {
    /// A player opening movies with `open`, found through `folders` in
    /// `dirs`, the first folder first.
    pub fn with(open: O, folders: F, dirs: Vec<PathBuf>) -> Self {
        Self {
            open,
            folders,
            dirs,
            state: RefCell::new(State {
                screen: MovieScreen::new(),
                playing: None,
            }),
        }
    }
}

/// Shows `frame` on `screen`: whether it was shown.
fn show(screen: &mut MovieScreen, frame: Frame) -> bool {
    screen.show(frame.width, frame.height, frame.rgba)
}

impl<O: Open, F: Folders> MoviePlayer for Player<O, F> {
    /// Finds `file`, opens it and shows its first frame, so a started
    /// player draws at once. A file that isn't found or doesn't open, a
    /// movie of no size, or one with no frame, doesn't start, and nothing
    /// is shown.
    fn start(&self, file: &str) -> Option<(u32, u32)> {
        let mut state = self.state.borrow_mut();
        state.screen.clear();
        state.playing = None;
        let path = lookup::find(&self.folders, &self.dirs, file)?;
        let mut source = self.open.open(&path)?;
        let (width, height) = source.size();
        if width == 0 || height == 0 {
            return None;
        }
        let first = source.next_frame().ok()??;
        let last_end = first.at + first.duration.unwrap_or_default();
        if !show(&mut state.screen, first) {
            return None;
        }
        state.playing = Some(Playing {
            duration: source.duration(),
            source: Some(source),
            clock: Duration::ZERO,
            next: None,
            last_end,
            failed: false,
            ended: false,
        });
        Some((width, height))
    }

    /// Moves the clock on by `dt` and shows the last frame due by then,
    /// skipping any before it. It plays until the clock reaches the
    /// movie's end; after a decode error, that is the end of the last
    /// frame decoded.
    fn advance(&self, dt: Duration) -> bool {
        let mut state = self.state.borrow_mut();
        let State { screen, playing } = &mut *state;
        let Some(playing) = playing.as_mut().filter(|playing| !playing.ended) else {
            return false;
        };
        playing.clock += dt;
        if let Some(frame) = playing.due() {
            show(screen, frame);
        }
        playing.ended = playing.end().is_some_and(|end| playing.clock >= end);
        !playing.ended
    }

    /// Lets the movie go. Its last frame stays shown, as `StopMovie`
    /// leaves it in the window until the window closes (@0x55ec5).
    fn stop(&self) {
        self.state.borrow_mut().playing = None;
    }

    /// Draws the frame shown last stretched into `rect`; nothing before a
    /// movie has started.
    fn draw_frame(&self, rect: Bounds, list: &mut DrawList) {
        if let Some(frame) = self.state.borrow().screen.frame() {
            list.movie_frame(frame, rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use nova_view::DrawCommand;
    use nova_view::MovieFrame;
    use nova_view::geometry::Point;

    use super::*;
    use crate::source::DecodeError;

    const MS: Duration = Duration::from_millis(1);

    /// A one-pixel frame at `at` ms, lasting 100 ms, whose red is `red`.
    fn frame(at: u64, red: u8) -> Frame {
        Frame {
            at: MS * at as u32,
            duration: Some(MS * 100),
            width: 1,
            height: 1,
            rgba: vec![red, 0, 0, 255],
        }
    }

    /// A movie as a source reads it.
    #[derive(Clone)]
    struct Script {
        frames: Vec<Result<Frame, DecodeError>>,
        duration: Option<Duration>,
        size: (u32, u32),
    }

    /// A movie of `frames`, `duration` long, 1x1.
    fn movie(frames: Vec<Frame>, duration: Option<u64>) -> Script {
        Script {
            frames: frames.into_iter().map(Ok).collect(),
            duration: duration.map(|ms| MS * ms as u32),
            size: (1, 1),
        }
    }

    /// Ten frames, 100 ms apart, red 0 to 90, one second long.
    fn ten() -> Script {
        movie(
            (0..10).map(|i| frame(i * 100, i as u8 * 10)).collect(),
            Some(1000),
        )
    }

    struct FakeSource(std::vec::IntoIter<Result<Frame, DecodeError>>, Script);

    impl FrameSource for FakeSource {
        fn next_frame(&mut self) -> Result<Option<Frame>, DecodeError> {
            self.0.next().transpose()
        }
        fn duration(&self) -> Option<Duration> {
            self.1.duration
        }
        fn size(&self) -> (u32, u32) {
            self.1.size
        }
    }

    /// Opens the movies it was given, by path; each open plays one from
    /// its start.
    #[derive(Default)]
    struct FakeOpen(HashMap<PathBuf, Script>);

    impl Open for FakeOpen {
        type Source = FakeSource;
        fn open(&self, path: &Path) -> Option<FakeSource> {
            let script = self.0.get(path)?.clone();
            Some(FakeSource(script.frames.clone().into_iter(), script))
        }
    }

    /// One folder, `movies`, holding every file `FakeOpen` was given.
    struct FakeFolders(Vec<String>);

    impl Folders for FakeFolders {
        fn files(&self, dir: &Path) -> Option<Vec<String>> {
            (dir == Path::new("movies")).then(|| self.0.clone())
        }
    }

    /// A player whose `movies` folder holds `Race 1.mov`, playing
    /// `script`.
    fn player(script: Script) -> Player<FakeOpen, FakeFolders> {
        let path = Path::new("movies").join("Race 1.mov");
        Player::with(
            FakeOpen(HashMap::from([(path, script)])),
            FakeFolders(vec!["Race 1.mov".to_owned()]),
            vec![PathBuf::from("movies")],
        )
    }

    fn rect() -> Bounds {
        Bounds::at(Point::new(10.0, 20.0), 64.0, 32.0)
    }

    /// The frames drawn into `rect`.
    fn drawn(player: &impl MoviePlayer) -> Vec<(MovieFrame, Bounds)> {
        let mut list = DrawList::new();
        player.draw_frame(rect(), &mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::MovieFrame { frame, area } => Some((frame.clone(), *area)),
                _ => None,
            })
            .collect()
    }

    /// The red of the frame drawn.
    fn red(player: &impl MoviePlayer) -> Option<u8> {
        drawn(player)
            .first()
            .map(|(frame, _)| frame.image().pixels()[0])
    }

    #[test]
    fn start_shows_the_first_frame_and_returns_the_size() {
        let mut script = ten();
        script.size = (768, 384);
        let player = player(script);
        assert_eq!(player.start("Race 1.mov"), Some((768, 384)));
        assert_eq!(red(&player), Some(0));
    }

    #[test]
    fn a_frame_shows_once_the_clock_reaches_it_not_before() {
        let player = player(ten());
        player.start("Race 1.mov");
        assert!(player.advance(MS * 99));
        assert_eq!(red(&player), Some(0));
        assert!(player.advance(MS));
        assert_eq!(red(&player), Some(10));
    }

    #[test]
    fn a_long_advance_skips_to_the_last_frame_due() {
        let player = player(ten());
        player.start("Race 1.mov");
        let before = drawn(&player)[0].0.generation();
        assert!(player.advance(MS * 450));
        assert_eq!(red(&player), Some(40));
        assert_eq!(
            drawn(&player)[0].0.generation(),
            before + 1,
            "the skipped frames are never shown"
        );
    }

    #[test]
    fn it_plays_until_the_clock_reaches_the_duration_then_stays_ended() {
        let player = player(ten());
        player.start("Race 1.mov");
        assert!(player.advance(MS * 999));
        assert_eq!(red(&player), Some(90));
        assert!(!player.advance(MS));
        assert!(!player.advance(MS * 100));
        assert!(!player.advance(Duration::ZERO));
    }

    #[test]
    fn the_duration_ends_it_even_with_frames_left() {
        let player = player(movie(
            vec![frame(0, 0), frame(100, 10), frame(200, 20)],
            Some(150),
        ));
        player.start("Race 1.mov");
        assert!(player.advance(MS * 149));
        assert!(!player.advance(MS));
    }

    #[test]
    fn the_end_falls_back_to_the_last_frame_s_end_when_no_duration_is_known() {
        let player = player(movie(vec![frame(0, 0), frame(100, 10)], None));
        player.start("Race 1.mov");
        assert!(player.advance(MS * 100));
        assert_eq!(red(&player), Some(10));
        assert!(player.advance(MS * 99), "the last frame lasts 100 ms");
        assert!(!player.advance(MS));
    }

    #[test]
    fn a_last_frame_of_no_known_duration_ends_the_movie_as_it_shows() {
        let mut last = frame(100, 10);
        last.duration = None;
        let player = player(movie(vec![frame(0, 0), last], None));
        player.start("Race 1.mov");
        assert!(player.advance(MS * 99));
        assert!(!player.advance(MS));
        assert_eq!(red(&player), Some(10));
    }

    #[test]
    fn a_single_frame_movie_of_no_duration_ends_after_its_frame() {
        let player = player(movie(vec![frame(0, 0)], None));
        player.start("Race 1.mov");
        assert!(player.advance(MS * 99));
        assert!(!player.advance(MS));
    }

    #[test]
    fn a_decode_error_ends_the_movie() {
        let mut script = ten();
        script.frames[3] = Err(DecodeError("broken".to_owned()));
        let player = player(script);
        player.start("Race 1.mov");
        assert!(player.advance(MS * 200));
        assert_eq!(red(&player), Some(20));
        assert!(!player.advance(MS * 100));
        assert!(!player.advance(MS * 100));
        assert_eq!(red(&player), Some(20), "the last frame stays");
    }

    #[test]
    fn a_decode_error_on_the_first_frame_doesn_t_start() {
        let mut script = ten();
        script.frames[0] = Err(DecodeError("broken".to_owned()));
        let player = player(script);
        assert_eq!(player.start("Race 1.mov"), None);
        assert!(drawn(&player).is_empty());
        assert!(!player.advance(MS * 100));
    }

    #[test]
    fn a_source_with_no_frames_doesn_t_start() {
        let player = player(movie(Vec::new(), Some(1000)));
        assert_eq!(player.start("Race 1.mov"), None);
        assert!(drawn(&player).is_empty());
        assert!(!player.advance(MS * 100));
    }

    #[test]
    fn a_zero_size_source_doesn_t_start() {
        for size in [(0, 1), (1, 0)] {
            let mut script = ten();
            script.size = size;
            let player = player(script);
            assert_eq!(player.start("Race 1.mov"), None, "{size:?}");
            assert!(drawn(&player).is_empty());
        }
    }

    #[test]
    fn a_frame_of_the_wrong_length_doesn_t_start() {
        let mut script = ten();
        if let Ok(first) = &mut script.frames[0] {
            first.rgba.pop();
        }
        let player = player(script);
        assert_eq!(player.start("Race 1.mov"), None);
        assert!(!player.advance(MS * 100));
    }

    #[test]
    fn a_name_not_found_doesn_t_start() {
        let player = player(ten());
        assert_eq!(player.start("Race 2.mov"), None);
        assert!(drawn(&player).is_empty());
        assert!(!player.advance(MS * 100));
    }

    #[test]
    fn a_found_file_that_doesn_t_open_doesn_t_start() {
        let player = Player::with(
            FakeOpen::default(),
            FakeFolders(vec!["Race 1.mov".to_owned()]),
            vec![PathBuf::from("movies")],
        );
        assert_eq!(player.start("Race 1.mov"), None);
    }

    #[test]
    fn a_failed_start_clears_the_last_movie() {
        let player = player(ten());
        player.start("Race 1.mov");
        assert_eq!(player.start("Race 2.mov"), None);
        assert!(drawn(&player).is_empty());
        assert!(!player.advance(MS * 100));
    }

    #[test]
    fn stop_keeps_the_last_frame_drawn() {
        let player = player(ten());
        player.start("Race 1.mov");
        player.advance(MS * 300);
        player.stop();
        assert_eq!(red(&player), Some(30));
        assert!(!player.advance(MS * 100));
        assert_eq!(red(&player), Some(30));
    }

    #[test]
    fn start_again_replays_from_the_first_frame_with_a_new_generation() {
        let player = player(ten());
        player.start("Race 1.mov");
        player.advance(MS * 500);
        let before = drawn(&player)[0].0.clone();
        player.stop();
        assert_eq!(player.start("Race 1.mov"), Some((1, 1)));
        let again = drawn(&player)[0].0.clone();
        assert_eq!(again.image().pixels()[0], 0);
        assert_eq!(again.stream(), before.stream());
        assert!(again.generation() > before.generation());
        assert!(player.advance(MS * 100));
        assert_eq!(red(&player), Some(10));
    }

    #[test]
    fn draw_frame_before_any_start_draws_nothing_and_inside_the_rect_after() {
        let player = player(ten());
        assert!(drawn(&player).is_empty());
        player.start("Race 1.mov");
        let drawn = drawn(&player);
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].1, rect());
    }

    #[test]
    fn it_meets_the_movie_player_contract() {
        use nova_view::movie::fixture::check_contract;
        assert_eq!(check_contract(&player(ten()), "Race 1.mov"), Ok(()));
        assert_eq!(check_contract(&player(ten()), "Race 2.mov"), Ok(()));
        let no_duration = movie((0..10).map(|i| frame(i * 100, 0)).collect(), None);
        assert_eq!(check_contract(&player(no_duration), "Race 1.mov"), Ok(()));
    }
}
