//! The window port over a real winit window.
//!
//! Each method maps onto winit's own: the monitor's modes onto
//! `MonitorHandle::video_modes`, and an exclusive kind onto
//! `Fullscreen::Exclusive` with the monitor's deepest handle of that size
//! and rate ([`deepest`]). Wayland lets no client switch the display's
//! mode, and winit does nothing there when asked, so on Wayland the
//! window lists no modes ([`offers_exclusive`]), and the Preferences
//! dialog offers only borderless.

use std::borrow::Cow;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::Arc;

use nova_view::{FullscreenKind, MonitorMode, VideoMode};
use winit::dpi::{PhysicalInsets, PhysicalSize};
use winit::monitor::{Fullscreen, VideoMode as WinitMode};
use winit::raw_window_handle::RawWindowHandle;
use winit::window::Window;

use crate::app::{PxRect, WindowPort};

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

/// One of a monitor's modes, in the core's terms. A bit depth or refresh
/// rate the platform doesn't give is 0, so [`deepest`] prefers any known
/// depth, and a saved mode, whose rate is known, never matches an unknown
/// one.
fn monitor_mode(mode: &WinitMode) -> MonitorMode {
    let size = mode.size();
    MonitorMode {
        mode: VideoMode {
            width: size.width,
            height: size.height,
            refresh_millihertz: mode.refresh_rate_millihertz().map_or(0, NonZeroU32::get),
        },
        bit_depth: mode.bit_depth().map_or(0, NonZeroU16::get),
    }
}

/// The part of a `surface` that `insets` leave uncovered, as the window
/// port's safe area: from the surface's top-left, `left` across and `top`
/// down, as wide and tall as the insets on both sides leave it. Insets
/// too large for the surface leave an empty rect at its edge, never one
/// outside it.
pub(crate) fn safe_rect(surface: PhysicalSize<u32>, insets: PhysicalInsets<u32>) -> PxRect {
    PxRect {
        x: insets.left.min(surface.width),
        y: insets.top.min(surface.height),
        width: surface
            .width
            .saturating_sub(insets.left)
            .saturating_sub(insets.right),
        height: surface
            .height
            .saturating_sub(insets.top)
            .saturating_sub(insets.bottom),
    }
}

/// Whether a window with the raw handle `raw` can enter an exclusive mode:
/// every platform's can but Wayland's.
pub(crate) fn offers_exclusive(raw: Option<RawWindowHandle>) -> bool {
    !matches!(raw, Some(RawWindowHandle::Wayland(_)))
}

/// `attributes` with the executable's own icon (the resource
/// `nova_winres::ICON_ID`, which `build.rs` embeds) in the title bar,
/// Alt+Tab and the taskbar, rather than Windows' generic one: Explorer
/// reads the icon from the file, but a window has only the icon it is
/// given. An icon that cannot be loaded leaves Windows' own.
#[cfg(windows)]
pub(crate) fn with_exe_icon(
    attributes: winit::window::WindowAttributes,
) -> winit::window::WindowAttributes {
    use winit::icon::Icon;
    use winit::platform::windows::{WinIcon, WindowAttributesWindows};
    let icon = WinIcon::from_resource(nova_winres::ICON_ID, None)
        .ok()
        .map(Icon::from);
    attributes
        .with_window_icon(icon.clone())
        .with_platform_attributes(Box::new(
            WindowAttributesWindows::default().with_taskbar_icon(icon),
        ))
}

/// `attributes` with the game's app id ([`APP_ID`](super::APP_ID)) as
/// the window's Wayland app id (on the Wayland backend, `wayland`) or its
/// X11 `WM_CLASS` (class and instance) otherwise, so the desktop file of
/// that name gives the dock and task switcher the game's icon and name.
/// Since winit 0.31 each backend has its own attributes, set in the one
/// platform slot, so the backend running decides which.
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) fn with_app_id(
    attributes: winit::window::WindowAttributes,
    wayland: bool,
) -> winit::window::WindowAttributes {
    use winit::platform::wayland::WindowAttributesWayland;
    use winit::platform::x11::WindowAttributesX11;
    let id = super::APP_ID;
    if wayland {
        attributes.with_platform_attributes(Box::new(
            WindowAttributesWayland::default().with_name(id, id),
        ))
    } else {
        attributes
            .with_platform_attributes(Box::new(WindowAttributesX11::default().with_name(id, id)))
    }
}

/// A [`WindowPort`] over a winit window.
#[derive(Clone, Debug)]
pub struct WinitWindow(pub Arc<dyn Window>);

impl WindowPort for WinitWindow {
    fn size_px(&self) -> (u32, u32) {
        let size = self.0.surface_size();
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
                        .map(|video_mode| (monitor_mode(&video_mode), video_mode));
                    let chosen = deepest(modes, mode)?;
                    Some(Fullscreen::Exclusive(monitor, chosen))
                })
                .unwrap_or(Fullscreen::Borderless(None)),
        });
        self.0.set_fullscreen(fullscreen);
    }

    fn video_modes(&self) -> Vec<MonitorMode> {
        let raw = self
            .0
            .rwh_06_window_handle()
            .window_handle()
            .ok()
            .map(|handle| handle.as_raw());
        match self.0.current_monitor() {
            Some(monitor) if offers_exclusive(raw) => monitor
                .video_modes()
                .map(|video_mode| monitor_mode(&video_mode))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn monitor_name(&self) -> Option<String> {
        self.0.current_monitor()?.name().map(Cow::into_owned)
    }

    fn is_fullscreen(&self) -> bool {
        self.0.fullscreen().is_some()
    }

    fn request_inner_size(&mut self, (width, height): (u32, u32)) {
        // The size it takes comes in a resize; the one returned at once
        // on some platforms adds nothing to that.
        let _ = self
            .0
            .request_surface_size(PhysicalSize::new(width, height).into());
    }

    fn safe_area(&self) -> PxRect {
        safe_rect(self.0.surface_size(), self.0.safe_area())
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

    /// winit's mode of `width` × `height`, at `bit_depth` bits and
    /// `millihertz`, each `None` where the platform doesn't say.
    fn winit_mode(
        (width, height): (u32, u32),
        bit_depth: Option<u16>,
        millihertz: Option<u32>,
    ) -> WinitMode {
        WinitMode::new(
            PhysicalSize::new(width, height),
            bit_depth.and_then(NonZeroU16::new),
            millihertz.and_then(NonZeroU32::new),
        )
    }

    #[test]
    fn a_known_mode_is_its_size_rate_and_depth() {
        assert_eq!(
            monitor_mode(&winit_mode((1920, 1080), Some(32), Some(59_940))),
            MonitorMode {
                mode: VideoMode {
                    width: 1920,
                    height: 1080,
                    refresh_millihertz: 59_940
                },
                bit_depth: 32
            }
        );
    }

    #[test]
    fn an_unknown_depth_is_zero() {
        let mode = monitor_mode(&winit_mode((1920, 1080), None, Some(60_000)));
        assert_eq!(mode, at(fhd(60), 0));
    }

    #[test]
    fn an_unknown_rate_is_zero() {
        let mode = monitor_mode(&winit_mode((1920, 1080), Some(24), None));
        assert_eq!(mode.mode.refresh_millihertz, 0);
        assert_eq!(mode.bit_depth, 24);
    }

    #[test]
    fn deepest_prefers_a_known_depth_to_an_unknown_one() {
        let modes = [
            winit_mode((1920, 1080), None, Some(60_000)),
            winit_mode((1920, 1080), Some(16), Some(60_000)),
        ];
        let chosen = deepest(modes.iter().map(|m| (monitor_mode(m), *m)), fhd(60));
        assert_eq!(chosen, Some(modes[1]));
    }

    #[test]
    fn a_saved_rate_never_matches_an_unknown_one() {
        let unknown = winit_mode((1920, 1080), Some(32), None);
        assert_eq!(deepest([(monitor_mode(&unknown), unknown)], fhd(60)), None);
    }

    fn insets(top: u32, left: u32, bottom: u32, right: u32) -> PhysicalInsets<u32> {
        PhysicalInsets::new(top, left, bottom, right)
    }

    fn rect(x: u32, y: u32, width: u32, height: u32) -> PxRect {
        PxRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn no_insets_leave_the_whole_surface_safe() {
        let surface = PhysicalSize::new(1024, 768);
        assert_eq!(
            safe_rect(surface, insets(0, 0, 0, 0)),
            rect(0, 0, 1024, 768)
        );
    }

    #[test]
    fn a_notch_s_top_inset_moves_the_top_down() {
        let surface = PhysicalSize::new(2560, 1664);
        assert_eq!(
            safe_rect(surface, insets(64, 0, 0, 0)),
            rect(0, 64, 2560, 1600)
        );
    }

    #[test]
    fn insets_on_every_side_shrink_it_from_each() {
        let surface = PhysicalSize::new(2388, 1668);
        assert_eq!(
            safe_rect(surface, insets(48, 20, 40, 30)),
            rect(20, 48, 2338, 1580)
        );
    }

    #[test]
    fn insets_larger_than_the_surface_leave_an_empty_rect_at_its_edge() {
        let surface = PhysicalSize::new(100, 80);
        assert_eq!(
            safe_rect(surface, insets(90, 70, 0, 50)),
            rect(70, 80, 0, 0)
        );
        assert_eq!(
            safe_rect(surface, insets(10, 120, 5, 0)),
            rect(100, 10, 0, 65)
        );
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

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    mod app_id {
        use winit::platform::wayland::WindowAttributesWayland;
        use winit::platform::x11::WindowAttributesX11;
        use winit::window::WindowAttributes;

        use super::super::with_app_id;
        use crate::platform::APP_ID;

        #[test]
        fn the_wayland_backend_gets_wayland_attributes_named_for_the_app() {
            let attributes = with_app_id(WindowAttributes::default(), true);
            let platform = attributes.platform.as_deref().expect("set");
            let wayland = platform
                .cast_ref::<WindowAttributesWayland>()
                .expect("Wayland's");
            let named = WindowAttributesWayland::default().with_name(APP_ID, APP_ID);
            assert_eq!(format!("{wayland:?}"), format!("{named:?}"));
            assert!(platform.cast_ref::<WindowAttributesX11>().is_none());
        }

        #[test]
        fn the_x11_backend_gets_x11_attributes_named_for_the_app() {
            let attributes = with_app_id(WindowAttributes::default(), false);
            let platform = attributes.platform.as_deref().expect("set");
            let x11 = platform.cast_ref::<WindowAttributesX11>().expect("X11's");
            let named = WindowAttributesX11::default().with_name(APP_ID, APP_ID);
            assert_eq!(format!("{x11:?}"), format!("{named:?}"));
            assert!(platform.cast_ref::<WindowAttributesWayland>().is_none());
        }
    }
}
