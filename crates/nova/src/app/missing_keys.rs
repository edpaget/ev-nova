//! The warnings for a model or planet key a frame drew with nothing
//! registered for it with the renderer
//! ([`RenderReport::missing_models`](nova_render::RenderReport::missing_models),
//! [`RenderReport::missing_planets`](nova_render::RenderReport::missing_planets)):
//! one line a key, kept with the app's other warnings
//! ([`App::take_warnings`](super::App::take_warnings)) for the caller to
//! show.
//!
//! - **Once a key.** The renderer owns that memory: its report names a key
//!   only the first time it is drawn, as it does a failed image, and the
//!   app forwards what it is handed. The renderer lives as long as the app
//!   and its meshes and planets are registered once, as the app is built
//!   ([`App::with_world_assets`](super::App::with_world_assets)), so a key
//!   is reported once a run.
//! - **The key alone.** The line names the kind and the key, not the class
//!   or stellar that asked for it: the table that maps them goes to the
//!   screen, one key serves every class of the same model, and a key the
//!   table doesn't hold maps to nothing.
//! - **No fallback.** The table and what is registered come from one
//!   builder (`nova_render::world_assets`), which gives a model that fails
//!   to load no key, so it keeps its stand-in. A key drawn with nothing
//!   registered is so a wiring fault of the engine's, never a state of the
//!   data: it draws nothing, as the renderer pins, and a stand-in drawn in
//!   its place would hide the fault.

use nova_view::{ModelKey, PlanetKey};

/// One line for each of `models`, then one for each of `planets`, in their
/// order: the keys a frame's report lists as drawn but not registered.
pub(super) fn warnings(models: &[ModelKey], planets: &[PlanetKey]) -> Vec<String> {
    let models = models.iter().map(|key| line("model", key.0, "mesh"));
    let planets = planets.iter().map(|key| line("planet", key.0, "planet"));
    models.chain(planets).collect()
}

/// The line for the `kind` key `key`, which the renderer was given no
/// `asset` for.
fn line(kind: &str, key: u32, asset: &str) -> String {
    format!(
        "nova: {kind} key {key} is drawn but the renderer was given no {asset} for it: it draws nothing"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_key_s_line_names_the_key_and_the_mesh_it_lacks() {
        assert_eq!(
            warnings(&[ModelKey(7)], &[]),
            [
                "nova: model key 7 is drawn but the renderer was given no mesh for it: it draws nothing"
            ]
        );
    }

    #[test]
    fn a_planet_key_s_line_names_the_key_and_the_planet_it_lacks() {
        assert_eq!(
            warnings(&[], &[PlanetKey(3)]),
            [
                "nova: planet key 3 is drawn but the renderer was given no planet for it: it draws nothing"
            ]
        );
    }

    #[test]
    fn the_models_come_first_then_the_planets_each_in_the_order_given() {
        let lines = warnings(&[ModelKey(7), ModelKey(2)], &[PlanetKey(9), PlanetKey(1)]);
        let named: Vec<&str> = lines
            .iter()
            .map(|line| line.split(" is drawn").next().expect("a line"))
            .collect();
        assert_eq!(
            named,
            [
                "nova: model key 7",
                "nova: model key 2",
                "nova: planet key 9",
                "nova: planet key 1"
            ]
        );
    }

    #[test]
    fn no_keys_give_no_lines() {
        assert_eq!(warnings(&[], &[]), Vec::<String>::new());
    }
}
