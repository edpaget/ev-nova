//! The placeholder screen: a picture, a grid of animated sprites, a label,
//! a line and dots, all from the game data. It stands in until the real
//! screens arrive and lets the renderer be stressed by hand.

use std::time::Duration;

use nova_data::GameData;
use nova_data::graphics::RLED;
use nova_render::ImageSource;
use nova_rsrc::ResType;
use nova_view::{Color, DrawList, ImageKey, ImageKind, Input, Key, Point, Screen, ScreenAction};

/// The `PICT` resource type.
const PICT: ResType = ResType::new(*b"PICT");

/// How many sprites the screen starts with.
pub const DEFAULT_COUNT: u32 = 300;
/// The most sprites Up can ask for.
pub const MAX_COUNT: u32 = 4096;
/// Sprite animation frames per second.
const FPS: u128 = 30;
/// Sprites per grid row, and the grid's spacing in logical units.
const COLUMNS: u32 = 32;
const ROWS: u32 = 30;
const SPACING: (f32, f32) = (32.0, 24.0);

/// What the placeholder shows: a picture's `PICT` ID, and a sprite's
/// `rlëD` ID with its frame count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaceholderContent {
    /// The picture, if any.
    pub picture: Option<i16>,
    /// The sprite sheet and how many frames it has, if any.
    pub sprite: Option<(i16, u16)>,
}

impl PlaceholderContent {
    /// The first `PICT` that decodes and the first `rlëD` with at least
    /// one frame that decodes, by ascending ID.
    #[must_use]
    pub fn from_data(data: &GameData) -> Self {
        let decodes = |kind, id| data.frames(kind, id).ok().filter(|f| !f.is_empty());
        let picture = data
            .ids(PICT)
            .iter()
            .copied()
            .find(|&id| decodes(ImageKind::Pict, id).is_some());
        let sprite = data.ids(RLED).iter().find_map(|&id| {
            let frames = decodes(ImageKind::Rled, id)?;
            Some((id, frames.len() as u16))
        });
        Self { picture, sprite }
    }
}

/// The single placeholder screen.
#[derive(Clone, Debug)]
pub struct Placeholder {
    content: PlaceholderContent,
    count: u32,
    elapsed: Duration,
}

impl Placeholder {
    /// The screen showing `content`, with [`DEFAULT_COUNT`] sprites.
    #[must_use]
    pub fn new(content: PlaceholderContent) -> Self {
        Self {
            content,
            count: DEFAULT_COUNT,
            elapsed: Duration::ZERO,
        }
    }

    /// How many sprites it draws.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.count
    }
}

impl Screen for Placeholder {
    /// Up doubles the sprite count and Down halves it, within 1 to
    /// [`MAX_COUNT`].
    fn input(&mut self, input: &Input) -> ScreenAction {
        match input {
            Input::Key {
                key: Key::Up,
                pressed: true,
            } => self.count = (self.count * 2).min(MAX_COUNT),
            Input::Key {
                key: Key::Down,
                pressed: true,
            } => self.count = (self.count / 2).max(1),
            _ => {}
        }
        ScreenAction::None
    }

    fn tick(&mut self, dt: Duration) {
        self.elapsed += dt;
    }

    fn draw(&self, list: &mut DrawList) {
        if let Some(id) = self.content.picture {
            list.picture(ImageKey::picture(id), Point::new(0.0, 0.0));
        }
        if let Some((id, frames @ 1..)) = self.content.sprite {
            let first = self.elapsed.as_nanos() * FPS / 1_000_000_000;
            for i in 0..self.count {
                let frame = (first + u128::from(i)) % u128::from(frames);
                let col = i % COLUMNS;
                let row = i / COLUMNS % ROWS;
                let center =
                    Point::new(16.0 + col as f32 * SPACING.0, 16.0 + row as f32 * SPACING.1);
                list.sprite(ImageKey::sprite(id, frame as u16), center, Color::WHITE);
            }
        }
        let grey = Color::rgba(128, 128, 128, 255);
        let yellow = Color::rgba(255, 255, 0, 255);
        list.text(
            format!("EV Nova: {} sprites", self.count),
            Point::new(8.0, 744.0),
            16.0,
            None,
            Color::WHITE,
        )
        .line(Point::new(0.0, 736.0), Point::new(1024.0, 736.0), 1.0, grey);
        for (x, y) in [(4.0, 4.0), (1020.0, 4.0), (4.0, 764.0), (1020.0, 764.0)] {
            list.dot(Point::new(x, y), 3.0, yellow);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_view::{DrawCommand, Input, Key, Screen, ScreenAction};

    use super::*;

    const FULL: PlaceholderContent = PlaceholderContent {
        picture: Some(128),
        sprite: Some((200, 3)),
    };

    fn press(key: Key) -> Input {
        Input::Key { key, pressed: true }
    }

    fn drawn(screen: &Placeholder) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    /// Each sprite's (image, centre).
    fn sprites(screen: &Placeholder) -> Vec<(ImageKey, Point)> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, center, .. } => Some((image, center)),
                _ => None,
            })
            .collect()
    }

    fn kinds(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::Sprite { .. } => "sprite",
                DrawCommand::Picture { .. } => "picture",
                DrawCommand::StretchedPicture { .. } => "stretched picture",
                DrawCommand::Text { .. } => "text",
                DrawCommand::Line { .. } => "line",
                DrawCommand::Dot { .. } => "dot",
            })
            .collect()
    }

    #[test]
    fn it_draws_the_picture_sprites_label_line_and_dots_in_order() {
        let screen = Placeholder::new(FULL);
        assert_eq!(screen.count(), 300);
        let commands = drawn(&screen);
        assert_eq!(commands.len(), 1 + 300 + 1 + 1 + 4);
        let mut expected = vec!["picture"];
        expected.extend(["sprite"; 300]);
        expected.extend(["text", "line", "dot", "dot", "dot", "dot"]);
        assert_eq!(kinds(&commands), expected);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: ImageKey::picture(128),
                top_left: Point::new(0.0, 0.0)
            }
        );
        assert_eq!(
            commands[301],
            DrawCommand::Text {
                text: "EV Nova: 300 sprites".to_owned(),
                origin: Point::new(8.0, 744.0),
                size: 16.0,
                wrap_width: None,
                color: Color::WHITE
            }
        );
        let DrawCommand::Line { from, to, .. } = commands[302] else {
            panic!("{:?}", commands[302]);
        };
        assert_eq!(
            (from, to),
            (Point::new(0.0, 736.0), Point::new(1024.0, 736.0))
        );
        let DrawCommand::Sprite { tint, .. } = commands[1] else {
            panic!("{:?}", commands[1]);
        };
        assert_eq!(tint, Color::WHITE);
    }

    #[test]
    fn sprites_sit_on_a_grid_with_staggered_frames() {
        let sprites = sprites(&Placeholder::new(FULL));
        assert_eq!(
            sprites[0],
            (ImageKey::sprite(200, 0), Point::new(16.0, 16.0))
        );
        assert_eq!(
            sprites[1],
            (ImageKey::sprite(200, 1), Point::new(48.0, 16.0))
        );
        assert_eq!(
            sprites[2],
            (ImageKey::sprite(200, 2), Point::new(80.0, 16.0))
        );
        assert_eq!(
            sprites[3],
            (ImageKey::sprite(200, 0), Point::new(112.0, 16.0))
        );
        assert_eq!(sprites[31].1, Point::new(1008.0, 16.0));
        assert_eq!(sprites[32].1, Point::new(16.0, 40.0));
        assert_eq!(sprites[65].1, Point::new(48.0, 64.0));
    }

    #[test]
    fn rows_wrap_back_to_the_top_after_thirty() {
        let mut screen = Placeholder::new(FULL);
        screen.input(&press(Key::Up));
        screen.input(&press(Key::Up));
        let sprites = sprites(&screen);
        assert_eq!(sprites.len(), 1200);
        assert_eq!(sprites[959].1, Point::new(1008.0, 712.0));
        assert_eq!(sprites[960].1, Point::new(16.0, 16.0));
    }

    #[test]
    fn frames_advance_at_thirty_per_second() {
        let mut screen = Placeholder::new(FULL);
        screen.tick(Duration::from_millis(33));
        assert_eq!(sprites(&screen)[0].0, ImageKey::sprite(200, 0));
        screen.tick(Duration::from_millis(1));
        assert_eq!(sprites(&screen)[0].0, ImageKey::sprite(200, 1));
        assert_eq!(sprites(&screen)[2].0, ImageKey::sprite(200, 0));
        screen.tick(Duration::from_secs(1));
        // 1.034 s is frame 31, and 31 % 3 = 1.
        assert_eq!(sprites(&screen)[0].0, ImageKey::sprite(200, 1));
    }

    #[test]
    fn up_doubles_and_down_halves_the_count_within_bounds() {
        let mut screen = Placeholder::new(FULL);
        assert_eq!(screen.input(&press(Key::Up)), ScreenAction::None);
        assert_eq!(screen.count(), 600);
        for _ in 0..4 {
            screen.input(&press(Key::Up));
        }
        assert_eq!(screen.count(), 4096);
        screen.input(&press(Key::Down));
        assert_eq!(screen.count(), 2048);
        for _ in 0..12 {
            screen.input(&press(Key::Down));
        }
        assert_eq!(screen.count(), 1);
        screen.input(&press(Key::Up));
        assert_eq!(screen.count(), 2);
        assert_eq!(sprites(&screen).len(), 2);
    }

    #[test]
    fn releases_and_other_keys_change_nothing() {
        let mut screen = Placeholder::new(FULL);
        let release = Input::Key {
            key: Key::Up,
            pressed: false,
        };
        assert_eq!(screen.input(&release), ScreenAction::None);
        assert_eq!(screen.input(&press(Key::Left)), ScreenAction::None);
        assert_eq!(
            screen.input(&Input::PointerMoved(Point::new(1.0, 1.0))),
            ScreenAction::None
        );
        assert_eq!(screen.count(), 300);
    }

    #[test]
    fn the_label_shows_the_count() {
        let mut screen = Placeholder::new(FULL);
        screen.input(&press(Key::Down));
        let label = drawn(&screen)
            .into_iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            });
        assert_eq!(label.as_deref(), Some("EV Nova: 150 sprites"));
    }

    #[test]
    fn a_sheet_without_frames_draws_no_sprites() {
        let screen = Placeholder::new(PlaceholderContent {
            picture: None,
            sprite: Some((200, 0)),
        });
        assert_eq!(sprites(&screen), []);
    }

    #[test]
    fn without_content_it_draws_only_the_label_line_and_dots() {
        let screen = Placeholder::new(PlaceholderContent::default());
        assert_eq!(
            kinds(&drawn(&screen)),
            ["text", "line", "dot", "dot", "dot", "dot"]
        );
    }

    mod from_data {
        use std::io;
        use std::path::Path;

        use nova_data::GameData;
        use nova_data::graphics::RLED;
        use nova_data::graphics::fixture::{PictBuilder, RledBuilder};
        use nova_data::store::fs::{DirLister, EntryKind, Listing};
        use nova_rsrc::fixture::ForkBuilder;
        use nova_rsrc::{Fork, ForkReader, ResType};

        use super::super::*;

        const PICT: ResType = ResType::new(*b"PICT");

        /// One data file, `/data/Nova Data`, holding a fork.
        struct OneFile(Vec<u8>);

        impl DirLister for OneFile {
            fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
                Ok(vec![Listing {
                    name: "Nova Data".into(),
                    kind: EntryKind::File,
                }])
            }
        }

        impl ForkReader for OneFile {
            fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
                Ok((fork == Fork::Data).then(|| self.0.clone()))
            }
        }

        fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
            let fork = resources
                .iter()
                .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                    fork.resource(*ty, *id, None, data)
                })
                .build()
                .bytes;
            let file = OneFile(fork);
            GameData::load(&file, &file, Path::new("/data"), None).unwrap()
        }

        fn sheet(frames: u16) -> Vec<u8> {
            (0..frames)
                .fold(RledBuilder::new(1, 1), |sheet, _| {
                    sheet.frame(|f| f.line().pixels(&[0x7C00]))
                })
                .build()
        }

        #[test]
        fn it_picks_the_first_picture_and_sprite_that_decode() {
            let data = store(&[
                (PICT, 128, vec![0; 3]),
                (PICT, 129, PictBuilder::new([0, 0, 2, 2]).end().build()),
                (PICT, 130, PictBuilder::new([0, 0, 4, 4]).end().build()),
                (RLED, 200, vec![0; 3]),
                (RLED, 201, sheet(3)),
                (RLED, 202, sheet(2)),
            ]);
            assert_eq!(
                PlaceholderContent::from_data(&data),
                PlaceholderContent {
                    picture: Some(129),
                    sprite: Some((201, 3)),
                }
            );
        }

        #[test]
        fn a_sheet_without_frames_is_skipped() {
            let data = store(&[(RLED, 200, sheet(0)), (RLED, 201, sheet(1))]);
            assert_eq!(PlaceholderContent::from_data(&data).sprite, Some((201, 1)));
        }

        #[test]
        fn nothing_decodable_is_no_content() {
            let data = store(&[(PICT, 128, vec![0; 3]), (RLED, 200, vec![0; 3])]);
            assert_eq!(
                PlaceholderContent::from_data(&data),
                PlaceholderContent::default()
            );
        }
    }
}
