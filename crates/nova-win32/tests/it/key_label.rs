//! `key_label` on the real keyboard layout: the US layout's names where
//! the machine has it (the CI runner does), and the port's contract
//! (never empty, the same twice) over every scancode on whatever layout
//! it has.

use nova_win32::{key_label, keyboard_layout_language};

/// The A key's scancode.
const A: u32 = 0x1e;
/// The `;` key's scancode on a US keyboard.
const SEMICOLON: u32 = 0x27;
/// US English's language ID, the low word of its layout handle.
const US_ENGLISH: u16 = 0x0409;

#[test]
fn the_us_layout_names_a_and_semicolon() {
    if keyboard_layout_language() == US_ENGLISH {
        assert_eq!(key_label(A).as_deref(), Some("a"));
        assert_eq!(key_label(SEMICOLON).as_deref(), Some(";"));
    } else {
        let name = key_label(A).expect("every layout prints on the A position");
        let mut chars = name.chars();
        let first = chars.next().expect("not empty");
        assert!(!first.is_control(), "{name:?}");
    }
}

#[test]
fn every_scancode_answers_the_same_twice_and_never_empty() {
    for code in (0x01..=0x7f).chain(0xe001..=0xe07f) {
        let first = key_label(code);
        assert_ne!(first.as_deref(), Some(""), "{code:#x}");
        assert_eq!(key_label(code), first, "{code:#x}");
    }
}
