//! The new game's world models given to the app's renderer
//! ([`App::with_world_assets`]): the meshes and planets the keys of the
//! flight's table name (`nova_render::world_assets`), registered once, as
//! the app is built, and the systems' own sky looks by their skies' seeds
//! (`nova_render::sky_looks`). The table goes to the screen
//! ([`AppScreen::with_world_models`](super::AppScreen::with_world_models));
//! a key the screen draws with nothing registered for it draws nothing and
//! is kept as a warning, once ([`App::take_warnings`], the `missing_keys`
//! module), and a sky whose seed has no look draws the default one.

use nova_render::sky::SkyLooks;
use nova_render::{ImageSource, LoadedPlanet, Mesh};
use nova_view::{ModelKey, PlanetKey, Screen};

use super::App;

impl<S: ImageSource, C: Screen> App<S, C> {
    /// The app with each of `meshes` and `planets` registered under its
    /// key with its renderer, and its skies drawn in `skies`' looks.
    #[must_use]
    pub fn with_world_assets(
        mut self,
        meshes: Vec<(ModelKey, Mesh)>,
        planets: Vec<(PlanetKey, LoadedPlanet)>,
        skies: SkyLooks,
    ) -> Self {
        for (key, mesh) in meshes {
            self.renderer.add_model(key, mesh);
        }
        for (key, planet) in planets {
            self.renderer.add_planet(key, planet);
        }
        self.renderer.set_sky_looks(skies);
        self
    }
}
