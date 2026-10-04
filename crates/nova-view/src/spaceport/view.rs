//! The spaceport screen: the stellar landed on, laid out by the interface
//! file's "Spaceport" dialog over its background picture.
//!
//! The screen reads everything once, when it is built: the stellar's
//! record and landscape through the [`SpaceportCatalog`] port, and its
//! description (`dësc` = `spöb` ID) and the button style through
//! [`DescriptionSource`]. It draws the background, then the landscape
//! stretched into its item, then the stellar's name centred in the name
//! bar, then the dialog: the description scrolling in its box, Leave, and
//! a button for each service the stellar's flags offer. The other service
//! items stay blank, and clicks on them do nothing.
//!
//! Leave, Return and Escape leave: the router takes off. A service's
//! button opens its [`ServiceScreen`], which takes every input until it
//! closes. Without the dialog, or the stellar's record, the screen says
//! why, and Return or Escape still leaves.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{Service, services};

use super::catalog::{SpaceportCatalog, StellarId};
use super::layout::{
    BACKGROUND, LANDSCAPE_ITEM, LEAVE_ITEM, LEAVE_LABEL, NAME_FONT, NAME_ITEM, NAME_SIZE,
    TEXT_ITEM, item_service, label, landscape_id, service_item,
};
use super::service::ServiceScreen;
use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::Point;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::catalog::DescriptionSource;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};

/// Where the reason the spaceport cannot be shown goes, its size and
/// colour.
pub const PROBLEM_AT: Point = Point::new(16.0, 32.0);
/// The reason's size.
pub const PROBLEM_SIZE: f32 = 20.0;

/// The spaceport, laid out.
#[derive(Clone, Debug)]
struct Port {
    dialog: Dialog,
    name: String,
    /// The landscape `PICT`, or the missing one's ID.
    landscape: Result<i16, i16>,
    offered: Vec<Service>,
    /// Why there is no description, if there is none.
    description_problem: Option<String>,
    style: ButtonStyle,
    metrics: MetricsHandle,
}

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// The spaceport of the stellar landed on.
#[derive(Clone, Debug)]
pub struct SpaceportView {
    stellar: StellarId,
    /// The spaceport, or why it cannot be shown.
    port: Result<Port, String>,
    /// The service open over it, if any.
    open: Option<ServiceScreen>,
    left: bool,
}

impl SpaceportView {
    /// The spaceport of `stellar`, read from `catalog` once and laid out by
    /// `layout`: the "Spaceport" dialog's template and the metrics its text
    /// is measured by, or why there are none.
    pub fn new(
        catalog: &(impl SpaceportCatalog + DescriptionSource),
        stellar: StellarId,
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
    ) -> Self {
        let port = layout.and_then(|(template, metrics)| {
            let record = catalog.stellar_port(stellar)?;
            let offered = services(record.flags);
            let (text, description_problem) = match catalog.description(stellar.0) {
                Ok(text) => (text, None),
                Err(reason) => (String::new(), Some(reason)),
            };
            let mut roles = vec![
                (LEAVE_ITEM, Role::Button(LEAVE_LABEL.to_owned())),
                (
                    TEXT_ITEM,
                    Role::ScrollText {
                        text,
                        font: Font::Geneva,
                        size: BODY_SIZE,
                        color: BODY_COLOR,
                    },
                ),
            ];
            roles.extend(offered.iter().map(|&service| {
                (
                    service_item(service),
                    Role::Button(label(service).to_owned()),
                )
            }));
            let style = catalog.button_style();
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(LEAVE_ITEM))
                .with_cancel(Some(LEAVE_ITEM));
            let landscape = landscape_id(record.cust_pic_id, record.graphic_type);
            Ok(Port {
                dialog,
                name: record.name,
                landscape: if catalog.picture_exists(landscape) {
                    Ok(landscape)
                } else {
                    Err(landscape)
                },
                offered,
                description_problem,
                style,
                metrics: MetricsHandle(metrics),
            })
        });
        Self {
            stellar,
            port,
            open: None,
            left: false,
        }
    }

    /// The stellar landed on.
    #[must_use]
    pub fn stellar(&self) -> StellarId {
        self.stellar
    }

    /// Whether the player has left: the router takes off.
    #[must_use]
    pub fn left(&self) -> bool {
        self.left
    }

    /// The services offered, in [`Service`] order.
    #[must_use]
    pub fn offered(&self) -> &[Service] {
        self.port.as_ref().map_or(&[], |port| &port.offered)
    }

    /// The service open, if any.
    #[must_use]
    pub fn open_service(&self) -> Option<&ServiceScreen> {
        self.open.as_ref()
    }

    /// The dialog, if the spaceport could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.port.as_ref().ok().map(|port| &port.dialog)
    }

    /// Why the spaceport cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.port.as_ref().err().map(String::as_str)
    }

    /// The landscape `PICT` that is missing, if it is.
    #[must_use]
    pub fn missing_landscape(&self) -> Option<i16> {
        self.port.as_ref().ok()?.landscape.err()
    }

    /// Why the description box is empty, if it is for want of a `dësc`.
    #[must_use]
    pub fn description_problem(&self) -> Option<&str> {
        self.port.as_ref().ok()?.description_problem.as_deref()
    }

    /// Activates dialog item `item`: Leave leaves, and an offered service's
    /// button opens it. Anything else does nothing.
    fn activate(&mut self, item: usize) {
        if item == LEAVE_ITEM {
            self.left = true;
            return;
        }
        let Ok(port) = &self.port else {
            return;
        };
        if let Some(service) = item_service(item).filter(|s| port.offered.contains(s)) {
            self.open = Some(ServiceScreen::new(
                service,
                port.style,
                Rc::clone(&port.metrics.0),
            ));
        }
    }
}

impl Screen for SpaceportView {
    /// An open service takes every input until it closes. Otherwise every
    /// input goes to the dialog. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(open) = &mut self.open {
            open.input(input);
            if open.closed() {
                self.open = None;
            }
            return ScreenAction::None;
        }
        let event = match &mut self.port {
            Ok(port) => port.dialog.input(input),
            Err(_) => match *input {
                Input::Key {
                    key: Key::Enter | Key::Escape,
                    pressed: true,
                    repeat: false,
                } => Some(DialogEvent::Item(LEAVE_ITEM)),
                _ => None,
            },
        };
        if let Some(DialogEvent::Item(item)) = event {
            self.activate(item);
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        if let Some(open) = &self.open {
            open.draw(list);
            return;
        }
        let port = match &self.port {
            Ok(port) => port,
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the spaceport: {reason}. Press Return or Escape to take off."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
                return;
            }
        };
        let dialog = &port.dialog;
        list.picture(BACKGROUND, dialog.bounds().min);
        if let (Ok(id), Some(area)) = (port.landscape, dialog.item_bounds(LANDSCAPE_ITEM)) {
            list.stretched_picture(ImageKey::picture(id), area.min, area.width(), area.height());
        }
        if let Some(bar) = dialog.item_bounds(NAME_ITEM) {
            let metrics = &port.metrics.0;
            let width = metrics.width(NAME_FONT, NAME_SIZE, &port.name);
            let height = metrics.line_height(NAME_FONT, NAME_SIZE);
            let centre = bar.center();
            let origin = Point::new(centre.x - width / 2.0, centre.y - height / 2.0);
            list.text_in(NAME_FONT, &port.name, origin, NAME_SIZE, None, Color::WHITE);
        }
        dialog.draw(list);
    }

    fn cancel_pointer(&mut self) {
        match (&mut self.open, &mut self.port) {
            (Some(open), _) => open.cancel_pointer(),
            (None, Ok(port)) => port.dialog.cancel_pointer(),
            (None, Err(_)) => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::landing::StellarFlags;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::Bounds;
    use crate::input::MouseButton;
    use crate::spaceport::catalog::PortRecord;
    use crate::spaceport::layout::SERVICE_ITEMS;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    /// Stellar 140, Earth: a planet with a trade center and a bar, of
    /// `Type` 3 with no custom picture, so its landscape is `PICT` 10003.
    struct FakePort {
        port: Result<PortRecord, String>,
        pictures: Vec<i16>,
        description: Result<String, String>,
        /// The stellars and descriptions asked for.
        asked: RefCell<Vec<String>>,
    }

    const FLAGS: u32 = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | StellarFlags::BAR;

    fn catalog() -> FakePort {
        FakePort {
            port: Ok(PortRecord {
                name: "Earth".to_owned(),
                flags: FLAGS,
                cust_pic_id: -1,
                graphic_type: 3,
            }),
            pictures: vec![10_003],
            description: Ok("Blue and green.\rHome.".to_owned()),
            asked: RefCell::default(),
        }
    }

    impl SpaceportCatalog for FakePort {
        fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
            self.asked.borrow_mut().push(format!("spöb {}", id.0));
            self.port.clone()
        }

        fn picture_exists(&self, id: i16) -> bool {
            self.pictures.contains(&id)
        }
    }

    impl DescriptionSource for FakePort {
        fn description(&self, id: i16) -> Result<String, String> {
            self.asked.borrow_mut().push(format!("dësc {id}"));
            self.description.clone()
        }

        fn button_style(&self) -> ButtonStyle {
            ButtonStyle {
                up: Color::BLACK,
                ..ButtonStyle::STOCK
            }
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock "Spaceport": `DLOG` 1000, 618 x 517 and centred, and its
    /// fifteen user items, where items 2, 3, 6, 14 and 15 are disabled and
    /// items 1, 2, 14 and 15 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (242.0, 551.0, 200.0, 25.0, true),
            (452.0, 549.0, 68.0, 30.0, false),
            (159.0, 297.0, 303.0, 18.0, false),
            (471.0, 416.0, 145.0, 25.0, true),
            (3.0, 3.0, 612.0, 285.0, true),
            (160.0, 327.0, 301.0, 185.0, false),
            (3.0, 414.0, 145.0, 25.0, true),
            (471.0, 375.0, 145.0, 25.0, true),
            (471.0, 333.0, 145.0, 25.0, true),
            (3.0, 374.0, 145.0, 25.0, true),
            (3.0, 333.0, 145.0, 25.0, true),
            (471.0, 456.0, 145.0, 25.0, true),
            (3.0, 456.0, 145.0, 25.0, true),
            (524.0, 550.0, 68.0, 30.0, false),
            (605.0, 548.0, 32.0, 32.0, false),
        ];
        DialogTemplate {
            bounds: Bounds {
                min: Point::new(60.0, -201.0),
                max: Point::new(678.0, 316.0),
            },
            placement: Placement::Center,
            items: items
                .into_iter()
                .map(|(x, y, w, h, enabled)| ItemTemplate {
                    bounds: rect(x, y, w, h),
                    enabled,
                    kind: ItemSpec::User,
                })
                .collect(),
        }
    }

    /// Where the dialog goes: (1024 - 618) / 2, (768 - 517) / 2 floored.
    const ORIGIN: Point = Point::new(203.0, 125.0);

    fn on_screen(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        rect(ORIGIN.x + x, ORIGIN.y + y, w, h)
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    fn view_of(catalog: &FakePort) -> SpaceportView {
        SpaceportView::new(catalog, StellarId(140), Ok(layout()))
    }

    fn earth() -> SpaceportView {
        view_of(&catalog())
    }

    fn drawn(view: &impl Screen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        view.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(commands: &[DrawCommand]) -> Vec<String> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click(view: &mut SpaceportView, at: Point) {
        for pressed in [true, false] {
            view.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(view: &mut SpaceportView, number: usize) {
        let at = item(view, number).center();
        click(view, at);
    }

    fn item(view: &SpaceportView, item: usize) -> Bounds {
        view.dialog()
            .expect("laid out")
            .item_bounds(item)
            .expect("an item")
    }

    #[test]
    fn it_reads_the_stellar_and_its_description_once() {
        let catalog = catalog();
        let view = view_of(&catalog);
        assert_eq!(*catalog.asked.borrow(), ["spöb 140", "dësc 140"]);
        assert_eq!(view.stellar(), StellarId(140));
        assert_eq!(
            view.offered(),
            [Service::TradeCenter, Service::Bar, Service::MissionBbs]
        );
        assert!(!view.left());
        assert_eq!(view.problem(), None);
        assert_eq!(view.missing_landscape(), None);
        assert_eq!(view.description_problem(), None);
        assert!(view.open_service().is_none());
        assert_eq!(view.dialog().expect("laid out").bounds().min, ORIGIN);
    }

    #[test]
    fn it_draws_the_background_the_landscape_the_name_then_the_dialog() {
        let view = earth();
        let commands = drawn(&view);
        let name_width = 5.0 * 7.2;
        assert_eq!(
            commands[..3],
            [
                DrawCommand::Picture {
                    image: ImageKey::picture(8500),
                    top_left: ORIGIN,
                },
                DrawCommand::StretchedPicture {
                    image: ImageKey::picture(10_003),
                    top_left: Point::new(206.0, 128.0),
                    width: 612.0,
                    height: 285.0,
                },
                DrawCommand::Text {
                    text: "Earth".to_owned(),
                    font: Font::Charcoal,
                    origin: Point::new(
                        203.0 + 159.0 + 151.5 - name_width / 2.0,
                        125.0 + 297.0 + 9.0 - 14.4 / 2.0
                    ),
                    size: 12.0,
                    wrap_width: None,
                    color: Color::WHITE,
                },
            ]
        );
        let mut dialog = DrawList::new();
        view.dialog().expect("laid out").draw(&mut dialog);
        assert_eq!(
            commands[3..],
            dialog.iter().cloned().collect::<Vec<_>>()[..]
        );
    }

    #[test]
    fn only_the_offered_services_and_leave_are_buttons() {
        let view = earth();
        let commands = drawn(&view);
        let labels = texts(&commands);
        for shown in ["Trade Center", "Bar", "Mission BBS", "Leave"] {
            assert!(labels.contains(&shown.to_owned()), "{shown}: {labels:?}");
        }
        for hidden in ["Shipyard", "Outfitter", "Recharge", "Done"] {
            assert!(!labels.contains(&hidden.to_owned()), "{hidden}: {labels:?}");
        }
        // Each in its own item, with the button style asked for.
        let label_at = |text: &str| {
            commands.iter().find_map(|command| match command {
                DrawCommand::Text {
                    text: t,
                    origin,
                    color,
                    ..
                } if t == text => Some((*origin, *color)),
                _ => None,
            })
        };
        for (text, item_number) in [
            ("Trade Center", 7),
            ("Bar", 10),
            ("Mission BBS", 11),
            ("Leave", 12),
        ] {
            let (origin, color) = label_at(text).expect(text);
            assert!(item(&view, item_number).contains(origin), "{text}");
            assert_eq!(color, Color::BLACK, "{text}");
        }
        assert_eq!(item(&view, 12), on_screen(471.0, 456.0, 145.0, 25.0));
    }

    #[test]
    fn the_description_scrolls_in_its_box() {
        let view = earth();
        let text = view
            .dialog()
            .expect("laid out")
            .scroll_text()
            .expect("the description");
        assert_eq!(text.lines(), ["Blue and green.", "Home."]);
        let first = drawn(&view)
            .into_iter()
            .find(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Blue and green."))
            .expect("drawn");
        let DrawCommand::Text {
            origin,
            font,
            size,
            color,
            ..
        } = first
        else {
            unreachable!()
        };
        assert_eq!(origin, on_screen(160.0, 327.0, 0.0, 0.0).min);
        assert_eq!((font, size, color), (Font::Geneva, 10.0, Color::WHITE));
    }

    #[test]
    fn leave_return_and_escape_leave() {
        let mut view = earth();
        click_item(&mut view, 12);
        assert!(view.left());
        for k in [Key::Enter, Key::Escape] {
            let mut view = earth();
            assert_eq!(view.input(&key(k)), ScreenAction::None);
            assert!(view.left(), "{k:?}");
        }
    }

    #[test]
    fn clicks_elsewhere_do_nothing() {
        let mut view = earth();
        // The shipyard's and outfitter's places, the blank buttons, the
        // landscape and the description.
        for number in [9, 8, 4, 13, 5, 6, 3] {
            click_item(&mut view, number);
        }
        click(&mut view, Point::new(5.0, 5.0));
        view.input(&Input::Key {
            key: Key::Escape,
            pressed: true,
            repeat: true,
        });
        assert!(!view.left());
        assert!(view.open_service().is_none());
    }

    #[test]
    fn a_service_opens_its_placeholder_which_done_closes() {
        let mut view = earth();
        click_item(&mut view, 7);
        let open = view.open_service().expect("open");
        assert_eq!(open.service(), Service::TradeCenter);
        let mut expected = DrawList::new();
        open.draw(&mut expected);
        assert_eq!(drawn(&view), expected.iter().cloned().collect::<Vec<_>>());
        assert!(texts(&drawn(&view)).contains(&"Trade Center".to_owned()));
        assert!(!texts(&drawn(&view)).contains(&"Leave".to_owned()));
        // Escape closes the service, not the spaceport.
        view.input(&key(Key::Escape));
        assert!(view.open_service().is_none());
        assert!(!view.left());
        // Done, clicked.
        click_item(&mut view, 10);
        assert_eq!(
            view.open_service().map(ServiceScreen::service),
            Some(Service::Bar)
        );
        click(&mut view, super::super::service::DONE_BUTTON.center());
        assert!(view.open_service().is_none());
        assert!(!view.left());
        // Every service's button opens it.
        for (service, number) in SERVICE_ITEMS {
            let catalog = FakePort {
                port: Ok(PortRecord {
                    flags: 0xFF & !StellarFlags::UNINHABITED,
                    ..catalog().port.expect("a record")
                }),
                ..catalog()
            };
            let mut view = view_of(&catalog);
            click_item(&mut view, number);
            assert_eq!(
                view.open_service().map(ServiceScreen::service),
                Some(service)
            );
        }
    }

    #[test]
    fn an_open_service_takes_cancel_pointer() {
        let mut view = earth();
        click_item(&mut view, 7);
        let done = super::super::service::DONE_BUTTON.center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert!(view.open_service().is_some(), "the click was abandoned");
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_leave() {
        let mut view = earth();
        let leave = item(&view, 12).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: leave,
        };
        view.input(&button(true));
        view.cancel_pointer();
        view.input(&button(false));
        assert!(!view.left());
    }

    #[test]
    fn a_missing_landscape_leaves_its_box_as_the_background_shows_it() {
        let catalog = FakePort {
            pictures: Vec::new(),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.missing_landscape(), Some(10_003));
        let commands = drawn(&view);
        assert!(matches!(commands[0], DrawCommand::Picture { .. }));
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, DrawCommand::StretchedPicture { image, .. } if *image == ImageKey::picture(10_003)))
        );
        assert!(matches!(&commands[1], DrawCommand::Text { text, .. } if text == "Earth"));
    }

    #[test]
    fn a_custom_landscape_is_drawn_when_named() {
        let catalog = FakePort {
            port: Ok(PortRecord {
                cust_pic_id: 9000,
                ..catalog().port.expect("a record")
            }),
            pictures: vec![9000],
            ..catalog()
        };
        let commands = drawn(&view_of(&catalog));
        assert!(
            matches!(commands[1], DrawCommand::StretchedPicture { image, .. } if image == ImageKey::picture(9000))
        );
    }

    #[test]
    fn a_missing_description_leaves_its_box_empty() {
        let catalog = FakePort {
            description: Err("no dësc 140".to_owned()),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.description_problem(), Some("no dësc 140"));
        let text = view
            .dialog()
            .expect("laid out")
            .scroll_text()
            .expect("a box");
        assert!(text.lines().is_empty());
        assert!(!texts(&drawn(&view)).iter().any(|t| t.contains("dësc")));
    }

    #[test]
    fn an_uninhabited_stellar_offers_only_leave() {
        let catalog = FakePort {
            port: Ok(PortRecord {
                flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                ..catalog().port.expect("a record")
            }),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.offered(), []);
        let labels = texts(&drawn(&view));
        assert!(labels.contains(&"Leave".to_owned()));
        assert!(!labels.contains(&"Mission BBS".to_owned()));
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the spaceport: {reason}. Press Return or Escape to take off."
            ),
            font: Font::Geneva,
            origin: PROBLEM_AT,
            size: PROBLEM_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        }
    }

    #[test]
    fn without_the_dialog_it_says_why_and_return_or_escape_leaves() {
        for k in [Key::Enter, Key::Escape] {
            let mut view =
                SpaceportView::new(&catalog(), StellarId(140), Err("no DLOG 1000".to_owned()));
            assert_eq!(view.problem(), Some("no DLOG 1000"));
            assert_eq!(drawn(&view), [problem("no DLOG 1000")]);
            assert!(view.dialog().is_none());
            assert_eq!(view.offered(), []);
            assert_eq!(view.missing_landscape(), None);
            assert_eq!(view.description_problem(), None);
            for other in [
                Input::Key {
                    key: k,
                    pressed: true,
                    repeat: true,
                },
                Input::Key {
                    key: k,
                    pressed: false,
                    repeat: false,
                },
                key(Key::Space),
            ] {
                view.input(&other);
            }
            click(&mut view, Point::new(500.0, 500.0));
            view.cancel_pointer();
            assert!(!view.left());
            view.input(&key(k));
            assert!(view.left(), "{k:?}");
        }
        assert_eq!((PROBLEM_AT, PROBLEM_SIZE), (Point::new(16.0, 32.0), 20.0));
    }

    #[test]
    fn an_unreadable_stellar_says_why() {
        let catalog = FakePort {
            port: Err("no spöb 140".to_owned()),
            ..catalog()
        };
        let view = view_of(&catalog);
        assert_eq!(view.problem(), Some("no spöb 140"));
        assert_eq!(drawn(&view), [problem("no spöb 140")]);
    }

    #[test]
    fn nothing_moves_with_time() {
        let mut view = earth();
        let before = drawn(&view);
        view.tick(Duration::from_secs(5));
        assert_eq!(drawn(&view), before);
        assert!(format!("{view:?}").contains("TextMetrics"));
    }
}
