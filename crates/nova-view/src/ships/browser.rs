//! The ship browser screen: pages through every ship by keyboard, drawing
//! the selected ship rotating through every frame of its sheet with its
//! glow and lights layers on top, and its name (in Charcoal), stats and
//! description in a column beside it. The lights blink by the ship's
//! [`Blink`](super::catalog::Blink) from the selection on, as
//! [`nova_sim_frame::lights_level`] says, rolling on [`HashedRolls`] from seed 0
//! for random blinking. The glow (at full level) and the lights (at
//! [`lights_tint`] of their level) are combined by OR with what is beneath
//! ([`Blend::Or`](crate::Blend::Or)), scaled by level/32, like the
//! original's `_BlitPixieRLETranslucent` (0xc1568) below full and
//! `_BlitPixieRLEAddOver` (0xc24bf) at full: see [`lights_tint`] for the
//! full record.
//!
//! A class with no sprite sheet (a new game's) whose model the world
//! models' table holds ([`ShipBrowser::with_world_models`]) turns its
//! model instead, with its as-built parts, in flight's top-down pose: one
//! of `MODEL_STEPS` (the frames flight gives such a class) headings a
//! tick, fitted to the placeholder's square, its [`frame`](ShipBrowser::frame)
//! the step shown (`model_turn`, rdm:phase/rendering-3d/phase-17-ship-browser-models).
//! A class with neither draws a crossed box and why it has no sprite.
//!
//! The screen reads ships only through the [`ShipCatalog`] port, once at
//! start-up and once per selection; drawing never resolves anything. A
//! class's model is resolved with it, from the table, which is data the
//! edge hands it, not game data.

use std::rc::Rc;
use std::time::Duration;

use nova_sim_frame::{HashedRolls, lights_level};

use super::catalog::{SheetInfo, ShipCatalog, ShipEntry, ShipId};
use super::model_turn::{draw_model, model_step};
use crate::bound_names::{Spelling, group, name_read};
use crate::draw::{crossed_box, lights_tint};
use crate::flight::WorldModels;
use crate::keymap::{Command, CommandInput, CommandReader, SharedBindings, UiCommand, contexts};
use crate::model_picture::ShipPortrait;
use crate::time::ticks;
use crate::{Color, DrawList, Font, Input, Point, Screen, ScreenAction};

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
/// The message under the placeholder (or a model in its place): its left
/// edge, size and wrap width.
const SPRITE_ERROR_LEFT: f32 = 48.0;
pub(super) const SPRITE_ERROR_WRAP: f32 = 384.0;
pub(super) const MESSAGE_SIZE: f32 = 16.0;
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
    /// The world models' table: empty unless a new game's assets give one
    /// ([`ShipBrowser::with_world_models`]).
    models: Rc<WorldModels>,
    /// The selected ship's model, for a class with no sprite sheet whose
    /// model `models` holds; resolved with the ship.
    portrait: Option<ShipPortrait>,
    /// Time since the selection.
    elapsed: Duration,
    /// What random running lights roll on.
    blink_rolls: HashedRolls,
    /// Its keys, the shared arrows, as commands
    /// ([`contexts::SHIP_BROWSER`]).
    keys: CommandReader,
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
            models: Rc::default(),
            portrait: None,
            elapsed: Duration::ZERO,
            blink_rolls: HashedRolls::new(0),
            keys: CommandReader::new(contexts::SHIP_BROWSER),
        }
    }

    /// The browser reading its keys through `bindings`.
    #[must_use]
    pub fn with_bindings(mut self, bindings: SharedBindings) -> Self {
        self.keys.set_bindings(bindings);
        self
    }

    /// The browser drawing a class with no sprite sheet as its model in
    /// `models`, turning (see the module docs). The ship already shown
    /// takes its model from the entry it holds; the catalog isn't read.
    #[must_use]
    pub fn with_world_models(mut self, models: Rc<WorldModels>) -> Self {
        self.models = models;
        self.portrait = self
            .current
            .as_ref()
            .and_then(|ship| self.resolve_portrait(ship));
        self
    }

    /// The world models' table it draws models from.
    #[must_use]
    pub fn world_models(&self) -> &Rc<WorldModels> {
        &self.models
    }

    /// `ship`'s model, if it has no sprite sheet and the table models it: a
    /// sheet always comes first.
    fn resolve_portrait(&self, ship: &ShipEntry) -> Option<ShipPortrait> {
        if ship.sprite.is_ok() {
            return None;
        }
        ShipPortrait::of(&self.models, ship.id)
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

    /// The base sprite's frame being shown, or for a class drawn as its
    /// model the model's step, if there is either.
    #[must_use]
    pub fn frame(&self) -> Option<u16> {
        let ship = self.current.as_ref()?;
        match &ship.sprite {
            Ok(sheet) => Some(self.frame_of(*sheet)),
            Err(_) => self.portrait.as_ref().map(|_| model_step(self.elapsed)),
        }
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
        let ship = self.catalog.ship(self.ids[self.selected]);
        self.portrait = self.resolve_portrait(&ship);
        self.current = Some(ship);
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
                list.sprite(base.image.frame(frame), SHIP_CENTER, Color::WHITE);
                let tick = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
                let lights = lights_level(&ship.blink, tick, &self.blink_rolls).map(lights_tint);
                for (layer, tint) in [(&ship.glow, Some(Color::WHITE)), (&ship.lights, lights)] {
                    if let (Some(Ok(layer)), Some(tint)) = (layer, tint) {
                        let frame = frame % layer.frames.get();
                        list.or_sprite(layer.image.frame(frame), SHIP_CENTER, tint);
                    }
                }
            }
            Err(message) => {
                let drew = self.portrait.as_ref().is_some_and(|portrait| {
                    draw_model(portrait, model_step(self.elapsed), message, list)
                });
                if !drew {
                    draw_placeholder(message, list);
                }
            }
        }
    }
}

/// A grey box with its diagonals where the ship would be, and why.
fn draw_placeholder(message: &str, list: &mut DrawList) {
    crossed_box(list, SHIP_CENTER, PLACEHOLDER_SIZE, PLACEHOLDER);
    list.text(
        format!("Sprite unavailable: {message}"),
        message_origin(),
        MESSAGE_SIZE,
        Some(SPRITE_ERROR_WRAP),
        Color::ERROR,
    );
}

/// Where the line saying why a ship has no sprite starts: a line under
/// the placeholder's square, whether the placeholder or a model fills it.
pub(super) fn message_origin() -> Point {
    let bottom = SHIP_CENTER.y + PLACEHOLDER_SIZE / 2.0;
    Point::new(SPRITE_ERROR_LEFT, bottom + LINE_SPACING)
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
    /// The shared Right or Down shows the next ship and Left or Up the
    /// previous one, wrapping round; holding the key keeps stepping, once
    /// per key repeat. Everything else, and every key release, is
    /// ignored; the browser never quits (Escape is the app's).
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(CommandInput {
            command: Command::Ui(command),
            pressed: true,
            ..
        }) = self.keys.follow(input)
        {
            match command {
                UiCommand::Right | UiCommand::Down => self.select(true),
                UiCommand::Left | UiCommand::Up => self.select(false),
                _ => {}
            }
        }
        ScreenAction::None
    }

    /// Lets go of the keys held.
    fn release_keys(&mut self) {
        self.keys.let_go();
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
        let browse = group(
            [UiCommand::Left, UiCommand::Right]
                .map(|command| name_read(&self.keys, command, Spelling::Bound)),
            "/",
        )
        .map(|keys| format!(" - {keys} to browse"))
        .unwrap_or_default();
        list.text(
            format!(
                "Ship {} of {} (shïp {}){browse}",
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
    use std::collections::BTreeMap;
    use std::num::NonZeroU16;
    use std::time::Duration;

    use nova_data::store::{Footing, Mount, Mounts, Shown};

    use super::super::model_turn::{MODEL_STEPS, model_area, model_orientation};
    use super::*;
    use crate::flight::world_models::ShipModel;
    use crate::model::{ModelKey, ModelPose};
    use crate::ships::catalog::{Blink, SheetInfo, ShipCatalog, ShipEntry, ShipId, ShipStats};
    use crate::{
        Blend, Color, DrawCommand, DrawList, Font, ImageKey, Input, Key, MouseButton, Point,
        Screen, ScreenAction,
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
            image: crate::SheetImage::Rled(image_id),
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
            blink: Blink::STEADY,
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
        for key in [
            Key::ArrowRight,
            Key::ArrowRight,
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::ArrowLeft,
        ] {
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
        for input in [
            press(Key::ArrowRight),
            held(Key::ArrowRight),
            held(Key::ArrowLeft),
        ] {
            assert_eq!(browser.input(&input), ScreenAction::None);
            visited.push(browser.selected().expect("a ship").0);
        }
        assert_eq!(visited, [129, 130, 129]);
    }

    #[test]
    fn a_rebound_right_steps_on_its_new_key_alone() {
        use crate::keymap::{Bindings, KeyCombo, SharedBindings, UiCommand};
        let catalog = three_ships();
        let bindings = SharedBindings::new(
            Bindings::default().rebound(UiCommand::Right, KeyCombo::key(Key::KeyL)),
        );
        let mut browser = ShipBrowser::new(&catalog).with_bindings(bindings);
        browser.input(&press(Key::ArrowRight));
        assert_eq!(
            browser.selected(),
            Some(ShipId(128)),
            "Right no longer steps"
        );
        browser.input(&press(Key::KeyL));
        assert_eq!(browser.selected(), Some(ShipId(129)), "L does");
    }

    /// Letting go of the keys forgets a modifier held: Right on Shift-L
    /// no longer steps on L once the keys are let go of.
    #[test]
    fn releasing_the_keys_forgets_a_modifier_held() {
        use crate::keymap::{Bindings, KeyCombo, Modifiers, SharedBindings, UiCommand};
        let catalog = three_ships();
        let bindings = SharedBindings::new(Bindings::default().rebound(
            UiCommand::Right,
            KeyCombo::with(Key::KeyL, Modifiers::SHIFT),
        ));
        let mut browser = ShipBrowser::new(&catalog).with_bindings(bindings.clone());
        browser.input(&press(Key::ShiftLeft));
        browser.release_keys();
        browser.input(&press(Key::KeyL));
        assert_eq!(browser.selected(), Some(ShipId(128)), "Shift was let go of");
        let mut held = ShipBrowser::new(&catalog).with_bindings(bindings);
        held.input(&press(Key::ShiftLeft));
        held.input(&press(Key::KeyL));
        assert_eq!(held.selected(), Some(ShipId(129)), "with Shift held");
    }

    #[test]
    fn down_and_up_mirror_right_and_left() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.input(&press(Key::ArrowUp));
        assert_eq!(browser.selected(), Some(ShipId(130)));
        browser.input(&press(Key::ArrowDown));
        browser.input(&press(Key::ArrowDown));
        assert_eq!(browser.selected(), Some(ShipId(129)));
        assert_eq!(catalog.calls().len(), 4);
    }

    #[test]
    fn changing_ship_restarts_the_animation() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.tick(tick() * 3);
        assert_eq!(browser.frame(), Some(3));
        browser.input(&press(Key::ArrowRight));
        assert_eq!(browser.frame(), Some(0));
        browser.tick(tick() * 2);
        browser.input(&press(Key::ArrowLeft));
        assert_eq!(browser.frame(), Some(0));
    }

    #[test]
    fn releases_other_keys_and_the_pointer_change_nothing() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.tick(tick() * 2);
        let others = [
            Input::Key {
                key: Key::ArrowRight,
                pressed: false,
                repeat: false,
            },
            Input::Key {
                key: Key::ArrowLeft,
                pressed: false,
                repeat: false,
            },
            press(Key::Enter),
            press(Key::Escape),
            press(Key::Space),
            press(Key::KeyD),
            press(Key::Unidentified),
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
    fn a_ship_whose_base_and_glow_came_from_pict_pairs_draws_their_cut_frames() {
        use nova_data::ShipImage;
        let pair = |image, frames| SheetInfo {
            image: crate::SheetImage::Ship { ship: 128, image },
            frames: NonZeroU16::new(frames).expect("non-zero"),
        };
        let ship = ShipEntry {
            sprite: Ok(pair(ShipImage::Base, 9)),
            glow: Some(Ok(pair(ShipImage::Glow, 3))),
            ..layered()
        };
        let catalog = FakeCatalog::with(vec![ship]);
        let mut browser = ShipBrowser::new(&catalog);
        browser.tick(tick() * 7);
        assert_eq!(
            sprites(&drawn(&browser)),
            [
                (ImageKey::ship_sheet(128, ShipImage::Base, 7), SHIP_CENTER),
                (ImageKey::ship_sheet(128, ShipImage::Glow, 1), SHIP_CENTER),
                (ImageKey::sprite(1200, 7), SHIP_CENTER),
            ]
        );
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
    fn the_glow_and_lights_or_onto_the_normal_base() {
        let catalog = FakeCatalog::with(vec![layered()]);
        let browser = ShipBrowser::new(&catalog);
        let blends: Vec<Blend> = drawn(&browser)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { blend, .. } => Some(*blend),
                _ => None,
            })
            .collect();
        assert_eq!(blends, [Blend::Normal, Blend::Or, Blend::Or]);
    }

    /// The Shuttle's blink: lit at ticks 1-3 and 10-12 of every 40.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// [`layered`] ship `id`, its lights blinking by `blink`.
    fn blinking(id: i16, blink: Blink) -> ShipEntry {
        ShipEntry {
            id: ShipId(id),
            blink,
            ..layered()
        }
    }

    /// The alphas the lights, `rlëD` 1200, are drawn with: one, or none
    /// while they are off.
    fn lights_alphas(browser: &impl Screen) -> Vec<u8> {
        drawn(browser)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite {
                    image,
                    tint,
                    blend: Blend::Or,
                    ..
                } if image.id == 1200 => {
                    assert_eq!((tint.r, tint.g, tint.b), (255, 255, 255));
                    Some(tint.a)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_lights_blink_by_the_ships_blink() {
        let catalog = FakeCatalog::with(vec![blinking(128, SHUTTLE)]);
        let mut browser = ShipBrowser::new(&catalog);
        assert_eq!(lights_alphas(&browser), [0_u8; 0], "tick 0: off");
        let ids: Vec<i16> = sprites(&drawn(&browser))
            .iter()
            .map(|(i, _)| i.id)
            .collect();
        assert_eq!(ids, [1000, 1100], "the base and glow still show");
        browser.tick(tick());
        assert_eq!(lights_alphas(&browser), [255], "tick 1: on");
        browser.tick(tick() * 3);
        assert_eq!(lights_alphas(&browser), [0_u8; 0], "tick 4: off");
        browser.tick(tick() * 6);
        assert_eq!(lights_alphas(&browser), [255], "tick 10: on again");
    }

    #[test]
    fn pulsing_lights_are_ored_at_their_level() {
        // The stock triangle: 10 to 31.
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let catalog = FakeCatalog::with(vec![blinking(128, triangle)]);
        let mut browser = ShipBrowser::new(&catalog);
        assert_eq!(lights_alphas(&browser), [0_u8; 0], "0.75: not drawn");
        browser.tick(tick());
        assert_eq!(lights_alphas(&browser), [8], "level 1");
        browser.tick(tick() * 40);
        assert_eq!(lights_alphas(&browser), [247], "level 31");
    }

    /// Every command drawing the lights, `rlëD` 1200, whatever its blend.
    fn lights_commands(browser: &ShipBrowser<&FakeCatalog>) -> Vec<DrawCommand> {
        drawn(browser)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 1200),
            )
            .cloned()
            .collect()
    }

    /// The lights drawn by OR with `tint` over the ship's centre, at the base's
    /// current frame.
    fn lights_at(browser: &ShipBrowser<&FakeCatalog>, tint: Color) -> DrawCommand {
        DrawCommand::Sprite {
            image: ImageKey::sprite(1200, browser.frame().expect("a sheet")),
            center: SHIP_CENTER,
            tint,
            blend: Blend::Or,
        }
    }

    #[test]
    fn lights_are_ored_scaled_by_level_below_full_and_whole_at_full() {
        // The stock triangle, 10 to 31, never full: the original draws
        // every level with its translucent blit, which ORs in the lights
        // scaled by level/32 (see `lights_tint`).
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let rolls = HashedRolls::new(0);
        let catalog = FakeCatalog::with(vec![blinking(128, triangle)]);
        let mut browser = ShipBrowser::new(&catalog);
        let mut levels = Vec::new();
        for tick_number in 0..=101 {
            let level = lights_level(&triangle, tick_number, &rolls);
            let want: Vec<_> = level
                .map(|n| lights_at(&browser, lights_tint(n)))
                .into_iter()
                .collect();
            assert_eq!(lights_commands(&browser), want, "tick {tick_number}");
            levels.extend(level);
            browser.tick(tick());
        }
        assert!(levels.contains(&10), "the lowest level is drawn");
        assert!(levels.contains(&31), "the highest level is drawn");

        // A steady light is full: ORed whole.
        let catalog = FakeCatalog::with(vec![blinking(128, Blink::STEADY)]);
        let browser = ShipBrowser::new(&catalog);
        assert_eq!(
            lights_commands(&browser),
            [lights_at(&browser, Color::WHITE)]
        );
    }

    #[test]
    fn choosing_another_ship_restarts_its_blink() {
        let catalog = FakeCatalog::with(vec![blinking(128, SHUTTLE), blinking(129, SHUTTLE)]);
        let mut browser = ShipBrowser::new(&catalog);
        browser.tick(tick());
        assert_eq!(lights_alphas(&browser), [255], "tick 1");
        browser.input(&press(Key::ArrowRight));
        assert_eq!(lights_alphas(&browser), [0_u8; 0], "tick 0 again");
        browser.tick(tick());
        assert_eq!(lights_alphas(&browser), [255]);
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

    /// The footer a browser of three ships draws, its keys read through
    /// `bindings`, for the device its keys read.
    fn footer_with(bindings: crate::keymap::Bindings) -> String {
        let catalog = three_ships();
        let browser = ShipBrowser::new(&catalog).with_bindings(SharedBindings::new(bindings));
        texts(&drawn(&browser))
            .into_iter()
            .find(|text| text.starts_with("Ship 1 of 3"))
            .unwrap_or_default()
    }

    #[test]
    fn the_footer_names_the_browse_keys_bound() {
        use crate::keymap::{Bindings, KeyCombo};
        assert_eq!(
            footer_with(Bindings::default().rebound(UiCommand::Left, KeyCombo::key(Key::KeyA))),
            "Ship 1 of 3 (shïp 128) - A/Right Arrow to browse"
        );
        assert_eq!(
            footer_with(
                Bindings::default()
                    .unbound(UiCommand::Left)
                    .unbound(UiCommand::Right)
            ),
            "Ship 1 of 3 (shïp 128)"
        );
    }

    #[test]
    fn with_a_pad_last_used_the_footer_names_its_stick_and_an_unbound_one_its_key() {
        use crate::keymap::Bindings;
        let on_pad = |bindings: Bindings| {
            let catalog = three_ships();
            let mut browser =
                ShipBrowser::new(&catalog).with_bindings(SharedBindings::new(bindings));
            browser.input(&Input::Surroundings(crate::input::Surroundings::of(
                crate::input::Device::Gamepad(crate::input::ControllerFamily::Xbox),
            )));
            texts(&drawn(&browser))
                .into_iter()
                .find(|text| text.starts_with("Ship 1 of 3"))
                .unwrap_or_default()
        };
        assert_eq!(
            on_pad(Bindings::default()),
            "Ship 1 of 3 (shïp 128) - L Stick Left/L Stick Right to browse"
        );
        let mut unbound = Bindings::default();
        unbound.gamepad.unbind(UiCommand::Left.into());
        assert_eq!(
            on_pad(unbound),
            "Ship 1 of 3 (shïp 128) - Left Arrow/L Stick Right to browse"
        );
    }

    #[test]
    fn the_footer_counts_from_the_selected_ship() {
        let catalog = three_ships();
        let mut browser = ShipBrowser::new(&catalog);
        browser.input(&press(Key::ArrowLeft));
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
        for key in [
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::ArrowUp,
            Key::ArrowDown,
        ] {
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

    /// Ship `id` with no sprite sheet (as a new game's class) and no layers.
    fn modelled(id: i16) -> ShipEntry {
        ShipEntry {
            sprite: Err("no sprite".into()),
            glow: None,
            lights: None,
            ..entry(id)
        }
    }

    /// A world models' table modelling each `(ship, model key)`, radius 4.
    fn table(ids: &[(i16, u32)]) -> Rc<WorldModels> {
        Rc::new(WorldModels {
            ships: ids
                .iter()
                .map(|&(id, key)| {
                    (
                        nova_sim_frame::ShipId(id),
                        ShipModel {
                            key: ModelKey(key),
                            scale: 1.0,
                            base_transp: 0,
                            radius: 4.0,
                        },
                    )
                })
                .collect(),
            ..WorldModels::default()
        })
    }

    /// Ship 128 modelled as `ModelKey(7)`, a gun shown on a nose mount.
    fn table_with_a_part() -> Rc<WorldModels> {
        let mut models = Rc::unwrap_or_clone(table(&[(128, 7)]));
        models.mounts = Mounts {
            hulls: BTreeMap::from([(
                nova_sim_frame::ShipId(128),
                vec![Mount {
                    hardpoint: "nose".to_owned(),
                    at: [0.0, 2.0, 0.5],
                    footing: Footing::Surface,
                    turns: false,
                    shows: Some(Shown {
                        asset: "gun".to_owned(),
                        lift: 0.25,
                    }),
                }],
            )]),
            kits: BTreeMap::new(),
        };
        models.outfits = BTreeMap::from([("gun".to_owned(), ModelKey(9))]);
        Rc::new(models)
    }

    fn commands(list: &DrawList) -> Vec<DrawCommand> {
        list.iter().cloned().collect()
    }

    fn models(list: &DrawList) -> Vec<(ModelKey, ModelPose)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Model { model, pose, .. } => Some((*model, *pose)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_modelled_class_with_no_sheet_draws_its_model_where_the_sprite_would_be() {
        let catalog = FakeCatalog::with(vec![modelled(128)]);
        let models = table(&[(128, 7)]);
        let browser = ShipBrowser::new(&catalog).with_world_models(Rc::clone(&models));
        let list = drawn(&browser);
        let got = commands(&list);
        let DrawCommand::Model { model, pose, .. } = &got[0] else {
            panic!("the model first: {got:?}");
        };
        assert_eq!(*model, ModelKey(7));
        assert_eq!(pose.center, SHIP_CENTER);
        let mut want = DrawList::new();
        let portrait = ShipPortrait::of(&models, nova_sim_frame::ShipId(128)).expect("a portrait");
        assert!(portrait.draw_posed_in(&mut want, model_area(), model_orientation(0)));
        assert_eq!(got[..want.len()], commands(&want)[..]);
        assert_eq!(lines(&list), 0, "no crossed box");
        assert_eq!(sprites(&list), []);
        assert_eq!(
            text_color(&list, "3D model (sprite unavailable: no sprite)"),
            Color::DIM
        );
        let texts = texts(&list);
        assert!(
            !texts.iter().any(|t| t.starts_with("Sprite unavailable")),
            "{texts:?}"
        );
        let shown = |text: &str| {
            got.iter()
                .position(|c| matches!(c, DrawCommand::Text { text: t, .. } if t.starts_with(text)))
        };
        for line in [
            "Ship 128",
            "Cost:",
            "Speed:",
            "Armour:",
            "Shields:",
            "Ship 1 of 1",
        ] {
            let at = shown(line).unwrap_or_else(|| panic!("{line:?} drawn"));
            assert!(at > 0, "{line:?} after the model");
        }
    }

    #[test]
    fn the_placeholder_s_message_and_the_model_s_share_a_place() {
        let catalog = FakeCatalog::with(vec![modelled(128)]);
        let placeholder = drawn(&ShipBrowser::new(&catalog));
        let model = drawn(&ShipBrowser::new(&catalog).with_world_models(table(&[(128, 7)])));
        assert_eq!(
            text_origin(&model, "3D model"),
            text_origin(&placeholder, "Sprite unavailable")
        );
    }

    #[test]
    fn its_parts_are_drawn_after_its_hull() {
        let catalog = FakeCatalog::with(vec![modelled(128)]);
        let browser = ShipBrowser::new(&catalog).with_world_models(table_with_a_part());
        let got = commands(&drawn(&browser));
        assert!(
            matches!(
                got[0],
                DrawCommand::Model {
                    model: ModelKey(7),
                    ..
                }
            ),
            "{got:?}"
        );
        assert!(
            matches!(
                got[1],
                DrawCommand::Mounted {
                    model: ModelKey(9),
                    ..
                }
            ),
            "{got:?}"
        );
    }

    #[test]
    fn the_model_turns_a_step_a_tick_and_frame_reads_the_step() {
        let catalog = FakeCatalog::with(vec![modelled(128)]);
        let mut browser = ShipBrowser::new(&catalog).with_world_models(table(&[(128, 7)]));
        let mut ticked = 0;
        for (at, want) in [
            (0, 0),
            (1, 1),
            (MODEL_STEPS - 1, MODEL_STEPS - 1),
            (MODEL_STEPS, 0),
            (MODEL_STEPS + 1, 1),
        ] {
            browser.tick(tick() * u32::from(at - ticked));
            ticked = at;
            assert_eq!(browser.frame(), Some(want), "{at} ticks");
            let drawn = models(&drawn(&browser));
            assert_eq!(drawn.len(), 1);
            assert_eq!(
                drawn[0].1.orientation,
                model_orientation(want),
                "{at} ticks"
            );
        }
    }

    #[test]
    fn choosing_another_ship_restarts_the_turn() {
        let catalog = FakeCatalog::with(vec![modelled(128), modelled(129)]);
        let mut browser =
            ShipBrowser::new(&catalog).with_world_models(table(&[(128, 7), (129, 8)]));
        browser.tick(tick() * 5);
        assert_eq!(browser.frame(), Some(5));
        browser.input(&press(Key::ArrowRight));
        assert_eq!(browser.frame(), Some(0));
        let drawn = models(&drawn(&browser));
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].0, ModelKey(8));
        assert_eq!(drawn[0].1.orientation, model_orientation(0));
    }

    #[test]
    fn a_class_with_a_sheet_draws_its_sheet_even_with_a_model() {
        let catalog = FakeCatalog::with(vec![layered()]);
        let mut plain = ShipBrowser::new(&catalog);
        let mut with = ShipBrowser::new(&catalog).with_world_models(table(&[(128, 7)]));
        for at in [0, 7] {
            plain.tick(tick() * at);
            with.tick(tick() * at);
            let list = drawn(&with);
            assert_eq!(commands(&list), commands(&drawn(&plain)), "{at}");
            assert_eq!(models(&list), []);
            assert_eq!(with.frame(), plain.frame());
        }
    }

    #[test]
    fn a_class_with_neither_draws_the_crossed_box() {
        let catalog = FakeCatalog::with(vec![modelled(129)]);
        let browser = ShipBrowser::new(&catalog).with_world_models(table(&[(128, 7)]));
        let list = drawn(&browser);
        assert_eq!(lines(&list), 6);
        assert_eq!(
            text_color(&list, "Sprite unavailable: no sprite"),
            Color::ERROR
        );
        assert_eq!(models(&list), []);
        assert_eq!(browser.frame(), None);
    }

    #[test]
    fn a_model_that_cannot_be_fitted_draws_the_crossed_box() {
        let catalog = FakeCatalog::with(vec![modelled(128)]);
        let mut huge = Rc::unwrap_or_clone(table(&[(128, 7)]));
        for model in huge.ships.values_mut() {
            model.radius = f32::MIN_POSITIVE;
        }
        let browser = ShipBrowser::new(&catalog).with_world_models(Rc::new(huge));
        let list = drawn(&browser);
        assert_eq!(models(&list), []);
        assert_eq!(lines(&list), 6);
        assert_eq!(
            text_color(&list, "Sprite unavailable: no sprite"),
            Color::ERROR
        );
    }

    #[test]
    fn an_empty_table_draws_as_before() {
        let catalogs = [
            three_ships(),
            FakeCatalog::with(vec![layered()]),
            FakeCatalog::with(vec![modelled(128)]),
            FakeCatalog::default(),
        ];
        for catalog in &catalogs {
            let mut plain = ShipBrowser::new(catalog);
            let mut with = ShipBrowser::new(catalog).with_world_models(Rc::default());
            for at in [0, 4] {
                plain.tick(tick() * at);
                with.tick(tick() * at);
                assert_eq!(commands(&drawn(&with)), commands(&drawn(&plain)));
                assert_eq!(with.frame(), plain.frame());
            }
        }
    }

    #[test]
    fn a_table_given_after_the_first_ship_is_resolved_draws_its_model() {
        let catalog = FakeCatalog::with(vec![modelled(128)]);
        let browser = ShipBrowser::new(&catalog);
        assert_eq!(catalog.calls(), [ShipId(128)]);
        let browser = browser.with_world_models(table(&[(128, 7)]));
        assert_eq!(models(&drawn(&browser)).len(), 1);
        assert_eq!(browser.frame(), Some(0));
        assert_eq!(
            catalog.calls(),
            [ShipId(128)],
            "the entry held is re-read, not the catalog"
        );
    }

    #[test]
    fn the_world_models_given_are_the_browser_s() {
        let catalog = three_ships();
        let models = table(&[(128, 7)]);
        let browser = ShipBrowser::new(&catalog).with_world_models(Rc::clone(&models));
        assert!(Rc::ptr_eq(browser.world_models(), &models));
        assert!(ShipBrowser::new(&catalog).world_models().ships.is_empty());
    }

    #[test]
    fn drawing_resolves_nothing_with_a_model_either() {
        let catalog = FakeCatalog::with(vec![modelled(128)]);
        let browser = ShipBrowser::new(&catalog).with_world_models(table(&[(128, 7)]));
        let before = catalog.calls();
        drawn(&browser);
        drawn(&browser);
        assert_eq!(catalog.calls(), before);
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
