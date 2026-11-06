//! The [`TextEntry`] port over winit's IME requests
//! (rdm:phase/key-bindings/phase-17-text-entry-port).
//!
//! winit 0.31 turns the IME, and the soft keyboard of iOS and Android, on
//! and off through `Window::request_ime_update`:
//!
//! - **Start** sends `Disable`, which can't fail, then `Enable` with the
//!   field as the cursor area, so the candidate window sits by the field
//!   on macOS, Windows and X11. Disable then enable is winit's own advice
//!   when the focus moves from one field to another, and on iOS it runs
//!   `becomeFirstResponder` again, which shows the soft keyboard the
//!   player hid. Neither hint and purpose nor the surrounding text is
//!   sent: of winit's backends only Wayland reads them.
//! - **Stop** sends `Disable` while started.
//! - A platform with no IME (`NotSupported`: Orbital, the web) is left
//!   alone: typed keys still type.
//! - **The soft keyboard** is iOS's and Android's, whose backends show the
//!   system keyboard on `Enable` (winit-uikit's `becomeFirstResponder`,
//!   winit-android's `show_soft_input`).
//!
//! The adapter talks to the window through [`ImeWindow`], one method, so
//! its tests run over a fake window that refuses what winit's backends
//! refuse.

use std::sync::Arc;

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::{
    ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData, ImeRequestError, Window,
};

use crate::app::PxRect;
use crate::text_entry::TextEntry;

/// The one call of winit's window the adapter makes.
pub trait ImeWindow {
    /// winit's `Window::request_ime_update`.
    ///
    /// # Errors
    ///
    /// As winit's: `NotEnabled`, `AlreadyEnabled` or `NotSupported`.
    fn request_ime_update(&self, request: ImeRequest) -> Result<(), ImeRequestError>;
}

impl ImeWindow for Arc<dyn Window> {
    fn request_ime_update(&self, request: ImeRequest) -> Result<(), ImeRequestError> {
        (**self).request_ime_update(request)
    }
}

/// The platform's text entry over a winit window `W`.
#[derive(Debug)]
pub struct WinitTextEntry<W> {
    window: W,
    /// Whether the IME was enabled and not disabled since.
    started: bool,
}

impl<W: ImeWindow> WinitTextEntry<W> {
    /// Entry over `window`, stopped.
    pub const fn new(window: W) -> Self {
        Self {
            window,
            started: false,
        }
    }
}

impl<W: ImeWindow> TextEntry for WinitTextEntry<W> {
    fn soft_keyboard(&self) -> bool {
        cfg!(any(target_os = "ios", target_os = "android"))
    }

    fn start(&mut self, field: PxRect) {
        // Disable can't fail; winit's backends refuse an Enable while
        // enabled.
        let _ = self.window.request_ime_update(ImeRequest::Disable);
        let data = ImeRequestData::default().with_cursor_area(
            PhysicalPosition::new(field.x, field.y).into(),
            PhysicalSize::new(field.width, field.height).into(),
        );
        let capabilities = ImeCapabilities::new().with_cursor_area();
        self.started = ImeEnableRequest::new(capabilities, data).is_some_and(|enable| {
            self.window
                .request_ime_update(ImeRequest::Enable(enable))
                .is_ok()
        });
    }

    fn stop(&mut self) {
        if std::mem::take(&mut self.started) {
            let _ = self.window.request_ime_update(ImeRequest::Disable);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use winit::dpi::{Position, Size};

    use super::*;
    use crate::text_entry::fixture::check_contract;

    /// A window that keeps winit's rules: `Enable` while enabled is
    /// `AlreadyEnabled`, `Update` before `Enable` is `NotEnabled`, and
    /// `Disable` always succeeds. It records each request with what it
    /// answered.
    #[derive(Default)]
    struct StrictIme {
        enabled: RefCell<bool>,
        supported: bool,
        log: RefCell<Vec<(ImeRequest, Result<(), ImeRequestError>)>>,
    }

    impl StrictIme {
        fn supported() -> Self {
            Self {
                supported: true,
                ..Self::default()
            }
        }

        fn requests(&self) -> Vec<ImeRequest> {
            self.log.borrow().iter().map(|(r, _)| r.clone()).collect()
        }

        fn refusals(&self) -> Vec<ImeRequestError> {
            self.log
                .borrow()
                .iter()
                .filter_map(|(_, answer)| answer.err())
                .collect()
        }
    }

    impl ImeWindow for &StrictIme {
        fn request_ime_update(&self, request: ImeRequest) -> Result<(), ImeRequestError> {
            let mut enabled = self.enabled.borrow_mut();
            let answer = match &request {
                ImeRequest::Disable => {
                    *enabled = false;
                    Ok(())
                }
                _ if !self.supported => Err(ImeRequestError::NotSupported),
                ImeRequest::Enable(_) if *enabled => Err(ImeRequestError::AlreadyEnabled),
                ImeRequest::Enable(_) => {
                    *enabled = true;
                    Ok(())
                }
                ImeRequest::Update(_) if !*enabled => Err(ImeRequestError::NotEnabled),
                _ => Ok(()),
            };
            self.log.borrow_mut().push((request, answer));
            answer
        }
    }

    const FIELD: PxRect = PxRect {
        x: 200,
        y: 400,
        width: 600,
        height: 32,
    };

    /// The cursor area an `Enable` carries, in physical pixels.
    fn cursor_area(request: &ImeRequest) -> Option<(Position, Size)> {
        match request {
            ImeRequest::Enable(enable) => enable.request_data().cursor_area,
            _ => None,
        }
    }

    #[test]
    fn start_disables_then_enables_with_the_field_as_cursor_area() {
        let ime = StrictIme::supported();
        let mut entry = WinitTextEntry::new(&ime);
        entry.start(FIELD);
        let requests = ime.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], ImeRequest::Disable);
        assert_eq!(
            cursor_area(&requests[1]),
            Some((
                PhysicalPosition::new(200, 400).into(),
                PhysicalSize::new(600, 32).into()
            ))
        );
        let ImeRequest::Enable(enable) = &requests[1] else {
            panic!("an enable");
        };
        assert!(enable.capabilities().cursor_area());
        assert!(!enable.capabilities().hint_and_purpose());
        assert!(!enable.capabilities().surrounding_text());
        assert_eq!(ime.refusals(), []);
    }

    #[test]
    fn starting_again_restarts() {
        let ime = StrictIme::supported();
        let mut entry = WinitTextEntry::new(&ime);
        entry.start(FIELD);
        let moved = PxRect { y: 480, ..FIELD };
        entry.start(moved);
        let requests = ime.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[2], ImeRequest::Disable);
        assert_eq!(
            cursor_area(&requests[3]).map(|(at, _)| at),
            Some(PhysicalPosition::new(200, 480).into())
        );
        assert_eq!(ime.refusals(), [], "never enabled while enabled");
    }

    #[test]
    fn stop_disables_once() {
        let ime = StrictIme::supported();
        let mut entry = WinitTextEntry::new(&ime);
        entry.stop();
        assert_eq!(ime.requests(), [], "nothing to stop");
        entry.start(FIELD);
        entry.stop();
        entry.stop();
        let requests = ime.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2], ImeRequest::Disable);
    }

    #[test]
    fn an_unsupported_platform_is_ignored() {
        let ime = StrictIme::default();
        let mut entry = WinitTextEntry::new(&ime);
        entry.start(FIELD);
        entry.stop();
        assert_eq!(ime.refusals(), [ImeRequestError::NotSupported]);
        assert_eq!(
            ime.requests().len(),
            2,
            "no Disable for an entry never started"
        );
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    #[test]
    fn the_desktop_has_no_soft_keyboard() {
        let ime = StrictIme::supported();
        assert!(!WinitTextEntry::new(&ime).soft_keyboard());
    }

    #[test]
    fn winit_text_entry_meets_the_contract() {
        let ime = StrictIme::supported();
        assert_eq!(check_contract(&mut WinitTextEntry::new(&ime)), Ok(()));
        assert_eq!(ime.refusals(), [], "no request winit refuses");
        let unsupported = StrictIme::default();
        assert_eq!(
            check_contract(&mut WinitTextEntry::new(&unsupported)),
            Ok(())
        );
    }
}
