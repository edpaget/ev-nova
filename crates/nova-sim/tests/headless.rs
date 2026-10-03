//! The simulation is headless: nothing it depends on opens a window, a GPU
//! device or an audio stream, so its tests run anywhere.

/// The crates that draw, open windows or play sound.
const HEADED: [&str; 7] = [
    "wgpu",
    "winit",
    "glyphon",
    "egui",
    "kira",
    "nova-render",
    "nova-view",
];

#[test]
fn no_dependency_renders_opens_windows_or_plays_sound() {
    let manifest = include_str!("../Cargo.toml");
    let dependencies: Vec<&str> = manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_once('=').map(|(name, _)| name.trim()))
        .collect();
    assert!(dependencies.contains(&"nova-data"), "{dependencies:?}");
    for headed in HEADED {
        assert!(
            !dependencies.iter().any(|name| name.starts_with(headed)),
            "{headed} in {dependencies:?}"
        );
    }
}
