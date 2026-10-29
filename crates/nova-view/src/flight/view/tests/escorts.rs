use super::*;

// Escorts.

#[test]
fn the_help_line_names_the_escort_keys() {
    assert!(
        HELP.contains("F/D/V/C: escorts   Alt-C: dock   E: escort menu"),
        "{HELP}"
    );
}

// Hiring escorts and paying them.
