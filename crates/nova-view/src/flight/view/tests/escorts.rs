use super::*;

// Escorts.

#[test]
fn the_help_line_names_the_escort_keys() {
    assert!(
        default_help().contains("F/D/V/C: escorts   Alt-C: dock   E: escort menu"),
        "{}",
        default_help()
    );
}

// Hiring escorts and paying them.
