//! A model or planet key a frame draws with nothing registered for it
//! with the renderer is kept as one warning, the first time it is drawn,
//! wherever in the frame it is drawn; a frame whose keys are registered
//! reports nothing.

use std::cell::RefCell;

use nova_render::Mesh;
use nova_view::geometry::Bounds;
use nova_view::{DrawCommand, ModelKey, ModelPose, Mounting, Orientation, PlanetKey, PlanetPose};

use super::*;

/// Draws the keys it holds, each where its field says.
#[derive(Default)]
struct Draws {
    /// Models drawn in the overlay.
    models: RefCell<Vec<ModelKey>>,
    /// Planets drawn in the overlay.
    planets: RefCell<Vec<PlanetKey>>,
    /// Models drawn in the world layer.
    world: Vec<ModelKey>,
    /// Models drawn clipped.
    clipped: Vec<ModelKey>,
    /// A model, and the parts mounted on it.
    mounted: Option<(ModelKey, Vec<ModelKey>)>,
}

fn pose() -> ModelPose {
    ModelPose {
        center: Point::new(100.0, 100.0),
        orientation: Orientation::IDENTITY,
        scale: 10.0,
    }
}

fn planet_pose() -> PlanetPose {
    PlanetPose {
        center: Point::new(200.0, 200.0),
        radius: 30.0,
        time: 0.0,
    }
}

impl Screen for Draws {
    fn input(&mut self, _input: &Input) -> ScreenAction {
        ScreenAction::None
    }

    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        list.world(|world| {
            for &model in &self.world {
                world.model(model, pose());
            }
        });
        for &model in self.models.borrow().iter() {
            list.model(model, pose());
        }
        for &planet in self.planets.borrow().iter() {
            list.planet(planet, planet_pose());
        }
        for &model in &self.clipped {
            list.clipped(
                Bounds::at(Point::new(0.0, 0.0), 300.0, 300.0),
                DrawCommand::model(model, pose()),
            );
        }
        if let Some((model, parts)) = &self.mounted {
            list.model(*model, pose());
            for &part in parts {
                list.mounted(
                    part,
                    Mounting {
                        at: [0.0; 3],
                        turn: 0.0,
                    },
                );
            }
        }
    }
}

/// The warnings one more redraw of `app` leaves.
fn redrawn(app: &mut App<NoImages, Draws>, window: &mut FakeWindow) -> Vec<String> {
    let redraw = WindowEvent::Redraw {
        elapsed: Duration::from_millis(16),
    };
    assert_eq!(
        app.handle(redraw, window, &mut RecordingGpu::new()),
        Control::Continue
    );
    app.take_warnings()
}

fn model_line(key: u32) -> String {
    format!(
        "nova: model key {key} is drawn but the renderer was given no mesh for it: it draws nothing"
    )
}

fn planet_line(key: u32) -> String {
    format!(
        "nova: planet key {key} is drawn but the renderer was given no planet for it: it draws nothing"
    )
}

const NONE: Vec<String> = Vec::new();

#[test]
fn an_unregistered_model_and_planet_key_are_each_reported_once() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let screen = Draws {
        models: RefCell::new(vec![ModelKey(9)]),
        planets: RefCell::new(vec![PlanetKey(4)]),
        ..Draws::default()
    };
    let mut app = App::new(&window, NoImages, screen);

    assert_eq!(
        redrawn(&mut app, &mut window),
        [model_line(9), planet_line(4)]
    );
    for _ in 0..3 {
        assert_eq!(redrawn(&mut app, &mut window), NONE, "already reported");
    }

    app.screen().models.borrow_mut().push(ModelKey(8));
    app.screen().planets.borrow_mut().push(PlanetKey(5));
    assert_eq!(
        redrawn(&mut app, &mut window),
        [model_line(8), planet_line(5)],
        "only the new keys"
    );
    assert_eq!(redrawn(&mut app, &mut window), NONE);
}

#[test]
fn a_key_drawn_in_the_world_layer_or_clipped_is_reported_too() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let screen = Draws {
        world: vec![ModelKey(21)],
        clipped: vec![ModelKey(22)],
        mounted: Some((ModelKey(1), vec![ModelKey(23)])),
        ..Draws::default()
    };
    let mut app = App::new(&window, NoImages, screen).with_world_assets(
        vec![(ModelKey(1), Mesh::cube([Color::WHITE; 6]))],
        Vec::new(),
        nova_render::sky::SkyLooks::default(),
    );

    let mut lines = redrawn(&mut app, &mut window);
    lines.sort();
    assert_eq!(lines, [model_line(21), model_line(22), model_line(23)]);
    assert_eq!(redrawn(&mut app, &mut window), NONE);
}

#[test]
fn a_frame_whose_keys_are_registered_reports_nothing() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let screen = Draws {
        models: RefCell::new(vec![ModelKey(9)]),
        ..Draws::default()
    };
    let mut app = App::new(&window, NoImages, screen).with_world_assets(
        vec![(ModelKey(9), Mesh::cube([Color::WHITE; 6]))],
        Vec::new(),
        nova_render::sky::SkyLooks::default(),
    );
    for _ in 0..3 {
        assert_eq!(redrawn(&mut app, &mut window), NONE);
    }
}

#[test]
fn a_frame_that_draws_no_model_reports_nothing() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app: TestApp = App::new(&window, NoImages, RecordingScreen::default());
    let redraw = WindowEvent::Redraw {
        elapsed: Duration::from_millis(16),
    };
    for _ in 0..3 {
        app.handle(redraw, &mut window, &mut RecordingGpu::new());
        assert_eq!(app.take_warnings(), NONE);
    }
}
