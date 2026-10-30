//! A system laid out for viewing: each stellar at its world position with
//! its sheet (or why not) and its animation, read once from the catalog
//! and stepped tick by tick ([`SystemScene::step`]); and
//! [`draw_stellars`], which draws them through a camera for every screen
//! that shows a system.
//!
//! A scene laid out starts each stellar's animation at frame 0. The flight
//! then resumes each stellar it has shown before where it was left
//! ([`SystemScene::resume`] from its [`KeptFrames`], stellar-fidelity
//! phase 12), as the original keeps the state on the `spöb` record; the
//! galaxy viewer, which has no counterpart in the original, does not.

use std::collections::BTreeSet;
use std::num::NonZeroU16;

use super::animation::Frames;
use super::camera::{CAMERA_MARGIN, Camera};
use super::kept_frames::KeptFrames;
use nova_sim::{Chance, DrawnSheet};

use super::catalog::{
    AnimationData, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemId,
};
use crate::draw::crossed_box;
use crate::geometry::Bounds;
use crate::image::ImageKey;
use crate::{Color, DrawList, Point};

/// The side of the box drawn where a stellar's sprite cannot be shown.
pub const PLACEHOLDER_SIZE: f32 = 64.0;
/// The placeholder's colour.
pub(crate) const PLACEHOLDER: Color = Color::rgba(128, 128, 128, 255);
/// How far below a stellar its name goes, and the name's size.
const NAME_GAP: f32 = 4.0;
const NAME_SIZE: f32 = 14.0;
/// How far below a placeholder's name the reason goes.
const MESSAGE_GAP: f32 = 18.0;
/// The side `_GetSpriteXSize` and `_GetSpriteYSize` give a slot whose
/// frames count none (@0x26e5-0x26ef, @0x2714-0x271e).
const FROZEN_SIDE: u32 = 32;

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
    pub animation_data: AnimationData,
    /// Whether it is destroyed, as last set.
    pub destroyed: bool,
    /// Which sheet it draws, as last set: the session's answer
    /// (`Session::drawn_sheet`).
    pub drawn: DrawnSheet,
    /// Which sheet's frame count it animates over, as last set: the
    /// session's answer (`Session::counted_sheet`).
    pub counted: DrawnSheet,
    /// Whether its sprite slot shows a frame frozen, as last set
    /// (`Session::frame_frozen`): the frame shown stays, and is drawn
    /// where a 32-pixel sprite's would be, its top left at the centre
    /// less 16 on each axis.
    pub frozen: bool,
    /// Whether it animates over no frames, as last set
    /// (`Session::counts_no_frames`): it never animates.
    pub counts_none: bool,
    /// Graphic type 0's sheet (`spïn` 1000), or why it cannot be shown.
    pub type_zero_sprite: Result<StellarSheet, String>,
    /// Its animation state: the frame of the sheet drawn that shows, and
    /// when the next shows. A sheet that cannot be read counts 1 frame.
    /// Fresh as the scene is laid out, and then, in a flight, as the
    /// stellar was left the last time it was shown
    /// ([`SystemScene::resume`]).
    pub frames: Frames,
    /// The frame a hypergate draws, as its session steps it
    /// (`Session::gate_frame`), in place of [`frames`](Self::frames)';
    /// `None` for every other stellar, and in a viewer with no session.
    pub gate_frame: Option<u16>,
}

/// How a stellar looks, for [`SystemScene::set_destroyed`] and
/// [`SystemScene::rebuild`]: whether it is destroyed, which its animation
/// reads, which sheet it draws (`Session::drawn_sheet`), which sheet's
/// frame count it animates over (`Session::counted_sheet`), whether its
/// frame is frozen (`Session::frame_frozen`), and whether it counts no
/// frames (`Session::counts_no_frames`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StellarLook {
    /// Whether it is destroyed.
    pub destroyed: bool,
    /// Which sheet it draws.
    pub drawn: DrawnSheet,
    /// Which sheet's frame count it animates over.
    pub counted: DrawnSheet,
    /// Whether its sprite slot shows a frame frozen.
    pub frozen: bool,
    /// Whether it animates over no frames until the next rebuild point.
    pub counts_none: bool,
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
            counted: DrawnSheet::Type,
            frozen: false,
            counts_none: false,
            type_zero_sprite: type_zero,
            frames: Frames::new(),
            gate_frame: None,
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

    /// The frame it shows, keyed, and that frame's width and height: at
    /// the frame its animation shows, or a hypergate's on the frame its
    /// session gave it, modulo its sheet's frames. None with no sheet to
    /// draw, or a slot showing no frame.
    #[must_use]
    pub fn shown_image(&self) -> Option<(ImageKey, (f32, f32))> {
        let sheet = self.drawn()?.as_ref().ok()?;
        let frame = match self.gate_frame {
            Some(frame) if !self.frozen => frame % sheet.frames.get(),
            _ => self.frames.shown(),
        };
        let size = (sheet.frame_width as f32, sheet.frame_height as f32);
        Some((sheet.image.frame(frame), size))
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

    /// One tick of its animation, over the counted sheet's frames (one
    /// when it counts none), shown on the sheet drawn; a frozen frame
    /// stays shown. A random next frame is drawn on `chance`.
    fn step(&mut self, chance: &mut dyn Chance) {
        let counted = if self.counts_none {
            NonZeroU16::MIN
        } else {
            self.frame_count(self.counted)
        };
        if self.frozen {
            self.frames
                .step_frozen(self.animation_data, self.destroyed, counted, chance);
        } else {
            let drawn = self.frame_count(self.drawn);
            self.frames
                .step(self.animation_data, self.destroyed, counted, drawn, chance);
        }
    }

    /// How it looks now.
    fn look(&self) -> StellarLook {
        StellarLook {
            destroyed: self.destroyed,
            drawn: self.drawn,
            counted: self.counted,
            frozen: self.frozen,
            counts_none: self.counts_none,
        }
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

    /// Marks each stellar destroyed or not, drawing the sheet and
    /// counting the frames, as `look` says ([`SceneStellar::drawn`]), as
    /// the original's sprite tick swaps the sheet: a stellar whose sheet
    /// drawn changed keeps the frame it shows while that is in range of
    /// the new sheet, and shows frame 0 otherwise ([`Frames::swap`]); its
    /// animation then reads the destroyed state at each tick.
    pub fn set_destroyed(&mut self, look: impl Fn(StellarId) -> StellarLook) {
        for stellar in &mut self.stellars {
            let now = look(stellar.id);
            stellar.destroyed = now.destroyed;
            stellar.counted = now.counted;
            stellar.frozen = now.frozen;
            stellar.counts_none = now.counts_none;
            if stellar.drawn != now.drawn {
                stellar.drawn = now.drawn;
                let drawn = stellar.frame_count(now.drawn);
                stellar.frames.swap(drawn);
            }
        }
    }

    /// A rebuild point: each stellar whose look `look` changed takes it,
    /// and shows its current frame on the sheet now drawn, or frame 0 on
    /// graphic type 0's ([`Frames::reshow`]), as the original's rebuild
    /// sets each slot's frame (`_HandleStellarSystemVisibility` @0x33096,
    /// @0x332d0). Every other stellar is left as it was.
    pub fn rebuild(&mut self, look: impl Fn(StellarId) -> StellarLook) {
        for stellar in &mut self.stellars {
            let now = look(stellar.id);
            if stellar.look() != now {
                stellar.destroyed = now.destroyed;
                stellar.drawn = now.drawn;
                stellar.counted = now.counted;
                stellar.frozen = now.frozen;
                stellar.counts_none = now.counts_none;
                let drawn = stellar.frame_count(now.drawn);
                stellar
                    .frames
                    .reshow(drawn, now.drawn == DrawnSheet::TypeZero);
            }
        }
    }

    /// Gives each stellar `kept` holds the animation state it was left
    /// in ([`KeptFrames`]), shown on the sheet now drawn: its current
    /// frame modulo that sheet's frame count, or frame 0 on graphic type
    /// 0's ([`Frames::reshow`]), as the original's rebuild sets the slot
    /// of a stellar shown again (`_HandleStellarSystemVisibility`
    /// @0x33096, @0x332d0). A stellar `kept` has never seen keeps the
    /// state it was laid out with. Called once the scene's look is set,
    /// so the sheet drawn is the session's.
    pub fn resume(&mut self, kept: &KeptFrames) {
        for stellar in &mut self.stellars {
            if let Some(frames) = kept.get(stellar.id) {
                stellar.frames = frames;
                let drawn = stellar.frame_count(stellar.drawn);
                stellar
                    .frames
                    .reshow(drawn, stellar.drawn == DrawnSheet::TypeZero);
            }
        }
    }

    /// The take-off's reset (`_HandlePlayerDockRequest` @0x675ce-0x675fa,
    /// play-feel-4 phase 14): every stellar's current and next frames to
    /// 0, its timer kept, and frame 0 shown, as the rebuild after it shows
    /// the current frame (@0x67653; [`Frames::take_off_reset`]).
    pub fn take_off_reset(&mut self) {
        for stellar in &mut self.stellars {
            stellar.frames.take_off_reset();
        }
    }

    /// Steps every stellar's animation `ticks` ticks of 1/30 s, each
    /// random next frame (`Flags2` 0x0002) drawn on `chance`: a visual-only
    /// source, never the session's, as nothing the session decides reads
    /// the frame a stellar shows.
    pub fn step(&mut self, ticks: u128, chance: &mut dyn Chance) {
        for _ in 0..ticks {
            for stellar in &mut self.stellars {
                stellar.step(chance);
            }
        }
    }

    /// Marks each stellar's frame frozen or not as `frozen` gives it
    /// (`Session::frame_frozen`): a flight frame's clone freezes or thaws
    /// a slot with no event to tell it, so the view reads it after each
    /// step, before the frame is drawn, and before the gate frames
    /// ([`SystemScene::set_gate_frames`]). A hypergate frozen keeps the
    /// frame its session gave it last, as the sprite tick sets a gate's
    /// frame through the `_SWSetCurrentFrameIndex` a slot whose frames
    /// count none refuses (@0x2ee20-0x2ee30).
    pub fn set_frozen(&mut self, frozen: impl Fn(StellarId) -> bool) {
        for stellar in &mut self.stellars {
            let now = frozen(stellar.id);
            if now
                && !stellar.frozen
                && let Some(frame) = stellar.gate_frame
            {
                let drawn = stellar.frame_count(stellar.drawn);
                stellar.frames.hold(frame % drawn.get());
            }
            stellar.frozen = now;
        }
    }

    /// Sets the frame each stellar draws as `frame` gives it: `Some` for a
    /// hypergate, drawn in place of its animation's.
    pub fn set_gate_frames(&mut self, frame: impl Fn(StellarId) -> Option<u16>) {
        for stellar in &mut self.stellars {
            stellar.gate_frame = frame(stellar.id);
        }
    }

    /// The frame stellar `id` shows and its size
    /// ([`SceneStellar::shown_image`]): what the planet comm dialog draws
    /// of it (`_PlanetCommDialogUpdate` @0x90cc6-0x90e26). None for a
    /// stellar not in it or showing nothing.
    #[must_use]
    pub fn shown_frame(&self, id: StellarId) -> Option<(ImageKey, (f32, f32))> {
        self.stellars
            .iter()
            .find(|stellar| stellar.id == id)?
            .shown_image()
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

/// Draws every stellar of `scene` where `camera` puts it, at the frame its
/// animation shows, or a hypergate on the frame its session gave it
/// ([`SystemScene::set_gate_frames`]), modulo its sheet's frames: each
/// sprite (or, without a sheet, a placeholder box and the reason, and with
/// a slot showing no frame, nothing), then every name under its stellar,
/// so no sprite covers a name.
///
/// Both passes go in ascending stellar ID, so of two overlapping stellars
/// the higher ID is on top. `_HandleStellarSystemVisibility` fills its
/// sprite slots in ID order (@0x32ec1-0x33229), `_SWAddSprite` appends each
/// new slot to the layer's tail (@0xac624) and `_SWAnimateSpriteWorld`
/// walks each layer from its head (@0xb9727). The scene itself stays in
/// navigation order.
pub fn draw_stellars(list: &mut DrawList, scene: &SystemScene, camera: &Camera) {
    let mut by_id: Vec<&SceneStellar> = scene.stellars().iter().collect();
    by_id.sort_by_key(|stellar| stellar.id);
    let mut labels = Vec::new();
    for stellar in by_id {
        let (at, (width, height)) = match (stellar.drawn(), stellar.shown_image()) {
            (None | Some(Ok(_)), None) => (camera.world_to_screen(stellar.position), (0.0, 0.0)),
            (_, Some((image, size))) => {
                let at = stellar.sprite_centre(camera, size);
                list.sprite(image, at, Color::WHITE);
                (at, size)
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
        }
    }

    const WORMHOLE: AnimationData = AnimationData {
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
                    counted: DrawnSheet::Type,
                    frozen: false,
                    counts_none: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    frames: Frames::new(),
                    gate_frame: None,
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
                    counted: DrawnSheet::Type,
                    frozen: false,
                    counts_none: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    frames: Frames::new(),
                    gate_frame: None,
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
                    counted: DrawnSheet::Type,
                    frozen: false,
                    counts_none: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    frames: Frames::new(),
                    gate_frame: None,
                },
            ]
        );
        assert_eq!(scene.problems(), ["spöb 140 Type: no spïn 1002"]);
        let mut scene = scene;
        scene.step(2, &mut NeverFires);
        let shown: Vec<u16> = scene.stellars().iter().map(|s| s.frames.shown()).collect();
        assert_eq!(shown, [1, 0, 0], "129 animates; one frame or no sheet stay");
    }

    /// Scripted draws, each `below(n)` recorded; panics once they run out.
    #[derive(Default)]
    struct Draws {
        draws: Vec<u32>,
        asked: Vec<u32>,
    }

    impl Chance for Draws {
        fn fires(&mut self, percent: u8) -> bool {
            panic!("asked whether {percent}% fires");
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            assert!(!self.draws.is_empty(), "an unscripted below({n})");
            self.draws.remove(0)
        }
    }

    #[test]
    fn a_random_stellar_steps_by_the_scenes_draws() {
        let random = StellarContents {
            animation: AnimationData {
                delay: 1,
                frame0_bias: 0,
                flags2: nova_sim::gate_sprite::RANDOM,
            },
            ..stellar(150, 0, 0, Ok(sheet(1150, 4)))
        };
        let plain = StellarContents {
            animation: AnimationData {
                delay: 1,
                ..WORMHOLE
            },
            ..stellar(151, 100, 0, Ok(sheet(1151, 4)))
        };
        let catalog = FakeCatalog::new(vec![system(134, vec![random, plain])]);
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        // From frame 1, 0, 2 and 3 are allowed; from 3, 0 to 2.
        let mut chance = Draws {
            draws: vec![2, 1],
            ..Draws::default()
        };
        let mut shown = Vec::new();
        for _ in 0..2 {
            scene.step(1, &mut chance);
            shown.push(
                scene
                    .stellars()
                    .iter()
                    .map(|s| s.frames.shown())
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(shown, [[1, 1], [3, 2]]);
        assert_eq!(chance.asked, [3, 3], "only the random stellar draws");
    }

    #[test]
    fn a_take_off_reset_shows_frame_0_on_every_stellar_and_restarts_them_from_next_0() {
        let every_tick = |id, x| StellarContents {
            animation: AnimationData {
                delay: 1,
                ..WORMHOLE
            },
            ..stellar(id, x, 0, Ok(sheet(1000 + id, 4)))
        };
        let catalog = FakeCatalog::new(vec![system(
            134,
            vec![every_tick(150, 0), every_tick(151, 100)],
        )]);
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        scene.step(2, &mut NeverFires);
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(1150, 2), ImageKey::sprite(1151, 2)]
        );
        scene.take_off_reset();
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(1150, 0), ImageKey::sprite(1151, 0)]
        );
        scene.step(2, &mut NeverFires);
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(1150, 1), ImageKey::sprite(1151, 1)],
            "frame 0 again from next 0, then 1"
        );
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
    const DEAD_ANIMATED: AnimationData = AnimationData {
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

    /// A whole stellar, drawing and counting its `Type` sheet.
    const WHOLE: StellarLook = StellarLook {
        destroyed: false,
        drawn: DrawnSheet::Type,
        counted: DrawnSheet::Type,
        frozen: false,
        counts_none: false,
    };

    /// A destroyed stellar drawing `drawn` and counting its frames.
    fn dead(drawn: DrawnSheet) -> StellarLook {
        StellarLook {
            destroyed: true,
            drawn,
            counted: drawn,
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
        draw_stellars(&mut list, scene, &Camera::centred_on(Point::default()));
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
        draw_stellars(&mut list, scene, &camera);
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
        draw_stellars(&mut list, scene, &camera);
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
        scene.set_gate_frames(|id| (id == StellarId(129)).then_some(2));
        assert_eq!(
            scene.shown_frame(StellarId(129)).map(|shown| shown.0),
            Some(ImageKey::sprite(1001, 2))
        );
        assert_eq!(scene.shown_frame(StellarId(999)), None, "not in it");
    }

    #[test]
    fn a_hypergate_draws_the_frame_its_session_gives_modulo_its_sheet() {
        let mut scene = SystemScene::load(&catalog(), SystemId(130));
        scene.step(3, &mut NeverFires);
        let before = drawn(&scene);
        assert_eq!(
            before[1],
            ImageKey::sprite(1001, 1),
            "its own animation (129 drawn second)"
        );
        let gate = |frame| move |id: StellarId| (id == StellarId(129)).then_some(frame);
        scene.set_gate_frames(gate(3));
        assert_eq!(scene.stellars()[0].gate_frame, Some(3));
        assert_eq!(scene.stellars()[1].gate_frame, None);
        let shown = drawn(&scene);
        assert_eq!(shown[1], ImageKey::sprite(1001, 3));
        assert_eq!(shown[0], before[0], "the rest as they were");
        scene.set_gate_frames(gate(0));
        assert_eq!(drawn(&scene)[1], ImageKey::sprite(1001, 0));
        // A frame past the sheet drawn wraps round its 4 frames.
        scene.set_gate_frames(gate(6));
        assert_eq!(drawn(&scene)[1], ImageKey::sprite(1001, 2));
        scene.set_gate_frames(|_| None);
        assert_eq!(drawn(&scene), before);
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
        scene.set_destroyed(only_150_dead);
        assert!(scene.stellars()[0].destroyed);
        assert_eq!(
            drawn(&scene)[0],
            ImageKey::sprite(2150, 0),
            "the DeadType sheet"
        );
        assert_eq!(drawn(&scene)[1..], whole[1..]);
        scene.set_destroyed(|_| WHOLE);
        assert!(!scene.stellars()[0].destroyed);
        assert_eq!(drawn(&scene), whole, "regenerated");
    }

    #[test]
    fn the_dead_type_sheet_drawn_with_no_different_graphic_is_the_type_sheet() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_destroyed(|_| dead(DrawnSheet::Dead));
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
        scene.set_destroyed(|_| dead(DrawnSheet::Type));
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
        scene.set_destroyed(only_150_dead);
        let dead = name_at(&scene, "Stellar 150");
        assert_eq!(dead, Point::new(at.x - 8.0, at.y + 8.0 + NAME_GAP));
    }

    #[test]
    fn the_animation_follows_the_destroyed_state_tick_by_tick() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.step(3, &mut NeverFires);
        assert_eq!(
            drawn(&scene)[0],
            ImageKey::sprite(1000, 0),
            "whole, so still"
        );
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(1003, 1));
        let before = scene.stellars().to_vec();
        scene.set_destroyed(only_150_dead);
        assert_eq!(scene.stellars()[1..], before[1..], "the others untouched");
        scene.step(3, &mut NeverFires);
        assert_eq!(
            drawn(&scene)[0],
            ImageKey::sprite(2150, 1),
            "three ticks in: frame 1 of the DeadType sheet"
        );
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(1003, 3));
        scene.set_destroyed(|id| match id.0 {
            150 => dead(DrawnSheet::Dead),
            153 => dead(DrawnSheet::Type),
            _ => WHOLE,
        });
        scene.step(1, &mut NeverFires);
        assert_eq!(drawn(&scene)[0], ImageKey::sprite(2150, 2));
        assert_eq!(
            drawn(&scene)[3],
            ImageKey::sprite(1003, 0),
            "a normal stellar stops while destroyed"
        );
    }

    #[test]
    fn a_swap_keeps_the_frame_shown_while_it_is_in_range() {
        let drawing = |sheet| {
            move |_| StellarLook {
                drawn: sheet,
                counted: sheet,
                ..WHOLE
            }
        };
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.step(2, &mut NeverFires);
        scene.set_destroyed(drawing(DrawnSheet::TypeZero));
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(TYPE_ZERO, 1), "kept");
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.step(6, &mut NeverFires);
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(1003, 3));
        scene.set_destroyed(drawing(DrawnSheet::TypeZero));
        assert_eq!(
            drawn(&scene)[3],
            ImageKey::sprite(TYPE_ZERO, 0),
            "frame 3 of 2"
        );
        scene.set_destroyed(drawing(DrawnSheet::TypeZero));
        scene.step(1, &mut NeverFires);
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(TYPE_ZERO, 0), "no swap");
    }

    /// A look: destroyed or not, drawing `drawn` and counting `counted`.
    fn look(destroyed: bool, drawn: DrawnSheet, counted: DrawnSheet) -> StellarLook {
        StellarLook {
            destroyed,
            drawn,
            counted,
            ..WHOLE
        }
    }

    #[test]
    fn a_stellar_animates_over_the_frames_of_its_counted_sheet() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        // 150 animates only when destroyed, over its 3-frame `DeadType`
        // sheet; its `Type` sheet has 1 frame.
        let dead = |counted| {
            move |id: StellarId| {
                if id == StellarId(150) {
                    look(true, DrawnSheet::Dead, counted)
                } else {
                    WHOLE
                }
            }
        };
        scene.set_destroyed(dead(DrawnSheet::Type));
        scene.step(8, &mut NeverFires);
        assert_eq!(drawn(&scene)[0], ImageKey::sprite(2150, 0), "one counted");
        scene.set_destroyed(dead(DrawnSheet::Dead));
        scene.step(2, &mut NeverFires);
        assert_eq!(drawn(&scene)[0], ImageKey::sprite(2150, 1), "three");
        scene.step(4, &mut NeverFires);
        assert_eq!(drawn(&scene)[0], ImageKey::sprite(2150, 0), "wrapped");
    }

    /// System 135: stellar 160 at (0, 0), a 4-frame wormhole whose
    /// `DeadType` sheet 2160 has 2 frames.
    fn wormhole_with_a_dead_sheet() -> FakeCatalog {
        FakeCatalog::new(vec![system(
            135,
            vec![StellarContents {
                dead_sprite: Some(Ok(sheet(2160, 2))),
                ..stellar(160, 0, 0, Ok(sheet(1160, 4)))
            }],
        )])
    }

    #[test]
    fn a_rebuild_shows_the_current_frame_of_each_stellar_whose_look_changed() {
        let mut scene = SystemScene::load(&wormhole_with_a_dead_sheet(), SystemId(135));
        scene.step(6, &mut NeverFires);
        assert_eq!(drawn(&scene), [ImageKey::sprite(1160, 3)]);
        let counting = |counted| move |_| look(false, DrawnSheet::Dead, counted);
        scene.set_destroyed(counting(DrawnSheet::Type));
        assert_eq!(drawn(&scene), [ImageKey::sprite(2160, 0)], "3 of 2");
        scene.rebuild(counting(DrawnSheet::Type));
        assert_eq!(drawn(&scene), [ImageKey::sprite(2160, 0)], "unchanged");
        scene.rebuild(counting(DrawnSheet::Dead));
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(2160, 1)],
            "frame 3 of 2 at the rebuild"
        );
        assert_eq!(scene.stellars()[0].counted, DrawnSheet::Dead);
        scene.rebuild(|_| look(true, DrawnSheet::TypeZero, DrawnSheet::TypeZero));
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(TYPE_ZERO, 0)],
            "graphic type 0 fills the slot at frame 0"
        );
        assert!(scene.stellars()[0].destroyed);
    }

    #[test]
    fn a_rebuild_swapping_the_sheet_shows_the_current_frame_not_the_one_kept() {
        let mut scene = SystemScene::load(&wormhole_with_a_dead_sheet(), SystemId(135));
        scene.step(6, &mut NeverFires);
        scene.rebuild(|_| look(false, DrawnSheet::Dead, DrawnSheet::Type));
        assert_eq!(drawn(&scene), [ImageKey::sprite(2160, 1)]);
    }

    /// Stellar 160's animation state 6 ticks in (current frame 3 of its
    /// `Type` sheet's 4), kept.
    fn kept_on_frame_3() -> KeptFrames {
        let mut left = SystemScene::load(&wormhole_with_a_dead_sheet(), SystemId(135));
        left.step(6, &mut NeverFires);
        assert_eq!(drawn(&left), [ImageKey::sprite(1160, 3)]);
        let mut kept = KeptFrames::new();
        kept.keep(&left);
        kept
    }

    #[test]
    fn resume_shows_the_current_frame_on_the_sheet_now_drawn() {
        let kept = kept_on_frame_3();
        let mut scene = SystemScene::load(&wormhole_with_a_dead_sheet(), SystemId(135));
        scene.set_destroyed(|_| look(false, DrawnSheet::Dead, DrawnSheet::Type));
        scene.resume(&kept);
        assert_eq!(drawn(&scene), [ImageKey::sprite(2160, 1)], "3 of 2");
        let mut scene = SystemScene::load(&wormhole_with_a_dead_sheet(), SystemId(135));
        scene.resume(&kept);
        assert_eq!(drawn(&scene), [ImageKey::sprite(1160, 3)], "3 of 4");
    }

    #[test]
    fn resume_on_graphic_type_0s_sheet_shows_frame_0() {
        let kept = kept_on_frame_3();
        let mut scene = SystemScene::load(&wormhole_with_a_dead_sheet(), SystemId(135));
        scene.set_destroyed(|_| look(true, DrawnSheet::TypeZero, DrawnSheet::Type));
        scene.resume(&kept);
        assert_eq!(drawn(&scene), [ImageKey::sprite(TYPE_ZERO, 0)]);
        assert_eq!(
            scene.stellars()[0].frames,
            {
                let mut frames = kept.get(StellarId(160)).expect("kept");
                frames.reshow(NonZeroU16::MIN, true);
                frames
            },
            "its frames kept"
        );
    }

    #[test]
    fn resume_on_a_slot_showing_no_frame_keeps_the_frames_and_draws_nothing() {
        let kept = kept_on_frame_3();
        let mut scene = SystemScene::load(&wormhole_with_a_dead_sheet(), SystemId(135));
        scene.set_destroyed(|_| look(false, DrawnSheet::NoFrame, DrawnSheet::Type));
        scene.resume(&kept);
        assert_eq!(drawn(&scene), []);
        scene.rebuild(|_| WHOLE);
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(1160, 3)],
            "the current frame kept through it"
        );
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
        scene.set_destroyed(|_| dead(DrawnSheet::TypeZero));
        assert_eq!(
            drawn(&scene),
            [ImageKey::sprite(TYPE_ZERO, 0); 4],
            "whatever other sheet it has"
        );
        let sheets: Vec<DrawnSheet> = scene.stellars().iter().map(|s| s.drawn).collect();
        assert_eq!(sheets, [DrawnSheet::TypeZero; 4]);
        scene.set_destroyed(|_| StellarLook {
            drawn: DrawnSheet::TypeZero,
            counted: DrawnSheet::TypeZero,
            ..WHOLE
        });
        scene.step(4, &mut NeverFires);
        assert_eq!(
            drawn(&scene)[3],
            ImageKey::sprite(TYPE_ZERO, 0),
            "over type 0's 2 frames, not Type's 4"
        );
        scene.step(2, &mut NeverFires);
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(TYPE_ZERO, 1));
    }

    #[test]
    fn the_name_sits_under_the_type_zero_sheet_drawn() {
        let mut catalog = dying();
        catalog.systems[0].type_zero_sprite = Ok(StellarSheet {
            frame_width: 40,
            frame_height: 40,
            ..sheet(TYPE_ZERO, 2)
        });
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        let at = Camera::centred_on(Point::default()).world_to_screen(Point::new(100.0, 0.0));
        scene.set_destroyed(|_| dead(DrawnSheet::TypeZero));
        assert_eq!(
            name_at(&scene, "Stellar 151"),
            Point::new(at.x - 20.0, at.y + 20.0 + NAME_GAP)
        );
    }

    // A slot showing no frame (stellar-fidelity phase 8).

    #[test]
    fn a_stellar_drawing_no_frame_draws_no_sprite_only_its_name_under_its_centre() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_destroyed(|id| {
            if id == StellarId(151) {
                StellarLook {
                    drawn: DrawnSheet::NoFrame,
                    counted: DrawnSheet::Type,
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
        draw_stellars(&mut list, &scene, &Camera::centred_on(Point::default()));
        assert!(
            !list
                .iter()
                .any(|command| matches!(command, crate::DrawCommand::Text { text, .. } if text.starts_with("Sprite unavailable"))),
            "no placeholder either"
        );
    }

    #[test]
    fn a_stellar_drawing_no_frame_counts_one_and_a_rebuild_to_it_shows_frame_0() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.step(6, &mut NeverFires);
        scene.rebuild(|_| StellarLook {
            drawn: DrawnSheet::NoFrame,
            counted: DrawnSheet::NoFrame,
            ..WHOLE
        });
        assert_eq!(scene.stellars()[3].frames.shown(), 0);
        scene.step(6, &mut NeverFires);
        assert_eq!(scene.stellars()[3].frames.shown(), 0, "one frame counted");
    }

    /// Where the sprite of `image` is centred.
    fn sprite_at(scene: &SystemScene, image: ImageKey) -> Point {
        let mut list = DrawList::new();
        draw_stellars(&mut list, scene, &Camera::centred_on(Point::default()));
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
    fn a_frozen_stellar_draws_its_frame_kept_where_a_32_pixel_slot_puts_it() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.step(3, &mut NeverFires);
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(1003, 1));
        let frozen_153 = |id: StellarId| StellarLook {
            frozen: id == StellarId(153),
            ..WHOLE
        };
        scene.set_destroyed(frozen_153);
        assert!(scene.stellars()[3].frozen);
        scene.step(6, &mut NeverFires);
        assert_eq!(drawn(&scene)[3], ImageKey::sprite(1003, 1), "kept");
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
        // Thawed, it steps on from the frame its count reached.
        scene.set_destroyed(|_| WHOLE);
        scene.step(2, &mut NeverFires);
        assert_eq!(
            drawn(&scene)[3],
            ImageKey::sprite(1003, 1),
            "its count stepped on while frozen"
        );
        assert_eq!(
            sprite_at(&scene, ImageKey::sprite(1003, 1)),
            at,
            "centred again"
        );
    }

    #[test]
    fn a_frozen_hypergate_keeps_the_frame_it_showed_whatever_its_session_steps_it_to() {
        let mut scene = SystemScene::load(&catalog(), SystemId(130));
        let gate = |frame| move |id: StellarId| (id == StellarId(129)).then_some(frame);
        let only_129 = |id: StellarId| id == StellarId(129);
        scene.set_gate_frames(gate(3));
        scene.set_frozen(only_129);
        scene.set_gate_frames(gate(1));
        scene.step(4, &mut NeverFires);
        // 129 is drawn second, after 128.
        assert_eq!(drawn(&scene)[1], ImageKey::sprite(1001, 3), "kept");
        scene.set_frozen(|_| false);
        assert_eq!(drawn(&scene)[1], ImageKey::sprite(1001, 1), "thawed");
    }

    #[test]
    fn a_stellar_counting_no_frames_never_animates() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_destroyed(|_| StellarLook {
            counts_none: true,
            ..WHOLE
        });
        assert!(scene.stellars()[3].counts_none);
        for _ in 0..8 {
            scene.step(1, &mut NeverFires);
            assert_eq!(drawn(&scene)[3], ImageKey::sprite(1003, 0));
        }
        scene.set_destroyed(|_| WHOLE);
        scene.step(3, &mut NeverFires);
        assert_eq!(
            drawn(&scene)[3],
            ImageKey::sprite(1003, 1),
            "it counts again"
        );
    }

    #[test]
    fn a_rebuild_takes_a_change_in_the_frozen_or_counting_state() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.rebuild(|_| StellarLook {
            frozen: true,
            counts_none: true,
            ..WHOLE
        });
        assert!(scene.stellars().iter().all(|s| s.frozen && s.counts_none));
    }

    /// A camera on a fractional point, as flight's follows the ship.
    fn off_pixel() -> Camera {
        Camera::centred_on(Point::new(0.4, -0.6))
    }

    /// What `draw_stellars` draws of `scene` with `camera`.
    fn list_with(scene: &SystemScene, camera: &Camera) -> DrawList {
        let mut list = DrawList::new();
        draw_stellars(&mut list, scene, camera);
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
        scene.set_destroyed(|id: StellarId| StellarLook {
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
        scene.set_destroyed(|_| dead(DrawnSheet::TypeZero));
        assert_eq!(
            scene.stellars()[1].drawn(),
            Some(&Err("no spïn 1000".to_owned()))
        );
    }
}
