//! The new game's world models given to the app's renderer
//! ([`App::with_world_assets`]): the meshes and planets the keys of the
//! flight's table name (`nova_render::world_assets`), registered once, as
//! the app is built. The table goes to the screen
//! ([`AppScreen::with_world_models`](super::AppScreen::with_world_models));
//! a key the screen draws with nothing registered for it draws nothing and
//! is reported once (`nova_render::RenderReport`).

use nova_render::{ImageSource, LoadedPlanet, Mesh};
use nova_view::{ModelKey, PlanetKey, Screen};

use super::App;

impl<S: ImageSource, C: Screen> App<S, C> {
    /// The app with each of `meshes` and `planets` registered under its
    /// key with its renderer.
    #[must_use]
    pub fn with_world_assets(
        mut self,
        meshes: Vec<(ModelKey, Mesh)>,
        planets: Vec<(PlanetKey, LoadedPlanet)>,
    ) -> Self {
        for (key, mesh) in meshes {
            self.renderer.add_model(key, mesh);
        }
        for (key, planet) in planets {
            self.renderer.add_planet(key, planet);
        }
        self
    }
}
