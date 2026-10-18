//! The main screen's pilot panel: what `_MainScreenUpdate`
//! (@0x17dcc-0x184ea) draws under the buttons about the pilot loaded, or
//! the message in its place, and the port its looks come through
//! ([`PilotPanelSource`]).
//!
//! Everything is placed from `mainMenuCenter` (cx, cy), in `cölr` 128's
//! `MenuFont` at `MenuFontSize` (`TextFont`/`TextSize` @0x17e17-0x17e32),
//! the labels in `MenuColor2` and the values in `MenuColor1`. A text's top
//! is its baseline less the font size.
//!
//! **A pilot loaded and alive**: the labels `STR#` 2002 #251, #255 and
//! #256 at cx−190 and their values at cx−185 (the pilot's name, the ship's
//! name, its class and, under it, its `shïp` Subtitle), and #278, #279,
//! #254 and #252 at cx+120 or cx+125 with the legal status, the combat
//! rating and the date at cx+125; their baselines are in [`ROWS`]. Then
//! the ship's target picture (`targetInfoPic[shipClass[type]+0xa0c]`,
//! @0x18247-0x18382; see [`PanelShip`]): its rectangle from
//! `QDGetPictureBounds`, moved to (cx − width/2, cy + 280), the halving
//! truncated toward zero, and copied with QuickDraw's addOver (`CopyBits`
//! mode 0x22). addOver adds per channel and wraps, where
//! [`Blend::Additive`] saturates; the two agree wherever the destination
//! is black, as it is under every stock picture. A ship with no picture
//! draws none (@0x18270).
//!
//! **A pilot loaded and dying** (`_IsDying`): only the name, a space and
//! #277 "has been killed", centred, or "Oh my God! They killed Kenny!"
//! for a name starting "Kenny", case-sensitively (@0x1838c-0x18437). The
//! original compares the Pascal name's bytes 1 to 5 with "Kenny", reading
//! past a shorter name's length; the engine tests the prefix.
//!
//! **No pilot loaded**: #276 "No Pilot File Loaded", centred, once slide 2
//! has finished (`_slideIndex[2] >= _slideFrameCount[2]`, @0x18472-0x184e5;
//! [`MainMenu::row_shown`](super::MainMenu::row_shown)) and not before.
//!
//! **Centred** is `_DrawCenteredText` @0x263d between cx−150 and cx+150 at
//! baseline cy+310: x = cx − width/2, truncated toward zero. All three
//! messages are in the labels' colour, `MenuColor2` (@0x17e41, @0x1838c,
//! @0x18488).
//!
//! The range plays no sound. Its "Creating Pilot..." (#275, @0x17e37-0x17e6f)
//! is left out: the engine creates and saves a pilot inside one input, so
//! no frame is drawn while the original's `_creatingPilotFile` is set
//! (rdm:phase/main-screen-fidelity/phase-5-pilot-panel-left-outs).

use std::rc::Rc;

use nova_sim::catalog::ShipId;
use nova_sim::pilot_panel::PilotPanel;

use crate::color::Color;
use crate::draw::{Blend, DrawList};
use crate::font::Font;
use crate::geometry::Point;
use crate::image::ImageKey;
use crate::text::TextMetrics;

/// The `STR#` 2002 labels and messages, by number.
pub const PILOT_NAME_LABEL: usize = 251;
/// "Current Date:".
pub const DATE_LABEL: usize = 252;
/// "Combat Rating:".
pub const RATING_LABEL: usize = 254;
/// "Ship Name:".
pub const SHIP_NAME_LABEL: usize = 255;
/// "Ship Class:".
pub const SHIP_CLASS_LABEL: usize = 256;
/// "No Pilot File Loaded".
pub const NO_PILOT: usize = 276;
/// "has been killed".
pub const KILLED: usize = 277;
/// "Legal status in".
pub const LEGAL_LABEL: usize = 278;
/// "current system:".
pub const SYSTEM_LABEL: usize = 279;

/// What a name starting "Kenny" shows in place of the killed message (the
/// Pascal literal @0xd8378).
pub const KENNY: &str = "Oh my God! They killed Kenny!";
/// The name prefix that shows [`KENNY`].
const KENNY_PREFIX: &str = "Kenny";

/// How far below cy the picture's top is.
pub const PICTURE_TOP: f32 = 280.0;
/// The centred messages' baseline, below cy.
pub const MESSAGE_BASELINE: f32 = 310.0;

/// The labels and messages the panel draws.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelLabels {
    /// "Pilot Name:" (#251).
    pub pilot_name: String,
    /// "Current Date:" (#252).
    pub date: String,
    /// "Combat Rating:" (#254).
    pub combat_rating: String,
    /// "Ship Name:" (#255).
    pub ship_name: String,
    /// "Ship Class:" (#256).
    pub ship_class: String,
    /// "No Pilot File Loaded" (#276).
    pub no_pilot: String,
    /// "has been killed" (#277).
    pub killed: String,
    /// "Legal status in" (#278).
    pub legal_status: String,
    /// "current system:" (#279).
    pub current_system: String,
}

impl PanelLabels {
    /// The labels from `STR#` 2002, `strings` its entries from #1; a
    /// missing one is empty.
    #[must_use]
    pub fn from_list(strings: &[String]) -> Self {
        let at = |n: usize| strings.get(n - 1).cloned().unwrap_or_default();
        Self {
            pilot_name: at(PILOT_NAME_LABEL),
            date: at(DATE_LABEL),
            combat_rating: at(RATING_LABEL),
            ship_name: at(SHIP_NAME_LABEL),
            ship_class: at(SHIP_CLASS_LABEL),
            no_pilot: at(NO_PILOT),
            killed: at(KILLED),
            legal_status: at(LEGAL_LABEL),
            current_system: at(SYSTEM_LABEL),
        }
    }
}

/// How the panel looks: `cölr` 128's menu font and colours, and the
/// labels.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelLooks {
    /// `MenuFont`.
    pub font: Font,
    /// `MenuFontSize`.
    pub size: f32,
    /// `MenuColor1`, the values'.
    pub value: Color,
    /// `MenuColor2`, the labels' and the messages'.
    pub label: Color,
    /// The labels and messages.
    pub labels: PanelLabels,
}

impl PanelLooks {
    /// Stock `cölr` 128's font and colours: Geneva 9, values 0xFF0000 and
    /// labels 0x800000.
    pub const STOCK_FONT: Font = Font::Geneva;
    /// Stock `MenuFontSize`.
    pub const STOCK_SIZE: f32 = 9.0;
    /// Stock `MenuColor1`.
    pub const STOCK_VALUE: Color = Color::from_rgb24(0x00FF_0000);
    /// Stock `MenuColor2`.
    pub const STOCK_LABEL: Color = Color::from_rgb24(0x0080_0000);

    /// The stock font and colours with `labels`.
    #[must_use]
    pub fn stock(labels: PanelLabels) -> Self {
        Self {
            font: Self::STOCK_FONT,
            size: Self::STOCK_SIZE,
            value: Self::STOCK_VALUE,
            label: Self::STOCK_LABEL,
            labels,
        }
    }
}

/// A ship's target picture: `PICT` `picture`, `width` x `height`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipPicture {
    /// The `PICT`'s ID.
    pub picture: i16,
    /// Its width.
    pub width: u32,
    /// Its height.
    pub height: u32,
}

/// What the panel shows of a ship type from the game data: its `shïp`
/// Subtitle and its target picture.
///
/// The picture is `PICT` 3000 + (ID − 128), the status bar's, when it
/// exists (`_LoadSprites` @0x217f2-0x2180c, `_LoadShipSprites`
/// @0x23f10-0x23f1e). Otherwise it is the base ship's: the first lower-ID
/// `shïp` whose `shän` has the same `BaseImage`
/// (`_LoadExtendedShipSprites` @0x227d6-0x22849), and its picture if it has
/// one (@0x23f28-0x23f3b). A `shïp` with no `shän` has none (@0x23ef3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelShip {
    /// The `shïp`'s Subtitle (`shipClass`+0xac).
    pub subtitle: String,
    /// Its target picture, if it has one.
    pub picture: Option<ShipPicture>,
}

/// Where the pilot panel's looks and ships come from.
pub trait PilotPanelSource {
    /// The panel's looks: `cölr` 128's menu font and colours, the stock
    /// ones without it ([`PanelLooks::stock`]), and the `STR#` 2002 labels.
    fn pilot_panel_looks(&self) -> PanelLooks;

    /// What the panel shows of ship type `ship` (see [`PanelShip`]); the
    /// default for a type with no `shïp`.
    fn panel_ship(&self, ship: ShipId) -> PanelShip;
}

/// A borrowed source is a source.
impl<T: PilotPanelSource + ?Sized> PilotPanelSource for &T {
    fn pilot_panel_looks(&self) -> PanelLooks {
        (**self).pilot_panel_looks()
    }

    fn panel_ship(&self, ship: ShipId) -> PanelShip {
        (**self).panel_ship(ship)
    }
}

/// A shared source is a source.
impl<T: PilotPanelSource + ?Sized> PilotPanelSource for Rc<T> {
    fn pilot_panel_looks(&self) -> PanelLooks {
        (**self).pilot_panel_looks()
    }

    fn panel_ship(&self, ship: ShipId) -> PanelShip {
        (**self).panel_ship(ship)
    }
}

/// The pilot the panel is about, as the router holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PanelPilot {
    /// No pilot is loaded: "No Pilot File Loaded" once slide 2 has
    /// finished.
    #[default]
    None,
    /// A pilot loaded and alive.
    Alive(Box<PilotPanel>),
    /// A pilot loaded and dying, by name.
    Dying(String),
    /// Nothing at all: a pilot whose session failed, which only the engine
    /// can hold.
    Blank,
}

/// The panel's state: its looks, the pilot shown and the ship read for it.
#[derive(Clone, Debug, PartialEq)]
pub struct PilotPanelArea {
    looks: PanelLooks,
    pilot: PanelPilot,
    /// The ship type last read, and what was read of it.
    ship: Option<(ShipId, PanelShip)>,
}

/// Each alive row's x and baseline from (cx, cy), in the phase table's
/// order: the left column's labels and values, then the right's.
pub const ROWS: [(f32, f32); 14] = [
    (-190.0, 250.0),
    (-185.0, 262.0),
    (-190.0, 286.0),
    (-185.0, 298.0),
    (-190.0, 322.0),
    (-185.0, 334.0),
    (-185.0, 346.0),
    (120.0, 250.0),
    (125.0, 262.0),
    (125.0, 274.0),
    (120.0, 298.0),
    (125.0, 310.0),
    (120.0, 334.0),
    (125.0, 346.0),
];

impl PilotPanelArea {
    /// A panel in `looks`, showing no pilot.
    #[must_use]
    pub fn new(looks: PanelLooks) -> Self {
        Self {
            looks,
            pilot: PanelPilot::None,
            ship: None,
        }
    }

    /// The pilot shown.
    #[must_use]
    pub fn pilot(&self) -> &PanelPilot {
        &self.pilot
    }

    /// Shows `pilot`, reading its ship from `source` only when the type
    /// differs from the one last read.
    pub fn show(&mut self, pilot: PanelPilot, source: &dyn PilotPanelSource) {
        if let PanelPilot::Alive(panel) = &pilot
            && self
                .ship
                .as_ref()
                .is_none_or(|(ship, _)| *ship != panel.ship)
        {
            self.ship = Some((panel.ship, source.panel_ship(panel.ship)));
        }
        self.pilot = pilot;
    }

    /// Draws the panel centred on `center`, `row_2_shown` whether slide 2
    /// has finished (see the module docs).
    pub fn draw(
        &self,
        list: &mut DrawList,
        center: Point,
        row_2_shown: bool,
        metrics: &dyn TextMetrics,
    ) {
        match &self.pilot {
            PanelPilot::None => {
                if row_2_shown {
                    self.centred(list, center, &self.looks.labels.no_pilot, metrics);
                }
            }
            PanelPilot::Dying(name) => {
                let text = if name.starts_with(KENNY_PREFIX) {
                    KENNY.to_owned()
                } else {
                    format!("{name} {}", self.looks.labels.killed)
                };
                self.centred(list, center, &text, metrics);
            }
            PanelPilot::Alive(panel) => self.draw_alive(list, center, panel),
            PanelPilot::Blank => {}
        }
    }

    fn draw_alive(&self, list: &mut DrawList, center: Point, panel: &PilotPanel) {
        let labels = &self.looks.labels;
        let ship = self
            .ship
            .as_ref()
            .filter(|(ship, _)| *ship == panel.ship)
            .map(|(_, ship)| ship);
        let subtitle = ship.map_or("", |ship| ship.subtitle.as_str());
        let (label, value) = (self.looks.label, self.looks.value);
        let texts: [(&str, Color); 14] = [
            (&labels.pilot_name, label),
            (&panel.pilot_name, value),
            (&labels.ship_name, label),
            (&panel.ship_name, value),
            (&labels.ship_class, label),
            (&panel.ship_class, value),
            (subtitle, value),
            (&labels.legal_status, label),
            (&labels.current_system, label),
            (&panel.legal_status, value),
            (&labels.combat_rating, label),
            (&panel.combat_rating, value),
            (&labels.date, label),
            (&panel.date, value),
        ];
        for ((text, color), (dx, dy)) in texts.into_iter().zip(ROWS) {
            self.text(list, text, Point::new(center.x + dx, center.y + dy), color);
        }
        if let Some(picture) = ship.and_then(|ship| ship.picture) {
            let half = picture.width / 2;
            list.blended_picture(
                ImageKey::picture(picture.picture),
                Point::new(center.x - half as f32, center.y + PICTURE_TOP),
                picture.width as f32,
                picture.height as f32,
                Blend::Additive,
            );
        }
    }

    /// Draws `text` centred as `_DrawCenteredText` centres it, in the
    /// labels' colour.
    fn centred(&self, list: &mut DrawList, center: Point, text: &str, metrics: &dyn TextMetrics) {
        let width = metrics.width(self.looks.font, self.looks.size, text);
        let x = center.x - (width / 2.0).trunc();
        let at = Point::new(x, center.y + MESSAGE_BASELINE);
        self.text(list, text, at, self.looks.label);
    }

    /// Draws `text` from `at`, its baseline, in `color`; an empty text
    /// draws nothing, as `DrawString` of an empty string draws nothing.
    fn text(&self, list: &mut DrawList, text: &str, at: Point, color: Color) {
        if text.is_empty() {
            return;
        }
        let size = self.looks.size;
        list.text_in(
            self.looks.font,
            text,
            Point::new(at.x, at.y - size),
            size,
            None,
            color,
        );
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    /// A centre other than the classic (512, 384).
    const CENTER: Point = Point::new(500.0, 400.0);
    const VALUE: Color = Color::rgba(1, 2, 3, 255);
    const LABEL: Color = Color::rgba(4, 5, 6, 255);

    fn labels() -> PanelLabels {
        PanelLabels {
            pilot_name: "Pilot Name:".to_owned(),
            date: "Current Date:".to_owned(),
            combat_rating: "Combat Rating:".to_owned(),
            ship_name: "Ship Name:".to_owned(),
            ship_class: "Ship Class:".to_owned(),
            no_pilot: "No Pilot File Loaded".to_owned(),
            killed: "has been killed".to_owned(),
            legal_status: "Legal status in".to_owned(),
            current_system: "current system:".to_owned(),
        }
    }

    fn looks() -> PanelLooks {
        PanelLooks {
            font: Font::Charcoal,
            size: 10.0,
            value: VALUE,
            label: LABEL,
            labels: labels(),
        }
    }

    /// A source whose every ship is subtitled "Light Transport" with a
    /// 41 x 20 `PICT` 3005, but for 200, which has no picture; it counts
    /// its ship reads.
    #[derive(Default)]
    struct Ships {
        reads: RefCell<Vec<ShipId>>,
    }

    impl PilotPanelSource for Ships {
        fn pilot_panel_looks(&self) -> PanelLooks {
            looks()
        }

        fn panel_ship(&self, ship: ShipId) -> PanelShip {
            self.reads.borrow_mut().push(ship);
            PanelShip {
                subtitle: "Light Transport".to_owned(),
                picture: (ship != ShipId(200)).then_some(ShipPicture {
                    picture: 3005,
                    width: 41,
                    height: 20,
                }),
            }
        }
    }

    fn model(ship: ShipId) -> PilotPanel {
        PilotPanel {
            pilot_name: "Ada".to_owned(),
            ship_name: "Bitsy".to_owned(),
            ship_class: "Shuttle".to_owned(),
            ship,
            legal_status: "Clean".to_owned(),
            combat_rating: "Average".to_owned(),
            date: "1/1/1177".to_owned(),
        }
    }

    fn alive(ship: ShipId) -> PanelPilot {
        PanelPilot::Alive(Box::new(model(ship)))
    }

    fn showing(pilot: PanelPilot) -> PilotPanelArea {
        let mut area = PilotPanelArea::new(looks());
        area.show(pilot, &Ships::default());
        area
    }

    fn drawn(area: &PilotPanelArea, row_2_shown: bool) -> DrawList {
        let mut list = DrawList::new();
        area.draw(&mut list, CENTER, row_2_shown, &MonoMetrics);
        list
    }

    /// Each text drawn: its text, x, baseline and colour, in Charcoal 10.
    fn texts(list: &DrawList) -> Vec<(String, f32, f32, Color)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    font,
                    origin,
                    size,
                    wrap_width,
                    color,
                } => {
                    assert_eq!((*font, *size, *wrap_width), (Font::Charcoal, 10.0, None));
                    Some((text.clone(), origin.x, origin.y + 10.0, *color))
                }
                _ => None,
            })
            .collect()
    }

    fn pictures(list: &DrawList) -> Vec<&DrawCommand> {
        list.iter()
            .filter(|command| matches!(command, DrawCommand::BlendedPicture { .. }))
            .collect()
    }

    fn row(text: &str, dx: f32, dy: f32, color: Color) -> (String, f32, f32, Color) {
        (text.to_owned(), CENTER.x + dx, CENTER.y + dy, color)
    }

    #[test]
    fn an_alive_pilot_draws_the_fourteen_rows_at_their_places() {
        let list = drawn(&showing(alive(ShipId(133))), false);
        assert_eq!(
            texts(&list),
            [
                row("Pilot Name:", -190.0, 250.0, LABEL),
                row("Ada", -185.0, 262.0, VALUE),
                row("Ship Name:", -190.0, 286.0, LABEL),
                row("Bitsy", -185.0, 298.0, VALUE),
                row("Ship Class:", -190.0, 322.0, LABEL),
                row("Shuttle", -185.0, 334.0, VALUE),
                row("Light Transport", -185.0, 346.0, VALUE),
                row("Legal status in", 120.0, 250.0, LABEL),
                row("current system:", 125.0, 262.0, LABEL),
                row("Clean", 125.0, 274.0, VALUE),
                row("Combat Rating:", 120.0, 298.0, LABEL),
                row("Average", 125.0, 310.0, VALUE),
                row("Current Date:", 120.0, 334.0, LABEL),
                row("1/1/1177", 125.0, 346.0, VALUE),
            ]
        );
    }

    #[test]
    fn an_alive_pilots_ship_picture_is_centred_with_its_top_at_cy_280_added() {
        let list = drawn(&showing(alive(ShipId(133))), false);
        assert_eq!(
            pictures(&list),
            [&DrawCommand::BlendedPicture {
                image: ImageKey::picture(3005),
                // 41 / 2 is 20, truncated.
                top_left: Point::new(CENTER.x - 20.0, CENTER.y + 280.0),
                width: 41.0,
                height: 20.0,
                blend: Blend::Additive,
            }]
        );
        assert!(
            matches!(list.iter().last(), Some(DrawCommand::BlendedPicture { .. })),
            "after the texts"
        );
    }

    #[test]
    fn a_ship_with_no_picture_draws_none() {
        let list = drawn(&showing(alive(ShipId(200))), true);
        assert!(pictures(&list).is_empty());
        assert_eq!(texts(&list).len(), 14);
    }

    #[test]
    fn the_ship_is_read_again_only_when_its_type_changes() {
        let source = Ships::default();
        let mut area = PilotPanelArea::new(looks());
        area.show(alive(ShipId(133)), &source);
        area.show(alive(ShipId(133)), &source);
        area.show(PanelPilot::None, &source);
        area.show(alive(ShipId(133)), &source);
        assert_eq!(*source.reads.borrow(), [ShipId(133)]);
        area.show(alive(ShipId(200)), &source);
        assert_eq!(*source.reads.borrow(), [ShipId(133), ShipId(200)]);
        assert!(pictures(&drawn(&area, true)).is_empty(), "200's");
        assert_eq!(area.pilot(), &alive(ShipId(200)));
    }

    /// Where a text `chars` long is centred: cx less half its width in
    /// Charcoal 10, 6 a character, truncated toward zero.
    fn centred_x(chars: usize) -> f32 {
        CENTER.x - (chars as f32 * 6.0 / 2.0).trunc()
    }

    #[test]
    fn a_dying_pilot_draws_only_the_killed_message_centred() {
        let list = drawn(&showing(PanelPilot::Dying("Ada".to_owned())), false);
        let text = "Ada has been killed";
        assert_eq!(
            texts(&list),
            [(
                text.to_owned(),
                centred_x(text.len()),
                CENTER.y + 310.0,
                LABEL
            )]
        );
        assert!(pictures(&list).is_empty());
        // A half-pixel half width is truncated.
        let mut odd = looks();
        odd.size = 9.0;
        odd.font = Font::Geneva;
        let mut area = PilotPanelArea::new(odd);
        area.show(PanelPilot::Dying("Al".to_owned()), &Ships::default());
        let mut list = DrawList::new();
        area.draw(&mut list, CENTER, true, &MonoMetrics);
        let Some(DrawCommand::Text { origin, .. }) = list.iter().next() else {
            panic!("a text")
        };
        // 18 characters, 4.5 each: 81 wide, 40.5 truncated to 40.
        assert_eq!(origin.x, CENTER.x - 40.0);
    }

    #[test]
    fn a_name_starting_kenny_shows_that_they_killed_kenny() {
        for name in ["Kenny", "Kennyboy", "Kenny Rogers"] {
            let list = drawn(&showing(PanelPilot::Dying(name.to_owned())), false);
            assert_eq!(
                texts(&list),
                [(
                    KENNY.to_owned(),
                    centred_x(KENNY.len()),
                    CENTER.y + 310.0,
                    LABEL
                )],
                "{name}"
            );
        }
        for name in ["kenny", "Ken", "Benny"] {
            let list = drawn(&showing(PanelPilot::Dying(name.to_owned())), false);
            assert_eq!(
                texts(&list)[0].0,
                format!("{name} has been killed"),
                "{name}"
            );
        }
    }

    #[test]
    fn no_pilot_shows_its_message_only_once_slide_2_has_finished() {
        let area = showing(PanelPilot::None);
        assert!(drawn(&area, false).is_empty());
        let text = "No Pilot File Loaded";
        assert_eq!(
            texts(&drawn(&area, true)),
            [(
                text.to_owned(),
                centred_x(text.len()),
                CENTER.y + 310.0,
                LABEL
            )]
        );
        assert_eq!(PilotPanelArea::new(looks()).pilot(), &PanelPilot::None);
    }

    #[test]
    fn an_empty_text_draws_nothing() {
        let mut empty = model(ShipId(133));
        empty.ship_name = String::new();
        let list = drawn(&showing(PanelPilot::Alive(Box::new(empty))), true);
        let drawn: Vec<String> = texts(&list).into_iter().map(|(text, ..)| text).collect();
        assert_eq!(drawn.len(), 13);
        assert!(!drawn.contains(&String::new()));
        let mut area = PilotPanelArea::new(PanelLooks::stock(PanelLabels::default()));
        area.show(PanelPilot::None, &Ships::default());
        let mut list = DrawList::new();
        area.draw(&mut list, CENTER, true, &MonoMetrics);
        assert!(list.is_empty(), "no #276");
    }

    #[test]
    fn a_blank_panel_draws_nothing() {
        assert!(drawn(&showing(PanelPilot::Blank), true).is_empty());
    }

    #[test]
    fn the_labels_are_read_by_number_and_a_missing_one_is_empty() {
        let mut strings = vec![String::new(); 279];
        for n in [251, 252, 254, 255, 256, 276, 277, 278, 279] {
            strings[n - 1] = format!("#{n}");
        }
        assert_eq!(
            PanelLabels::from_list(&strings),
            PanelLabels {
                pilot_name: "#251".to_owned(),
                date: "#252".to_owned(),
                combat_rating: "#254".to_owned(),
                ship_name: "#255".to_owned(),
                ship_class: "#256".to_owned(),
                no_pilot: "#276".to_owned(),
                killed: "#277".to_owned(),
                legal_status: "#278".to_owned(),
                current_system: "#279".to_owned(),
            }
        );
        let short = PanelLabels::from_list(&strings[..278]);
        assert_eq!(short.current_system, "");
        assert_eq!(short.legal_status, "#278");
    }

    #[test]
    fn the_stock_looks_are_geneva_9_in_the_stock_reds() {
        let stock = PanelLooks::stock(labels());
        assert_eq!((stock.font, stock.size), (Font::Geneva, 9.0));
        assert_eq!(stock.value, Color::rgba(255, 0, 0, 255));
        assert_eq!(stock.label, Color::rgba(128, 0, 0, 255));
        assert_eq!(stock.labels, labels());
    }

    #[test]
    fn a_borrowed_or_shared_source_is_the_source() {
        fn ship_of(source: impl PilotPanelSource) -> (PanelLooks, PanelShip) {
            (source.pilot_panel_looks(), source.panel_ship(ShipId(130)))
        }
        let ships = Ships::default();
        let expected = (looks(), ships.panel_ship(ShipId(130)));
        assert_eq!(ship_of(&ships), expected);
        assert_eq!(ship_of(Rc::new(Ships::default())), expected);
    }
}
