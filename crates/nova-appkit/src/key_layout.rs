//! The FFI module for the keyboard layout: what the player's current
//! layout prints on a key, through Text Input Sources and
//! `UCKeyTranslate`.
//!
//! It asks as the original's Keys window does (`-[SRKeyCodeTransformer
//! transformedValue:]` in `ShortcutRecorder.framework`, `UCKeyTranslate`
//! @0xae0e): the layout's `uchr` data, `kUCKeyActionDisplay`, no
//! modifiers, `LMGetKbdType()`'s keyboard type, no options (so a dead key
//! answers its spacing character), a fresh dead-key state and a 4-unit
//! buffer. The original reached the layout through `KLGetCurrentKeyboardLayout`,
//! which 64-bit macOS lacks; `TISCopyCurrentKeyboardLayoutInputSource` is its
//! replacement. Which part of the answer to show is the game's: this
//! returns every unit it got.
//!
//! Every call is made on the main thread alone. `HIToolbox`'s input source
//! calls aren't safe from several threads at once: on macOS 27.2, four
//! threads calling them together abort the process, where one thread
//! alone doesn't (rdm:phase/key-bindings/phase-5-key-names-adapters's
//! findings). So off the main thread the answer is `None`.
//!
//! Every `unsafe` block here carries a `SAFETY` comment.

#![allow(unsafe_code)]

use std::ffi::{c_char, c_void};

use objc2::MainThreadMarker;

/// A Core Foundation object (`CFTypeRef`).
type CfType = *const c_void;

/// `kUCKeyActionDisplay`: what the key shows, not what a press types.
const KEY_ACTION_DISPLAY: u16 = 3;

/// `kCFStringEncodingUTF8`.
const CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    /// The current keyboard layout's input source, owned by the caller
    /// (a Copy rule), or null.
    fn TISCopyCurrentKeyboardLayoutInputSource() -> CfType;
    /// A property of `source`, owned by `source` (a Get rule), or null.
    fn TISGetInputSourceProperty(source: CfType, key: CfType) -> CfType;
    /// The key for a layout's `uchr` data, a `CFDataRef`.
    static kTISPropertyUnicodeKeyLayoutData: CfType;
    /// The key for an input source's ID, a `CFStringRef`.
    static kTISPropertyInputSourceID: CfType;
    /// The keyboard type of the last keyboard event.
    fn LMGetKbdType() -> u8;
    /// Translates a key code through a `uchr` layout.
    fn UCKeyTranslate(
        layout: *const c_void,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_key_state: *mut u32,
        max_string_length: usize,
        actual_string_length: *mut usize,
        unicode_string: *mut u16,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetBytePtr(data: CfType) -> *const u8;
    fn CFRelease(object: CfType);
    fn CFStringGetCString(
        string: CfType,
        buffer: *mut c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> u8;
}

/// The current layout's input source, released when dropped.
struct CurrentLayout(CfType);

impl CurrentLayout {
    /// Copies the current layout's input source, or `None` if there is none.
    fn copy(_mtm: MainThreadMarker) -> Option<Self> {
        // SAFETY: no arguments; called on the main thread (the marker), so
        // no other thread is in HIToolbox's input source calls. It returns
        // an owned reference or null, and `Drop` releases the reference.
        let source = unsafe { TISCopyCurrentKeyboardLayoutInputSource() };
        (!source.is_null()).then_some(Self(source))
    }

    /// The source's property for `key`, owned by the source, or null.
    fn property(&self, key: CfType) -> CfType {
        // SAFETY: `self.0` is a live input source this value owns, `key` is
        // one of HIToolbox's property key constants, and the result is not
        // retained, so it lives no longer than `self`, which every caller
        // keeps alive while it reads the result.
        unsafe { TISGetInputSourceProperty(self.0, key) }
    }
}

impl Drop for CurrentLayout {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the non-null reference `copy` received under
        // the Copy rule, released exactly once, here.
        unsafe { CFRelease(self.0) }
    }
}

/// What the player's current keyboard layout prints on the key with
/// `virtual_key_code` (a `kVK_*` code), with no modifier held: every
/// UTF-16 unit `UCKeyTranslate` gives, decoded, with a dead key's spacing
/// character. It copies the layout on every call and caches nothing, so a
/// layout change shows at the next call.
///
/// `None` off the main thread, when the layout has no `uchr` data, or
/// when the translation fails or gives nothing.
#[must_use]
pub fn key_label(virtual_key_code: u16) -> Option<String> {
    let mtm = MainThreadMarker::new()?;
    let layout = CurrentLayout::copy(mtm)?;
    // SAFETY: reading an immutable framework constant.
    let key = unsafe { kTISPropertyUnicodeKeyLayoutData };
    let data = layout.property(key);
    if data.is_null() {
        return None;
    }
    // SAFETY: `data` is the non-null `CFDataRef` the layout owns, alive
    // while `layout` is.
    let uchr = unsafe { CFDataGetBytePtr(data) };
    if uchr.is_null() {
        return None;
    }
    // SAFETY: no arguments; on the main thread, as above.
    let keyboard_type = unsafe { LMGetKbdType() };
    let mut dead_key_state = 0_u32;
    let mut units = [0_u16; 4];
    let mut length = 0_usize;
    // SAFETY: `uchr` points at the layout's `uchr` data, alive while
    // `layout` is; the dead-key state and length are live locals; the
    // buffer holds `units.len()` units, the maximum passed, so
    // `UCKeyTranslate` writes no further.
    let status = unsafe {
        UCKeyTranslate(
            uchr.cast(),
            virtual_key_code,
            KEY_ACTION_DISPLAY,
            0,
            u32::from(keyboard_type),
            0,
            &raw mut dead_key_state,
            units.len(),
            &raw mut length,
            units.as_mut_ptr(),
        )
    };
    let got = units.get(..length).filter(|_| status == 0 && length > 0)?;
    Some(String::from_utf16_lossy(got))
}

/// The current keyboard layout's input source ID, such as
/// `com.apple.keylayout.US`, so a test can tell which layout it reads.
/// `None` off the main thread or when there is none.
#[must_use]
pub fn keyboard_layout_id() -> Option<String> {
    let mtm = MainThreadMarker::new()?;
    let layout = CurrentLayout::copy(mtm)?;
    // SAFETY: reading an immutable framework constant.
    let key = unsafe { kTISPropertyInputSourceID };
    let id = layout.property(key);
    if id.is_null() {
        return None;
    }
    let mut buffer = [0 as c_char; 256];
    // SAFETY: `id` is the non-null `CFStringRef` the layout owns, alive
    // while `layout` is; the buffer's size is passed, and the call writes
    // a NUL-terminated string within it or fails.
    let copied = unsafe {
        CFStringGetCString(
            id,
            buffer.as_mut_ptr(),
            buffer.len() as isize,
            CF_STRING_ENCODING_UTF8,
        )
    };
    if copied == 0 {
        return None;
    }
    // SAFETY: `CFStringGetCString` succeeded, so `buffer` holds a
    // NUL-terminated string.
    let id = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) };
    Some(id.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real names are read on the main thread, in the integration
    /// binary; off it nothing is asked at all.
    #[test]
    fn off_the_main_thread_it_answers_nothing() {
        let answers = std::thread::spawn(|| (key_label(0x00), keyboard_layout_id()))
            .join()
            .expect("returns without panicking");
        assert_eq!(answers, (None, None));
    }
}
