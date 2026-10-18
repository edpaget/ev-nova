//! Display modes as the player picks them, for exclusive fullscreen.
//!
//! - [`VideoMode`]: a size and a refresh rate, as the Preferences
//!   dialog's picker lists it.
//! - [`MonitorMode`]: one of a monitor's modes as the platform reports it,
//!   bit depth included.
//! - [`FullscreenKind`]: how the game runs fullscreen, borderless or in
//!   one exclusive mode.
//! - [`listed_modes`]: the modes the picker offers from a monitor's:
//!   those at least [`MIN_MODE`], each once, largest first.

/// The smallest mode offered, in pixels: the logical space the game draws
/// in (`nova_render::LOGICAL`), so a mode never shrinks the game below its
/// own size.
pub const MIN_MODE: (u32, u32) = (1024, 768);

/// A display mode as the player picks it: a size in pixels and a refresh
/// rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VideoMode {
    /// The width, in pixels.
    pub width: u32,
    /// The height, in pixels.
    pub height: u32,
    /// The refresh rate, in thousandths of a hertz.
    pub refresh_millihertz: u32,
}

impl VideoMode {
    /// The mode as the picker shows it: "1920 x 1080, 60 Hz", or with the
    /// rate's fraction ("59.94 Hz") when it isn't whole, so 59.94 and 60
    /// never read the same.
    #[must_use]
    pub fn label(&self) -> String {
        let whole = self.refresh_millihertz / 1000;
        let fraction = self.refresh_millihertz % 1000;
        let rate = if fraction == 0 {
            whole.to_string()
        } else {
            let digits = format!("{fraction:03}");
            format!("{whole}.{}", digits.trim_end_matches('0'))
        };
        format!("{} x {}, {rate} Hz", self.width, self.height)
    }

    /// The number of pixels it has.
    fn pixels(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// One of a monitor's modes, as the platform reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorMode {
    /// Its size and refresh rate.
    pub mode: VideoMode,
    /// Its colour depth, in bits a pixel.
    pub bit_depth: u16,
}

/// How the game runs fullscreen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FullscreenKind {
    /// Borderless, at the desktop's own mode.
    #[default]
    Borderless,
    /// The display switched to this mode, the game alone on it.
    Exclusive(VideoMode),
}

impl FullscreenKind {
    /// The kind as the picker shows it: "Borderless", or the mode's label.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Borderless => "Borderless".to_owned(),
            Self::Exclusive(mode) => mode.label(),
        }
    }
}

/// The modes the picker offers from a monitor's `modes`: those at least
/// [`MIN_MODE`] in both directions, each size and rate once whatever its
/// bit depths, the most pixels first, then the widest, then the fastest.
#[must_use]
pub fn listed_modes(modes: &[MonitorMode]) -> Vec<VideoMode> {
    let (min_width, min_height) = MIN_MODE;
    let mut listed: Vec<VideoMode> = modes
        .iter()
        .map(|monitor| monitor.mode)
        .filter(|mode| mode.width >= min_width && mode.height >= min_height)
        .collect();
    listed.sort_by(|a, b| {
        (b.pixels(), b.width, b.refresh_millihertz).cmp(&(
            a.pixels(),
            a.width,
            a.refresh_millihertz,
        ))
    });
    listed.dedup();
    listed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(width: u32, height: u32, hz: u32) -> VideoMode {
        VideoMode {
            width,
            height,
            refresh_millihertz: hz * 1000,
        }
    }

    fn at_depth(mode: VideoMode, bit_depth: u16) -> MonitorMode {
        MonitorMode { mode, bit_depth }
    }

    fn monitor(modes: &[VideoMode]) -> Vec<MonitorMode> {
        modes.iter().map(|&mode| at_depth(mode, 32)).collect()
    }

    #[test]
    fn modes_smaller_than_the_game_are_dropped() {
        let modes = monitor(&[
            mode(1280, 720, 60),
            mode(800, 600, 60),
            mode(1024, 600, 60),
            mode(1024, 768, 60),
            mode(1000, 800, 60),
        ]);
        assert_eq!(listed_modes(&modes), [mode(1024, 768, 60)]);
        assert_eq!(MIN_MODE, (1024, 768));
    }

    #[test]
    fn two_depths_of_one_mode_are_listed_once() {
        let fhd = mode(1920, 1080, 60);
        let modes = [at_depth(fhd, 32), at_depth(fhd, 24), at_depth(fhd, 16)];
        assert_eq!(listed_modes(&modes), [fhd]);
    }

    #[test]
    fn two_rates_of_one_size_are_both_listed_the_fastest_first() {
        let modes = monitor(&[mode(1920, 1080, 60), mode(1920, 1080, 144)]);
        assert_eq!(
            listed_modes(&modes),
            [mode(1920, 1080, 144), mode(1920, 1080, 60)]
        );
    }

    #[test]
    fn the_most_pixels_come_first_then_the_widest_then_the_fastest() {
        let modes = monitor(&[
            mode(1024, 768, 60),
            mode(1920, 1080, 60),
            mode(2560, 1440, 60),
            mode(1920, 1080, 144),
            mode(1920, 1200, 60),
            // As many pixels as 1920x1200, narrower.
            mode(1600, 1440, 60),
            mode(1024, 768, 60),
        ]);
        assert_eq!(
            listed_modes(&modes),
            [
                mode(2560, 1440, 60),
                mode(1920, 1200, 60),
                mode(1600, 1440, 60),
                mode(1920, 1080, 144),
                mode(1920, 1080, 60),
                mode(1024, 768, 60),
            ]
        );
    }

    #[test]
    fn no_modes_list_nothing() {
        assert_eq!(listed_modes(&[]), Vec::<VideoMode>::new());
    }

    #[test]
    fn a_label_shows_the_size_and_the_rate_its_fraction_only_when_there_is_one() {
        assert_eq!(mode(1920, 1080, 60).label(), "1920 x 1080, 60 Hz");
        let ntsc = VideoMode {
            refresh_millihertz: 59_940,
            ..mode(1920, 1080, 0)
        };
        assert_eq!(ntsc.label(), "1920 x 1080, 59.94 Hz");
        let odd = VideoMode {
            refresh_millihertz: 74_973,
            ..mode(1024, 768, 0)
        };
        assert_eq!(odd.label(), "1024 x 768, 74.973 Hz");
        let tenth = VideoMode {
            refresh_millihertz: 29_500,
            ..mode(1024, 768, 0)
        };
        assert_eq!(tenth.label(), "1024 x 768, 29.5 Hz");
        let slow = VideoMode {
            refresh_millihertz: 50_005,
            ..mode(1024, 768, 0)
        };
        assert_eq!(slow.label(), "1024 x 768, 50.005 Hz");
    }

    #[test]
    fn a_kind_shows_borderless_or_its_modes_label() {
        assert_eq!(FullscreenKind::Borderless.label(), "Borderless");
        assert_eq!(FullscreenKind::default(), FullscreenKind::Borderless);
        assert_eq!(
            FullscreenKind::Exclusive(mode(2560, 1440, 120)).label(),
            "2560 x 1440, 120 Hz"
        );
    }
}
