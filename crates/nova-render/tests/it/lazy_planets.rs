//! A textured planet read at its first draw, and planets let go when
//! their system is left, wired from the loader to the recording Gpu
//! (rdm:phase/rendering-3d/phase-19-release-planets-of-systems-left): the
//! new game's loader (`nova_data::new_game::load`), the world builder
//! ([`world_assets`]) and a [`Renderer`] reading textures through
//! [`GalaxyPlanetFiles`], all over one counting source, so a test can see
//! when the texture's file is read.
//!
//! The sample is the 3D sample (Kestrel Reach, a generated gas giant in
//! `luyten-crossing`) with Pale, a textured moon, in `marrow`;
//! `brightwater` has no planet. Each frame flies in one system, naming it
//! as flight does ([`DrawList::in_system`]), and draws that system's
//! planets as flight commands them.

use std::path::Path;
use std::rc::Rc;

use nova_data::new_game::fixture::modelled_sample_3d;
use nova_data::new_game::{NewGame, load};
use nova_data::store::StandInOf;
use nova_data::wire::id::{StellarId, SystemId};
use nova_galaxy_format::source::{CountingSource, MemSource};
use nova_galaxy_media::fixture::png;
use nova_render::recording::{GpuCall, NoImages, RecordingGpu};
use nova_render::{
    Batch, Frame, GalaxyPlanetFiles, LOGICAL, PlanetError, RenderReport, Renderer, Viewport,
    world_assets,
};
use nova_view::flight::StellarModel;
use nova_view::outline::{StellarStandIn, draw_stellar};
use nova_view::{DrawCommand, DrawList, PlanetKey, PlanetPose, Point};

/// Pale's texture's file.
const PALE_PNG: &str = "media/export/pack/pale.png";

/// The source every part reads through, counted.
type Shared = Rc<CountingSource<MemSource>>;

/// The 3D sample with Pale in `marrow`, its texture's file `png`.
fn sample(png: &[u8]) -> Shared {
    let source = modelled_sample_3d()
        .with(
            "stellars/pale.md",
            "---\nname: Pale\nkind: moon\nsystem: marrow\nposition: [30, 30]\n\
             visual: {asset: pale-surface}\n---\n",
        )
        .with(
            "assets/pale-surface.md",
            &format!(
                "---\nkind: texture\nplaceholder: false\nfiles: [{PALE_PNG}]\n\
                 licence: CC0-1.0\nauthor: test\n---\n"
            ),
        )
        .with_bytes(PALE_PNG, png);
    Rc::new(CountingSource::new(source))
}

/// A 16 by 8 texture for Pale.
fn texture() -> Vec<u8> {
    let rgba: Vec<u8> = (0..8_u8)
        .flat_map(|y| (0..16_u8).flat_map(move |x| [x * 16, y * 32, 90, 255]))
        .collect();
    png(16, 8, &rgba)
}

/// The game started over a source: loaded, its world built, and a
/// renderer holding its planets and reading textures through the same
/// source.
struct Started {
    game: NewGame,
    /// Each stellar with a planet: its system's id and its key.
    planets: Vec<(StellarId, String, PlanetKey)>,
    renderer: Renderer<NoImages>,
}

impl Started {
    fn over(source: &Shared) -> Self {
        let game = load(source, Path::new("sample"));
        let assets = world_assets(&game, source);
        let mut renderer = Renderer::new(NoImages);
        renderer.set_planet_files(Box::new(GalaxyPlanetFiles::new(Rc::clone(source))));
        for (key, planet) in assets.planets.iter().cloned() {
            renderer.add_planet(key, planet);
        }
        let planets = (game.ids.stellars.iter())
            .filter_map(|(id, &nova)| match assets.models.stellar(StellarId(nova)) {
                Some(StellarModel::Planet { key, .. }) => {
                    let system = game.galaxy.stellars[id].data.system.to_string();
                    Some((StellarId(nova), system, key))
                }
                _ => None,
            })
            .collect();
        Started {
            game,
            planets,
            renderer,
        }
    }

    /// A frame flying in `system`, drawing its planets side by side.
    fn flying(&self, system: &str) -> DrawList {
        let nova = (self.game.ids.systems.iter())
            .find_map(|(id, &nova)| (id.as_str() == system).then_some(nova))
            .expect("a system");
        let mut list = DrawList::new();
        list.in_system(SystemId(nova));
        let shown = self.planets.iter().filter(|(_, at, _)| at == system);
        for (n, (_, _, key)) in shown.enumerate() {
            list.planet(*key, pose(n));
        }
        list
    }

    /// Draws a frame flying in `system` through `gpu`.
    fn fly(&mut self, system: &str, gpu: &mut RecordingGpu) -> RenderReport {
        let list = self.flying(system);
        self.renderer.render(&list, &viewport(), gpu)
    }

    /// Pale's stellar.
    fn pale(&self) -> StellarId {
        let id = "pale".to_owned().try_into().expect("an id");
        StellarId(self.game.ids.stellars[&id])
    }

    /// Pale's planet key.
    fn pale_key(&self) -> PlanetKey {
        let pale = self.pale();
        (self.planets.iter())
            .find_map(|(stellar, _, key)| (*stellar == pale).then_some(*key))
            .expect("Pale's key")
    }
}

/// Where planet `n` of a frame is drawn.
fn pose(n: usize) -> PlanetPose {
    PlanetPose {
        center: Point::new(100.0 + 60.0 * n as f32, 200.0),
        radius: 20.0,
        time: 0.0,
    }
}

/// The viewport every frame is drawn into.
fn viewport() -> Viewport {
    Viewport::new(LOGICAL, (LOGICAL.w, LOGICAL.h), 1.0)
}

/// The planet uploads among `calls`.
fn uploads(calls: &[GpuCall]) -> Vec<GpuCall> {
    (calls.iter())
        .filter(|call| matches!(call, GpuCall::UploadPlanet { .. }))
        .cloned()
        .collect()
}

/// How many planet instances `frame` draws.
fn instances(frame: &Frame) -> usize {
    (frame.batches.iter())
        .map(|batch| match batch {
            Batch::Planets { planets, .. } => planets.len(),
            _ => 0,
        })
        .sum()
}

#[test]
fn a_textured_planet_s_file_is_first_read_in_the_frame_that_first_draws_it() {
    let source = sample(&texture());
    let mut world = Started::over(&source);
    assert_eq!(source.bytes_asked(PALE_PNG), 0, "not at start-up");
    let mut gpu = RecordingGpu::new();
    world.fly("luyten-crossing", &mut gpu);
    assert_eq!(source.bytes_asked(PALE_PNG), 0, "nor in another system");
    let report = world.fly("marrow", &mut gpu);
    assert_eq!(report, RenderReport::default());
    assert_eq!(
        source.bytes_asked(PALE_PNG),
        1,
        "in its system's first frame"
    );
    world.fly("marrow", &mut gpu);
    assert_eq!(source.bytes_asked(PALE_PNG), 1, "and only then");
    assert_eq!(gpu.unheld_planets_drawn(), []);
}

#[test]
fn leaving_two_systems_releases_it_and_returning_reads_it_again_and_draws_it_in_that_frame() {
    let source = sample(&texture());
    let mut world = Started::over(&source);
    let mut gpu = RecordingGpu::new();
    world.fly("marrow", &mut gpu);
    let first = uploads(&gpu.take());
    assert_eq!(first.len(), 1, "Pale's");
    world.fly("luyten-crossing", &mut gpu);
    world.fly("brightwater", &mut gpu);
    let left = gpu.take();
    assert!(
        left.iter()
            .any(|call| matches!(call, GpuCall::ReleasePlanet { .. })),
        "released two systems on"
    );
    world.fly("marrow", &mut gpu);
    let back = gpu.take();
    assert_eq!(source.bytes_asked(PALE_PNG), 2, "read again");
    assert_eq!(uploads(&back), first, "uploaded again, equal");
    let at = (back.iter())
        .position(|call| matches!(call, GpuCall::Submit(_)))
        .expect("a frame");
    assert!(
        matches!(back[0], GpuCall::UploadPlanet { .. }) && at > 0,
        "uploaded before the frame"
    );
    let GpuCall::Submit(frame) = &back[at] else {
        unreachable!("found above");
    };
    assert_eq!(instances(frame), 1, "drawn in that frame");
}

#[test]
fn a_texture_corrupt_past_its_header_draws_its_disc_and_is_reported_once() {
    let mut corrupt = texture()[..33].to_vec();
    corrupt.extend([0xA5; 64]);
    let source = sample(&corrupt);
    let mut world = Started::over(&source);
    let mut gpu = RecordingGpu::new();
    let report = world.fly("marrow", &mut gpu);
    let [(key, PlanetError::Unreadable { stellar, why, .. })] = &report.failed_planets[..] else {
        panic!("one failure: {:?}", report.failed_planets);
    };
    assert_eq!((*key, stellar.as_str()), (world.pale_key(), "pale"));
    assert!(why.contains(PALE_PNG), "{why}");
    // Its disc, as the screens draw it, in its place.
    let stand_in = world
        .game
        .data
        .stand_in(StandInOf::Stellar(world.pale()))
        .expect("a stand-in");
    let mut disc = DrawList::new();
    let at = pose(0);
    draw_stellar(
        &mut disc,
        &StellarStandIn::from(stand_in),
        at.center,
        at.radius,
    );
    assert!(!disc.is_empty());
    let mut expected = RecordingGpu::new();
    Renderer::new(NoImages).render(&disc, &viewport(), &mut expected);
    assert_eq!(gpu.submits()[0].batches, expected.submits()[0].batches);
    let again = world.fly("marrow", &mut gpu);
    assert_eq!(again.failed_planets, [], "reported once");
    assert_eq!(gpu.submits()[1].batches, expected.submits()[0].batches);
    assert_eq!(source.bytes_asked(PALE_PNG), 1);
}

#[test]
fn no_frame_that_commands_a_planet_lacks_its_instance_or_its_stand_in() {
    let source = sample(&texture());
    let mut world = Started::over(&source);
    let mut gpu = RecordingGpu::new();
    let tour = [
        "marrow",
        "luyten-crossing",
        "marrow",
        "brightwater",
        "luyten-crossing",
        "marrow",
        "brightwater",
        "marrow",
        "marrow",
    ];
    for system in tour {
        let list = world.flying(system);
        let commanded = (list.iter())
            .filter(|c| matches!(c, DrawCommand::Planet { .. }))
            .count();
        let report = world.renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(report, RenderReport::default(), "{system}: nothing failed");
        let frame = gpu.submits().last().copied().cloned().expect("a frame");
        assert_eq!(instances(&frame), commanded, "{system}: each drawn");
    }
    assert!(
        (gpu.calls.iter()).any(|call| matches!(call, GpuCall::ReleasePlanet { .. })),
        "the tour lets planets go"
    );
    assert_eq!(
        gpu.unheld_planets_drawn(),
        [],
        "no frame draws a planet the GPU doesn't hold"
    );
}
