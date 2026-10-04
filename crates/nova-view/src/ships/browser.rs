//! The ship browser screen: pages through every ship by keyboard, drawing
//! the selected ship rotating through every frame of its sheet with its
//! glow and lights layers on top, and its name (in Charcoal), stats and
//! description in a column beside it.
//!
//! The screen reads ships only through the [`ShipCatalog`] port, once at
//! start-up and once per selection; drawing never resolves anything.

use std::time::Duration;

use super::catalog::{SheetInfo, ShipCatalog, ShipEntry, ShipId};
use crate::draw::crossed_box;
use crate::time::ticks;
use crate::{Color, DrawList, Font, ImageKey, Input, Key, Point, Screen, ScreenAction};

/// Where the ship's centre goes, in the 1024x768 logical space. The base,
/// glow and lights frames are all centred here: a layer frame is often
/// larger than the base's, and centring is how Nova lines them up.
pub const SHIP_CENTER: Point = Point::new(240.0, 320.0);
/// The side of the placeholder box drawn when the sprite cannot be shown.
pub const PLACEHOLDER_SIZE: f32 = 128.0;
/// The left edge of the text column.
pub const TEXT_LEFT: f32 = 496.0;
/// The ship's name: where it goes and its size.
const NAME_TOP: f32 = 56.0;
const NAME_SIZE: f32 = 32.0;
/// The stat lines (and any layer errors after them): where the first goes,
/// the distance between them and their size.
const STATS_TOP: f32 = 120.0;
const LINE_SPACING: f32 = 28.0;
const STATS_SIZE: f32 = 20.0;
/// The description: where it starts, its size and its wrap width.
const DESCRIPTION_TOP: f32 = 300.0;
const DESCRIPTION_SIZE: f32 = 16.0;
/// The description's (and the record error's) wrap width.
pub const DESCRIPTION_WRAP: f32 = 496.0;
/// The message under the placeholder: its left edge, size and wrap width.
const SPRITE_ERROR_LEFT: f32 = 48.0;
const SPRITE_ERROR_WRAP: f32 = 384.0;
const MESSAGE_SIZE: f32 = 16.0;
/// The footer: where it goes and its size.
const FOOTER: Point = Point::new(16.0, 736.0);
const FOOTER_SIZE: f32 = 16.0;

// The ship and its placeholder stay left of the text column, and the
// wrapped text stays on screen.
const _: () = assert!(SHIP_CENTER.x + PLACEHOLDER_SIZE < TEXT_LEFT);
const _: () = assert!(TEXT_LEFT + DESCRIPTION_WRAP <= 1024.0);

/// The placeholder's colour.
const PLACEHOLDER: Color = Color::rgba(128, 128, 128, 255);

/// The ship browser.
#[derive(Clone, Debug)]
pub struct ShipBrowser<C> {
    catalog: C,
    /// Every ship, ascending; read once.
    ids: Vec<ShipId>,
    /// The index of the selected ship in `ids`.
    selected: usize,
    /// The selected ship, resolved when it was selected.
    current: Option<ShipEntry>,
    /// Time since the selection.
    elapsed: Duration,
}

impl<C: ShipCatalog> ShipBrowser<C> {
    /// The browser over `catalog`, showing its first ship.
    pub fn new(catalog: C) -> Self {
        let ids = catalog.ship_ids();
        let current = ids.first().map(|&id| catalog.ship(id));
        Self {
            catalog,
            ids,
            selected: 0,
            current,
            elapsed: Duration::ZERO,
        }
    }

    /// The selected ship, if there are any.
    #[must_use]
    pub fn selected(&self) -> Option<ShipId> {
        self.current.as_ref().map(|ship| ship.id)
    }

    /// The selected ship, as the catalog resolved it.
    #[must_use]
    pub fn current(&self) -> Option<&ShipEntry> {
        self.current.as_ref()
    }

    /// The base sprite's frame being shown, if there is one.
    #[must_use]
    pub fn frame(&self) -> Option<u16> {
        let sheet = self.current.as_ref()?.sprite.as_ref().ok()?;
        Some(self.frame_of(*sheet))
    }

    /// The frame of `sheet` for the time since the selection: one frame per
    /// tick of Nova's 1/30 s clock, wrapping round.
    fn frame_of(&self, sheet: SheetInfo) -> u16 {
        (ticks(self.elapsed) % u128::from(sheet.frames.get())) as u16
    }

    /// Selects the next ship (or the previous one), wrapping round,
    /// resolves it and restarts its animation.
    fn select(&mut self, forward: bool) {
        let count = self.ids.len();
        if count == 0 {
            return;
        }
        self.selected = if forward {
            (self.selected + 1) % count
        } else {
            (self.selected + count - 1) % count
        };
        self.current = Some(self.catalog.ship(self.ids[self.selected]));
        self.elapsed = Duration::ZERO;
    }

    fn draw_ship(&self, ship: &ShipEntry, list: &mut DrawList) {
        match &ship.sprite {
            Ok(base) => {
                // Stock layers have as many frames as the base, or one set
                // of rotations under a multi-set base; frames are stored set
                // by set, so wrapping the base frame picks the same frame or
                // the same rotation.
                let frame = self.frame_of(*base);
                list.sprite(
                    ImageKey::sprite(base.image_id, frame),
                    SHIP_CENTER,
                    Color::WHITE,
                );
                for layer in [&ship.glow, &ship.lights] {
                    if let Some(Ok(layer)) = layer {
                        let frame = frame % layer.frames.get();
                        list.sprite(
                            ImageKey::sprite(layer.image_id, frame),
                            SHIP_CENTER,
                            Color::WHITE,
                        );
                    }
                }
            }
            Err(message) => draw_placeholder(message, list),
        }
    }
}

/// A grey box with its diagonals where the ship would be, and why.
fn draw_placeholder(message: &str, list: &mut DrawList) {
    crossed_box(list, SHIP_CENTER, PLACEHOLDER_SIZE, PLACEHOLDER);
    let bottom = SHIP_CENTER.y + PLACEHOLDER_SIZE / 2.0;
    list.text(
        format!("Sprite unavailable: {message}"),
        Point::new(SPRITE_ERROR_LEFT, bottom + LINE_SPACING),
        MESSAGE_SIZE,
        Some(SPRITE_ERROR_WRAP),
        Color::ERROR,
    );
}

/// The name, the stats (or why not), any layer errors and the description.
fn draw_text(ship: &ShipEntry, list: &mut DrawList) {
    list.text_in(
        Font::Charcoal,
        ship.name.clone(),
        Point::new(TEXT_LEFT, NAME_TOP),
        NAME_SIZE,
        Some(DESCRIPTION_WRAP),
        Color::WHITE,
    );
    let mut lines: Vec<(String, Color)> = match &ship.stats {
        Ok(stats) => vec![
            (format!("Cost: {} credits", stats.cost), Color::WHITE),
            (format!("Speed: {}", stats.speed), Color::WHITE),
            (format!("Armour: {}", stats.armor), Color::WHITE),
            (format!("Shields: {}", stats.shield), Color::WHITE),
        ],
        Err(message) => vec![(format!("Record unavailable: {message}"), Color::ERROR)],
    };
    for (name, layer) in [("Glow", &ship.glow), ("Lights", &ship.lights)] {
        if let Some(Err(message)) = layer {
            lines.push((format!("{name} unavailable: {message}"), Color::ERROR));
        }
    }
    for (at, (line, color)) in lines.into_iter().enumerate() {
        let top = STATS_TOP + at as f32 * LINE_SPACING;
        list.text(
            line,
            Point::new(TEXT_LEFT, top),
            STATS_SIZE,
            Some(DESCRIPTION_WRAP),
            color,
        );
    }
    let description = match &ship.description {
        Ok(Some(text)) => Some((text.clone(), Color::WHITE)),
        Ok(None) => None,
        Err(message) => Some((format!("Description unavailable: {message}"), Color::ERROR)),
    };
    if let Some((text, color)) = description {
        list.text(
            text,
            Point::new(TEXT_LEFT, DESCRIPTION_TOP),
            DESCRIPTION_SIZE,
            Some(DESCRIPTION_WRAP),
            color,
        );
    }
}

impl<C: ShipCatalog> Screen for ShipBrowser<C> {
    /// Right or Down shows the next ship and Left or Up the previous one,
    /// wrapping round; holding the key keeps stepping, once per key repeat.
    /// Everything else, and every key release, is ignored; the browser never
    /// quits (Escape is the app's).
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key, pressed: true, ..
        } = input
        {
            match key {
                Key::Right | Key::Down => self.select(true),
                Key::Left | Key::Up => self.select(false),
                _ => {}
            }
        }
        ScreenAction::None
    }

    fn tick(&mut self, dt: Duration) {
        self.elapsed += dt;
    }

    fn draw(&self, list: &mut DrawList) {
        let Some(ship) = &self.current else {
            list.text(
                "No ships in the game data",
                Point::new(TEXT_LEFT, NAME_TOP),
                NAME_SIZE,
                None,
                Color::WHITE,
            );
            return;
        };
        self.draw_ship(ship, list);
        draw_text(ship, list);
        list.text(
            format!(
                "Ship {} of {} (shïp {}) - Left/Right to browse",
                self.selected + 1,
                self.ids.len(),
                ship.id.0
            ),
            FOOTER,
            FOOTER_SIZE,
            None,
            Color::DIM,
        );
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroU16;
    use std::time::Duration;

    use super::*;
    use crate::ships::catalog::{SheetInfo, ShipCatalog, ShipEntry, ShipId, ShipStats};
    use crate::{
        Color, DrawCommand, DrawList, Font, ImageKey, Input, Key, MouseButton, Point, Screen,
        ScreenAction,
    };

    /// Canned entries in ID order; records every `ship` call.
    #[derive(Default)]
    struct FakeCatalog {
        entries: Vec<ShipEntry>,
        calls: RefCell<Vec<ShipId>>,
    }

    impl FakeCatalog {
        fn with(entries: Vec<ShipEntry>) -> Self {
            Self {
                entries,
                calls: RefCell::default(),
            }
        }

        fn calls(&self) -> Vec<ShipId> {
            self.calls.borrow().clone()
        }
    }

    impl ShipCatalog for FakeCatalog {
        fn ship_ids(&self) -> Vec<ShipId> {
            self.entries.iter().map(|entry| entry.id).collect()
        }

        fn ship(&self, id: ShipId) -> ShipEntry {
            self.calls.borrow_mut().push(id);
            self.entries
                .iter()
                .find(|entry| entry.id == id)
                .cloned()
                .expect("a canned ship")
        }
    }

    fn sheet(image_id: i16, frames: u16) -> SheetInfo {
        SheetInfo {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    const STATS: ShipStats = ShipStats {
        cost: 10_000,
        speed: 300,
        armor: 20,
        shield: 50,
    };

    /// Ship `id` named "Ship {id}" with a 5-frame sprite, `rlëD` `id` x 10,
    /// and no layers or description.
    fn entry(id: i16) -> ShipEntry {
        ShipEntry {
            id: ShipId(id),
            name: format!("Ship {id}"),
            stats: Ok(STATS),
            description: Ok(None),
            sprite: Ok(sheet(id * 10, 5)),
            glow: None,
            lights: None,
        }
    }

    fn three_ships() -> FakeCatalog {
        FakeCatalog::with(vec![entry(128), entry(129), entry(130)])
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    /// A key held down past the OS key-repeat delay.
    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    /// One frame period: 1/30 s.
    fn tick() -> Duration {
        Duration::from_secs(1) / 30
    }

    fn drawn(screen: &impl Screen) -> DrawList {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list
    }

    fn texts(list: &DrawList) -> Vec<String> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn text_color(list: &DrawList, starting: &str) -> Color {
        list.iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text.starts_with(starting) => Some(*color),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text starting {starting:?}"))
    }

    fn text_origin(list: &DrawList, starting: &str) -> Point {
        list.iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if text.starts_with(starting) => {
                    Some(*origin)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text starting {starting:?}"))
    }

    fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite {
                    image,
                    center,
                    tint,
                    ..
                } => {
                    assert_eq!(*tint, Color::WHITE);
                    Some((*image, *center))
                }
                _ => None,
            })
            .collect()
    }

    fn lines(list: &DrawList) -> usize {
        list.iter()
            .filter(|command| matches!(command, DrawCommand::Line { .. }))
            .count()
    }

    #[test]
    fn it_starts_on_the_first_ship_and_resolves_only_that_one() {
        let catalog = three_ships();
        let browser = ShipBrowser::new(&catalog);
        assert_eq!(browser.selected(), Some(ShipId(128)));
        assert_eq!(browser.current(), Some(&entry(128)));
        assert_eq!(browser.frame(), Some(0));
        assert_eq!(catalog.calls(), [ShipId(128)]);
        drawn(&browser);
        assert_eq!(catalog.calls(), [ShipId(128)], "drawing resolves nothing");
    }

    #[test]
    fn right_and_left_step_through_the_ships_and_wrap() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        let mut visited = Vec::new();
        for key in [Key::Right, Key::Right, Key::Right, Key::Left, Key::Left] {
            assert_eq!(browser.input(&press(key)), ScreenAction::None);
            visited.push(browser.selected().expect("a ship").0);
        }
        assert_eq!(visited, [129, 130, 128, 130, 129]);
        let resolved: Vec<i16> = catalog.calls().iter().map(|id| id.0).collect();
        assert_eq!(resolved, [128, 129, 130, 128, 130, 129]);
    }

    #[test]
    fn holding_an_arrow_keeps_stepping() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        let mut visited = Vec::new();
        for input in [press(Key::Right), held(Key::Right), held(Key::Left)] {
            assert_eq!(browser.input(&input), ScreenAction::None);
            visited.push(browser.selected().expect("a ship").0);
        }
        assert_eq!(visited, [129, 130, 129]);
    }

    #[test]
    fn down_and_up_mirror_right_and_left() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.input(&press(Key::Up));
        assert_eq!(browser.selected(), Some(ShipId(130)));
        browser.input(&press(Key::Down));
        browser.input(&press(Key::Down));
        assert_eq!(browser.selected(), Some(ShipId(129)));
        assert_eq!(catalog.calls().len(), 4);
    }

    #[test]
    fn changing_ship_restarts_the_animation() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.tick(tick() * 3);
        assert_eq!(browser.frame(), Some(3));
        browser.input(&press(Key::Right));
        assert_eq!(browser.frame(), Some(0));
        browser.tick(tick() * 2);
        browser.input(&press(Key::Left));
        assert_eq!(browser.frame(), Some(0));
    }

    #[test]
    fn releases_other_keys_and_the_pointer_change_nothing() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.tick(tick() * 2);
        let others = [
            Input::Key {
                key: Key::Right,
                pressed: false,
                repeat: false,
            },
            Input::Key {
                key: Key::Left,
                pressed: false,
                repeat: false,
            },
            press(Key::Enter),
            press(Key::Escape),
            press(Key::Space),
            press(Key::Char('d')),
            press(Key::Other),
            Input::PointerMoved(Point::new(1.0, 2.0)),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at: Point::new(1.0, 2.0),
            },
        ];
        for input in others {
            assert_eq!(browser.input(&input), ScreenAction::None, "{input:?}");
        }
        assert_eq!(browser.selected(), Some(ShipId(128)));
        assert_eq!(browser.frame(), Some(2));
        assert_eq!(catalog.calls(), [ShipId(128)]);
    }

    #[test]
    fn ticking_a_thirtieth_of_a_second_visits_every_frame_in_turn() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        let mut frames = vec![browser.frame().expect("a frame")];
        for _ in 0..5 {
            browser.tick(tick());
            frames.push(browser.frame().expect("a frame"));
        }
        assert_eq!(frames, [0, 1, 2, 3, 4, 0]);
    }

    #[test]
    fn partial_ticks_add_up() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        let step = Duration::from_millis(20);
        browser.tick(step);
        assert_eq!(browser.frame(), Some(0));
        browser.tick(step);
        assert_eq!(browser.frame(), Some(1));
        // To a nanosecond short of two whole periods (66,666,666 ns).
        browser.tick(Duration::from_nanos(26_666_665));
        assert_eq!(browser.frame(), Some(1));
        browser.tick(Duration::from_nanos(1));
        assert_eq!(browser.frame(), Some(2));
    }

    #[test]
    fn a_long_run_stays_in_step_with_whole_frames() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        for _ in 0..3001 {
            browser.tick(tick());
        }
        assert_eq!(browser.frame(), Some(1));
    }

    /// Ship 128 with a 3-set, 3-frame-per-set base (9 frames), a one-set
    /// glow (3 frames) and full lights (9 frames).
    fn layered() -> ShipEntry {
        ShipEntry {
            sprite: Ok(sheet(1000, 9)),
            glow: Some(Ok(sheet(1100, 3))),
            lights: Some(Ok(sheet(1200, 9))),
            ..entry(128)
        }
    }

    #[test]
    fn the_base_glow_and_lights_share_one_centre_in_draw_order() {
        let catalog = FakeCatalog::with(vec![layered()]);
        let mut browser = ShipBrowser::new(&catalog);
        browser.tick(tick() * 7);
        let list = drawn(&browser);
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1000, 7), SHIP_CENTER),
                (ImageKey::sprite(1100, 1), SHIP_CENTER),
                (ImageKey::sprite(1200, 7), SHIP_CENTER),
            ]
        );
        assert!(
            matches!(list.iter().next(), Some(DrawCommand::Sprite { .. })),
            "the ship is drawn first"
        );
    }

    #[test]
    fn the_layers_follow_the_base_frame_round() {
        let catalog = FakeCatalog::with(vec![layered()]);
        let mut browser = ShipBrowser::new(&catalog);
        let mut glow = Vec::new();
        for _ in 0..9 {
            let list = drawn(&browser);
            glow.push(sprites(&list)[1].0.frame);
            browser.tick(tick());
        }
        assert_eq!(glow, [0, 1, 2, 0, 1, 2, 0, 1, 2]);
        assert_eq!(sprites(&drawn(&browser))[0].0.frame, 0);
    }

    #[test]
    fn the_name_and_stats_match_the_record() {
        let ship = ShipEntry {
            name: "Shuttle".to_owned(),
            stats: Ok(ShipStats {
                cost: 1_234_567,
                speed: 280,
                armor: 31,
                shield: -7,
            }),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship, entry(129)]);
        let list = drawn(&ShipBrowser::new(&catalog));
        let texts = texts(&list);
        for expected in [
            "Shuttle",
            "Cost: 1234567 credits",
            "Speed: 280",
            "Armour: 31",
            "Shields: -7",
            "Ship 1 of 2 (shïp 128) - Left/Right to browse",
        ] {
            assert!(
                texts.iter().any(|t| t == expected),
                "{expected:?} in {texts:?}"
            );
        }
        assert_eq!(text_color(&list, "Shuttle"), Color::WHITE);
    }

    #[test]
    fn the_footer_counts_from_the_selected_ship() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.input(&press(Key::Left));
        let texts = texts(&drawn(&browser));
        assert!(
            texts.contains(&"Ship 3 of 3 (shïp 130) - Left/Right to browse".to_owned()),
            "{texts:?}"
        );
        assert!(texts.contains(&"Ship 130".to_owned()), "{texts:?}");
    }

    #[test]
    fn the_name_is_in_charcoal_and_everything_else_in_geneva() {
        let ship = ShipEntry {
            description: Ok(Some("A small ship.".to_owned())),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let fonts: Vec<(String, Font)> = drawn(&ShipBrowser::new(&catalog))
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, font, .. } => Some((text.clone(), *font)),
                _ => None,
            })
            .collect();
        let (name, rest) = fonts.split_first().expect("text");
        assert_eq!(name, &("Ship 128".to_owned(), Font::Charcoal));
        assert_eq!(rest.len(), 6, "{rest:?}");
        assert!(
            rest.iter().all(|(_, font)| *font == Font::Geneva),
            "{rest:?}"
        );
    }

    #[test]
    fn the_description_is_shown_wrapped_in_the_text_column() {
        let ship = ShipEntry {
            description: Ok(Some("A small ship.\nIt flies.".to_owned())),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let list = drawn(&ShipBrowser::new(&catalog));
        let wrap = list.iter().find_map(|command| match command {
            DrawCommand::Text {
                text, wrap_width, ..
            } if text == "A small ship.\nIt flies." => Some(*wrap_width),
            _ => None,
        });
        assert_eq!(wrap, Some(Some(DESCRIPTION_WRAP)));
    }

    #[test]
    fn no_description_draws_no_description_text() {
        let catalog = FakeCatalog::with(vec![entry(128)]);
        let texts = texts(&drawn(&ShipBrowser::new(&catalog)));
        // The name, four stats and the footer.
        assert_eq!(texts.len(), 6, "{texts:?}");
    }

    #[test]
    fn a_description_error_is_shown_in_its_place() {
        let ship = ShipEntry {
            description: Err("dësc 13000: bad".to_owned()),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let list = drawn(&ShipBrowser::new(&catalog));
        assert_eq!(
            text_color(&list, "Description unavailable: dësc 13000: bad"),
            Color::ERROR
        );
    }

    #[test]
    fn a_sprite_error_draws_a_placeholder_and_the_message() {
        let ship = ShipEntry {
            sprite: Err("no shän 128 for shïp 128".to_owned()),
            glow: Some(Ok(sheet(1100, 3))),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let browser = ShipBrowser::new(&catalog);
        let list = drawn(&browser);
        assert_eq!(sprites(&list), [], "no ship and no layers");
        assert_eq!(browser.frame(), None);
        // A box and its two diagonals.
        assert_eq!(lines(&list), 6);
        let ends: Vec<(Point, Point)> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Line { from, to, .. } => Some((*from, *to)),
                _ => None,
            })
            .collect();
        let half = PLACEHOLDER_SIZE / 2.0;
        let (left, right) = (SHIP_CENTER.x - half, SHIP_CENTER.x + half);
        let (top, bottom) = (SHIP_CENTER.y - half, SHIP_CENTER.y + half);
        let [top_left, top_right, bottom_right, bottom_left] = [
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(left, bottom),
        ];
        assert_eq!(
            ends,
            [
                (top_left, top_right),
                (top_right, bottom_right),
                (bottom_right, bottom_left),
                (bottom_left, top_left),
                (top_left, bottom_right),
                (top_right, bottom_left),
            ]
        );
        assert_eq!(
            text_color(&list, "Sprite unavailable: no shän 128 for shïp 128"),
            Color::ERROR
        );
        let message = text_origin(&list, "Sprite unavailable:");
        assert!(message.y > bottom, "the message is under the box");
        assert!(message.y < bottom + 2.0 * LINE_SPACING, "and close to it");
    }

    #[test]
    fn a_layer_error_skips_that_layer_and_says_why() {
        let ship = ShipEntry {
            glow: Some(Err("glow rlëD 1100: bad".to_owned())),
            lights: Some(Ok(sheet(1200, 5))),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let list = drawn(&ShipBrowser::new(&catalog));
        let keys: Vec<ImageKey> = sprites(&list).into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, [ImageKey::sprite(1280, 0), ImageKey::sprite(1200, 0)]);
        assert_eq!(
            text_color(&list, "Glow unavailable: glow rlëD 1100: bad"),
            Color::ERROR
        );

        let ship = ShipEntry {
            lights: Some(Err("lights rlëD 1200: bad".to_owned())),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let list = drawn(&ShipBrowser::new(&catalog));
        assert_eq!(sprites(&list).len(), 1);
        assert_eq!(
            text_color(&list, "Lights unavailable: lights rlëD 1200: bad"),
            Color::ERROR
        );
    }

    #[test]
    fn the_stat_and_layer_lines_are_evenly_spaced_under_the_name() {
        let ship = ShipEntry {
            glow: Some(Err("bad glow".to_owned())),
            lights: Some(Err("bad lights".to_owned())),
            description: Ok(Some("Text.".to_owned())),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let list = drawn(&ShipBrowser::new(&catalog));
        let tops: Vec<f32> = [
            "Cost:",
            "Speed:",
            "Armour:",
            "Shields:",
            "Glow unavailable:",
            "Lights unavailable:",
        ]
        .into_iter()
        .map(|line| text_origin(&list, line).y)
        .collect();
        let expected: Vec<f32> = (0..6u8)
            .map(|at| STATS_TOP + f32::from(at) * LINE_SPACING)
            .collect();
        assert_eq!(tops, expected);
        assert!(text_origin(&list, "Ship 128").y < STATS_TOP);
        assert!(text_origin(&list, "Text.").y >= tops[5] + LINE_SPACING);
    }

    #[test]
    fn a_record_error_replaces_the_stats() {
        let ship = ShipEntry {
            stats: Err("shïp 128: short".to_owned()),
            ..entry(128)
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let list = drawn(&ShipBrowser::new(&catalog));
        let texts = texts(&list);
        assert!(!texts.iter().any(|t| t.starts_with("Cost:")), "{texts:?}");
        assert_eq!(
            text_color(&list, "Record unavailable: shïp 128: short"),
            Color::ERROR
        );
    }

    #[test]
    fn an_empty_catalog_says_so_and_ignores_keys() {
        let catalog = FakeCatalog::default();
        let mut browser = ShipBrowser::new(&catalog);
        for key in [Key::Right, Key::Left, Key::Up, Key::Down] {
            assert_eq!(browser.input(&press(key)), ScreenAction::None);
        }
        browser.tick(tick());
        assert_eq!(browser.selected(), None);
        assert_eq!(browser.frame(), None);
        assert_eq!(catalog.calls(), []);
        let list = drawn(&browser);
        assert_eq!(texts(&list), ["No ships in the game data"]);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn the_layout_keeps_the_ship_left_of_the_text_column() {
        let catalog = FakeCatalog::with(vec![ShipEntry {
            description: Ok(Some("Text.".to_owned())),
            ..layered()
        }]);
        let list = drawn(&ShipBrowser::new(&catalog));
        for command in &list {
            if let DrawCommand::Text { origin, text, .. } = command {
                let footer = text.starts_with("Ship 1 of");
                assert!(
                    footer || origin.x >= TEXT_LEFT,
                    "{text:?} at {origin:?} is in the text column"
                );
                assert!(origin.y >= 0.0 && origin.y < 768.0, "{text:?} on screen");
            }
        }
    }
}
