//! The FFI module for the keyboard layout: `GetKeyboardLayout`,
//! `MapVirtualKeyExW` and `ToUnicodeEx`.
//!
//! Every `unsafe` block here carries a `SAFETY` comment.

#![allow(unsafe_code)]

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, HKL, MAPVK_VSC_TO_VK_EX, MapVirtualKeyExW, ToUnicodeEx, VK_DECIMAL,
    VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7,
    VK_NUMPAD8, VK_NUMPAD9,
};

/// `ToUnicodeEx`'s flag that leaves the thread's keyboard state alone
/// (Windows 10 1607 and later), so asking about a dead key doesn't compose
/// with the player's next real press, and asking twice gives one answer.
const KEEP_KEYBOARD_STATE: u32 = 0x4;

/// The keypad's digit and decimal keys by scancode, each with its
/// Num Lock-on virtual key. `MapVirtualKeyExW` gives their Num Lock-off
/// keys (Home, Up, …), which print nothing, so the layout is asked about
/// what the key prints with Num Lock on, as its legend shows.
const KEYPAD: [(u32, u16); 11] = [
    (0x47, VK_NUMPAD7),
    (0x48, VK_NUMPAD8),
    (0x49, VK_NUMPAD9),
    (0x4b, VK_NUMPAD4),
    (0x4c, VK_NUMPAD5),
    (0x4d, VK_NUMPAD6),
    (0x4f, VK_NUMPAD1),
    (0x50, VK_NUMPAD2),
    (0x51, VK_NUMPAD3),
    (0x52, VK_NUMPAD0),
    (0x53, VK_DECIMAL),
];

/// The thread's current keyboard layout.
fn current_layout() -> HKL {
    // SAFETY: `GetKeyboardLayout` takes a thread ID (0 for this thread)
    // and only reads; it has no preconditions.
    unsafe { GetKeyboardLayout(0) }
}

/// The current keyboard layout's language: the low word of its `HKL`
/// (0x0409 for US English), so a test can tell which layout it reads.
#[must_use]
pub fn keyboard_layout_language() -> u16 {
    current_layout() as usize as u16
}

/// What the thread's current keyboard layout prints on the key with
/// `scancode` (an extended scancode: 0xE0 in the high byte for an
/// extended key, as winit's `to_scancode` gives), with no modifier held,
/// decoded from the UTF-16 `ToUnicodeEx` writes. A dead key answers the
/// spacing character Windows leaves in the buffer. It asks the current
/// layout on every call and caches nothing, so a layout change shows at
/// the next call.
///
/// `None` when the key has no virtual key in the layout or prints nothing.
#[must_use]
pub fn key_label(scancode: u32) -> Option<String> {
    let layout = current_layout();
    let virtual_key = match KEYPAD.iter().find(|&&(code, _)| code == scancode) {
        Some(&(_, key)) => u32::from(key),
        // SAFETY: `MapVirtualKeyExW` reads its arguments only; `layout` is
        // the thread's own layout handle. An unknown code gives 0.
        None => unsafe { MapVirtualKeyExW(scancode, MAPVK_VSC_TO_VK_EX, layout) },
    };
    if virtual_key == 0 {
        return None;
    }
    let keys_down = [0_u8; 256];
    let mut units = [0_u16; 8];
    // SAFETY: `keys_down` is the 256-byte key state `ToUnicodeEx` reads;
    // `units` is a live buffer whose length is passed, so it writes no
    // further; `layout` is the thread's own. Only the scancode's low byte
    // is passed: bit 15 would mean a release, and 0xE0 sets it.
    let written = unsafe {
        ToUnicodeEx(
            virtual_key,
            scancode & 0xff,
            keys_down.as_ptr(),
            units.as_mut_ptr(),
            units.len() as i32,
            KEEP_KEYBOARD_STATE,
            layout,
        )
    };
    // A dead key returns -1, with its spacing character in the buffer.
    let length = if written < 0 { 1 } else { written as usize };
    let got = units.get(..length).filter(|got| !got.is_empty())?;
    Some(String::from_utf16_lossy(got))
}
