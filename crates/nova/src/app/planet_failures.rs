//! The warnings for a planet the renderer couldn't make in the frame that
//! first drew it
//! ([`RenderReport::failed_planets`](nova_render::RenderReport::failed_planets)):
//! one line a planet, kept with the app's other warnings
//! ([`App::take_warnings`](super::App::take_warnings)) for the caller to
//! show.
//!
//! A textured planet is checked from its file's header at start-up, and
//! what fails there is reported there (`nova_render::world_assets`, "it
//! draws its disc"). Its file is read only in the first frame that draws
//! it, and a read can still fail then: a file that doesn't decode, or one
//! changed on disk since. The renderer draws the planet's disc in its
//! place from that frame on and names it in that frame's report alone,
//! so each is one line a run, in the words start-up uses. Unlike a key
//! with nothing registered (the `missing_keys` module), this is a state of
//! the data, not a wiring fault, and the disc is what the player sees.

use nova_render::PlanetError;
use nova_view::PlanetKey;

/// One line for each of `failed`, in its order: the planets a frame's
/// report lists as failed when they were first drawn.
pub(super) fn warnings(failed: &[(PlanetKey, PlanetError)]) -> Vec<String> {
    (failed.iter())
        .map(|(_, error)| format!("nova: {error}; it draws its disc"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_planet_s_line_names_its_error_and_its_disc() {
        let error = PlanetError::Unreadable {
            stellar: "pale".into(),
            asset: "pale-surface".into(),
            why: "`media/export/pack/pale.png`: bad".into(),
        };
        let missing = PlanetError::Missing {
            stellar: "dim".into(),
            asset: "dim-surface".into(),
            file: "media/export/pack/dim.png".into(),
        };
        assert_eq!(
            warnings(&[
                (PlanetKey(2), error.clone()),
                (PlanetKey(5), missing.clone())
            ]),
            [
                format!("nova: {error}; it draws its disc"),
                format!("nova: {missing}; it draws its disc"),
            ]
        );
    }

    #[test]
    fn no_failures_give_no_lines() {
        assert_eq!(warnings(&[]), Vec::<String>::new());
    }
}
