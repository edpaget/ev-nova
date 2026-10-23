//! The navigator: the galaxy map, and the system view opened from it.
//!
//! The navigator shows the map until the map asks to enter a system (by
//! Return or its "Enter system" button), then that system's view. Escape
//! goes back from the system to the map, which kept its view and
//! selection. Escape on the map does nothing here: what it means there
//! (quitting) is the app's router's to decide.

use std::rc::Rc;
use std::time::Duration;

use crate::flight::SharedChance;
use crate::galaxy::{GalaxyCatalog, GalaxyMap};
use crate::system::{SystemCatalog, SystemView};
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::DialogTemplate;
use crate::{DrawList, Input, Key, Screen, ScreenAction, Sound};

/// The galaxy map and, while one is open, a system's view.
#[derive(Clone, Debug)]
pub struct Navigator<C> {
    catalog: C,
    map: GalaxyMap,
    system: Option<SystemView>,
    /// What each system view opened draws its stellars' random frames on
    /// ([`SystemView::with_chance`]).
    chance: SharedChance,
}

impl<C: GalaxyCatalog + SystemCatalog> Navigator<C> {
    /// The map of `catalog`'s galaxy, with no system open.
    pub fn new(catalog: C) -> Self {
        let map = GalaxyMap::new(&catalog);
        Self {
            catalog,
            map,
            system: None,
            chance: SharedChance::default(),
        }
    }

    /// The galaxy map, shown or not.
    #[must_use]
    pub fn map(&self) -> &GalaxyMap {
        &self.map
    }

    /// The open system's view, if one is open.
    #[must_use]
    pub fn system(&self) -> Option<&SystemView> {
        self.system.as_ref()
    }

    /// The catalog both screens read.
    #[must_use]
    pub fn catalog(&self) -> &C {
        &self.catalog
    }
}

impl<C> Navigator<C> {
    /// The navigator with each system view it opens drawing its stellars'
    /// random frames on `chance` ([`SystemView::with_chance`]).
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// The navigator with the map's text measured by `metrics`, which its
    /// Find needs ([`GalaxyMap::with_metrics`]).
    #[must_use]
    pub fn with_metrics(self, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            map: self.map.with_metrics(metrics),
            ..self
        }
    }

    /// The navigator with the map's Find laid out from `template`, or its
    /// fallback for `None`, its buttons labelled in `style`
    /// ([`GalaxyMap::with_find`]).
    #[must_use]
    pub fn with_find(self, template: Option<DialogTemplate>, style: ButtonStyle) -> Self {
        Self {
            map: self.map.with_find(template, style),
            ..self
        }
    }
}

impl<C> Navigator<C> {
    /// The screen shown: the open system's view, or the map.
    fn shown(&self) -> &dyn Screen {
        match &self.system {
            Some(view) => view,
            None => &self.map,
        }
    }

    fn shown_mut(&mut self) -> &mut dyn Screen {
        match &mut self.system {
            Some(view) => view,
            None => &mut self.map,
        }
    }
}

impl<C: GalaxyCatalog + SystemCatalog> Screen for Navigator<C> {
    /// In a system, an Escape press (not its repeats) goes back to the map,
    /// and everything else goes to the system's view. On the map, Escape is
    /// ignored, everything else goes to the map, and a request to enter a
    /// system opens its view, abandoning any gesture on the map. Never
    /// quits: that is the app's router's.
    fn input(&mut self, input: &Input) -> ScreenAction {
        let escape = matches!(
            input,
            Input::Key {
                key: Key::Escape,
                ..
            }
        );
        if let Some(view) = &mut self.system {
            if escape
                && matches!(
                    input,
                    Input::Key {
                        pressed: true,
                        repeat: false,
                        ..
                    }
                )
            {
                self.system = None;
                return ScreenAction::None;
            }
            return view.input(input);
        }
        if escape {
            return ScreenAction::None;
        }
        let action = self.map.input(input);
        if let Some(id) = self.map.take_entry() {
            self.map.cancel_pointer();
            self.system = Some(SystemView::new(&self.catalog, id).with_chance(self.chance.clone()));
        }
        action
    }

    /// Only the screen shown ticks; the map does not animate anyway.
    fn tick(&mut self, dt: Duration) {
        self.shown_mut().tick(dt);
    }

    fn draw(&self, list: &mut DrawList) {
        self.shown().draw(list);
    }

    fn cancel_pointer(&mut self) {
        self.shown_mut().cancel_pointer();
    }

    fn release_keys(&mut self) {
        self.shown_mut().release_keys();
    }

    /// The map's beeps: Find's ([`GalaxyMap::take_ui_sounds`]).
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.map.take_sounds()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::num::NonZeroU16;

    use super::*;
    use crate::galaxy::map::ENTER_BUTTON;
    use crate::galaxy::{Galaxy, SystemEntry, SystemId};
    use crate::system::{AnimationData, StellarContents, StellarId, StellarSheet, SystemContents};
    use crate::{MouseButton, Point};

    /// Alpha (128) at (0, 0) and Beta (129) at (600, 0) on the map; inside,
    /// Alpha holds Alpha Prime at (0, 0) and Beta Beta Flicker, of 4
    /// frames, each next one random (`Flags2` 0x0002) every tick.
    struct FakeCatalog;

    fn entry(id: i16, name: &str, x: i16) -> SystemEntry {
        SystemEntry {
            id: SystemId(id),
            name: name.to_owned(),
            x,
            y: 0,
            links: Vec::new(),
            govt: None,
            stellars: Vec::new(),
        }
    }

    impl GalaxyCatalog for FakeCatalog {
        fn galaxy(&self) -> Galaxy {
            Galaxy {
                systems: vec![entry(128, "Alpha", 0), entry(129, "Beta", 600)],
                ..Galaxy::default()
            }
        }
    }

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            let stellars = if id == SystemId(128) {
                vec![StellarContents {
                    id: StellarId(128),
                    name: "Alpha Prime".to_owned(),
                    x: 0,
                    y: 0,
                    sprite: Ok(StellarSheet {
                        image: crate::SheetImage::Rled(1000),
                        frames: NonZeroU16::MIN,
                        frame_width: 8,
                        frame_height: 8,
                    }),
                    dead_sprite: None,
                    animation: AnimationData::default(),
                    radar_square: false,
                }]
            } else {
                vec![StellarContents {
                    id: StellarId(130),
                    name: "Beta Flicker".to_owned(),
                    x: 0,
                    y: 0,
                    sprite: Ok(StellarSheet {
                        image: crate::SheetImage::Rled(1001),
                        frames: NonZeroU16::new(4).expect("non-zero"),
                        frame_width: 8,
                        frame_height: 8,
                    }),
                    dead_sprite: None,
                    animation: AnimationData {
                        delay: 1,
                        frame0_bias: 0,
                        flags2: nova_sim::gate_sprite::RANDOM,
                    },
                    radar_square: false,
                }]
            };
            SystemContents {
                id,
                name: format!("System {}", id.0),
                stellars,
                type_zero_sprite: Ok(StellarSheet {
                    image: crate::SheetImage::Rled(1000),
                    frames: NonZeroU16::MIN,
                    frame_width: 8,
                    frame_height: 8,
                }),
                problems: Vec::new(),
            }
        }
    }

    fn navigator() -> Navigator<FakeCatalog> {
        Navigator::new(FakeCatalog)
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn press(k: Key) -> Input {
        key(k, true, false)
    }

    fn left(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(navigator: &mut Navigator<FakeCatalog>, at: Point) {
        for pressed in [true, false] {
            assert_eq!(navigator.input(&left(pressed, at)), ScreenAction::None);
        }
    }

    /// Where system `id`'s dot is on the map.
    fn dot(navigator: &Navigator<FakeCatalog>, id: i16) -> Point {
        let map = navigator.map();
        let system = map.model().system(SystemId(id)).expect("a system");
        map.transform().world_to_screen(system.position())
    }

    fn select(navigator: &mut Navigator<FakeCatalog>, id: i16) {
        let at = dot(navigator, id);
        click(navigator, at);
        assert_eq!(navigator.map().selected(), Some(SystemId(id)));
    }

    fn drawn(screen: &impl Screen) -> DrawList {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list
    }

    fn open(navigator: &Navigator<FakeCatalog>) -> Option<SystemId> {
        navigator.system().map(|view| view.scene().id())
    }

    #[test]
    fn it_starts_on_the_map_with_no_system_open() {
        let navigator = navigator();
        assert_eq!(open(&navigator), None);
        assert_eq!(navigator.map().model().systems().len(), 2);
        assert_eq!(drawn(&navigator), drawn(navigator.map()));
        let _: &FakeCatalog = navigator.catalog();
    }

    #[test]
    fn return_on_a_selected_system_opens_its_view() {
        let mut navigator = navigator();
        select(&mut navigator, 128);
        assert_eq!(navigator.input(&press(Key::Enter)), ScreenAction::None);
        let view = navigator.system().expect("a system open");
        assert_eq!(view.scene().id(), SystemId(128));
        let names: Vec<&str> = view
            .scene()
            .stellars()
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(names, ["Alpha Prime"]);
        assert_eq!(drawn(&navigator), drawn(view));
    }

    #[test]
    fn the_enter_button_opens_the_selected_systems_view() {
        let mut navigator = navigator();
        select(&mut navigator, 129);
        click(&mut navigator, ENTER_BUTTON.center());
        assert_eq!(open(&navigator), Some(SystemId(129)));
    }

    #[test]
    fn return_with_nothing_selected_opens_nothing() {
        let mut navigator = navigator();
        navigator.input(&press(Key::Enter));
        assert_eq!(open(&navigator), None);
    }

    #[test]
    fn escape_goes_back_to_the_map_as_it_was() {
        let mut navigator = navigator();
        navigator.input(&press(Key::Char('-')));
        select(&mut navigator, 129);
        let (view, selected) = (
            *navigator.map().view().expect("the viewer"),
            navigator.map().selected(),
        );
        navigator.input(&press(Key::Enter));
        assert_eq!(navigator.input(&press(Key::Escape)), ScreenAction::None);
        assert_eq!(open(&navigator), None);
        assert_eq!(*navigator.map().view().expect("the viewer"), view);
        assert_eq!(navigator.map().selected(), selected);
        assert_eq!(drawn(&navigator), drawn(navigator.map()));
        assert_eq!(navigator.map().clone().take_entry(), None, "taken");
    }

    #[test]
    fn an_escape_repeat_or_release_in_a_system_stays_there() {
        let mut navigator = navigator();
        select(&mut navigator, 128);
        navigator.input(&press(Key::Enter));
        for input in [key(Key::Escape, true, true), key(Key::Escape, false, false)] {
            assert_eq!(navigator.input(&input), ScreenAction::None);
            assert_eq!(open(&navigator), Some(SystemId(128)), "{input:?}");
        }
    }

    #[test]
    fn escape_on_the_map_neither_quits_nor_changes_anything() {
        let mut navigator = navigator();
        select(&mut navigator, 129);
        let (view, selected) = (
            *navigator.map().view().expect("the viewer"),
            navigator.map().selected(),
        );
        for input in [
            press(Key::Escape),
            key(Key::Escape, true, true),
            key(Key::Escape, false, false),
        ] {
            assert_eq!(navigator.input(&input), ScreenAction::None);
        }
        assert_eq!(open(&navigator), None);
        assert_eq!(
            (
                *navigator.map().view().expect("the viewer"),
                navigator.map().selected()
            ),
            (view, selected)
        );
    }

    #[test]
    fn input_in_a_system_goes_to_its_view_not_the_map() {
        let mut navigator = navigator();
        select(&mut navigator, 128);
        let view = *navigator.map().view().expect("the viewer");
        navigator.input(&press(Key::Enter));
        navigator.input(&press(Key::Right));
        navigator.tick(Duration::from_millis(250));
        let camera = navigator.system().expect("open").camera().center();
        assert_eq!(camera, Point::new(240.0, 0.0));
        assert_eq!(
            *navigator.map().view().expect("the viewer"),
            view,
            "the map stays put"
        );
        // A click on the map's dots does nothing either.
        let beta = dot(&navigator, 129);
        click(&mut navigator, beta);
        assert_eq!(navigator.map().selected(), Some(SystemId(128)));
    }

    #[test]
    fn releasing_the_keys_reaches_the_open_system() {
        let mut navigator = navigator();
        select(&mut navigator, 128);
        navigator.input(&press(Key::Enter));
        navigator.input(&press(Key::Down));
        navigator.release_keys();
        navigator.tick(Duration::from_millis(250));
        let camera = navigator.system().expect("open").camera().center();
        assert_eq!(camera, Point::new(0.0, 0.0));
    }

    #[test]
    fn cancelling_the_pointer_reaches_the_map() {
        let mut navigator = navigator();
        let view = *navigator.map().view().expect("the viewer");
        navigator.input(&left(true, Point::new(300.0, 300.0)));
        navigator.cancel_pointer();
        navigator.input(&Input::PointerMoved(Point::new(350.0, 300.0)));
        assert_eq!(
            *navigator.map().view().expect("the viewer"),
            view,
            "no drag"
        );
        // In a system it does nothing there, and the map is left alone.
        select(&mut navigator, 128);
        navigator.input(&press(Key::Enter));
        navigator.cancel_pointer();
        assert_eq!(open(&navigator), Some(SystemId(128)));
    }

    #[test]
    fn entering_cancels_a_drag_in_progress_on_the_map() {
        let mut navigator = navigator();
        select(&mut navigator, 128);
        let view = *navigator.map().view().expect("the viewer");
        // The button goes down on the map, then Return enters the system,
        // where the button's release goes.
        navigator.input(&left(true, Point::new(300.0, 300.0)));
        navigator.input(&press(Key::Enter));
        navigator.input(&left(false, Point::new(300.0, 300.0)));
        navigator.input(&press(Key::Escape));
        navigator.input(&Input::PointerMoved(Point::new(400.0, 350.0)));
        assert_eq!(
            *navigator.map().view().expect("the viewer"),
            view,
            "no drag"
        );
        assert_eq!(navigator.map().selected(), Some(SystemId(128)));
    }

    #[test]
    fn re_entering_a_system_starts_a_fresh_camera() {
        let mut navigator = navigator();
        select(&mut navigator, 128);
        navigator.input(&press(Key::Enter));
        navigator.input(&press(Key::Right));
        navigator.tick(Duration::from_millis(500));
        navigator.input(&key(Key::Right, false, false));
        navigator.input(&press(Key::Escape));
        navigator.input(&press(Key::Enter));
        let view = navigator.system().expect("open");
        assert_eq!(view.camera().center(), Point::new(0.0, 0.0));
        assert_eq!(view.frame(0), Some(0));
    }

    #[test]
    fn ticks_reach_the_open_system() {
        let mut navigator = navigator();
        select(&mut navigator, 128);
        navigator.input(&press(Key::Enter));
        navigator.input(&press(Key::Up));
        navigator.tick(Duration::from_millis(250));
        let camera = navigator.system().expect("open").camera().center();
        assert_eq!(camera, Point::new(0.0, -240.0));
    }

    /// Draws 0 every time, and records each `below(n)`.
    #[derive(Default)]
    struct Zeros(Vec<u32>);

    impl nova_sim::Chance for Zeros {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.0.push(n);
            0
        }
    }

    /// The frames Beta Flicker shows over 3 ticks in a navigator drawing
    /// on `chance`.
    fn flicker(navigator: Navigator<FakeCatalog>) -> Vec<Option<u16>> {
        let mut navigator = navigator;
        select(&mut navigator, 129);
        navigator.input(&press(Key::Enter));
        (0..3)
            .map(|_| {
                navigator.tick(Duration::from_secs(1) / 30);
                navigator.system().expect("open").frame(0)
            })
            .collect()
    }

    #[test]
    fn an_opened_systems_random_stellar_steps_by_the_navigators_chance() {
        let zeros = std::rc::Rc::new(std::cell::RefCell::new(Zeros::default()));
        let shown = flicker(navigator().with_chance(SharedChance::new(zeros.clone())));
        // From 1, a draw of 0 is frame 0; from 0, frame 1.
        assert_eq!(shown, [Some(1), Some(0), Some(1)]);
        assert_eq!(zeros.borrow().0, [3, 3, 3]);
        // The default takes the last frame allowed.
        assert_eq!(flicker(navigator()), [Some(1), Some(3), Some(2)]);
    }

    #[test]
    fn the_maps_find_is_laid_out_as_given_and_its_beeps_come_through() {
        let mut template = crate::galaxy::find::fallback();
        template.items[crate::galaxy::find::FIND_ITEM - 1].kind =
            crate::ui::dialog::ItemSpec::Button("Seek".to_owned());
        let mut navigator = navigator()
            .with_metrics(std::rc::Rc::new(crate::text::fixture::MonoMetrics))
            .with_find(Some(template), crate::ui::ButtonStyle::STOCK);
        navigator.input(&press(Key::Char('s')));
        assert!(navigator.map().finding());
        let mut list = DrawList::new();
        navigator.draw(&mut list);
        assert!(list.iter().any(|command| matches!(
            command,
            crate::DrawCommand::Text { text, .. } if text == "Seek"
        )));
        for c in ['s', 'b', 'e'] {
            navigator.input(&Input::Text(c));
        }
        navigator.input(&press(Key::Enter));
        assert_eq!(navigator.map().selected(), Some(SystemId(129)));
        assert_eq!(
            navigator.take_sounds(),
            [crate::Sound::Ui(crate::sound::UiSound::MapSystemFound)]
        );
        assert_eq!(navigator.take_sounds(), []);
    }
}
