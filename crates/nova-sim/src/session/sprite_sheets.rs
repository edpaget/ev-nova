//! The sheets the session's sprites touch by: the run-length tokens of
//! each sheet a contact may read (see `nova_sim_base::sprite_mask`), asked
//! of the catalog ([`PilotCatalog::rle_sheet`]) once per session and
//! kept, an unreadable one too ([`SpriteMasks`]). Every reader shares the
//! one cache: the deadly stellars (the `deadly_masks` module), the
//! contact passes and the scoop (the `contact_masks` module).
//!
//! **What is asked** ([`Session::load_sprite_masks`]), in every system:
//! the player's and each NPC's sprite base (`HullRecord::sprite_base`),
//! every weapon `Graphic` the arsenal holds and each a shot in flight
//! shows (sub-munitions among them), each asteroid kind in the field and
//! each box graphic in space; and, while the system lists a deadly
//! stellar, each stellar shown's `Type` and `DeadType` and graphic type
//! 0's. The advance pass asks right after the player's move, before the
//! scoop, its first reader, and again before its stellar hazards, so a
//! ship spawned in the pass is asked for. Double time's second pass has
//! no catalog and reads what the first kept: a shot it fires is met by
//! the next pass, which asks first.
//!
//! **A ship's sprite** ([`SpriteMasks::ship_sprite`]). Its sheet is its
//! sprite base's base image, its frame `FrameSpec::index` of the set its
//! last display drew and its heading now. Each ship's set steps at its
//! own display, as the original's does (see the `ship_display` module):
//! the player's before the scoop and the collision passes, so they read
//! this pass's; each NPC's in the ship loop, after them, so they read its
//! last pass's and `_HandleDeadlyStellars` this pass's. Placed
//! ([`SpriteMasks::ship`]), its rect is the sheet's frame
//! size by the standard rule, as `_HandleShipDisplay` lays a ship out: the
//! player at (−w/2, −h/2).
//!
//! None of this plays a sound.

use std::collections::BTreeMap;
use std::rc::Rc;

use super::{Session, Space};
use crate::asteroid::KINDS;
use crate::catalog::{MaskSheet, PilotCatalog, ShipId};
use crate::combat::contact::ShipSprite;
use crate::combat::ship_frames::FrameSpec;
use crate::deadly_stellar::is_deadly;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::scoop::BoxSheets;
use crate::sprite_mask::{MaskRect, PlacedMask, RleSheet};
use crate::story::sprite_pool::offset;

/// The sheets the session's sprites touch by, each asked of the catalog
/// once per session and kept, an unreadable one as none (see the module
/// docs). It belongs to the system's contents ([`Space`]); never saved.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SpriteMasks {
    sheets: BTreeMap<MaskSheet, Option<Rc<RleSheet>>>,
}

impl SpriteMasks {
    /// Asks `catalog` for `sheet`, unless it was asked before.
    pub(super) fn load(&mut self, catalog: &dyn PilotCatalog, sheet: MaskSheet) {
        self.sheets
            .entry(sheet)
            .or_insert_with(|| catalog.rle_sheet(sheet).map(Rc::new));
    }

    /// `sheet`'s tokens, if asked for and read.
    pub(super) fn sheet(&self, sheet: MaskSheet) -> Option<&Rc<RleSheet>> {
        self.sheets.get(&sheet)?.as_ref()
    }

    /// Frame `frame` of `sheet`, of `size`, its top left at `top_left`
    /// from the player; none when the sheet isn't read.
    pub(super) fn placed(
        &self,
        sheet: MaskSheet,
        frame: u16,
        top_left: (i32, i32),
        size: (u32, u32),
    ) -> Option<PlacedMask> {
        Some(PlacedMask {
            sheet: Rc::clone(self.sheet(sheet)?),
            frame,
            rect: MaskRect::placed(top_left, size),
        })
    }

    /// The sprite of a ship of sprite base `base`, of class display
    /// `frames`, drawn on set `set` facing `heading` (see the module
    /// docs); none when its sheet isn't read.
    pub(super) fn ship_sprite(
        &self,
        base: ShipId,
        frames: &FrameSpec,
        set: u16,
        heading: f32,
    ) -> Option<ShipSprite> {
        Some(ShipSprite {
            sheet: Rc::clone(self.sheet(MaskSheet::Ship(base))?),
            frame: frames.index(set, heading),
        })
    }

    /// The mask of a ship of sprite base `base`, of class display
    /// `frames`, drawn on set `set` in `state`, the player at `player` (see
    /// the module docs).
    pub(super) fn ship(
        &self,
        base: ShipId,
        frames: &FrameSpec,
        set: u16,
        state: &ShipState,
        player: Vec2,
    ) -> Option<PlacedMask> {
        let sprite = self.ship_sprite(base, frames, set, state.heading)?;
        let size = (sprite.sheet.width, sprite.sheet.height);
        Some(PlacedMask {
            rect: MaskRect::placed(offset(state.position, player, size), size),
            sheet: sprite.sheet,
            frame: sprite.frame,
        })
    }

    /// Each weapon `Graphic`'s sheet that was read.
    pub(super) fn shot_sheets(&self) -> BTreeMap<i16, Rc<RleSheet>> {
        self.sheets
            .iter()
            .filter_map(|(sheet, read)| match (sheet, read) {
                (MaskSheet::Shot(graphic), Some(rle)) => Some((*graphic, Rc::clone(rle))),
                _ => None,
            })
            .collect()
    }

    /// Each asteroid kind's sheet, where it was read.
    pub(super) fn asteroid_sheets(&self) -> [Option<Rc<RleSheet>>; KINDS] {
        std::array::from_fn(|kind| {
            let kind = u8::try_from(kind).ok()?;
            self.sheet(MaskSheet::Asteroid(kind)).map(Rc::clone)
        })
    }

    /// Each box graphic's sheet, where it was read.
    pub(super) fn box_sheets(&self) -> BoxSheets {
        BoxSheets(std::array::from_fn(|graphic| {
            let graphic = u16::try_from(graphic).ok()?;
            self.sheet(MaskSheet::CargoBox(graphic)).map(Rc::clone)
        }))
    }
}

impl Space {
    /// The sheets the contents of the system may be touched by: each
    /// asteroid kind in the field, each box graphic in space and each
    /// `Graphic` a shot in flight shows.
    fn content_sheets(&self) -> Vec<MaskSheet> {
        let asteroids = self
            .asteroids
            .indexed()
            .map(|(_, asteroid)| MaskSheet::Asteroid(asteroid.kind));
        let boxes = self
            .boxes
            .boxes()
            .iter()
            .map(|cargo_box| MaskSheet::CargoBox(cargo_box.graphic));
        let shots = self
            .shots()
            .iter()
            .map(|shot| MaskSheet::Shot(shot.weapon.graphic));
        asteroids.chain(boxes).chain(shots).collect()
    }
}

impl Session {
    /// Asks `catalog` for each sheet a contact may read and the session
    /// hasn't asked for (see the module docs).
    pub(super) fn load_sprite_masks(&mut self, catalog: &dyn PilotCatalog) {
        let arsenal = &self.statics.arsenal;
        let ships = std::iter::once(self.pilot.ship)
            .chain(self.npcs().iter().map(|npc| npc.ship))
            .map(|ship| MaskSheet::Ship(arsenal.sprite_base(ship)));
        let weapons = arsenal
            .weapons()
            .map(|weapon| MaskSheet::Shot(weapon.graphic));
        let mut wanted: Vec<MaskSheet> = ships.chain(weapons).collect();
        wanted.extend(self.space.content_sheets());
        if self.chart.listed.iter().any(|site| is_deadly(site.flags2)) {
            let stellars = self
                .chart
                .sites
                .iter()
                .flat_map(|site| std::iter::once(site.graphic).chain(site.dead_graphic))
                .chain([0])
                .map(MaskSheet::Stellar);
            wanted.extend(stellars);
        }
        for sheet in wanted {
            if let MaskSheet::Shot(graphic) = sheet
                && graphic < 0
            {
                continue;
            }
            self.space.sprite_masks.load(catalog, sheet);
        }
    }
}
