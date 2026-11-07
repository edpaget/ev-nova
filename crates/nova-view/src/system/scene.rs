//! A system laid out for viewing: each stellar at its world position with
//! its sheet (or why not) and the frame it shows, read once from the
//! catalog; and [`draw_stellars`], which draws them through a camera for
//! every screen that shows a system.
//!
//! In a flight the session steps every stellar's frame
//! (`nova_sim::Session::stellar_frame`, rendering phase 11), and the
//! flight view hands the scene each stellar's look and the frame its slot
//! shows ([`SystemScene::set_looks`]): the scene keeps no animation state
//! of its own for a flight. The system viewer, which runs no session,
//! steps each stellar's [`StellarFrames`] itself by the sim's rule
//! ([`SystemScene::step`]), from frame 0 as the scene is laid out.

use std::collections::BTreeSet;
use std::num::NonZeroU16;

use super::camera::{CAMERA_MARGIN, Camera};
use nova_sim::landing::UNKNOWN_STELLAR_RADIUS;
use nova_sim::stellar_frames::{StellarAnimation, StellarFrames};
use nova_sim::{Chance, DrawnSheet, SpeedMult, Vec2};

use super::catalog::{StellarContents, StellarId, StellarSheet, SystemCatalog, SystemId};
use crate::draw::crossed_box;
use crate::eight_bit_blit::{DrawProcState, EightBitBlit};
use crate::flight::murk::SeenMurk;
use crate::geometry::Bounds;
use crate::image::ImageKey;
use crate::outline::{self, StellarStandIn};
use crate::{Blend, Color, DrawCommand, DrawList, Point, SheetImage};

/// The side of the box drawn where a stellar's sprite cannot be shown.
pub const PLACEHOLDER_SIZE: f32 = 64.0;
/// The placeholder's colour.
pub(crate) const PLACEHOLDER: Color = Color::rgba(128, 128, 128, 255);
/// How far below a stellar its name goes, and the name's size.
const NAME_GAP: f32 = 4.0;
const NAME_SIZE: f32 = 14.0;
/// How far below a placeholder's name the reason goes.
const MESSAGE_GAP: f32 = 18.0;
/// The side of the square a stellar's stand-in fills: as wide as the
/// circle a click or a landing reaches on a stellar with no sprite
/// (`nova_sim::landing::UNKNOWN_STELLAR_RADIUS`), so what is drawn is
/// where the player can click and land.
const STAND_IN_SIDE: f32 = 2.0 * UNKNOWN_STELLAR_RADIUS;
/// [`STAND_IN_SIDE`] as a frame's size.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
const STAND_IN_FRAME: (u32, u32) = (STAND_IN_SIDE as u32, STAND_IN_SIDE as u32);
/// The side `_GetSpriteXSize` and `_GetSpriteYSize` give a slot whose
/// frames count none (@0x26e5-0x26ef, @0x2714-0x271e).
const FROZEN_SIDE: u32 = 32;

/// A stellar's frame as [`SystemScene::shown_frame`] gives it: keyed,
/// with its width and height, and the width and height its sprite slot is
/// placed as ([`SceneStellar::slot_size`]).
pub type ShownFrame = (ImageKey, (f32, f32), (f32, f32));

/// One stellar, placed in the scene.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // independent facts the session sets
pub struct SceneStellar {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its display name.
    pub name: String,
    /// Where it is, in world units.
    pub position: Point,
    /// Its sprite sheet (its `Type`'s), or why it cannot be shown.
    pub sprite: Result<StellarSheet, String>,
    /// The sheet it draws while destroyed (its `DeadType`'s), or why it
    /// cannot be shown; `None` for no different graphic.
    pub dead_sprite: Option<Result<StellarSheet, String>>,
    /// Its animation fields.
    pub animation_data: StellarAnimation,
    /// Whether it is destroyed, as last set.
    pub destroyed: bool,
    /// Which sheet it draws, as last set: the session's answer
    /// (`Session::drawn_sheet`).
    pub drawn: DrawnSheet,
    /// Whether its sprite slot shows a frame frozen, as last set
    /// (`Session::frame_frozen`): the frame is drawn where a 32-pixel
    /// sprite's would be, its top left at the centre less 16 on each axis.
    pub frozen: bool,
    /// Graphic type 0's sheet (`spïn` 1000), or why it cannot be shown.
    pub type_zero_sprite: Result<StellarSheet, String>,
    /// Its animation state in a viewer with no session, stepped by the
    /// sim's rule ([`SystemScene::step`]): fresh as the scene is laid out.
    /// A sheet that cannot be read counts 1 frame. A flight never steps
    /// it, drawing [`reported`](Self::reported) instead.
    pub frames: StellarFrames,
    /// What it draws in place of a sprite that cannot be shown, as its
    /// data gives it; `None` draws the crossed box and the reason.
    pub stand_in: Option<StellarStandIn>,
    /// The frame of the sheet drawn its session reports its slot shows
    /// (`nova_sim::flight_frame::FrameStellar::frame`), drawn in place of
    /// [`frames`](Self::frames)'; `None` in a viewer with no session.
    pub reported: Option<u16>,
}

/// How a stellar looks, as its session holds it, for
/// [`SystemScene::set_looks`]: whether it is destroyed
/// (`Session::galaxy`), which sheet it draws (`Session::drawn_sheet`),
/// whether its frame is frozen (`Session::frame_frozen`), and the frame
/// of that sheet its slot shows (`Session::stellar_frame`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StellarLook {
    /// Whether it is destroyed.
    pub destroyed: bool,
    /// Which sheet it draws.
    pub drawn: DrawnSheet,
    /// Whether its sprite slot shows a frame frozen.
    pub frozen: bool,
    /// The frame of the sheet drawn its slot shows; `None` to draw the
    /// scene's own animation.
    pub frame: Option<u16>,
}

impl SceneStellar {
    /// `stellar`, not destroyed, with graphic type 0's sheet `type_zero`.
    fn new(stellar: StellarContents, type_zero: Result<StellarSheet, String>) -> Self {
        Self {
            id: stellar.id,
            name: stellar.name,
            position: Point::new(f32::from(stellar.x), f32::from(stellar.y)),
            sprite: stellar.sprite,
            dead_sprite: stellar.dead_sprite,
            animation_data: stellar.animation,
            destroyed: false,
            drawn: DrawnSheet::Type,
            frozen: false,
            type_zero_sprite: type_zero,
            frames: StellarFrames::new(),
            reported: None,
            stand_in: stellar.stand_in,
        }
    }

    /// The sheet drawn, as [`drawn`](Self::drawn) names it: the `Type`
    /// sheet; the `DeadType` sheet (or why it cannot be shown), or the
    /// `Type` sheet when the stellar has no different graphic; or graphic
    /// type 0's sheet, as the original's rebuild fills a sprite slot it
    /// left empty (story-runtime phase 9). `None` when its slot shows no
    /// frame ([`DrawnSheet::NoFrame`], stellar-fidelity phase 8): nothing
    /// is drawn.
    #[must_use]
    pub fn drawn(&self) -> Option<&Result<StellarSheet, String>> {
        self.sheet(self.drawn)
    }

    /// The frame it shows, keyed, and that frame's width and height: the
    /// frame its session reports, modulo its sheet's frames, which guards
    /// only a sheet the view read otherwise, as both count its frames by
    /// `nova_data::store::master_frame_count`; or, with no session, the
    /// frame its own animation shows. None with no sheet to draw, or a
    /// slot showing no frame.
    #[must_use]
    pub fn shown_image(&self) -> Option<(ImageKey, (f32, f32))> {
        let sheet = self.drawn()?.as_ref().ok()?;
        let frame = self
            .reported
            .map_or(self.frames.shown(), |frame| frame % sheet.frames.get());
        let size = (sheet.frame_width as f32, sheet.frame_height as f32);
        Some((sheet.image.frame(frame), size))
    }

    /// The size `_GetSpriteXSize` and `_GetSpriteYSize` give its sprite
    /// slot (@0x26d4-0x2731): 32 square when the slot's frames count none,
    /// a frame [`frozen`](Self::frozen) or none shown
    /// ([`DrawnSheet::NoFrame`], @0x26da-0x26ef); else the frame of the
    /// sheet drawn; `None` when that sheet can't be read.
    #[must_use]
    pub fn slot_size(&self) -> Option<(u32, u32)> {
        if self.frozen || self.drawn == DrawnSheet::NoFrame {
            return Some((FROZEN_SIDE, FROZEN_SIDE));
        }
        let sheet = self.drawn()?.as_ref().ok()?;
        Some((sheet.frame_width, sheet.frame_height))
    }

    /// Whether the sheet drawn draws past its frame box, as a 32-bit
    /// `rlëD` does ([`StellarSheet::image_size`]).
    fn overhangs(&self) -> bool {
        matches!(
            self.drawn(),
            Some(Ok(sheet)) if sheet.image_size != (sheet.frame_width, sheet.frame_height)
        )
    }

    /// The image of the sheet drawn, when that sheet can draw otherwise
    /// at the world's edge than cut to its box
    /// ([`StellarSheet::clip_quirks`]).
    fn quirky_image(&self) -> Option<SheetImage> {
        match self.drawn() {
            Some(Ok(sheet)) if sheet.clip_quirks => Some(sheet.image),
            _ => None,
        }
    }

    /// Whether the sheet drawn is an 8-bit `rlëD`
    /// ([`StellarSheet::eight_bit`]).
    fn eight_bit(&self) -> bool {
        matches!(self.drawn(), Some(Ok(sheet)) if sheet.eight_bit)
    }

    /// The sheet `which` names (see [`drawn`](Self::drawn)).
    fn sheet(&self, which: DrawnSheet) -> Option<&Result<StellarSheet, String>> {
        match (which, &self.dead_sprite) {
            (DrawnSheet::Type, _) | (DrawnSheet::Dead, None) => Some(&self.sprite),
            (DrawnSheet::Dead, Some(dead)) => Some(dead),
            (DrawnSheet::TypeZero, _) => Some(&self.type_zero_sprite),
            (DrawnSheet::NoFrame, _) => None,
        }
    }

    /// Whether it has a `DeadType` sheet that could be read.
    fn has_dead_sheet(&self) -> bool {
        matches!(self.dead_sprite, Some(Ok(_)))
    }

    /// How many frames the sheet `which` names has: 1 when it cannot be
    /// read or names none.
    fn frame_count(&self, which: DrawnSheet) -> NonZeroU16 {
        self.sheet(which)
            .and_then(|sheet| sheet.as_ref().ok())
            .map_or(NonZeroU16::MIN, |sheet| sheet.frames)
    }

    /// One pass of its animation at `gain` in a viewer with no session,
    /// over the frames of the sheet it draws, a random next frame drawn on
    /// `chance`.
    fn step_by(&mut self, gain: SpeedMult, chance: &mut dyn Chance) {
        let drawn = self.frame_count(self.drawn);
        self.frames.step_by(
            gain,
            self.animation_data,
            self.destroyed,
            drawn,
            drawn,
            chance,
        );
    }

    /// Where its sprite of `size` is centred as `camera` draws it: placed
    /// as the original places the slot, at whole pixels from the camera's
    /// centre ([`Camera::sprite_top_left`], `_HandleStellarSprites`
    /// @0x2eeab-0x2ef1a), by the frame's size, or, for a frozen frame, as
    /// a 32-pixel slot (`_GetSpriteXSize` @0x26d4-0x2701 and
    /// `_SWSetSpriteLocation` moving the rect without resizing it,
    /// @0xab66d-0xab6a0). The frame keeps its own size from that top left.
    fn sprite_centre(&self, camera: &Camera, (width, height): (f32, f32)) -> Point {
        // A frame's size is a whole number of pixels.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let placed_as = if self.frozen {
            (FROZEN_SIDE, FROZEN_SIDE)
        } else {
            (width as u32, height as u32)
        };
        let top_left = camera.sprite_top_left(self.position, placed_as);
        Point::new(top_left.x + width / 2.0, top_left.y + height / 2.0)
    }
}

/// One system's stellars, in navigation order.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemScene {
    id: SystemId,
    name: String,
    stellars: Vec<SceneStellar>,
    problems: Vec<String>,
}

impl SystemScene {
    /// System `id`, read from `catalog` once.
    ///
    /// Its problems are the catalog's, then each stellar's `Type` sheet
    /// that cannot be read and `DeadType` sheet that exists but cannot be
    /// read, then graphic type 0's sheet
    /// when it cannot be read and some stellar might fall back to it (one
    /// with no `DeadType` sheet that could be read, or with a `Type` sheet
    /// that cannot be), whatever the readings of
    /// [`RuleKey::DeadTypeFallback`](nova_sim::RuleKey::DeadTypeFallback)
    /// and [`RuleKey::TypeSheetFallback`](nova_sim::RuleKey::TypeSheetFallback):
    /// the sheet is broken either way.
    pub fn load(catalog: &impl SystemCatalog, id: SystemId) -> Self {
        Self::loaded(catalog, id, |_| true)
    }

    /// System `id`, read from `catalog` once, with only the stellars of
    /// `shown`, in navigation order: the stellars the flight's session
    /// shows (`Session::stellars_shown`), as the original draws only the
    /// stellars the player's system holds. Its problems are as
    /// [`SystemScene::load`] gives them, for the stellars kept alone.
    pub fn load_shown(
        catalog: &impl SystemCatalog,
        id: SystemId,
        shown: &BTreeSet<StellarId>,
    ) -> Self {
        Self::loaded(catalog, id, |stellar| shown.contains(&stellar))
    }

    /// System `id`, read from `catalog` once, with the stellars `keep`
    /// keeps (see [`SystemScene::load`]).
    fn loaded(
        catalog: &impl SystemCatalog,
        id: SystemId,
        keep: impl Fn(StellarId) -> bool,
    ) -> Self {
        let contents = catalog.system(id);
        let type_zero = contents.type_zero_sprite;
        let stellars: Vec<SceneStellar> = contents
            .stellars
            .into_iter()
            .filter(|stellar| keep(stellar.id))
            .map(|stellar| SceneStellar::new(stellar, type_zero.clone()))
            .collect();
        let mut problems = contents.problems;
        for stellar in &stellars {
            if let Err(reason) = &stellar.sprite {
                problems.push(format!("spöb {} Type: {reason}", stellar.id.0));
            }
            if let Some(Err(reason)) = &stellar.dead_sprite {
                problems.push(format!("spöb {} DeadType: {reason}", stellar.id.0));
            }
        }
        if let Err(reason) = &type_zero
            && stellars
                .iter()
                .any(|stellar| !stellar.has_dead_sheet() || stellar.sprite.is_err())
        {
            problems.push(format!("spïn 1000 (graphic type 0): {reason}"));
        }
        Self {
            id: contents.id,
            name: contents.name,
            stellars,
            problems,
        }
    }

    /// The system's ID.
    #[must_use]
    pub fn id(&self) -> SystemId {
        self.id
    }

    /// The system's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Every stellar, in navigation order.
    #[must_use]
    pub fn stellars(&self) -> &[SceneStellar] {
        &self.stellars
    }

    /// What could not be read.
    #[must_use]
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    /// Sets each stellar's look as `look` gives it: whether it is
    /// destroyed, the sheet it draws ([`SceneStellar::drawn`]), whether
    /// its frame is frozen and the frame its session reports its slot
    /// shows. The scene keeps no frame state for it: the session settles
    /// every swap, rebuild and freeze (see `nova_sim::Session`'s
    /// `stellar_frames` module).
    pub fn set_looks(&mut self, look: impl Fn(StellarId) -> StellarLook) {
        for stellar in &mut self.stellars {
            let now = look(stellar.id);
            stellar.destroyed = now.destroyed;
            stellar.drawn = now.drawn;
            stellar.frozen = now.frozen;
            stellar.reported = now.frame;
        }
    }

    /// Steps every stellar's animation `ticks` ticks of 1/30 s in a
    /// viewer with no session, each random next frame (`Flags2` 0x0002)
    /// drawn on `chance`. A flight never calls it: its session steps the
    /// frames.
    pub fn step(&mut self, ticks: u128, chance: &mut dyn Chance) {
        self.step_by(ticks, SpeedMult::FULL, chance);
    }

    /// Steps every stellar's animation `ticks` ticks of 1/30 s, each at
    /// `gain`, the frame's `_gSpeedMult` (see [`SystemScene::step`]).
    pub fn step_by(&mut self, ticks: u128, gain: SpeedMult, chance: &mut dyn Chance) {
        for _ in 0..ticks {
            for stellar in &mut self.stellars {
                stellar.step_by(gain, chance);
            }
        }
    }

    /// The frame stellar `id` shows and its size
    /// ([`SceneStellar::shown_image`]), with the size its slot is placed
    /// as ([`SceneStellar::slot_size`], 32 square for a frozen frame): what
    /// the planet comm dialog draws of it (`_PlanetCommDialogUpdate`
    /// @0x90cc6-0x90e26, sizing the rect by `_GetSpriteXSize` and
    /// `_GetSpriteYSize` @0x90f97-0x90faf). None for a stellar not in it
    /// or showing nothing.
    #[must_use]
    pub fn shown_frame(&self, id: StellarId) -> Option<ShownFrame> {
        let stellar = self.stellars.iter().find(|stellar| stellar.id == id)?;
        let (image, size) = stellar.shown_image()?;
        // A slot's side is far within an `f32`'s whole numbers.
        #[allow(clippy::cast_precision_loss)]
        let placed_as = stellar
            .slot_size()
            .map_or(size, |(width, height)| (width as f32, height as f32));
        Some((image, size, placed_as))
    }

    /// Where the camera may go: the box round the system's centre and every
    /// stellar, grown by [`CAMERA_MARGIN`].
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        let positions = self.stellars.iter().map(|stellar| stellar.position);
        Bounds::around(std::iter::once(Point::default()).chain(positions))
            .expect("the centre is always in it")
            .grown(CAMERA_MARGIN)
    }
}

/// Draws every stellar of `scene` where `camera` puts it, on the frame its
/// session reports ([`SystemScene::set_looks`]), modulo its sheet's
/// frames, or, with no session, the frame its own animation shows: each
/// sprite (or, without a sheet, its stand-in fitted to the landing reach,
/// [`outline::draw_stellar`], or with none a placeholder box and the
/// reason; and with a slot showing no frame, nothing), then every name
/// under its stellar, so no sprite covers a name.
///
/// Both passes go in ascending stellar ID, so of two overlapping stellars
/// the higher ID is on top. `_HandleStellarSystemVisibility` fills its
/// sprite slots in ID order (@0x32ec1-0x33229), `_SWAddSprite` appends each
/// new slot to the layer's tail (@0xac624) and `_SWAnimateSpriteWorld`
/// walks each layer from its head (@0xb9727). The scene itself stays in
/// navigation order.
///
/// With `seen` (how the player sees the murk), each sprite is murked as
/// `_HandleStellarSprites` murks it at its stellar's position
/// (@0x2ee57-0x2eea4, [`SeenMurk::at`]); the placeholder and the names are
/// not, as they are not the original's. The system view passes none. An
/// 8-bit sheet goes through the draw proc's depth-8 dispatch
/// ([`EightBitBlit`]): plain when unmurked, and murked as a 16-bit sheet
/// is where the original's depth-8 routine crashes.
///
/// A sheet that draws past its frame box (a 32-bit `rlëD`, rendering
/// phase 1) is cut to the box when the box is not wholly inside the
/// list's world clip ([`DrawList::world_clip`], standing in for the
/// original's world rect, the play area). The draw proc
/// (`_BlitPixieRLETranslucentDrawProc` @0xbb3fc) clips the box to the
/// world and takes the plain blit only when that leaves it whole
/// (@0xbb56d-0xbb579, @0xbb770-0xbb789); otherwise the clipped blit
/// (`_BlitPixieRLEClipped` @0xbde9c-0xbe0b3) draws only the box's columns
/// and rows within the world, so the overhang disappears, and a box wholly
/// outside the world draws nothing (@0xbb480-0xbb4a9).
///
/// A sheet whose clipped draw can differ from that crop
/// ([`StellarSheet::clip_quirks`]: a line count that lies, a two-colour
/// run, a token ending on an even byte) draws, when the world's edge cuts
/// its box, the clipped blit's frame instead
/// ([`SheetImage::clipped_frame`], `nova_data::graphics::decode_rled_clipped`),
/// keyed by how many of the box's rows and pixels the world cuts at the
/// top and the left (a box cut only at the right or bottom cuts none),
/// and cut to the box the same way. It is murked, and dispatched by
/// depth, as the plain frame would be.
pub fn draw_stellars(
    list: &mut DrawList,
    scene: &SystemScene,
    camera: &Camera,
    seen: Option<SeenMurk>,
) {
    let mut by_id: Vec<&SceneStellar> = scene.stellars().iter().collect();
    by_id.sort_by_key(|stellar| stellar.id);
    let mut labels = Vec::new();
    for stellar in by_id {
        let (at, (width, height)) = match (stellar.drawn(), stellar.shown_image()) {
            (None | Some(Ok(_)), None) => (camera.world_to_screen(stellar.position), (0.0, 0.0)),
            (_, Some((image, size))) => {
                let at = stellar.sprite_centre(camera, size);
                let position = Vec2::new(stellar.position.x, stellar.position.y);
                let colorize = seen.map(|seen| seen.at(position));
                // A stellar's translucency and factors are zeroed every
                // tick (@0x2ee57-0x2ee7e), so it never reaches `Nothing`.
                let state = DrawProcState {
                    translucency: 0,
                    full_factors: false,
                    bias: DrawProcState::bias_of(colorize),
                };
                let look = EightBitBlit::of_sheet(stellar.eight_bit(), state).applied((
                    Color::WHITE,
                    colorize,
                    Blend::Normal,
                ));
                if let Some((tint, colorize, blend)) = look {
                    let sprite =
                        |image| DrawCommand::mixed_sprite(image, at, tint, blend, colorize);
                    let frame_box = Bounds::at(
                        Point::new(at.x - size.0 / 2.0, at.y - size.1 / 2.0),
                        size.0,
                        size.1,
                    );
                    let edge = list.world_clip().filter(|world| {
                        !(world.contains(frame_box.min) && world.contains(frame_box.max))
                    });
                    let clipped = edge.and_then(|world| {
                        let cut = cut_by(world, frame_box, size);
                        stellar.quirky_image()?.clipped_frame(image.frame, cut)
                    });
                    match (edge, clipped) {
                        (Some(_), Some(clipped)) => {
                            list.clipped(frame_box, sprite(clipped));
                        }
                        (Some(_), None) if stellar.overhangs() => {
                            list.clipped(frame_box, sprite(image));
                        }
                        _ => {
                            list.push(sprite(image));
                        }
                    }
                }
                (at, size)
            }
            (Some(Err(_)), None) if let Some(stand_in) = &stellar.stand_in => {
                // Placed as a sprite as wide as the stellar's landing reach.
                let centre = camera.sprite_centre(stellar.position, STAND_IN_FRAME);
                outline::draw_stellar(list, stand_in, centre, UNKNOWN_STELLAR_RADIUS);
                (centre, (STAND_IN_SIDE, STAND_IN_SIDE))
            }
            (Some(Err(message)), None) => {
                // Placed as a sprite of its side.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let side = PLACEHOLDER_SIZE as u32;
                let centre = camera.sprite_centre(stellar.position, (side, side));
                crossed_box(list, centre, PLACEHOLDER_SIZE, PLACEHOLDER);
                let below = Point::new(
                    centre.x - PLACEHOLDER_SIZE / 2.0,
                    centre.y + PLACEHOLDER_SIZE / 2.0 + NAME_GAP + MESSAGE_GAP,
                );
                list.text(
                    format!("Sprite unavailable: {message}"),
                    below,
                    NAME_SIZE,
                    None,
                    Color::ERROR,
                );
                (centre, (PLACEHOLDER_SIZE, PLACEHOLDER_SIZE))
            }
        };
        let name_at = Point::new(at.x - width / 2.0, at.y + height / 2.0 + NAME_GAP);
        labels.push((stellar.name.as_str(), name_at));
    }
    for (name, at) in labels {
        list.text(name, at, NAME_SIZE, None, Color::DIM);
    }
}

/// How much of `frame_box`, a frame of `size`, the `world` cuts at its
/// top and its left, as `(top, left)`: each rounded to whole pixels, as
/// the renderer rounds a clip's edge, and kept within the box.
fn cut_by(world: Bounds, frame_box: Bounds, (width, height): (f32, f32)) -> (u16, u16) {
    // Within a box whose sides are a header's u16s.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = |cut: f32, side: f32| cut.round().clamp(0.0, side) as u16;
    (
        side(world.min.y - frame_box.min.y, height),
        side(world.min.x - frame_box.min.x, width),
    )
}

/// Draws `scene`'s stellars as [`draw_stellars`] does in a world clipped
/// to `clip`, each command, the names included, cut to `clip` and appended
/// to `list` in the overlay, with no world clip of its own. The flight's
/// rescue repaints them so over its space area, which is no sprite world:
/// what the closing's `_SWUpdateSpriteWorld` (@0x6c0d) draws through the
/// same clipped draw proc, so a sheet that draws past its box is cut to
/// the box at the play area's edge there too.
pub fn draw_stellars_clipped_to(
    list: &mut DrawList,
    scene: &SystemScene,
    camera: &Camera,
    seen: Option<SeenMurk>,
    clip: Bounds,
) {
    let mut stellars = DrawList::new();
    stellars.world_in(clip, |stellars| {
        draw_stellars(stellars, scene, camera, seen);
    });
    for command in &stellars {
        list.clipped(clip, command.clone());
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::NeverFires;

    use super::*;
    use crate::ImageKey;
    use crate::system::catalog::SystemContents;

    /// Canned systems; records every system asked for.
    struct FakeCatalog {
        systems: Vec<SystemContents>,
        asked: RefCell<Vec<SystemId>>,
    }

    impl FakeCatalog {
        fn new(systems: Vec<SystemContents>) -> Self {
            Self {
                systems,
                asked: RefCell::default(),
            }
        }
    }

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            self.asked.borrow_mut().push(id);
            self.systems
                .iter()
                .find(|system| system.id == id)
                .cloned()
                .expect("a canned system")
        }
    }

    fn sheet(image_id: i16, frames: u16) -> StellarSheet {
        StellarSheet {
            image: crate::SheetImage::Rled(image_id),
            frames: NonZeroU16::new(frames).expect("non-zero"),
            frame_width: 8,
            frame_height: 8,
            image_size: (8, 8),
            eight_bit: false,
            clip_quirks: false,
        }
    }

    const WORMHOLE: StellarAnimation = StellarAnimation {
        delay: 2,
        frame0_bias: -1,
        flags2: 0x2000,
    };

    fn stellar(id: i16, x: i16, y: i16, sprite: Result<StellarSheet, String>) -> StellarContents {
        StellarContents {
            id: StellarId(id),
            name: format!("Stellar {id}"),
            x,
            y,
            sprite,
            dead_sprite: None,
            animation: WORMHOLE,
            stand_in: None,
        }
    }

    /// Graphic type 0's sheet's `rlëD` (`spïn` 1000), of 2 frames.
    const TYPE_ZERO: i16 = 3000;

    fn system(id: i16, stellars: Vec<StellarContents>) -> SystemContents {
        SystemContents {
            id: SystemId(id),
            name: format!("System {id}"),
            stellars,
            type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
            problems: Vec::new(),
        }
    }

    /// System 130 holds a 4-frame stellar 129 at (900, -600), a square on
    /// the radar, a 1-frame
    /// stellar 128 at (0, 0) and stellar 140, with no sheet, at (-1700, 900).
    fn catalog() -> FakeCatalog {
        FakeCatalog::new(vec![
            system(
                130,
                vec![
                    StellarContents {
                        ..stellar(129, 900, -600, Ok(sheet(1001, 4)))
                    },
                    stellar(128, 0, 0, Ok(sheet(1000, 1))),
                    stellar(140, -1700, 900, Err("no spïn 1002".to_owned())),
                ],
            ),
            system(131, Vec::new()),
        ])
    }

    #[test]
    fn a_system_id_gives_its_stellars_at_their_world_positions_in_order() {
        let catalog = catalog();
        let scene = SystemScene::load(&catalog, SystemId(130));
        assert_eq!(scene.id(), SystemId(130));
        assert_eq!(scene.name(), "System 130");
        assert_eq!(
            scene.stellars(),
            [
                SceneStellar {
                    id: StellarId(129),
                    name: "Stellar 129".to_owned(),
                    position: Point::new(900.0, -600.0),
                    sprite: Ok(sheet(1001, 4)),
                    dead_sprite: None,
                    animation_data: WORMHOLE,
                    destroyed: false,
                    drawn: DrawnSheet::Type,
                    frozen: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    frames: StellarFrames::new(),
                    reported: None,
                    stand_in: None,
                },
                SceneStellar {
                    id: StellarId(128),
                    name: "Stellar 128".to_owned(),
                    position: Point::new(0.0, 0.0),
                    sprite: Ok(sheet(1000, 1)),
                    dead_sprite: None,
                    animation_data: WORMHOLE,
                    destroyed: false,
                    drawn: DrawnSheet::Type,
                    frozen: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    frames: StellarFrames::new(),
                    reported: None,
                    stand_in: None,
                },
                SceneStellar {
                    id: StellarId(140),
                    name: "Stellar 140".to_owned(),
                    position: Point::new(-1700.0, 900.0),
                    sprite: Err("no spïn 1002".to_owned()),
                    dead_sprite: None,
                    animation_data: WORMHOLE,
                    destroyed: false,
                    drawn: DrawnSheet::Type,
                    frozen: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    frames: StellarFrames::new(),
                    reported: None,
                    stand_in: None,
                },
            ]
        );
        assert_eq!(scene.problems(), ["spöb 140 Type: no spïn 1002"]);
        // The viewer steps each stellar by the sim's rule over the frames
        // of the sheet it draws (one with none).
        let mut scene = scene;
        scene.step(2, &mut NeverFires);
        let oracle: Vec<u16> = scene
            .stellars()
            .iter()
            .map(|stellar| {
                let count = stellar
                    .sprite
                    .as_ref()
                    .map_or(NonZeroU16::MIN, |sheet| sheet.frames);
                let mut frames = StellarFrames::new();
                for _ in 0..2 {
                    frames.step(stellar.animation_data, false, count, count, &mut NeverFires);
                }
                frames.shown()
            })
            .collect();
        let shown: Vec<u16> = scene.stellars().iter().map(|s| s.frames.shown()).collect();
        assert_eq!(shown, oracle);
        assert_ne!(shown[0], 0, "129 animates");
    }

    #[test]
    fn the_catalog_is_read_once_for_that_system() {
        let catalog = catalog();
        let scene = SystemScene::load(&catalog, SystemId(131));
        let _ = (scene.bounds(), scene.stellars());
        assert_eq!(*catalog.asked.borrow(), [SystemId(131)]);
    }

    #[test]
    fn the_bounds_hold_the_centre_and_every_stellar_with_the_margin() {
        let scene = SystemScene::load(&catalog(), SystemId(130));
        assert_eq!(
            scene.bounds(),
            Bounds {
                min: Point::new(-1700.0 - CAMERA_MARGIN, -600.0 - CAMERA_MARGIN),
                max: Point::new(900.0 + CAMERA_MARGIN, 900.0 + CAMERA_MARGIN),
            }
        );
    }

    #[test]
    fn an_empty_systems_bounds_are_the_centre_and_the_margin() {
        let scene = SystemScene::load(&catalog(), SystemId(131));
        assert_eq!(
            scene.bounds(),
            Bounds {
                min: Point::new(-CAMERA_MARGIN, -CAMERA_MARGIN),
                max: Point::new(CAMERA_MARGIN, CAMERA_MARGIN),
            }
        );
    }

    #[test]
    fn the_bounds_include_the_centre_when_every_stellar_is_off_to_one_side() {
        let catalog = FakeCatalog::new(vec![system(
            132,
            vec![
                stellar(150, 3000, 2000, Ok(sheet(1000, 1))),
                stellar(151, 5000, 2500, Ok(sheet(1000, 1))),
            ],
        )]);
        let bounds = SystemScene::load(&catalog, SystemId(132)).bounds();
        assert_eq!(
            bounds,
            Bounds {
                min: Point::new(-CAMERA_MARGIN, -CAMERA_MARGIN),
                max: Point::new(5000.0 + CAMERA_MARGIN, 2500.0 + CAMERA_MARGIN),
            }
        );
    }

    /// Animates only when destroyed, every 2 ticks.
    const DEAD_ANIMATED: StellarAnimation = StellarAnimation {
        delay: 2,
        frame0_bias: -1,
        flags2: 0x0080,
    };

    /// System 134: stellar 150 at (0, 0), whose `DeadType` sheet 2150 has
    /// 3 frames of 16 x 16 and animates only when destroyed; 151 with no
    /// different graphic; 152 whose `DeadType` sheet could not be read;
    /// and a 4-frame wormhole 153, with no `DeadType`.
    fn dying() -> FakeCatalog {
        let dead_sheet = StellarSheet {
            frame_width: 16,
            frame_height: 16,
            image_size: (16, 16),
            ..sheet(2150, 3)
        };
        FakeCatalog::new(vec![system(
            134,
            vec![
                StellarContents {
                    dead_sprite: Some(Ok(dead_sheet)),
                    animation: DEAD_ANIMATED,
                    ..stellar(150, 0, 0, Ok(sheet(1000, 1)))
                },
                stellar(151, 100, 0, Ok(sheet(1001, 1))),
                StellarContents {
                    dead_sprite: Some(Err("no spïn 1007".to_owned())),
                    ..stellar(152, 200, 0, Ok(sheet(1002, 1)))
                },
                stellar(153, 300, 0, Ok(sheet(1003, 4))),
            ],
        )])
    }

    /// A whole stellar, drawing its `Type` sheet on the frame its own
    /// animation shows.
    const WHOLE: StellarLook = StellarLook {
        destroyed: false,
        drawn: DrawnSheet::Type,
        frozen: false,
        frame: None,
    };

    /// A destroyed stellar drawing `drawn`.
    fn dead(drawn: DrawnSheet) -> StellarLook {
        StellarLook {
            destroyed: true,
            drawn,
            ..WHOLE
        }
    }

    /// Stellar 150 destroyed, drawing its `DeadType` sheet; every other
    /// whole.
    fn only_150_dead(id: StellarId) -> StellarLook {
        if id == StellarId(150) {
            dead(DrawnSheet::Dead)
        } else {
            WHOLE
        }
    }

    /// Each stellar's sprite drawn, in order.
    fn drawn(scene: &SystemScene) -> Vec<ImageKey> {
        let mut list = DrawList::new();
        draw_stellars(
            &mut list,
            scene,
            &Camera::centred_on(Point::default()),
            None,
        );
        list.iter()
            .filter_map(|command| match command {
                crate::DrawCommand::Sprite { image, .. } => Some(*image),
                _ => None,
            })
            .collect()
    }

    /// Where `name` is drawn.
    fn name_at(scene: &SystemScene, name: &str) -> Point {
        let mut list = DrawList::new();
        let camera = Camera::centred_on(Point::default());
        draw_stellars(&mut list, scene, &camera, None);
        list.iter()
            .find_map(|command| match command {
                crate::DrawCommand::Text { text, origin, .. } if text == name => Some(*origin),
                _ => None,
            })
            .expect("the name is drawn")
    }

    /// The names drawn, in order.
    fn names_drawn(scene: &SystemScene) -> Vec<String> {
        let mut list = DrawList::new();
        let camera = Camera::centred_on(Point::default());
        draw_stellars(&mut list, scene, &camera, None);
        list.iter()
            .filter_map(|command| match command {
                crate::DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// Two stellars at the same place, listed in `navigation` order.
    fn overlapping(navigation: [i16; 2]) -> SystemScene {
        let stellars = navigation
            .iter()
            .map(|&id| stellar(id, 0, 0, Ok(sheet(id + 1000, 1))))
            .collect();
        SystemScene::load(
            &FakeCatalog::new(vec![system(135, stellars)]),
            SystemId(135),
        )
    }

    #[test]
    fn a_stellar_sprite_is_murked_seen_from_the_viewer_and_its_placeholder_and_name_are_not() {
        let scene = SystemScene::load(
            &FakeCatalog::new(vec![system(
                135,
                vec![
                    stellar(200, 200, 0, Ok(sheet(1200, 1))),
                    stellar(300, 200, 0, Err("no spïn".to_owned())),
                ],
            )]),
            SystemId(135),
        );
        let tints = |seen| {
            let mut list = DrawList::new();
            draw_stellars(
                &mut list,
                &scene,
                &Camera::centred_on(Point::default()),
                seen,
            );
            list.iter()
                .filter_map(|command| match command {
                    crate::DrawCommand::Sprite { tint, .. } => Some((*tint, None)),
                    crate::DrawCommand::Colorized { tint, colorize, .. } => {
                        Some((*tint, Some(*colorize)))
                    }
                    crate::DrawCommand::Text { color, .. } => Some((*color, None)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let seen_from = |viewer, murk| SeenMurk {
            viewer,
            murk,
            colour: 0x1081,
        };
        let level = seen_from(Vec2::ZERO, 50).at(Vec2::new(200.0, 0.0)).level;
        assert_eq!(level, 24, "200 off in murk 50");
        let murked = tints(Some(seen_from(Vec2::ZERO, 50)));
        assert_eq!(
            murked[0],
            (
                Color::WHITE,
                Some(crate::Colorize::toward_pixel(24, 0x1081))
            )
        );
        let plain = tints(None);
        assert_eq!(plain[0], (Color::WHITE, None), "no murk draws plainly");
        assert_eq!(murked[1..], plain[1..], "the placeholder and the names");
        let beside = tints(Some(seen_from(Vec2::new(200.0, 0.0), 100)));
        assert_eq!(beside, plain, "nothing at the viewer");
    }

    #[test]
    fn overlapping_stellars_draw_in_ascending_id_whatever_their_navigation_order() {
        let lower_on_bottom = [ImageKey::sprite(1200, 0), ImageKey::sprite(1300, 0)];
        for navigation in [[300, 200], [200, 300]] {
            let scene = overlapping(navigation);
            let ids: Vec<_> = scene.stellars().iter().map(|s| s.id.0).collect();
            assert_eq!(ids, navigation, "the scene keeps navigation order");
            assert_eq!(drawn(&scene), lower_on_bottom);
            assert_eq!(names_drawn(&scene), ["Stellar 200", "Stellar 300"]);
        }
    }

    #[test]
    fn a_stellars_shown_frame_is_the_one_drawn_with_its_frame_size() {
        let mut scene = SystemScene::load(&catalog(), SystemId(130));
        scene.step(3, &mut NeverFires);
        let shown = scene.shown_frame(StellarId(129)).expect("drawn");
        assert_eq!(shown.0, drawn(&scene)[1], "as drawn");
        let sheet = scene.stellars()[1]
            .drawn()
            .and_then(|sheet| sheet.as_ref().ok())
            .copied()
            .expect("a sheet");
        assert_eq!(
            shown.1,
            (sheet.frame_width as f32, sheet.frame_height as f32)
        );
        scene.set_looks(|id| StellarLook {
            frame: (id == StellarId(129)).then_some(2),
            ..WHOLE
        });
        assert_eq!(
            scene.shown_frame(StellarId(129)).map(|shown| shown.0),
            Some(ImageKey::sprite(1001, 2))
        );
        assert_eq!(scene.shown_frame(StellarId(999)), None, "not in it");
    }

    /// `scene` with stellar 153 frozen, drawing its `Type`, and 151
    /// showing no frame.
    fn frozen_153_frameless_151(scene: &mut SystemScene) {
        scene.set_looks(|id| match id.0 {
            153 => StellarLook {
                frozen: true,
                ..WHOLE
            },
            151 => StellarLook {
                drawn: DrawnSheet::NoFrame,
                ..WHOLE
            },
            _ => WHOLE,
        });
    }

    #[test]
    fn a_slot_whose_frames_count_none_is_32_square_frozen_or_frameless() {
        // `_GetSpriteXSize` @0x26d4-0x2702: 32 for a frame index not below
        // the count (a frozen frame) or a null frame (no frame).
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let size = |scene: &SystemScene, id: i16| {
            scene
                .stellars()
                .iter()
                .find(|stellar| stellar.id == StellarId(id))
                .expect("held")
                .slot_size()
        };
        assert_eq!(size(&scene, 153), Some((8, 8)), "its sheet's frame");
        frozen_153_frameless_151(&mut scene);
        assert_eq!(size(&scene, 153), Some((32, 32)), "frozen");
        assert_eq!(size(&scene, 151), Some((32, 32)), "showing no frame");
        let unreadable = SystemScene::load(&catalog(), SystemId(130));
        assert_eq!(size(&unreadable, 140), None, "a sheet that can't be read");
    }

    #[test]
    fn a_frozen_stellars_shown_frame_is_placed_as_a_32_pixel_sprite() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        assert_eq!(
            scene.shown_frame(StellarId(153)),
            Some((ImageKey::sprite(1003, 0), (8.0, 8.0), (8.0, 8.0)))
        );
        frozen_153_frameless_151(&mut scene);
        assert_eq!(
            scene.shown_frame(StellarId(153)),
            Some((ImageKey::sprite(1003, 0), (8.0, 8.0), (32.0, 32.0)))
        );
        assert_eq!(scene.shown_frame(StellarId(151)), None, "no frame");
    }

    #[test]
    fn a_stellar_draws_the_frame_its_session_reports_modulo_its_sheet() {
        // A planet and a hypergate, each of 4 frames: the scene draws
        // whatever frame its session reports, gate or not.
        let gate = StellarContents {
            animation: StellarAnimation {
                flags2: nova_sim::gate::HYPERGATE,
                ..WORMHOLE
            },
            ..stellar(151, 100, 0, Ok(sheet(1151, 4)))
        };
        let catalog = FakeCatalog::new(vec![system(
            134,
            vec![stellar(150, 0, 0, Ok(sheet(1150, 4))), gate],
        )]);
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        let reporting = |frame| {
            move |_| StellarLook {
                frame: Some(frame),
                ..WHOLE
            }
        };
        scene.set_looks(reporting(3));
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(1150, 3), ImageKey::sprite(1151, 3)]
        );
        // A frame past the sheet drawn wraps round its 4 frames.
        scene.set_looks(reporting(6));
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(1150, 2), ImageKey::sprite(1151, 2)]
        );
        // None draws the scene's own animation, which the viewer steps.
        scene.step(3, &mut NeverFires);
        scene.set_looks(|_| WHOLE);
        let own: Vec<ImageKey> = scene
            .stellars()
            .iter()
            .map(|stellar| ImageKey::sprite(1000 + stellar.id.0, stellar.frames.shown()))
            .collect();
        assert_eq!(drawn(&scene), own);
    }

    #[test]
    fn set_looks_sets_the_sheet_drawn_frozen_and_the_frame_with_no_animation_state() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let before: Vec<StellarFrames> = scene.stellars().iter().map(|s| s.frames).collect();
        scene.set_looks(|id| StellarLook {
            destroyed: true,
            drawn: DrawnSheet::Dead,
            frozen: id == StellarId(153),
            frame: Some(2),
        });
        let stellar = &scene.stellars()[0];
        assert!(stellar.destroyed);
        assert_eq!(stellar.drawn, DrawnSheet::Dead);
        assert!(!stellar.frozen);
        assert_eq!(stellar.reported, Some(2));
        assert!(scene.stellars()[3].frozen);
        assert_eq!(drawn(&scene)[0], ImageKey::sprite(2150, 2));
        let after: Vec<StellarFrames> = scene.stellars().iter().map(|s| s.frames).collect();
        assert_eq!(after, before, "no frame state touched");
    }

    #[test]
    fn a_destroyed_stellar_draws_its_dead_type_sheet_until_it_is_whole_again() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let whole = [
            ImageKey::sprite(1000, 0),
            ImageKey::sprite(1001, 0),
            ImageKey::sprite(1002, 0),
            ImageKey::sprite(1003, 0),
        ];
        assert_eq!(drawn(&scene), whole, "loaded whole");
        scene.set_looks(only_150_dead);
        assert!(scene.stellars()[0].destroyed);
        assert_eq!(
            drawn(&scene)[0],
            ImageKey::sprite(2150, 0),
            "the DeadType sheet"
        );
        assert_eq!(drawn(&scene)[1..], whole[1..]);
        scene.set_looks(|_| WHOLE);
        assert!(!scene.stellars()[0].destroyed);
        assert_eq!(drawn(&scene), whole, "regenerated");
    }

    #[test]
    fn the_dead_type_sheet_drawn_with_no_different_graphic_is_the_type_sheet() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_looks(|_| dead(DrawnSheet::Dead));
        assert!(scene.stellars().iter().all(|stellar| stellar.destroyed));
        assert_eq!(scene.stellars()[1].drawn(), Some(&Ok(sheet(1001, 1))));
        assert_eq!(
            scene.stellars()[2].drawn(),
            Some(&Err("no spïn 1007".to_owned())),
            "an unreadable one says why"
        );
    }

    #[test]
    fn a_destroyed_stellar_drawing_its_type_sheet_keeps_it() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_looks(|_| dead(DrawnSheet::Type));
        assert!(scene.stellars().iter().all(|stellar| stellar.destroyed));
        assert_eq!(
            drawn(&scene),
            [
                ImageKey::sprite(1000, 0),
                ImageKey::sprite(1001, 0),
                ImageKey::sprite(1002, 0),
                ImageKey::sprite(1003, 0),
            ]
        );
    }

    #[test]
    fn the_name_sits_under_the_sheet_drawn() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let at = Camera::centred_on(Point::default()).world_to_screen(Point::default());
        let whole = name_at(&scene, "Stellar 150");
        assert_eq!(whole, Point::new(at.x - 4.0, at.y + 4.0 + NAME_GAP));
        scene.set_looks(only_150_dead);
        let dead = name_at(&scene, "Stellar 150");
        assert_eq!(dead, Point::new(at.x - 8.0, at.y + 8.0 + NAME_GAP));
    }

    #[test]
    fn a_scene_loaded_for_a_shown_set_keeps_only_those_stellars_in_navigation_order() {
        let mut catalog = dying();
        catalog.systems[0].problems = vec!["sÿst 134: no spöb 160".to_owned()];
        catalog.systems[0].type_zero_sprite = Err("no spïn 1000".to_owned());
        let shown = [StellarId(153), StellarId(150)].into();
        let scene = SystemScene::load_shown(&catalog, SystemId(134), &shown);
        let ids: Vec<i16> = scene
            .stellars()
            .iter()
            .map(|stellar| stellar.id.0)
            .collect();
        assert_eq!(ids, [150, 153]);
        assert_eq!(
            scene.problems(),
            [
                "sÿst 134: no spöb 160",
                "spïn 1000 (graphic type 0): no spïn 1000"
            ],
            "152's DeadType left out with it"
        );
        let shown = [StellarId(150)].into();
        let scene = SystemScene::load_shown(&catalog, SystemId(134), &shown);
        assert_eq!(
            scene.problems(),
            ["sÿst 134: no spöb 160"],
            "150 has its DeadType, so none falls back"
        );
        let scene = SystemScene::load_shown(&catalog, SystemId(134), &BTreeSet::new());
        assert_eq!(scene.stellars(), []);
        assert_eq!(scene.id(), SystemId(134));
    }

    #[test]
    fn problems_pass_through() {
        let mut broken = system(133, Vec::new());
        broken.problems = vec!["sÿst 133: no spöb 150".to_owned(), "x".to_owned()];
        let scene = SystemScene::load(&FakeCatalog::new(vec![broken]), SystemId(133));
        assert_eq!(scene.problems(), ["sÿst 133: no spöb 150", "x"]);
    }

    // Graphic type 0's sheet drawn.

    #[test]
    fn a_dead_type_sheet_that_cannot_be_read_is_a_problem() {
        let scene = SystemScene::load(&dying(), SystemId(134));
        assert_eq!(scene.problems(), ["spöb 152 DeadType: no spïn 1007"]);
    }

    #[test]
    fn the_catalogs_problems_come_first() {
        let mut catalog = dying();
        catalog.systems[0].problems = vec!["sÿst 134: no spöb 160".to_owned()];
        let scene = SystemScene::load(&catalog, SystemId(134));
        assert_eq!(
            scene.problems(),
            ["sÿst 134: no spöb 160", "spöb 152 DeadType: no spïn 1007"]
        );
    }

    #[test]
    fn a_missing_type_zero_sheet_is_a_problem_only_when_a_stellar_might_fall_back_to_it() {
        let mut catalog = dying();
        catalog.systems[0].type_zero_sprite = Err("no spïn 1000".to_owned());
        let scene = SystemScene::load(&catalog, SystemId(134));
        assert_eq!(
            scene.problems(),
            [
                "spöb 152 DeadType: no spïn 1007",
                "spïn 1000 (graphic type 0): no spïn 1000",
            ]
        );
        let first = catalog.systems[0].stellars[0].clone();
        catalog.systems[0].stellars = vec![first];
        let scene = SystemScene::load(&catalog, SystemId(134));
        assert_eq!(scene.problems(), [] as [String; 0], "150 has its DeadType");
    }

    #[test]
    fn a_missing_type_zero_sheet_is_a_problem_when_a_stellar_has_no_type_sheet() {
        let mut broken = system(
            135,
            vec![StellarContents {
                dead_sprite: Some(Ok(sheet(1009, 1))),
                ..stellar(160, 0, 0, Err("no spïn 1010".to_owned()))
            }],
        );
        broken.type_zero_sprite = Err("no spïn 1000".to_owned());
        let scene = SystemScene::load(&FakeCatalog::new(vec![broken]), SystemId(135));
        assert_eq!(
            scene.problems(),
            [
                "spöb 160 Type: no spïn 1010",
                "spïn 1000 (graphic type 0): no spïn 1000"
            ],
            "whole, it may fall back to graphic type 0"
        );
    }

    #[test]
    fn a_type_sheet_that_cannot_be_read_is_a_problem() {
        let scene = SystemScene::load(&catalog(), SystemId(130));
        assert_eq!(scene.problems(), ["spöb 140 Type: no spïn 1002"]);
        let shown = BTreeSet::from([StellarId(128), StellarId(129)]);
        let scene = SystemScene::load_shown(&catalog(), SystemId(130), &shown);
        assert_eq!(scene.problems(), [] as [String; 0], "140 is not shown");
    }

    #[test]
    fn a_stellar_drawing_type_zero_draws_graphic_type_0s_sheet_over_its_frames() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_looks(|_| dead(DrawnSheet::TypeZero));
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(TYPE_ZERO, 0); 4],
            "whatever other sheet it has"
        );
        let sheets: Vec<DrawnSheet> = scene.stellars().iter().map(|s| s.drawn).collect();
        assert_eq!(sheets, [DrawnSheet::TypeZero; 4]);
    }

    #[test]
    fn the_name_sits_under_the_type_zero_sheet_drawn() {
        let mut catalog = dying();
        catalog.systems[0].type_zero_sprite = Ok(StellarSheet {
            frame_width: 40,
            frame_height: 40,
            image_size: (40, 40),
            ..sheet(TYPE_ZERO, 2)
        });
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        let at = Camera::centred_on(Point::default()).world_to_screen(Point::new(100.0, 0.0));
        scene.set_looks(|_| dead(DrawnSheet::TypeZero));
        assert_eq!(
            name_at(&scene, "Stellar 151"),
            Point::new(at.x - 20.0, at.y + 20.0 + NAME_GAP)
        );
    }

    // A slot showing no frame (stellar-fidelity phase 8).

    #[test]
    fn a_stellar_drawing_no_frame_draws_no_sprite_only_its_name_under_its_centre() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_looks(|id| {
            if id == StellarId(151) {
                StellarLook {
                    drawn: DrawnSheet::NoFrame,
                    ..WHOLE
                }
            } else {
                WHOLE
            }
        });
        assert_eq!(scene.stellars()[1].drawn(), None);
        assert_eq!(
            drawn(&scene),
            [
                ImageKey::sprite(1000, 0),
                ImageKey::sprite(1002, 0),
                ImageKey::sprite(1003, 0),
            ],
            "nothing for 151"
        );
        let at = Camera::centred_on(Point::default()).world_to_screen(Point::new(100.0, 0.0));
        assert_eq!(
            name_at(&scene, "Stellar 151"),
            Point::new(at.x, at.y + NAME_GAP)
        );
        let mut list = DrawList::new();
        draw_stellars(
            &mut list,
            &scene,
            &Camera::centred_on(Point::default()),
            None,
        );
        assert!(
            !list
                .iter()
                .any(|command| matches!(command, crate::DrawCommand::Text { text, .. } if text.starts_with("Sprite unavailable"))),
            "no placeholder either"
        );
    }

    /// Where the sprite of `image` is centred.
    fn sprite_at(scene: &SystemScene, image: ImageKey) -> Point {
        let mut list = DrawList::new();
        draw_stellars(
            &mut list,
            scene,
            &Camera::centred_on(Point::default()),
            None,
        );
        list.iter()
            .find_map(|command| match command {
                crate::DrawCommand::Sprite {
                    image: drawn,
                    center,
                    ..
                } if *drawn == image => Some(*center),
                _ => None,
            })
            .expect("the sprite is drawn")
    }

    #[test]
    fn a_frozen_stellar_draws_its_frame_where_a_32_pixel_slot_puts_it() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let frozen_153 = |id: StellarId| StellarLook {
            frozen: id == StellarId(153),
            frame: Some(1),
            ..WHOLE
        };
        scene.set_looks(frozen_153);
        assert!(scene.stellars()[3].frozen);
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(1003, 1));
        // An 8-pixel frame whose top left is the centre less 16: its
        // centre is 12 up and left of the stellar's.
        let at = Camera::centred_on(Point::default()).world_to_screen(Point::new(300.0, 0.0));
        assert_eq!(
            sprite_at(&scene, ImageKey::sprite(1003, 1)),
            Point::new(at.x - 12.0, at.y - 12.0)
        );
        assert_eq!(
            name_at(&scene, "Stellar 153"),
            Point::new(at.x - 16.0, at.y - 8.0 + NAME_GAP),
            "under the sprite drawn"
        );
        // Thawed, it is centred again.
        scene.set_looks(|_| StellarLook {
            frame: Some(1),
            ..WHOLE
        });
        assert_eq!(
            sprite_at(&scene, ImageKey::sprite(1003, 1)),
            at,
            "centred again"
        );
    }

    /// A camera on a fractional point, as flight's follows the ship.
    fn off_pixel() -> Camera {
        Camera::centred_on(Point::new(0.4, -0.6))
    }

    /// What `draw_stellars` draws of `scene` with `camera`.
    fn list_with(scene: &SystemScene, camera: &Camera) -> DrawList {
        let mut list = DrawList::new();
        draw_stellars(&mut list, scene, camera, None);
        list
    }

    /// The top left of the sprite of `image` drawn in `list`, `size` in
    /// size.
    fn top_left_in(list: &DrawList, image: ImageKey, (width, height): (f32, f32)) -> Point {
        list.iter()
            .find_map(|command| match command {
                crate::DrawCommand::Sprite {
                    image: drawn,
                    center,
                    ..
                } if *drawn == image => {
                    Some(Point::new(center.x - width / 2.0, center.y - height / 2.0))
                }
                _ => None,
            })
            .expect("the sprite is drawn")
    }

    /// System 136: stellars of 7 x 6 and 6 x 7 frames on each side of the
    /// centre.
    fn odd_sized() -> SystemScene {
        let sized = |image_id, (frame_width, frame_height)| StellarSheet {
            frame_width,
            frame_height,
            image_size: (frame_width, frame_height),
            ..sheet(image_id, 1)
        };
        let stellars = vec![
            stellar(160, 10, 20, Ok(sized(1160, (7, 6)))),
            stellar(161, -10, -20, Ok(sized(1161, (7, 6)))),
            stellar(162, 10, -20, Ok(sized(1162, (6, 7)))),
            stellar(163, -10, 20, Ok(sized(1163, (6, 7)))),
        ];
        SystemScene::load(
            &FakeCatalog::new(vec![system(136, stellars)]),
            SystemId(136),
        )
    }

    #[test]
    fn a_stellar_is_drawn_at_whole_pixels_from_the_camera_its_odd_pixel_right_and_below() {
        let scene = odd_sized();
        let camera = off_pixel();
        let list = list_with(&scene, &camera);
        for (image, (x, y), size) in [
            (1160, (10.0, 20.0), (7, 6)),
            (1161, (-10.0, -20.0), (7, 6)),
            (1162, (10.0, -20.0), (6, 7)),
            (1163, (-10.0, 20.0), (6, 7)),
        ] {
            #[allow(clippy::cast_precision_loss)]
            let drawn = top_left_in(
                &list,
                ImageKey::sprite(image, 0),
                (size.0 as f32, size.1 as f32),
            );
            assert_eq!(
                drawn,
                camera.sprite_top_left(Point::new(x, y), size),
                "{image}"
            );
        }
        // (9.6, 20.6) truncates to (9, 20) and 7 x 6 halves to (3, 3);
        // (-10.4, -19.4) to (-10, -19) and 6 x 7 to (3, 3), toward zero.
        let anchor = camera.anchor();
        assert_eq!(
            top_left_in(&list, ImageKey::sprite(1160, 0), (7.0, 6.0)),
            Point::new(anchor.x + 6.0, anchor.y + 17.0)
        );
        assert_eq!(
            top_left_in(&list, ImageKey::sprite(1163, 0), (6.0, 7.0)),
            Point::new(anchor.x - 13.0, anchor.y + 17.0)
        );
    }

    #[test]
    fn a_stellars_name_sits_under_its_placed_sprite() {
        let scene = odd_sized();
        let camera = off_pixel();
        let list = list_with(&scene, &camera);
        let top_left = top_left_in(&list, ImageKey::sprite(1160, 0), (7.0, 6.0));
        let name = list
            .iter()
            .find_map(|command| match command {
                crate::DrawCommand::Text { text, origin, .. } if text == "Stellar 160" => {
                    Some(*origin)
                }
                _ => None,
            })
            .expect("named");
        assert_eq!(name, Point::new(top_left.x, top_left.y + 6.0 + NAME_GAP));
    }

    #[test]
    fn a_frozen_stellar_sits_as_a_32_pixel_slot_at_whole_pixels() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_looks(|id: StellarId| StellarLook {
            frozen: id == StellarId(153),
            ..WHOLE
        });
        let camera = off_pixel();
        let list = list_with(&scene, &camera);
        // Its 8-pixel frame's top left is the 32-pixel slot's.
        assert_eq!(
            top_left_in(&list, ImageKey::sprite(1003, 0), (8.0, 8.0)),
            camera.sprite_top_left(Point::new(300.0, 0.0), (32, 32))
        );
    }

    #[test]
    fn a_stellars_placeholder_sits_as_a_sprite_of_its_size() {
        let scene = SystemScene::load(&catalog(), SystemId(130));
        let camera = off_pixel();
        let list = list_with(&scene, &camera);
        let side = PLACEHOLDER_SIZE;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let top_left =
            camera.sprite_top_left(Point::new(-1700.0, 900.0), (side as u32, side as u32));
        let corner = list.iter().find_map(|command| match command {
            crate::DrawCommand::Line { from, .. } if *from == top_left => Some(*from),
            _ => None,
        });
        assert_eq!(corner, Some(top_left), "its box's top left");
        let reason = list
            .iter()
            .find_map(|command| match command {
                crate::DrawCommand::Text { text, origin, .. }
                    if text.starts_with("Sprite unavailable") =>
                {
                    Some(*origin)
                }
                _ => None,
            })
            .expect("why");
        assert_eq!(
            reason,
            Point::new(top_left.x, top_left.y + side + NAME_GAP + MESSAGE_GAP)
        );
    }

    #[test]
    fn a_type_zero_sheet_that_cannot_be_read_draws_the_placeholder_and_why() {
        let mut catalog = dying();
        catalog.systems[0].type_zero_sprite = Err("no spïn 1000".to_owned());
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        scene.set_looks(|_| dead(DrawnSheet::TypeZero));
        assert_eq!(
            scene.stellars()[1].drawn(),
            Some(&Err("no spïn 1000".to_owned()))
        );
    }

    // Stand-ins (rdm:phase/new-game-engine/phase-4-draw-flat-outlines):
    // a stellar whose sprite cannot be shown draws its data's stand-in at
    // the landing reach. Which stellars have which is the data's
    // (nova-data's `new_game::looks`); how each is drawn is the outline
    // module's.

    /// A dart 4 from its centre to its nose, red.
    fn dart() -> crate::outline::Outline {
        crate::outline::Outline {
            triangles: vec![crate::outline::OutlineTriangle {
                corners: [
                    Point::new(0.0, 4.0),
                    Point::new(1.0, -2.0),
                    Point::new(-1.0, -2.0),
                ],
                color: Color::rgba(255, 0, 0, 255),
            }],
        }
    }

    /// A scene of one stellar, 140 at (-1700, 900), whose sprite is
    /// `sprite` and whose stand-in is `stand_in`.
    fn standing_in(
        sprite: Result<StellarSheet, String>,
        stand_in: Option<StellarStandIn>,
    ) -> SystemScene {
        let one = StellarContents {
            stand_in,
            ..stellar(140, -1700, 900, sprite)
        };
        SystemScene::load(
            &FakeCatalog::new(vec![system(135, vec![one])]),
            SystemId(135),
        )
    }

    /// The stand-in's centre with `camera`: a sprite as wide as the
    /// landing reach's centre.
    fn stand_in_centre(camera: &Camera) -> Point {
        let side = 2.0 * UNKNOWN_STELLAR_RADIUS;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frame = (side as u32, side as u32);
        camera.sprite_centre(Point::new(-1700.0, 900.0), frame)
    }

    fn triangles(list: &DrawList) -> Vec<crate::DrawCommand> {
        list.iter()
            .filter(|command| matches!(command, crate::DrawCommand::Triangle { .. }))
            .cloned()
            .collect()
    }

    fn no_box_or_reason(list: &DrawList) -> bool {
        !list.iter().any(|command| {
            matches!(command, crate::DrawCommand::Line { .. })
                || matches!(command, crate::DrawCommand::Text { text, .. } if text.starts_with("Sprite"))
        })
    }

    #[test]
    fn a_stellar_with_no_sprite_draws_its_outline_fitted_to_the_landing_reach_nose_up() {
        let scene = standing_in(
            Err("no spïn 1002".to_owned()),
            Some(StellarStandIn::Outline(dart())),
        );
        let camera = off_pixel();
        let list = list_with(&scene, &camera);
        let mut expected = DrawList::new();
        crate::outline::draw(
            &mut expected,
            &dart(),
            stand_in_centre(&camera),
            0.0,
            UNKNOWN_STELLAR_RADIUS / 4.0,
        );
        assert_eq!(triangles(&list), triangles(&expected));
        assert!(no_box_or_reason(&list));
    }

    #[test]
    fn a_stellar_s_disc_and_ring_are_drawn_at_the_landing_reach_in_their_colours() {
        let camera = off_pixel();
        let teal = Color::rgba(0, 128, 128, 255);
        for stand_in in [StellarStandIn::Disc(teal), StellarStandIn::Ring(teal)] {
            let scene = standing_in(Err("no spïn 1002".to_owned()), Some(stand_in.clone()));
            let list = list_with(&scene, &camera);
            let mut expected = DrawList::new();
            crate::outline::draw_stellar(
                &mut expected,
                &stand_in,
                stand_in_centre(&camera),
                UNKNOWN_STELLAR_RADIUS,
            );
            assert_eq!(triangles(&list), triangles(&expected), "{stand_in:?}");
            assert!(!triangles(&list).is_empty());
            assert!(no_box_or_reason(&list), "{stand_in:?}");
        }
    }

    #[test]
    fn a_stellar_s_name_sits_under_its_stand_in_as_under_a_sprite_of_its_size() {
        let scene = standing_in(
            Err("no spïn 1002".to_owned()),
            Some(StellarStandIn::Disc(Color::WHITE)),
        );
        let centre = stand_in_centre(&Camera::centred_on(Point::default()));
        let half = UNKNOWN_STELLAR_RADIUS;
        assert_eq!(
            name_at(&scene, "Stellar 140"),
            Point::new(centre.x - half, centre.y + half + NAME_GAP)
        );
    }

    #[test]
    fn a_stellar_with_no_stand_in_draws_the_crossed_box_and_why() {
        let scene = standing_in(Err("no spïn 1002".to_owned()), None);
        let list = list_with(&scene, &off_pixel());
        assert!(triangles(&list).is_empty());
        assert!(!no_box_or_reason(&list));
    }

    #[test]
    fn a_stellar_with_a_sprite_ignores_its_stand_in() {
        let scene = standing_in(Ok(sheet(1000, 1)), Some(StellarStandIn::Ring(Color::WHITE)));
        let list = list_with(&scene, &off_pixel());
        assert!(triangles(&list).is_empty());
        assert_eq!(
            list.iter()
                .filter(|command| matches!(command, crate::DrawCommand::Sprite { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn a_stellar_carries_its_stand_in_into_the_scene() {
        let scene = standing_in(
            Err("no spïn 1002".to_owned()),
            Some(StellarStandIn::Ring(Color::WHITE)),
        );
        assert_eq!(
            scene.stellars()[0].stand_in,
            Some(StellarStandIn::Ring(Color::WHITE))
        );
    }
}
