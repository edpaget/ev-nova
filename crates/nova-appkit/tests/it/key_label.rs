//! `key_label` on the real keyboard layout, on the main thread: the US
//! layout's names where the machine has it (the CI runner does), and the
//! port's contract (never empty, the same twice) on whatever layout it has.

use nova_appkit::{key_label, keyboard_layout_id};

/// `kVK_ANSI_A`.
const A: u16 = 0x00;
/// `kVK_ANSI_Semicolon`.
const SEMICOLON: u16 = 0x29;

/// Whether the current layout is US English: the US or ABC layout.
fn us_layout() -> bool {
    matches!(
        keyboard_layout_id().as_deref(),
        Some("com.apple.keylayout.US" | "com.apple.keylayout.ABC")
    )
}

pub fn the_us_layout_names_a_and_semicolon() {
    assert!(
        keyboard_layout_id().is_some(),
        "the main thread reads the layout"
    );
    if us_layout() {
        assert_eq!(key_label(A).as_deref(), Some("a"));
        assert_eq!(key_label(SEMICOLON).as_deref(), Some(";"));
    } else {
        let name = key_label(A).expect("every layout prints on the A position");
        assert!(
            !name.is_empty() && !name.chars().any(char::is_control),
            "{name:?}"
        );
    }
}

pub fn every_key_code_answers_the_same_twice_and_never_empty() {
    for code in 0x00..=0x7f {
        let first = key_label(code);
        assert_ne!(first.as_deref(), Some(""), "{code:#x}");
        assert_eq!(key_label(code), first, "{code:#x}");
    }
}
