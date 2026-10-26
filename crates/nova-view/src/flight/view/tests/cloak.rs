//! The cloak key, U held.

use super::*;

#[test]
fn cloak_key_is_u() {
    // `cloakKey`, keymap slot 0x52, keycode 0x20 (`_loadKeys`
    // @0xce461-0xce4d4).
    assert_eq!(CLOAK_KEY, Key::Char('u'));
}
