//! The [`Renderer`](crate::Renderer)'s planets: what each [`PlanetKey`]
//! draws ([`Renderer::add_planet`](crate::Renderer::add_planet)), which of
//! them the GPU holds, and which keys with no planet have been reported;
//! and the [`Batch::Planets`] a draw list's planets become.
//!
//! Each planet is numbered once, the first time its key is added, and its
//! textures ([`crate::planet::textures`]) uploaded
//! ([`Gpu::upload_planet`]) the first frame that draws it. Adding a key
//! again replaces its planet under the same number, uploaded again at its
//! next draw. Planets are not atlas pages, so a new enlarger leaves them
//! where they are.
//!
//! # A planet is made ready at its first draw
//!
//! A planet may be added unbaked ([`LoadedPlanet::unbaked`]) or unread
//! ([`LoadedPlanet::unread`]): the new game adds every planet of its
//! galaxy that way at start-up, since a bake takes tens of milliseconds
//! and a texture's decode several. Adding costs nothing. The first frame
//! whose draw list commands the planet with a pose that draws
//! ([`planet::quad`]) makes it ([`LoadedPlanet::make`]: bakes its look,
//! or reads its texture through the renderer's [`PlanetFiles`]), builds
//! its mips and uploads it, all before that frame is submitted, so the
//! frame draws it: nothing is ever drawn as nothing while it waits. A
//! planet never commanded is never made.
//!
//! Once uploaded, a planet's pixels are let go: the renderer keeps its
//! [`Figure`] (all its instance is made from) and how to make it again (a
//! look, or a texture's file), so it holds no planet pixels on the CPU.
//! A planet given as a surface ([`LoadedPlanet::surface`], tests) is
//! spent once uploaded, and has nothing to be made again from.
//!
//! # A planet is let go when its system is left
//!
//! Each planet notes the visit it was last drawn in
//! ([`crate::visits`]: a visit begins with each frame that flies in
//! another system). After the frame that begins a visit is submitted, the
//! renderer releases ([`Gpu::release_planet`]) the textures of every
//! planet not drawn in this visit or the one before, that can be made
//! again, and it counts as not ready: the first frame that draws it again
//! makes and uploads it again, as a first draw does, under the same
//! number. So the GPU holds the planets of the last two flight systems
//! (and those another screen drew meanwhile), not every planet ever
//! drawn. A planet given as a surface is never released.
//!
//! # A planet that can't be made
//!
//! A bake can't fail, but a read can (a file that doesn't decode, or one
//! changed since start-up checked it). The planet is reported in that
//! frame ([`crate::RenderReport::failed_planets`]), and from then on
//! draws, in that frame too, the stand-in it carries
//! ([`LoadedPlanet::with_stand_in`]: its stellar's disc), drawn by the
//! renderer in its place with its clip, so no screen has to know. It isn't
//! made again until its key is given a new planet; one with no stand-in
//! draws nothing.
//!
//! So what a screen commands decides when the pause falls. The flight
//! commands every stellar of its system each frame, on screen or off
//! (`nova_view`'s `stellar_models`), so a system's planets are all made
//! ready in the first frame drawn there, which after a jump is drawn
//! under the arrival's white. A screen that culled its planets would move
//! each bake to the frame the planet scrolls into view.
//!
//! Consecutive planets share one [`Batch::Planets`], lit by
//! [`PLANET_LIGHT`], while they are clipped alike
//! ([`nova_view::DrawCommand::Clipped`]): the batch carries their clip
//! ([`Batch::clip`]), which the adapter scissors it to, so a planet with
//! another clip, or none, opens a batch of its own. Any other batch
//! between them splits it too, so the list's order holds: planets test no
//! depth and write none, and a models batch after them clears its own. A
//! planet clipped to no area draws nothing and is not made ready; one
//! inside a clip with area is made ready even when it lies wholly outside
//! the clip.

use std::collections::{HashMap, HashSet};

use nova_view::outline::StellarStandIn;
use nova_view::{PlanetKey, PlanetPose};

use crate::gpu::{Batch, Gpu, PlanetId, PlanetInstance, Rect};
use crate::planet::{self, Figure, PLANET_LIGHT};
use crate::planet_files::{NoFiles, PlanetFiles};
use crate::planet_source::{LoadedPlanet, PlanetError};
use crate::visits::Visits;

/// A planet the renderer was given: how to make it, and what of it is
/// ready. It holds no pixels once they are uploaded.
#[derive(Debug)]
struct Held {
    id: PlanetId,
    /// The planet as it was given, to make it (again) from: a look, a
    /// texture's file, or a surface; `None` once a given surface has been
    /// uploaded, which has nothing to be made again from.
    planet: Option<LoadedPlanet>,
    /// What it draws instead if it can't be made.
    stand_in: Option<StellarStandIn>,
    /// How far it reaches, in its radii, known before it is made.
    reach: f32,
    /// Its figure, once it is made and its textures uploaded as it is:
    /// all that drawing it needs from then on.
    figure: Option<Figure>,
    /// Whether making it failed: it draws its stand-in, and isn't made
    /// again until its key is given a new planet.
    failed: bool,
    /// Whether the GPU holds textures under its id: its own, or those of
    /// the planet it replaced.
    on_gpu: bool,
    /// The visit it was last drawn in ([`Visits`]).
    drawn_in: u64,
}

/// The planets the renderer was given, by key.
#[derive(Debug)]
pub(crate) struct Planets {
    held: HashMap<PlanetKey, Held>,
    /// The number the next new key's planet takes.
    next: u32,
    /// The keys with no planet already reported.
    reported: HashSet<PlanetKey>,
    /// What reads a texture left unread.
    files: Box<dyn PlanetFiles>,
}

impl Default for Planets {
    fn default() -> Self {
        Planets {
            held: HashMap::new(),
            next: 0,
            reported: HashSet::new(),
            files: Box::new(NoFiles),
        }
    }
}

impl Planets {
    /// Gives `key` `planet`, replacing any planet it had: the replacement
    /// keeps the old one's number and is made and uploaded at its next
    /// draw.
    pub(crate) fn add(&mut self, key: PlanetKey, planet: LoadedPlanet) {
        let stand_in = planet.stand_in().cloned();
        let reach = planet.reach();
        if let Some(held) = self.held.get_mut(&key) {
            *held = Held {
                planet: Some(planet),
                stand_in,
                reach,
                figure: None,
                failed: false,
                ..*held
            };
            return;
        }
        let id = PlanetId(self.next);
        self.next += 1;
        self.held.insert(
            key,
            Held {
                id,
                planet: Some(planet),
                stand_in,
                reach,
                figure: None,
                failed: false,
                on_gpu: false,
                drawn_in: 0,
            },
        );
    }

    /// Reads the textures of planets left unread through `files` from now
    /// on.
    pub(crate) fn set_files(&mut self, files: Box<dyn PlanetFiles>) {
        self.files = files;
    }

    /// Draws planet `key` at `pose`, only inside `clip` when it has one,
    /// onto `batches`: its instance, the planet made first if it isn't
    /// ready (its look baked, or its texture read) and its textures
    /// uploaded through `gpu`, its pixels then let go. A key with no
    /// planet is pushed onto `missing` once, and draws nothing. A planet
    /// that can't be made is pushed onto `failed` with why, once, and
    /// from then on gives back its stand-in for the caller to draw in its
    /// place (`None` when it has none). A pose [`planet::quad`] refuses
    /// draws nothing, and makes nothing. A planet wholly outside its clip
    /// is still made ready: the GPU's scissor, not the core, cuts it. A
    /// planet drawn is noted as drawn in `visit`, the current one.
    pub(crate) fn draw(
        &mut self,
        key: PlanetKey,
        (pose, clip): (&PlanetPose, Option<Rect>),
        batches: &mut Vec<Batch>,
        gpu: &mut impl Gpu,
        (missing, failed): (&mut Vec<PlanetKey>, &mut Vec<(PlanetKey, PlanetError)>),
        visit: u64,
    ) -> Option<StellarStandIn> {
        let Some(held) = self.held.get_mut(&key) else {
            if self.reported.insert(key) {
                missing.push(key);
            }
            return None;
        };
        // A pose that draws nothing is refused before the work it would
        // waste: the reach needs none.
        planet::quad(pose, held.reach)?;
        if held.failed {
            return held.stand_in.clone();
        }
        held.drawn_in = visit;
        if held.figure.is_none() {
            let made = (held.planet.as_ref()).map(|planet| planet.make(self.files.as_ref()));
            match made {
                Some(Ok(surface)) => {
                    gpu.upload_planet(held.id, &planet::textures(&surface));
                    held.on_gpu = true;
                    held.figure = Some(Figure::of(&surface));
                    // A given surface is spent: holding it would hold
                    // its pixels.
                    if held.planet.as_ref().and_then(LoadedPlanet::baked).is_some() {
                        held.planet = None;
                    }
                }
                Some(Err(error)) => {
                    held.failed = true;
                    failed.push((key, error));
                    return held.stand_in.clone();
                }
                None => return None,
            }
        }
        let figure = held.figure.as_ref()?;
        push(
            batches,
            planet::figure_instance(held.id, figure, pose)?,
            clip,
        );
        None
    }

    /// Releases through `gpu` the textures of every planet `visits` no
    /// longer keeps ([`Visits::kept`]): held on the GPU, able to be made
    /// again, and not drawn in this visit or the one before. Each counts
    /// as not ready from then on, so its next draw makes it again. A
    /// planet given as a surface can't be made again, and one that failed
    /// holds nothing, so neither is released. Called once a frame that
    /// began a new visit is submitted, so no frame names a planet
    /// released; the releases go in the order of the planets' ids.
    pub(crate) fn release_left(&mut self, visits: &Visits, gpu: &mut impl Gpu) {
        let mut left: Vec<&mut Held> = (self.held.values_mut())
            .filter(|held| held.on_gpu && held.planet.is_some() && !visits.kept(held.drawn_in))
            .collect();
        left.sort_by_key(|held| held.id);
        for held in left {
            gpu.release_planet(held.id);
            held.on_gpu = false;
            held.figure = None;
        }
    }

    /// The bytes of planet pixels held: those of each planet given as a
    /// surface and not uploaded yet. A look or a texture holds none until
    /// it is made, and none once it is uploaded.
    pub(crate) fn bytes_held(&self) -> usize {
        (self.held.values())
            .filter_map(|held| held.planet.as_ref()?.baked())
            .map(|surface| {
                surface.surface.rgba.len()
                    + surface.clouds.as_ref().map_or(0, |c| c.rgba.len())
                    + surface.rings.as_ref().map_or(0, |r| r.profile.len())
            })
            .sum()
    }
}

/// Appends `planet`, drawn only inside `clip` when it has one, extending
/// the last batch when it is a planets batch clipped alike.
fn push(batches: &mut Vec<Batch>, planet: PlanetInstance, clip: Option<Rect>) {
    match batches.last_mut() {
        Some(Batch::Planets {
            planets,
            clip: last,
            ..
        }) if *last == clip => planets.push(planet),
        _ => batches.push(Batch::Planets {
            light: PLANET_LIGHT,
            planets: vec![planet],
            clip,
        }),
    }
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::Image;
    use nova_galaxy_format::model::Look;
    use nova_galaxy_media::planet::{Image as Layer, Sizes, Surface, bake};
    use nova_view::geometry::Bounds;
    use nova_view::{
        Color, DrawCommand, DrawList, ImageKey, ImageKind, ModelKey, ModelPose, Mounting,
        Orientation, Point,
    };

    use std::rc::Rc;

    use nova_view::outline::StellarStandIn;

    use super::*;
    use crate::Renderer;
    use crate::gpu::{Frame, PlanetTextures};
    use crate::images::{ImageError, ImageSource};
    use crate::model::Mesh;
    use crate::planet_files::{FakePlanetFiles, TextureFile};
    use crate::planet_source::PlanetError;
    use crate::recording::{GpuCall, RecordingGpu};
    use crate::viewport::{LogicalSize, Viewport};

    /// One white 4x4 sprite, `rlëD` 1.
    struct OneSprite;

    impl ImageSource for OneSprite {
        fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
            match (kind, id) {
                (ImageKind::Rled, 1) => Ok(vec![
                    Image::from_rgba(4, 4, [255; 64].to_vec()).expect("4x4"),
                ]),
                _ => Err(ImageError::Missing),
            }
        }
    }

    const LOGICAL: LogicalSize = LogicalSize { w: 64, h: 48 };
    const GLOBE: PlanetKey = PlanetKey(1);
    const CUBE: ModelKey = ModelKey(1);

    fn viewport() -> Viewport {
        Viewport::new(LOGICAL, (128, 96), 2.0)
    }

    /// A bare 4 by 2 surface in grey `level`.
    fn surface(level: u8) -> Surface {
        Surface {
            surface: Layer {
                width: 4,
                height: 2,
                rgba: vec![level; 32],
            },
            clouds: None,
            rings: None,
            atmosphere: None,
            day: 100.0,
            tilt: 0.0,
        }
    }

    /// A planet that draws with [`surface`] `level`.
    fn planet(level: u8) -> LoadedPlanet {
        LoadedPlanet::surface(surface(level))
    }

    /// A renderer whose key [`GLOBE`] is [`planet`] 9 and [`CUBE`] the cube.
    fn renderer() -> Renderer<OneSprite> {
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_planet(GLOBE, planet(9));
        renderer.add_model(CUBE, Mesh::cube([Color::WHITE; 6]));
        renderer
    }

    fn pose(x: f32, y: f32) -> PlanetPose {
        PlanetPose {
            center: Point::new(x, y),
            radius: 5.0,
            time: 25.0,
        }
    }

    fn model_pose(x: f32, y: f32) -> ModelPose {
        ModelPose {
            center: Point::new(x, y),
            orientation: Orientation::IDENTITY,
            scale: 4.0,
        }
    }

    /// The instance the core makes for [`planet`] 9, as planet 0, at `pose`.
    fn instance(pose: &PlanetPose) -> PlanetInstance {
        planet::instance(PlanetId(0), &surface(9), pose).expect("drawable")
    }

    fn frame_of(
        renderer: &mut Renderer<OneSprite>,
        list: &DrawList,
        gpu: &mut RecordingGpu,
    ) -> Frame {
        renderer.render(list, &viewport(), gpu);
        gpu.submits()
            .last()
            .map(|frame| (*frame).clone())
            .expect("a frame")
    }

    fn uploads(gpu: &RecordingGpu) -> Vec<(PlanetId, PlanetTextures)> {
        gpu.calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::UploadPlanet { planet, textures } => Some((*planet, textures.clone())),
                _ => None,
            })
            .collect()
    }

    fn kinds(frame: &Frame) -> Vec<&'static str> {
        frame.batches.iter().map(Batch::kind).collect()
    }

    #[test]
    fn a_planet_uploads_its_textures_once_then_draws_an_instance_each_frame() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(20.0, 20.0));
        let first = frame_of(&mut renderer, &list, &mut gpu);
        let second = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(0), planet::textures(&surface(9)))],
            "once, before the frame that draws it"
        );
        assert!(matches!(gpu.calls[0], GpuCall::UploadPlanet { .. }));
        let expected = vec![Batch::Planets {
            light: PLANET_LIGHT,
            planets: vec![instance(&pose(20.0, 20.0))],
            clip: None,
        }];
        assert_eq!(first.batches, expected);
        assert_eq!(second.batches, expected);
    }

    /// Looks baked small, so the tests stay fast.
    const SMALL: Sizes = Sizes {
        surface: 32,
        clouds: 16,
        ring: 16,
    };

    /// A generated look of seed `seed`, with rings and an atmosphere.
    fn look(seed: u32) -> Look {
        use nova_galaxy_format::model::{Stellar, Visual};
        let text = format!(
            "---\nname: Verdance\nkind: planet\nsystem: alpha\nposition: [0, 0]\nvisual:\n\
             \x20 generated:\n    type: garden\n    palette: [\"#3a7a3a\", \"#2a4a8a\"]\n\
             \x20   clouds: 0.5\n    rings: {{inner: 1.4, outer: 2.3, color: \"#d8c8a8\"}}\n\
             \x20   atmosphere: {{color: \"#f0e0c0\", thickness: 0.25}}\n    seed: {seed}\n---\n"
        );
        let stellar = nova_galaxy_format::frontmatter::parse::<Stellar>(&text).expect("a stellar");
        match stellar.0.visual {
            Some(Visual::Generated(look)) => look,
            other => panic!("a generated look: {other:?}"),
        }
    }

    /// The look of seed `seed`, not baked.
    fn unbaked(seed: u32) -> LoadedPlanet {
        LoadedPlanet::unbaked(look(seed), SMALL)
    }

    /// Whether the renderer holds the planet `key` ready to draw: made and
    /// uploaded.
    fn is_baked(renderer: &Renderer<OneSprite>, key: PlanetKey) -> bool {
        renderer.planets().held[&key].figure.is_some()
    }

    #[test]
    fn an_unbaked_planet_added_and_never_drawn_is_not_baked_and_uploads_nothing() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(20.0, 20.0));
        frame_of(&mut renderer, &list, &mut gpu);
        let ids: Vec<PlanetId> = uploads(&gpu).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, [PlanetId(0)], "only the globe drawn");
        assert!(!is_baked(&renderer, PlanetKey(2)));
    }

    #[test]
    fn the_first_frame_that_draws_an_unbaked_planet_bakes_and_uploads_it_once() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        let first = frame_of(&mut renderer, &list, &mut gpu);
        assert!(is_baked(&renderer, PlanetKey(2)));
        let baked = bake(&look(77), SMALL);
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(1), planet::textures(&baked))],
            "its bake's textures, before the frame that draws it"
        );
        assert!(matches!(gpu.calls[0], GpuCall::UploadPlanet { .. }));
        // Its tilt, turns, rings and atmosphere are its bake's.
        let instance = planet::instance(PlanetId(1), &baked, &pose(20.0, 20.0)).expect("drawable");
        assert!(instance.rings[1] > 2.0 && instance.atmosphere[3] > 0.0);
        let expected = vec![Batch::Planets {
            light: PLANET_LIGHT,
            planets: vec![instance],
            clip: None,
        }];
        assert_eq!(first.batches, expected);
        let second = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(second.batches, expected);
        assert_eq!(uploads(&gpu).len(), 1, "the second frame uploads nothing");
    }

    /// The bytes of `surface`'s pixels: its surface, clouds and ring
    /// profile.
    fn pixel_bytes(surface: &Surface) -> usize {
        surface.surface.rgba.len()
            + surface.clouds.as_ref().map_or(0, |c| c.rgba.len())
            + surface.rings.as_ref().map_or(0, |r| r.profile.len())
    }

    #[test]
    fn the_renderer_counts_the_planet_pixels_it_holds() {
        let mut renderer = Renderer::new(OneSprite);
        assert_eq!(renderer.planet_bytes_held(), 0);
        renderer.add_planet(GLOBE, planet(9));
        assert_eq!(renderer.planet_bytes_held(), pixel_bytes(&surface(9)));
        renderer.add_planet(PlanetKey(2), unbaked(77));
        assert_eq!(
            renderer.planet_bytes_held(),
            pixel_bytes(&surface(9)),
            "an unbaked planet holds no pixels"
        );
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        frame_of(&mut renderer, &list, &mut RecordingGpu::new());
        assert_eq!(
            renderer.planet_bytes_held(),
            pixel_bytes(&surface(9)),
            "a baked one holds none of its bake's once uploaded"
        );
        list.planet(GLOBE, pose(40.0, 20.0));
        frame_of(&mut renderer, &list, &mut RecordingGpu::new());
        assert_eq!(
            renderer.planet_bytes_held(),
            0,
            "nor one given as a surface"
        );
    }

    /// A stand-in disc.
    const DISC: Color = Color::rgba(30, 60, 90, 255);

    /// The file [`unread`] names.
    const FILE: &str = "media/export/pack/moon.png";

    /// A texture's 4 by 2 surface, its texels `level`.
    fn textured(level: u8) -> Surface {
        Surface::from_texture(nova_galaxy_media::texture::Texture {
            file: FILE.into(),
            width: 4,
            height: 2,
            disk_bytes: 0,
            rgba: vec![level; 32],
        })
    }

    /// The texture at [`FILE`], not read yet.
    fn unread() -> LoadedPlanet {
        LoadedPlanet::unread(TextureFile {
            stellar: "pale".into(),
            asset: "moon-surface".into(),
            file: FILE.into(),
            limits: nova_galaxy_media::budget::Limits::default(),
        })
    }

    /// Files holding [`FILE`] as [`textured`] 7, shared so a test can
    /// count its reads.
    fn files() -> Rc<FakePlanetFiles> {
        Rc::new(FakePlanetFiles::default().with(FILE, textured(7)))
    }

    /// A renderer as [`renderer`] reading textures from `files`.
    fn reading(files: &Rc<FakePlanetFiles>) -> Renderer<OneSprite> {
        let mut renderer = renderer();
        renderer.set_planet_files(Box::new(Rc::clone(files)));
        renderer
    }

    /// The index of the first `Submit` among `gpu`'s calls.
    fn first_submit(gpu: &RecordingGpu) -> usize {
        (gpu.calls.iter())
            .position(|call| matches!(call, GpuCall::Submit(_)))
            .expect("a frame")
    }

    #[test]
    fn after_its_upload_the_renderer_holds_none_of_a_planet_s_pixels() {
        for (given, made) in [
            (unbaked(77), bake(&look(77), SMALL)),
            (unread(), textured(7)),
            (planet(30), surface(30)),
        ] {
            let files = files();
            let mut renderer = reading(&files);
            renderer.add_planet(PlanetKey(2), given);
            let mut gpu = RecordingGpu::new();
            let mut list = DrawList::new();
            list.planet(PlanetKey(2), pose(20.0, 20.0));
            let frame = frame_of(&mut renderer, &list, &mut gpu);
            assert_eq!(
                renderer.planets().bytes_held(),
                pixel_bytes(&surface(9)),
                "the globe's"
            );
            assert_eq!(uploads(&gpu), [(PlanetId(1), planet::textures(&made))]);
            assert!(matches!(gpu.calls[0], GpuCall::UploadPlanet { .. }));
            assert_eq!(first_submit(&gpu), 1, "uploaded before the frame");
            assert_eq!(
                frame.batches,
                [Batch::Planets {
                    light: PLANET_LIGHT,
                    planets: vec![planet::instance(PlanetId(1), &made, &pose(20.0, 20.0)).unwrap()],
                    clip: None,
                }]
            );
        }
    }

    #[test]
    fn an_unread_texture_is_read_in_the_first_frame_that_draws_it_and_never_before() {
        let files = files();
        let mut renderer = reading(&files);
        renderer.add_planet(PlanetKey(2), unread());
        assert_eq!(files.reads(FILE), 0, "not when it is added");
        let mut gpu = RecordingGpu::new();
        let mut elsewhere = DrawList::new();
        elsewhere.planet(GLOBE, pose(20.0, 20.0));
        frame_of(&mut renderer, &elsewhere, &mut gpu);
        assert_eq!(files.reads(FILE), 0, "nor in a frame that doesn't draw it");
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(files.reads(FILE), 1, "in the frame that draws it");
        frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(files.reads(FILE), 1, "and only then");
    }

    /// An error the files give for [`FILE`].
    fn unreadable() -> PlanetError {
        PlanetError::Unreadable {
            stellar: "pale".into(),
            asset: "moon-surface".into(),
            why: format!("`{FILE}`: bad"),
        }
    }

    /// Files that fail to read [`FILE`] with [`unreadable`].
    fn failing() -> Rc<FakePlanetFiles> {
        Rc::new(FakePlanetFiles::default().with_error(FILE, unreadable()))
    }

    /// `commands` among a sprite before them and a line after.
    fn among(commands: &[DrawCommand]) -> DrawList {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE);
        for command in commands {
            list.push(command.clone());
        }
        list.line(
            Point::new(0.0, 0.0),
            Point::new(9.0, 9.0),
            1.0,
            Color::WHITE,
        );
        list
    }

    /// The commands the disc stand-in draws in a planet's place at `pose`.
    fn disc_at(pose: &PlanetPose) -> Vec<DrawCommand> {
        let mut disc = DrawList::new();
        nova_view::outline::draw_stellar(
            &mut disc,
            &StellarStandIn::Disc(DISC),
            pose.center,
            pose.radius,
        );
        disc.iter().cloned().collect()
    }

    #[test]
    fn a_texture_that_fails_to_read_draws_its_stand_in_in_that_frame_and_is_reported_once() {
        let files = failing();
        let mut renderer = reading(&files);
        renderer.add_planet(
            PlanetKey(2),
            unread().with_stand_in(StellarStandIn::Disc(DISC)),
        );
        let at = pose(20.0, 20.0);
        let list = among(&[DrawCommand::Planet {
            planet: PlanetKey(2),
            pose: at,
        }]);
        let expected = frame_of(
            &mut self::renderer(),
            &among(&disc_at(&at)),
            &mut RecordingGpu::new(),
        );
        assert!(!disc_at(&at).is_empty());
        let mut gpu = RecordingGpu::new();
        let report = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(report.failed_planets, [(PlanetKey(2), unreadable())]);
        assert_eq!(
            gpu.submits()[0].batches,
            expected.batches,
            "its disc, in place"
        );
        let again = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(again.failed_planets, [], "reported once");
        assert_eq!(gpu.submits()[1].batches, expected.batches, "its disc again");
        assert_eq!(uploads(&gpu), []);
        assert_eq!(files.reads(FILE), 1, "and not asked again");
    }

    #[test]
    fn a_clipped_planet_that_fails_draws_its_stand_in_clipped_alike() {
        let mut renderer = reading(&failing());
        renderer.add_planet(
            PlanetKey(2),
            unread().with_stand_in(StellarStandIn::Disc(DISC)),
        );
        let at = pose(20.0, 20.0);
        let clip = bounds(0.0, 0.0, 22.0, 30.0);
        let mut list = DrawList::new();
        list.clipped(
            clip,
            DrawCommand::Planet {
                planet: PlanetKey(2),
                pose: at,
            },
        );
        let mut reference = DrawList::new();
        for command in disc_at(&at) {
            reference.clipped(clip, command);
        }
        let expected = frame_of(&mut self::renderer(), &reference, &mut RecordingGpu::new());
        let frame = frame_of(&mut renderer, &list, &mut RecordingGpu::new());
        assert!(!frame.batches.is_empty());
        assert_eq!(frame.batches, expected.batches);
    }

    #[test]
    fn a_planet_that_fails_with_no_stand_in_draws_nothing_and_is_reported_once() {
        let mut renderer = reading(&failing());
        renderer.add_planet(PlanetKey(2), unread());
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        let mut gpu = RecordingGpu::new();
        let first = renderer.render(&list, &viewport(), &mut gpu);
        let second = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(first.failed_planets, [(PlanetKey(2), unreadable())]);
        assert_eq!(second.failed_planets, []);
        assert!(gpu.submits().iter().all(|frame| frame.batches.is_empty()));
    }

    #[test]
    fn a_failed_key_given_a_new_planet_is_made_again() {
        let files = failing();
        let mut renderer = reading(&files);
        renderer.add_planet(PlanetKey(2), unread());
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        let mut gpu = RecordingGpu::new();
        frame_of(&mut renderer, &list, &mut gpu);
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(1), planet::textures(&bake(&look(77), SMALL)))]
        );
        assert_eq!(kinds(&frame), ["planets"]);
        // And a texture again, read again.
        renderer.add_planet(PlanetKey(2), unread());
        let report = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(report.failed_planets, [(PlanetKey(2), unreadable())]);
        assert_eq!(files.reads(FILE), 2);
    }

    #[test]
    fn with_no_files_given_an_unread_texture_fails_as_missing() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unread());
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        let report = renderer.render(&list, &viewport(), &mut RecordingGpu::new());
        assert_eq!(
            report.failed_planets,
            [(
                PlanetKey(2),
                PlanetError::Missing {
                    stellar: "pale".into(),
                    asset: "moon-surface".into(),
                    file: FILE.into(),
                }
            )]
        );
    }

    #[test]
    fn a_planet_with_a_refused_pose_is_not_made_and_does_not_fail() {
        let files = failing();
        let mut renderer = reading(&files);
        renderer.add_planet(
            PlanetKey(2),
            unread().with_stand_in(StellarStandIn::Disc(DISC)),
        );
        let mut list = DrawList::new();
        list.planet(
            PlanetKey(2),
            PlanetPose {
                radius: 0.0,
                ..pose(20.0, 20.0)
            },
        );
        let mut gpu = RecordingGpu::new();
        let report = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(report.failed_planets, []);
        assert_eq!(files.reads(FILE), 0);
        assert!(gpu.submits()[0].batches.is_empty());
    }

    #[test]
    fn a_refused_pose_neither_bakes_nor_uploads_and_a_later_good_pose_does_once() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(
            PlanetKey(2),
            PlanetPose {
                radius: 0.0,
                ..pose(20.0, 20.0)
            },
        )
        .planet(PlanetKey(2), pose(f32::NAN, 20.0));
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(frame.batches, []);
        assert_eq!(uploads(&gpu), []);
        assert!(!is_baked(&renderer, PlanetKey(2)), "nothing drew it");
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        for _ in 0..2 {
            let frame = frame_of(&mut renderer, &list, &mut gpu);
            assert_eq!(kinds(&frame), ["planets"]);
        }
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(1), planet::textures(&bake(&look(77), SMALL)))]
        );
    }

    #[test]
    fn two_unbaked_planets_drawn_in_one_frame_are_both_drawn_in_it() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        renderer.add_planet(PlanetKey(3), unbaked(78));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0))
            .planet(PlanetKey(3), pose(40.0, 20.0));
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        let (first, second) = (bake(&look(77), SMALL), bake(&look(78), SMALL));
        assert_ne!(first, second, "two looks");
        assert_eq!(
            uploads(&gpu),
            [
                (PlanetId(1), planet::textures(&first)),
                (PlanetId(2), planet::textures(&second))
            ]
        );
        let instance = |id, surface, x| {
            planet::instance(PlanetId(id), surface, &pose(x, 20.0)).expect("drawable")
        };
        assert_eq!(
            frame.batches,
            [Batch::Planets {
                light: PLANET_LIGHT,
                planets: vec![instance(1, &first, 20.0), instance(2, &second, 40.0)],
                clip: None,
            }]
        );
    }

    #[test]
    fn a_key_given_a_new_unbaked_planet_bakes_again_at_its_next_draw() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        frame_of(&mut renderer, &list, &mut RecordingGpu::new());
        renderer.add_planet(PlanetKey(2), unbaked(78));
        assert!(!is_baked(&renderer, PlanetKey(2)), "not until it is drawn");
        let mut gpu = RecordingGpu::new();
        for _ in 0..2 {
            frame_of(&mut renderer, &list, &mut gpu);
        }
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(1), planet::textures(&bake(&look(78), SMALL)))],
            "the new look, under the same number, once"
        );
    }

    #[test]
    fn a_planet_is_numbered_once_and_adding_it_again_replaces_it() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), planet(30));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0))
            .planet(GLOBE, pose(40.0, 20.0));
        frame_of(&mut renderer, &list, &mut gpu);
        let ids: Vec<PlanetId> = uploads(&gpu).into_iter().map(|(id, _)| id).collect();
        assert_eq!(
            ids,
            [PlanetId(1), PlanetId(0)],
            "numbered as they were added"
        );
        renderer.add_planet(GLOBE, planet(50));
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(0), planet::textures(&surface(50)))],
            "the replacement, under the same number, and only it"
        );
        let Batch::Planets { planets, .. } = &frame.batches[0] else {
            panic!("{:?}", frame.batches);
        };
        assert_eq!(planets[1].planet, PlanetId(0));
    }

    #[test]
    fn a_missing_planet_draws_nothing_and_is_reported_once() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(9), pose(20.0, 20.0))
            .planet(PlanetKey(9), pose(30.0, 20.0))
            .planet(PlanetKey(8), pose(30.0, 20.0));
        let first = renderer.render(&list, &viewport(), &mut gpu);
        let second = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(first.missing_planets, [PlanetKey(9), PlanetKey(8)]);
        assert_eq!(second.missing_planets, []);
        assert!(first.missing_models.is_empty());
        assert!(gpu.submits().iter().all(|frame| frame.batches.is_empty()));
        assert_eq!(uploads(&gpu), []);
    }

    #[test]
    fn consecutive_planets_share_one_batch_and_anything_between_splits_it() {
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(10.0, 10.0))
            .planet(GLOBE, pose(30.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(
            frame.batches,
            [Batch::Planets {
                light: PLANET_LIGHT,
                planets: vec![instance(&pose(10.0, 10.0)), instance(&pose(30.0, 10.0))],
                clip: None,
            }]
        );
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(10.0, 10.0))
            .sprite(ImageKey::sprite(1, 0), Point::new(14.0, 12.0), Color::WHITE)
            .planet(GLOBE, pose(30.0, 10.0))
            .model(CUBE, model_pose(30.0, 10.0))
            .planet(GLOBE, pose(40.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(
            kinds(&frame),
            ["planets", "sprites", "planets", "models", "planets"]
        );
    }

    #[test]
    fn list_order_holds_between_planets_and_models() {
        // A model after a planet covers it, and a planet after a model
        // covers it: each is its own batch, in order, and the planets are
        // in no models batch, so in no depth band.
        let mut list = DrawList::new();
        list.model(CUBE, model_pose(20.0, 20.0))
            .planet(GLOBE, pose(20.0, 20.0))
            .model(CUBE, model_pose(20.0, 20.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(kinds(&frame), ["models", "planets", "models"]);
        let [
            Batch::Models { models: before, .. },
            _,
            Batch::Models { models: after, .. },
        ] = frame.batches.as_slice()
        else {
            panic!("{:?}", frame.batches);
        };
        assert_eq!(
            (before.len(), after.len()),
            (1, 1),
            "each model alone in its batch"
        );
    }

    #[test]
    fn a_part_after_a_planet_has_no_model_to_mount_on() {
        let mut list = DrawList::new();
        list.model(CUBE, model_pose(20.0, 20.0))
            .planet(GLOBE, pose(20.0, 20.0))
            .mounted(
                CUBE,
                Mounting {
                    at: [0.0; 3],
                    turn: 0.0,
                },
            );
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(kinds(&frame), ["models", "planets"]);
    }

    #[test]
    fn a_planet_with_a_bad_pose_draws_and_uploads_nothing() {
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        for bad in [
            PlanetPose {
                radius: 0.0,
                ..pose(10.0, 10.0)
            },
            PlanetPose {
                radius: f32::NAN,
                ..pose(10.0, 10.0)
            },
            PlanetPose {
                time: f64::INFINITY,
                ..pose(10.0, 10.0)
            },
            pose(f32::INFINITY, 10.0),
        ] {
            list.planet(GLOBE, bad);
        }
        let frame = frame_of(&mut renderer(), &list, &mut gpu);
        assert_eq!(frame.batches, []);
        assert_eq!(uploads(&gpu), []);
    }

    #[test]
    fn a_world_planet_marks_its_batch_world() {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE)
            .world(|list| {
                list.planet(GLOBE, pose(12.0, 12.0));
            });
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(frame.world, [1]);
        assert_eq!(frame.batches[1].kind(), "planets");
    }

    fn bounds(x0: f32, y0: f32, x1: f32, y1: f32) -> Bounds {
        Bounds {
            min: Point::new(x0, y0),
            max: Point::new(x1, y1),
        }
    }

    /// `bounds` as a batch carries it.
    fn rect_of(bounds: Bounds) -> crate::Rect {
        crate::Rect {
            x: bounds.min.x,
            y: bounds.min.y,
            w: bounds.width(),
            h: bounds.height(),
        }
    }

    fn globe_at(pose: PlanetPose) -> DrawCommand {
        DrawCommand::Planet {
            planet: GLOBE,
            pose,
        }
    }

    #[test]
    fn a_clipped_planet_carries_its_clip_on_its_batch() {
        // A clip far smaller than the planet's square.
        let clip = bounds(0.0, 0.0, 13.0, 13.0);
        let mut clipped = DrawList::new();
        clipped.clipped(clip, globe_at(pose(12.0, 12.0)));
        let frame = frame_of(&mut renderer(), &clipped, &mut RecordingGpu::new());
        assert_eq!(
            frame.batches,
            [Batch::Planets {
                light: PLANET_LIGHT,
                planets: vec![instance(&pose(12.0, 12.0))],
                clip: Some(crate::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 13.0,
                    h: 13.0
                }),
            }]
        );
    }

    #[test]
    fn planets_clipped_alike_share_a_batch_and_another_clip_splits_it() {
        let (c1, c2) = (bounds(0.0, 0.0, 20.0, 20.0), bounds(10.0, 0.0, 40.0, 30.0));
        let mut list = DrawList::new();
        list.clipped(c1, globe_at(pose(10.0, 10.0)))
            .clipped(c1, globe_at(pose(14.0, 10.0)))
            .clipped(c2, globe_at(pose(30.0, 10.0)))
            .planet(GLOBE, pose(40.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        let batch = |clip, poses: &[PlanetPose]| Batch::Planets {
            light: PLANET_LIGHT,
            planets: poses.iter().map(instance).collect(),
            clip,
        };
        assert_eq!(
            frame.batches,
            [
                batch(Some(rect_of(c1)), &[pose(10.0, 10.0), pose(14.0, 10.0)]),
                batch(Some(rect_of(c2)), &[pose(30.0, 10.0)]),
                batch(None, &[pose(40.0, 10.0)]),
            ]
        );
    }

    #[test]
    fn a_clipped_planet_wholly_outside_its_clip_is_still_made_ready() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let clip = bounds(0.0, 0.0, 10.0, 10.0);
        let mut list = DrawList::new();
        list.clipped(
            clip,
            DrawCommand::Planet {
                planet: PlanetKey(2),
                pose: pose(40.0, 30.0),
            },
        );
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert!(is_baked(&renderer, PlanetKey(2)));
        assert_eq!(uploads(&gpu).len(), 1);
        assert_eq!(frame.batches[0].clip(), Some(rect_of(clip)));
        assert_eq!(frame.batches[0].draws(), 1);
        frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(uploads(&gpu).len(), 1, "once");
    }

    #[test]
    fn a_planet_clipped_to_no_area_draws_bakes_and_uploads_nothing() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut list = DrawList::new();
        let planet = DrawCommand::Planet {
            planet: PlanetKey(2),
            pose: pose(12.0, 12.0),
        };
        list.clipped(bounds(5.0, 0.0, 30.0, 0.0), planet);
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(frame.batches, []);
        assert_eq!(uploads(&gpu), []);
        assert!(!is_baked(&renderer, PlanetKey(2)));
    }

    // Leaving systems.

    /// System A's planets: a look and a texture.
    const A1: PlanetKey = PlanetKey(10);
    const A2: PlanetKey = PlanetKey(11);
    /// System B's and system C's.
    const B1: PlanetKey = PlanetKey(20);
    const C1: PlanetKey = PlanetKey(30);
    /// A planet drawn on a screen that flies in no system.
    const PORT: PlanetKey = PlanetKey(40);
    /// The systems flown in, by their ids.
    const SYSTEM_A: i16 = 130;
    const SYSTEM_B: i16 = 131;
    const SYSTEM_C: i16 = 132;
    const SYSTEM_D: i16 = 133;

    /// A renderer reading `files`, with A's, B's and C's planets and
    /// [`PORT`], numbered 1 to 5 in that order after the globe's 0.
    fn galaxy(files: &Rc<FakePlanetFiles>) -> Renderer<OneSprite> {
        let mut renderer = reading(files);
        renderer.add_planet(A1, unbaked(70));
        renderer.add_planet(A2, unread());
        renderer.add_planet(B1, unbaked(71));
        renderer.add_planet(C1, unbaked(72));
        renderer.add_planet(PORT, unbaked(73));
        renderer
    }

    /// A frame flying in `system`, drawing `keys` side by side.
    fn flying(system: i16, keys: &[PlanetKey]) -> DrawList {
        let mut list = elsewhere(keys);
        list.in_system(nova_data::SystemId(system));
        list
    }

    /// A frame that names no system, drawing `keys` side by side.
    fn elsewhere(keys: &[PlanetKey]) -> DrawList {
        let mut list = DrawList::new();
        for (n, key) in keys.iter().enumerate() {
            list.planet(*key, pose(10.0 + 12.0 * n as f32, 20.0));
        }
        list
    }

    /// The calls `gpu` receives while `renderer` draws `list`.
    fn calls_drawing(
        renderer: &mut Renderer<OneSprite>,
        list: &DrawList,
        gpu: &mut RecordingGpu,
    ) -> Vec<GpuCall> {
        let before = gpu.calls.len();
        renderer.render(list, &viewport(), gpu);
        gpu.calls[before..].to_vec()
    }

    /// The planets `calls` release, in order.
    fn released(calls: &[GpuCall]) -> Vec<PlanetId> {
        (calls.iter())
            .filter_map(|call| match call {
                GpuCall::ReleasePlanet { planet } => Some(*planet),
                _ => None,
            })
            .collect()
    }

    /// The planets `calls` upload, in order.
    fn uploaded(calls: &[GpuCall]) -> Vec<PlanetId> {
        (calls.iter())
            .filter_map(|call| match call {
                GpuCall::UploadPlanet { planet, .. } => Some(*planet),
                _ => None,
            })
            .collect()
    }

    /// The planets `calls`' one frame draws.
    fn drawn_in(calls: &[GpuCall]) -> Vec<PlanetId> {
        let [frame] = &(calls.iter())
            .filter_map(|call| match call {
                GpuCall::Submit(frame) => Some(frame),
                _ => None,
            })
            .collect::<Vec<_>>()[..]
        else {
            panic!("one frame");
        };
        (frame.batches.iter())
            .flat_map(|batch| match batch {
                Batch::Planets { planets, .. } => planets.iter().map(|p| p.planet).collect(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// Whether every release in `calls` comes after its frame's submit.
    fn released_after_the_submit(calls: &[GpuCall]) -> bool {
        let submit = (calls.iter()).position(|call| matches!(call, GpuCall::Submit(_)));
        calls.iter().enumerate().all(|(at, call)| {
            !matches!(call, GpuCall::ReleasePlanet { .. }) || submit.is_some_and(|s| at > s)
        })
    }

    #[test]
    fn leaving_a_system_releases_its_planets_two_systems_on() {
        let files = files();
        let mut renderer = galaxy(&files);
        let mut gpu = RecordingGpu::new();
        let in_a = calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1, A2]), &mut gpu);
        assert_eq!(uploaded(&in_a), [PlanetId(1), PlanetId(2)]);
        let in_b = calls_drawing(&mut renderer, &flying(SYSTEM_B, &[B1]), &mut gpu);
        assert_eq!(released(&in_b), [], "the system before is kept");
        let still_b = calls_drawing(&mut renderer, &flying(SYSTEM_B, &[B1]), &mut gpu);
        assert_eq!(released(&still_b), []);
        let in_c = calls_drawing(&mut renderer, &flying(SYSTEM_C, &[C1]), &mut gpu);
        assert_eq!(
            released(&in_c),
            [PlanetId(1), PlanetId(2)],
            "A's, and only A's"
        );
        assert!(released_after_the_submit(&in_c), "{in_c:?}");
        assert_eq!(drawn_in(&in_c), [PlanetId(4)], "C's planet drawn");
        assert_eq!(
            gpu.planets_held(),
            [PlanetId(3), PlanetId(4)].into_iter().collect(),
            "B's and C's"
        );
        assert_eq!(gpu.unheld_planets_drawn(), []);
        let still_c = calls_drawing(&mut renderer, &flying(SYSTEM_C, &[C1]), &mut gpu);
        assert_eq!(released(&still_c), [], "once a visit");
    }

    #[test]
    fn returning_draws_them_again_in_the_first_frame_uploaded_again_equal() {
        let files = files();
        let mut renderer = galaxy(&files);
        let mut gpu = RecordingGpu::new();
        let first = calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1, A2]), &mut gpu);
        calls_drawing(&mut renderer, &flying(SYSTEM_B, &[B1]), &mut gpu);
        calls_drawing(&mut renderer, &flying(SYSTEM_C, &[C1]), &mut gpu);
        assert_eq!(files.reads(FILE), 1);
        let back = calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1, A2]), &mut gpu);
        let uploads_of = |calls: &[GpuCall]| -> Vec<GpuCall> {
            (calls.iter())
                .filter(|call| matches!(call, GpuCall::UploadPlanet { .. }))
                .cloned()
                .collect()
        };
        assert_eq!(
            uploads_of(&back),
            uploads_of(&first),
            "equal, generated and textured"
        );
        assert_eq!(
            uploads_of(&back),
            [
                GpuCall::UploadPlanet {
                    planet: PlanetId(1),
                    textures: planet::textures(&bake(&look(70), SMALL)),
                },
                GpuCall::UploadPlanet {
                    planet: PlanetId(2),
                    textures: planet::textures(&textured(7)),
                },
            ]
        );
        assert_eq!(files.reads(FILE), 2, "the texture read again");
        let submit = (back.iter())
            .position(|call| matches!(call, GpuCall::Submit(_)))
            .expect("a frame");
        assert!(
            (back.iter().take(submit)).all(|call| matches!(call, GpuCall::UploadPlanet { .. })),
            "uploaded before the frame: {back:?}"
        );
        assert_eq!(drawn_in(&back), drawn_in(&first), "drawn in that frame");
        assert_eq!(drawn_in(&back), [PlanetId(1), PlanetId(2)]);
        assert_eq!(released(&back), [PlanetId(3)], "and B's let go");
        assert_eq!(gpu.unheld_planets_drawn(), []);
    }

    #[test]
    fn flying_back_and_forth_between_two_systems_makes_nothing_again() {
        let files = files();
        let mut renderer = galaxy(&files);
        let mut gpu = RecordingGpu::new();
        for system in [SYSTEM_A, SYSTEM_B, SYSTEM_A, SYSTEM_B, SYSTEM_A] {
            let keys: &[PlanetKey] = if system == SYSTEM_A { &[A1, A2] } else { &[B1] };
            calls_drawing(&mut renderer, &flying(system, keys), &mut gpu);
        }
        assert_eq!(
            uploaded(&gpu.calls),
            [PlanetId(1), PlanetId(2), PlanetId(3)],
            "each once"
        );
        assert_eq!(released(&gpu.calls), []);
        assert_eq!(files.reads(FILE), 1);
    }

    #[test]
    fn frames_naming_no_system_start_no_visit_and_their_planets_count_in_the_current_one() {
        let files = files();
        let mut renderer = galaxy(&files);
        let mut gpu = RecordingGpu::new();
        calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1]), &mut gpu);
        // Landed in A: the port shows its planet, and A's own.
        calls_drawing(&mut renderer, &elsewhere(&[PORT, A1]), &mut gpu);
        // Taken off in A again: no new visit, nothing made again.
        calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1]), &mut gpu);
        let in_b = calls_drawing(&mut renderer, &flying(SYSTEM_B, &[B1]), &mut gpu);
        assert_eq!(released(&in_b), []);
        calls_drawing(&mut renderer, &elsewhere(&[]), &mut gpu);
        let in_c = calls_drawing(&mut renderer, &flying(SYSTEM_C, &[C1]), &mut gpu);
        assert_eq!(
            released(&in_c),
            [PlanetId(1), PlanetId(5)],
            "A's and the port's"
        );
        assert_eq!(
            uploaded(&gpu.calls),
            [PlanetId(1), PlanetId(5), PlanetId(3), PlanetId(4)],
            "each once"
        );
    }

    #[test]
    fn a_planet_drawn_only_on_another_screen_is_released_two_systems_later() {
        let files = files();
        let mut renderer = galaxy(&files);
        let mut gpu = RecordingGpu::new();
        // The main menu, before any flight.
        calls_drawing(&mut renderer, &elsewhere(&[PORT]), &mut gpu);
        let in_a = calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1]), &mut gpu);
        assert_eq!(released(&in_a), []);
        let in_b = calls_drawing(&mut renderer, &flying(SYSTEM_B, &[B1]), &mut gpu);
        assert_eq!(released(&in_b), [PlanetId(5)]);
    }

    #[test]
    fn a_planet_given_as_a_surface_is_never_released() {
        let mut renderer = galaxy(&files());
        let mut gpu = RecordingGpu::new();
        calls_drawing(&mut renderer, &flying(SYSTEM_A, &[GLOBE]), &mut gpu);
        for system in [SYSTEM_B, SYSTEM_C, SYSTEM_D] {
            calls_drawing(&mut renderer, &flying(system, &[]), &mut gpu);
        }
        assert_eq!(
            released(&gpu.calls),
            [],
            "it has nothing to be made again from"
        );
        let back = calls_drawing(&mut renderer, &flying(SYSTEM_A, &[GLOBE]), &mut gpu);
        assert_eq!(uploaded(&back), []);
        assert_eq!(drawn_in(&back), [PlanetId(0)]);
        assert_eq!(gpu.unheld_planets_drawn(), []);
    }

    #[test]
    fn a_failed_planet_releases_nothing_and_stays_failed() {
        let files = failing();
        let mut renderer = galaxy(&files);
        let mut gpu = RecordingGpu::new();
        let report = renderer.render(&flying(SYSTEM_A, &[A2]), &viewport(), &mut gpu);
        assert_eq!(report.failed_planets.len(), 1);
        for system in [SYSTEM_B, SYSTEM_C] {
            calls_drawing(&mut renderer, &flying(system, &[]), &mut gpu);
        }
        assert_eq!(released(&gpu.calls), []);
        let report = renderer.render(&flying(SYSTEM_A, &[A2]), &viewport(), &mut gpu);
        assert_eq!(report.failed_planets, [], "still failed, not asked again");
        assert_eq!(files.reads(FILE), 1);
    }

    #[test]
    fn a_replaced_planet_never_drawn_again_has_its_old_textures_released_on_leaving() {
        let mut renderer = galaxy(&files());
        let mut gpu = RecordingGpu::new();
        calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1]), &mut gpu);
        renderer.add_planet(A1, unbaked(99));
        calls_drawing(&mut renderer, &flying(SYSTEM_B, &[]), &mut gpu);
        let in_c = calls_drawing(&mut renderer, &flying(SYSTEM_C, &[]), &mut gpu);
        assert_eq!(released(&in_c), [PlanetId(1)]);
        assert!(gpu.planets_held().is_empty());
        // Drawn again, its new look is made.
        let back = calls_drawing(&mut renderer, &flying(SYSTEM_A, &[A1]), &mut gpu);
        assert_eq!(
            back[0],
            GpuCall::UploadPlanet {
                planet: PlanetId(1),
                textures: planet::textures(&bake(&look(99), SMALL)),
            }
        );
    }
}
