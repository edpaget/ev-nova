//! A planet the renderer couldn't make in the frame that first drew it (a
//! texture that didn't read through the files the app was given) is kept
//! as one warning, in that frame alone.

use nova_render::planet_files::{FakePlanetFiles, Limits};
use nova_render::{LoadedPlanet, PlanetError, TextureFile};
use nova_view::outline::StellarStandIn;
use nova_view::{PlanetKey, PlanetPose};

use super::*;

/// The planet key the screen draws.
const PALE: PlanetKey = PlanetKey(3);

/// The file the planet's texture would be read from.
const FILE: &str = "media/export/pack/pale.png";

/// Draws [`PALE`].
struct DrawsPale;

impl Screen for DrawsPale {
    fn input(&mut self, _input: &Input) -> ScreenAction {
        ScreenAction::None
    }

    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        list.planet(
            PALE,
            PlanetPose {
                center: Point::new(200.0, 200.0),
                radius: 30.0,
                time: 0.0,
            },
        );
    }
}

fn unreadable() -> PlanetError {
    PlanetError::Unreadable {
        stellar: "pale".into(),
        asset: "pale-surface".into(),
        why: format!("`{FILE}`: bad"),
    }
}

#[test]
fn a_planet_that_fails_at_first_draw_is_one_warning() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let unread = LoadedPlanet::unread(TextureFile {
        stellar: "pale".into(),
        asset: "pale-surface".into(),
        file: FILE.into(),
        limits: Limits::default(),
    })
    .with_stand_in(StellarStandIn::Disc(Color::WHITE));
    let mut app = App::new(&window, NoImages, DrawsPale)
        .with_world_assets(
            Vec::new(),
            vec![(PALE, unread)],
            nova_render::sky::SkyLooks::default(),
        )
        .with_planet_files(Box::new(
            FakePlanetFiles::default().with_error(FILE, unreadable()),
        ));
    let redraw = WindowEvent::Redraw {
        elapsed: Duration::from_millis(16),
    };
    app.handle(redraw, &mut window, &mut RecordingGpu::new());
    assert_eq!(
        app.take_warnings(),
        [format!("nova: {}; it draws its disc", unreadable())]
    );
    app.handle(redraw, &mut window, &mut RecordingGpu::new());
    assert_eq!(app.take_warnings(), Vec::<String>::new(), "once");
}
