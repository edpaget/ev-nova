//! The HUD's, the radar's and the target panel's component tests on
//! [`FakeFlight`] (rdm:phase/view-sim-seam/phase-8-hud-and-target-frame):
//! a view over a scripted frame, its status bar and target panel drawn
//! from the frame's HUD and target parts, and the keys and loads they
//! send through the port. Kept apart from `tests/mod.rs`, which is large.

use super::*;

#[test]
fn a_description_is_read_from_the_catalog_and_loaded_through_the_flight() {
    let movie = nova_sim::DescMovie::from_desc("Port.mov", 0x0001).expect("named");
    let catalog = FakeCatalog {
        descriptions: vec![(3000, "raw".to_owned())],
        desc_movies: vec![(3000, movie.clone())],
        ..catalog()
    };
    let mut view = faked_in(catalog, in_sol());
    view.sim.queue_description("loaded");
    assert_eq!(view.load_description(3000).as_deref(), Ok("loaded"));
    assert_eq!(
        view.load_description(9999),
        Err("no dësc 9999".to_owned()),
        "a missing one"
    );
    assert_eq!(
        view.sim.descriptions_loaded(),
        [(Some("raw".to_owned()), Some(movie)), (None, None)],
        "the missing one loaded as missing"
    );
}
