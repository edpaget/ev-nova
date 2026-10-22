//! The window port over a real winit window.
//!
//! Each method maps onto winit's own: the monitor's modes onto
//! `MonitorHandle::video_modes`, and an exclusive kind onto
//! `Fullscreen::Exclusive` with the monitor's deepest handle of that size
//! and rate ([`deepest`]). Wayland lets no client switch the display's
//! mode, and winit does nothing there when asked, so on Wayland the
//! window lists no modes ([`offers_exclusive`]), and the Preferences
//! dialog offers only borderless.

use std::sync::Arc;

use nova_view::{FullscreenKind, MonitorMode, VideoMode};
use winit::dpi::PhysicalSize;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Fullscreen, Window};

use crate::app::WindowPort;

/// The value paired with the deepest of `modes` that has `want`'s size and
/// refresh rate, if any has.
pub(crate) fn deepest<T>(
    modes: impl IntoIterator<Item = (MonitorMode, T)>,
    want: VideoMode,
) -> Option<T> {
    modes
        .into_iter()
        .filter(|(mode, _)| mode.mode == want)
        .max_by_key(|(mode, _)| mode.bit_depth)
        .map(|(_, value)| value)
}

/// One of a monitor's modes, in the core's terms.
fn monitor_mode(handle: &winit::monitor::VideoModeHandle) -> MonitorMode {
    let size = handle.size();
    MonitorMode {
        mode: VideoMode {
            width: size.width,
            height: size.height,
            refresh_millihertz: handle.refresh_rate_millihertz(),
        },
        bit_depth: handle.bit_depth(),
    }
}

/// Whether a window with the raw handle `raw` can enter an exclusive mode:
/// every platform's can but Wayland's.
pub(crate) fn offers_exclusive(raw: Option<RawWindowHandle>) -> bool {
    !matches!(raw, Some(RawWindowHandle::Wayland(_)))
}

/// A [`WindowPort`] over a winit window.
#[derive(Clone, Debug)]
pub struct WinitWindow(pub Arc<Window>);

impl WindowPort for WinitWindow {
    fn size_px(&self) -> (u32, u32) {
        let size = self.0.inner_size();
        (size.width, size.height)
    }

    fn scale_factor(&self) -> f64 {
        self.0.scale_factor()
    }

    fn request_redraw(&mut self) {
        self.0.request_redraw();
    }

    fn set_pointer_visible(&mut self, visible: bool) {
        self.0.set_cursor_visible(visible);
    }

    fn set_fullscreen(&mut self, fullscreen: Option<FullscreenKind>) {
        // On the window's current monitor.
        let fullscreen = fullscreen.map(|kind| match kind {
            FullscreenKind::Borderless => Fullscreen::Borderless(None),
            FullscreenKind::Exclusive(mode) => self
                .0
                .current_monitor()
                .and_then(|monitor| {
                    let modes = monitor
                        .video_modes()
                        .map(|handle| (monitor_mode(&handle), handle));
                    deepest(modes, mode)
                })
                .map_or(Fullscreen::Borderless(None), Fullscreen::Exclusive),
        });
        self.0.set_fullscreen(fullscreen);
    }

    fn video_modes(&self) -> Vec<MonitorMode> {
        let raw = self.0.window_handle().ok().map(|handle| handle.as_raw());
        match self.0.current_monitor() {
            Some(monitor) if offers_exclusive(raw) => monitor
                .video_modes()
                .map(|handle| monitor_mode(&handle))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn monitor_name(&self) -> Option<String> {
        self.0.current_monitor()?.name()
    }

    fn is_fullscreen(&self) -> bool {
        self.0.fullscreen().is_some()
    }

    fn request_inner_size(&mut self, (width, height): (u32, u32)) {
        // The size it takes comes in a resize; the one returned at once
        // on some platforms adds nothing to that.
        let _ = self.0.request_inner_size(PhysicalSize::new(width, height));
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;
    use std::ptr::NonNull;

    use winit::raw_window_handle::{AppKitWindowHandle, WaylandWindowHandle, Win32WindowHandle};

    use super::*;

    fn fhd(hz: u32) -> VideoMode {
        VideoMode {
            width: 1920,
            height: 1080,
            refresh_millihertz: hz * 1000,
        }
    }

    fn at(mode: VideoMode, bit_depth: u16) -> MonitorMode {
        MonitorMode { mode, bit_depth }
    }

    #[test]
    fn no_mode_of_the_size_and_rate_matches_nothing() {
        assert_eq!(deepest([(at(fhd(144), 32), "144")], fhd(60)), None);
        assert_eq!(deepest(Vec::<(MonitorMode, &str)>::new(), fhd(60)), None);
    }

    #[test]
    fn the_one_mode_of_the_size_and_rate_matches() {
        let modes = [(at(fhd(144), 32), "144"), (at(fhd(60), 24), "60")];
        assert_eq!(deepest(modes, fhd(60)), Some("60"));
    }

    #[test]
    fn of_two_depths_the_deepest_matches() {
        let modes = [
            (at(fhd(60), 16), "16"),
            (at(fhd(60), 32), "32"),
            (at(fhd(60), 24), "24"),
        ];
        assert_eq!(deepest(modes, fhd(60)), Some("32"));
    }

    #[test]
    fn only_wayland_offers_no_exclusive_modes() {
        let surface = NonNull::<c_void>::dangling();
        let wayland = RawWindowHandle::Wayland(WaylandWindowHandle::new(surface));
        assert!(!offers_exclusive(Some(wayland)));
        let appkit = RawWindowHandle::AppKit(AppKitWindowHandle::new(surface));
        assert!(offers_exclusive(Some(appkit)));
        let hwnd = std::num::NonZeroIsize::new(1).expect("not zero");
        assert!(offers_exclusive(Some(RawWindowHandle::Win32(
            Win32WindowHandle::new(hwnd)
        ))));
        assert!(offers_exclusive(None), "no handle: let winit decide");
    }
}
