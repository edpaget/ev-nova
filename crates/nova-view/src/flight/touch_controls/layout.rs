//! Where the touch controls go: a pure function of the region they may
//! use and the menu's state ([`layout`]), in logical units, every size set
//! in points ([`Region::points_per_pixel`]).
//!
//! # The region
//!
//! [`Region::for_flight`] is the safe area the app tells
//! ([`Surroundings::safe_area`]) within flight's play area (the view left
//! of the status bar, [`STATUS_BAR_WIDTH`]), inset by [`MARGIN_PT`]. The
//! controls never cover the status bar, so the radar, the shields and the
//! target display stay clear.
//!
//! # The tiers
//!
//! | Tier | Commands | Where |
//! |---|---|---|
//! | Stick | Accelerate, Reverse Course, Rotate Left, Rotate Right, and the two stick axes | A floating stick ([`ControlId::Stick`]) in the stick zone: the region's left 40% (at least the ring wide) and the bottom 60% below the edge row (at least the ring high) |
//! | Large | Fire Primary (80 pt), Fire Secondary (64 pt), Afterburner (64 pt), round | The region's bottom-right corner: primary in the corner, secondary above it, afterburner left of it |
//! | Edge | Land/Dock, Hyper Jump, Communicate, Board, Target Select, Closest Target, Galaxy Map, Engage Cloak, Escort Menu ([`EDGE`]), then More | A row along the region's top edge, left to right, with More at its right end, wrapping to a second row when it doesn't fit |
//! | Escort groups | Group 1-5 ([`GROUPS`]) | Only while the escort menu shows: columns below the edge row, beside the escort menu's own list ([`MENU_AT`], [`MENU_WIDTH`]), each filled top down before the next |
//! | Menu | Everything else in flight's contexts ([`MENU`]), Eject and Self-Destruct last | The grid behind More, page by page |
//!
//! # Sizes
//!
//! Every control is at least [`MIN_PT`] across, Apple's minimum hit
//! target; fire is [`PRIMARY_FIRE_PT`] and [`SECONDARY_FIRE_PT`];
//! afterburner [`AFTERBURNER_PT`]; edge buttons [`EDGE_WIDTH_PT`] ×
//! [`EDGE_PT`]; menu cells [`CELL_PT`]; gaps [`GAP_PT`]. One point is
//! `1 / points_per_pixel` logical units.
//!
//! # A canvas too small
//!
//! No control is shrunk. Controls are placed in priority order (More,
//! primary fire, the stick zone, secondary fire, afterburner, then the
//! edge row left to right, then the escort groups); one that doesn't fit
//! the region, or would overlap one placed before it, joins the front of
//! the menu ([`Layout::overflow`]), and the menu pages, so every command
//! stays reachable. A region that can't hold More and one page of the
//! menu (a row of cells and a row of page arrows below the edge row)
//! places nothing, and the overlay draws nothing.
//!
//! # The menu
//!
//! While open, its panel covers the region below the first edge row: the
//! stick zone, the large buttons, a second edge row and the escort groups
//! are neither placed nor claimed. Cells fill the panel row by row in
//! [`Layout::menu`]'s order. When they don't fit, the panel's last row
//! holds the page arrows, and the menu pages.

use crate::flight::escorts::{MENU_AT, MENU_WIDTH};
use crate::flight::hud::STATUS_BAR_WIDTH;
use crate::geometry::{Bounds, Point, intersection};
use crate::input::Surroundings;
use crate::keymap::command::{
    AppCommand, Command, EscortMenuCommand, FlightCommand as F, PauseCommand, PlayCommand,
};

/// The smallest a control is along either axis, in points: Apple's
/// minimum hit target.
pub const MIN_PT: f32 = 44.0;
/// Primary fire's diameter, in points.
pub const PRIMARY_FIRE_PT: f32 = 80.0;
/// Secondary fire's diameter, in points.
pub const SECONDARY_FIRE_PT: f32 = 64.0;
/// The afterburner's diameter, in points.
pub const AFTERBURNER_PT: f32 = 64.0;
/// An edge button's height, and More's size, in points.
pub const EDGE_PT: f32 = 44.0;
/// An edge button's width, in points: room for its short label.
pub const EDGE_WIDTH_PT: f32 = 64.0;
/// The gap between controls, in points.
pub const GAP_PT: f32 = 8.0;
/// The margin inside the region's edges, in points.
pub const MARGIN_PT: f32 = 8.0;
/// The stick's ring radius, in points: a 112 pt ring.
pub const STICK_RADIUS_PT: f32 = 56.0;
/// A menu cell's width and height, in points.
pub const CELL_PT: (f32, f32) = (96.0, 44.0);
/// A page arrow's width, in points; it is as high as a cell.
pub const ARROW_PT: f32 = 64.0;

/// The stick's tier: what the floating stick holds.
pub const STICK: [Command; 4] = [
    Command::Flight(F::Accelerate),
    Command::Flight(F::ReverseCourse),
    Command::Flight(F::RotateLeft),
    Command::Flight(F::RotateRight),
];

/// The large buttons, in the order they are placed.
pub const LARGE: [Command; 3] = [
    Command::Flight(F::FirePrimary),
    Command::Flight(F::FireSecondary),
    Command::Flight(F::Afterburner),
];

/// The edge row, left to right; More follows at its right end.
pub const EDGE: [Command; 9] = [
    Command::Flight(F::LandDock),
    Command::Flight(F::HyperJump),
    Command::Flight(F::Communicate),
    Command::Flight(F::Board),
    Command::Flight(F::TargetSelect),
    Command::Flight(F::ClosestTarget),
    Command::Play(PlayCommand::GalaxyMap),
    Command::Flight(F::EngageCloak),
    Command::Flight(F::EscortMenu),
];

/// The escort menu's groups, shown only while it shows.
pub const GROUPS: [Command; 5] = [
    Command::EscortMenu(EscortMenuCommand::Group1),
    Command::EscortMenu(EscortMenuCommand::Group2),
    Command::EscortMenu(EscortMenuCommand::Group3),
    Command::EscortMenu(EscortMenuCommand::Group4),
    Command::EscortMenu(EscortMenuCommand::Group5),
];

/// The menu's own commands, in order, Eject and Self-Destruct last.
pub const MENU: [Command; 36] = [
    Command::Flight(F::Autopilot),
    Command::Flight(F::FaceStellar),
    Command::Flight(F::HyperspaceMode),
    Command::Flight(F::HyperSelect),
    Command::Flight(F::NavOff),
    Command::Flight(F::ClearTarget),
    Command::Flight(F::SelectSecondary),
    Command::Flight(F::SelectSecondaryBack),
    Command::Flight(F::WeaponSafety),
    Command::Flight(F::TargetSelectBack),
    Command::Flight(F::TargetEscort),
    Command::Flight(F::TargetEscortBack),
    Command::Flight(F::ClosestShip),
    Command::Flight(F::HailStellar),
    Command::Flight(F::AttackTarget),
    Command::Flight(F::DefendMe),
    Command::Flight(F::HoldPosition),
    Command::Flight(F::Recall),
    Command::Flight(F::RecallToHangar),
    Command::Flight(F::Acknowledge),
    Command::Flight(F::JettisonCargo),
    Command::Flight(F::JettisonNonMission),
    Command::Flight(F::StellarSlot1),
    Command::Flight(F::StellarSlot2),
    Command::Flight(F::StellarSlot3),
    Command::Flight(F::StellarSlot4),
    Command::Flight(F::NearestStellar),
    Command::Flight(F::ZoomIn),
    Command::Flight(F::ZoomOut),
    Command::Flight(F::ShowFramerate),
    Command::Play(PlayCommand::PlayerInfo),
    Command::Play(PlayCommand::MissionInfo),
    Command::Pause(PauseCommand::Pause),
    Command::App(AppCommand::DoubleTime),
    Command::Flight(F::Eject),
    Command::Flight(F::SelfDestruct),
];

/// Where the controls may go, in logical units, and how many points a
/// logical unit is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    /// The rect they may use.
    pub rect: Bounds,
    /// Points per logical pixel.
    pub points_per_pixel: f32,
}

impl Region {
    /// Flight's region in a `view` of that size: the safe area within the
    /// play area, inset by [`MARGIN_PT`]; empty when they don't meet.
    #[must_use]
    pub fn for_flight(told: &Surroundings, view: (f32, f32)) -> Self {
        let points_per_pixel = told.points_per_pixel;
        let play = Bounds::at(Point::new(0.0, 0.0), view.0 - STATUS_BAR_WIDTH, view.1);
        let margin = MARGIN_PT / points_per_pixel;
        let rect = intersection(told.safe_area, play)
            .map_or(Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0), |rect| {
                rect.inset(margin, margin)
            });
        Self {
            rect,
            points_per_pixel,
        }
    }

    /// `pt` points in logical units.
    fn units(&self, pt: f32) -> f32 {
        pt / self.points_per_pixel
    }
}

/// The menu's state, and the escort menu's, as the layout reads them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuView {
    /// Whether the command menu is open.
    pub open: bool,
    /// The page it shows, clamped to its pages.
    pub page: usize,
    /// Whether flight's escort menu shows.
    pub escort_menu: bool,
}

/// What a placed control is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlId {
    /// The stick zone: a finger landing here starts the floating stick.
    Stick,
    /// A button holding a command.
    Command(Command),
    /// The button that opens and closes the menu.
    More,
    /// The menu's previous page.
    PagePrev,
    /// The menu's next page.
    PageNext,
}

/// A placed control: what it is, its rect (a round one's bounding
/// square, so its hit area is at least what is drawn), and whether it is
/// drawn round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// What it is.
    pub id: ControlId,
    /// Where it is, in logical units.
    pub rect: Bounds,
    /// Whether it is drawn round.
    pub round: bool,
}

/// The controls placed for one region and menu state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    /// Every control placed, in priority order; none overlap.
    pub placed: Vec<Placed>,
    /// The open menu's panel, which claims a touch anywhere on it.
    pub panel: Option<Bounds>,
    /// The commands that didn't fit, in priority order: the front of the
    /// menu.
    pub overflow: Vec<Command>,
    /// Every command the menu holds, in order: the overflow, then
    /// [`MENU`].
    pub menu: Vec<Command>,
    /// How many pages the menu has; none when nothing is placed.
    pub pages: usize,
}

impl Layout {
    /// The control under `at`, if any.
    #[must_use]
    pub fn hit(&self, at: Point) -> Option<ControlId> {
        self.placed
            .iter()
            .find(|placed| placed.rect.contains(at))
            .map(|placed| placed.id)
    }

    /// The control placed as `id`, if any.
    #[must_use]
    pub fn get(&self, id: ControlId) -> Option<&Placed> {
        self.placed.iter().find(|placed| placed.id == id)
    }
}

/// The menu's grid in its panel: columns and rows of cells.
struct Grid {
    panel: Bounds,
    cell: (f32, f32),
    gap: f32,
    cols: usize,
    rows: usize,
}

impl Grid {
    /// The grid `panel` holds, or `None` when it can't hold a row of
    /// cells and a row of page arrows.
    fn fit(region: &Region, panel: Bounds) -> Option<Self> {
        let cell = (region.units(CELL_PT.0), region.units(CELL_PT.1));
        let gap = region.units(GAP_PT);
        let count = |room: f32, size: f32| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = ((room + gap) / (size + gap)).floor().max(0.0) as usize;
            n
        };
        let cols = count(panel.width(), cell.0);
        let rows = count(panel.height(), cell.1);
        let arrows = 2.0 * region.units(ARROW_PT) + gap;
        (cols >= 1 && rows >= 2 && panel.width() >= arrows).then_some(Self {
            panel,
            cell,
            gap,
            cols,
            rows,
        })
    }

    /// The cells a page holds when the menu's `total` commands need more
    /// than one: every row but the arrows'.
    const fn per_page(&self, total: usize) -> usize {
        if total <= self.cols * self.rows {
            self.cols * self.rows
        } else {
            self.cols * (self.rows - 1)
        }
    }

    /// The rect of the cell at `index` on its page.
    #[allow(clippy::cast_precision_loss)] // A few cells.
    fn cell(&self, index: usize) -> Bounds {
        let (col, row) = (index % self.cols, index / self.cols);
        Bounds::at(
            Point::new(
                self.panel.min.x + col as f32 * (self.cell.0 + self.gap),
                self.panel.min.y + row as f32 * (self.cell.1 + self.gap),
            ),
            self.cell.0,
            self.cell.1,
        )
    }

    /// The page arrows' row's top.
    #[allow(clippy::cast_precision_loss)] // A few rows.
    fn arrows_top(&self) -> f32 {
        self.panel.min.y + (self.rows - 1) as f32 * (self.cell.1 + self.gap)
    }
}

/// Places controls one after another, refusing one that leaves the region
/// or overlaps one placed before, and keeps those refused for the menu.
struct Placer<'a> {
    region: &'a Region,
    placed: Vec<Placed>,
    /// The commands refused, in the order tried.
    overflow: Vec<Command>,
    /// The edge commands placed in the first row.
    first_row: Vec<Command>,
}

impl Placer<'_> {
    /// `pt` points in logical units.
    fn u(&self, pt: f32) -> f32 {
        self.region.units(pt)
    }

    /// An edge row's height with the gap below it.
    fn row_height(&self) -> f32 {
        self.u(EDGE_PT) + self.u(GAP_PT)
    }

    /// Whether `rect` lies in the region and clear of every control
    /// placed.
    fn fits(&self, rect: Bounds) -> bool {
        let r = self.region.rect;
        let inside = rect.min.x >= r.min.x
            && rect.min.y >= r.min.y
            && rect.max.x <= r.max.x
            && rect.max.y <= r.max.y;
        inside
            && self
                .placed
                .iter()
                .all(|placed| intersection(placed.rect, rect).is_none())
    }

    /// Places `id` at `rect` if it fits; whether it did.
    fn place(&mut self, id: ControlId, rect: Bounds, round: bool) -> bool {
        let fits = self.fits(rect);
        if fits {
            self.placed.push(Placed { id, rect, round });
        }
        fits
    }

    /// Places `command`'s button at `rect`, or sends it to the menu.
    fn command(&mut self, command: Command, rect: Bounds, round: bool) {
        if !self.place(ControlId::Command(command), rect, round) {
            self.overflow.push(command);
        }
    }

    fn rect(&self, id: ControlId) -> Option<Bounds> {
        self.placed
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| placed.rect)
    }

    /// Primary fire in the corner, the stick zone bottom left, then
    /// secondary fire above primary and the afterburner left of it.
    fn large_and_stick(&mut self) {
        let r = self.region.rect;
        let gap = self.u(GAP_PT);
        let primary = Command::Flight(F::FirePrimary);
        let side = self.u(PRIMARY_FIRE_PT);
        let corner = Bounds::at(Point::new(r.max.x - side, r.max.y - side), side, side);
        self.command(primary, corner, true);
        let ring = 2.0 * self.u(STICK_RADIUS_PT);
        let below_row = r.height() - self.row_height();
        let zone_w = (0.4 * r.width()).max(ring);
        let zone_h = (0.6 * below_row).max(ring);
        let zone = Bounds::at(Point::new(r.min.x, r.max.y - zone_h), zone_w, zone_h);
        if !self.place(ControlId::Stick, zone, false) {
            self.overflow.extend(STICK);
        }
        let anchor = self
            .rect(ControlId::Command(primary))
            .unwrap_or(Bounds::at(r.max, 0.0, 0.0));
        let side = self.u(SECONDARY_FIRE_PT);
        let above = Bounds::at(
            Point::new(anchor.center().x - side / 2.0, anchor.min.y - gap - side),
            side,
            side,
        );
        self.command(Command::Flight(F::FireSecondary), above, true);
        let side = self.u(AFTERBURNER_PT);
        let left = Bounds::at(
            Point::new(anchor.min.x - gap - side, anchor.max.y - side),
            side,
            side,
        );
        self.command(Command::Flight(F::Afterburner), left, true);
    }

    /// An edge button's rect at slot `index`: row by row from the
    /// region's top left.
    fn edge_slot(&self, index: usize) -> Bounds {
        let r = self.region.rect;
        let width = self.u(EDGE_WIDTH_PT);
        #[allow(clippy::cast_precision_loss)] // A few slots.
        let (col, row) = ((index % SLOTS_A_ROW) as f32, (index / SLOTS_A_ROW) as f32);
        Bounds::at(
            Point::new(
                r.min.x + col * (width + self.u(GAP_PT)),
                r.min.y + row * self.row_height(),
            ),
            width,
            self.u(EDGE_PT),
        )
    }

    /// The edge row, left to right in up to two rows, skipping any slot a
    /// control placed before covers.
    fn edge_row(&mut self) {
        let mut next = 0;
        for command in EDGE {
            let found = (next..2 * SLOTS_A_ROW).find(|&index| self.fits(self.edge_slot(index)));
            match found {
                Some(index) => {
                    self.place(ControlId::Command(command), self.edge_slot(index), false);
                    if index < SLOTS_A_ROW {
                        self.first_row.push(command);
                    }
                    next = index + 1;
                }
                None => self.overflow.push(command),
            }
        }
    }

    /// The escort groups, in columns beside the escort menu's list, each
    /// filled top down before the next, skipping any slot a control placed
    /// before covers.
    fn escort_groups(&mut self) {
        let r = self.region.rect;
        let gap = self.u(GAP_PT);
        let (width, height) = (self.u(EDGE_WIDTH_PT), self.u(EDGE_PT));
        let left = r.min.x.max(MENU_AT.x + MENU_WIDTH + gap);
        let top = self
            .placed
            .iter()
            .filter(|placed| matches!(placed.id, ControlId::Command(c) if EDGE.contains(&c)))
            .map(|placed| placed.rect.max.y)
            .fold(r.min.y + height, f32::max)
            + gap;
        let row_height = self.row_height();
        let mut slots =
            (0..SLOTS_A_ROW).flat_map(|col| (0..SLOTS_A_ROW).map(move |row| (col, row)));
        for group in GROUPS {
            let found = slots.by_ref().find(|&(col, row)| {
                #[allow(clippy::cast_precision_loss)] // A few slots.
                let at = Point::new(
                    left + col as f32 * (width + gap),
                    top + row as f32 * row_height,
                );
                self.place(
                    ControlId::Command(group),
                    Bounds::at(at, width, height),
                    false,
                )
            });
            if found.is_none() {
                self.overflow.push(group);
            }
        }
    }
}

/// The controls for `region` with the menu as `menu` says (see the module
/// docs).
#[must_use]
pub fn layout(region: &Region, menu: MenuView) -> Layout {
    let r = region.rect;
    let mut placer = Placer {
        region,
        placed: Vec::new(),
        overflow: Vec::new(),
        first_row: Vec::new(),
    };
    let side = placer.u(EDGE_PT);
    let more = Bounds::at(Point::new(r.max.x - side, r.min.y), side, side);
    let panel = Bounds {
        min: Point::new(r.min.x, r.min.y + placer.row_height()),
        max: r.max,
    };
    let grid = Grid::fit(region, panel);
    let Some(grid) = grid.filter(|_| placer.place(ControlId::More, more, false)) else {
        return Layout::default();
    };
    placer.large_and_stick();
    placer.edge_row();
    if menu.escort_menu {
        placer.escort_groups();
    }
    let Placer {
        placed: shut,
        overflow,
        first_row,
        ..
    } = placer;
    let commands: Vec<Command> = overflow.iter().copied().chain(MENU).collect();
    let per_page = grid.per_page(commands.len());
    let pages = commands.len().div_ceil(per_page);
    if !menu.open {
        return Layout {
            placed: shut,
            panel: None,
            overflow,
            menu: commands,
            pages,
        };
    }
    // Open: More, the first edge row, and the page's cells over the panel.
    let mut open: Vec<Placed> = shut
        .into_iter()
        .filter(|placed| match placed.id {
            ControlId::More => true,
            ControlId::Command(command) => first_row.contains(&command),
            _ => false,
        })
        .collect();
    let page = menu.page.min(pages - 1);
    let cells = commands.iter().skip(page * per_page).take(per_page);
    open.extend(cells.enumerate().map(|(index, &command)| Placed {
        id: ControlId::Command(command),
        rect: grid.cell(index),
        round: false,
    }));
    if pages > 1 {
        let top = grid.arrows_top();
        let arrow = region.units(ARROW_PT);
        for (id, x) in [
            (ControlId::PagePrev, panel.min.x),
            (ControlId::PageNext, panel.max.x - arrow),
        ] {
            open.push(Placed {
                id,
                rect: Bounds::at(Point::new(x, top), arrow, grid.cell.1),
                round: false,
            });
        }
    }
    Layout {
        placed: open,
        panel: Some(panel),
        overflow,
        menu: commands,
        pages,
    }
}

/// Slots in an edge row, more than any region holds: a row's slots past
/// the region's right edge are refused as any that doesn't fit.
const SLOTS_A_ROW: usize = 64;

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
