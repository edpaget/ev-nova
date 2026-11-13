//! The render scale: the size flight's world layer is drawn at, before the
//! Scaling's [`Upscaler`](crate::Upscaler) enlarges it onto the viewport.
//!
//! It is a plain value, shared by the settings file, the Preferences
//! dialog's list ([`RenderScale::list`], in
//! [`PrefsDialog`](crate::ui::prefs::PrefsDialog)) and the renderer that
//! sizes the world texture with it (`nova_render::world_space`).

/// The size the world layer is drawn at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderScale {
    /// The size the Scaling draws the world at on its own: the window's
    /// resolution under Native (and Super-xBR), the logical resolution
    /// under the enlarging upscalers. It changes nothing.
    #[default]
    Auto,
    /// This percentage of the backing size (the viewport's content in
    /// target pixels), for every Scaling, from [`RenderScale::MIN`] to
    /// [`RenderScale::MAX`]. Build it with [`RenderScale::percent`], which
    /// clamps.
    Percent(u8),
}

impl RenderScale {
    /// The smallest percentage.
    pub const MIN: u8 = 25;
    /// The largest percentage: the backing size itself.
    pub const MAX: u8 = 100;

    /// `p` percent of the backing size, clamped to
    /// [`MIN`](Self::MIN)..=[`MAX`](Self::MAX).
    #[must_use]
    pub fn percent(p: u8) -> Self {
        Self::Percent(p.clamp(Self::MIN, Self::MAX))
    }

    /// What the Preferences dialog steps through: Auto, then the usual
    /// upscaler presets from the backing size down to
    /// [`MIN`](Self::MIN).
    pub const STEPS: [Self; 7] = [
        Self::Auto,
        Self::Percent(100),
        Self::Percent(75),
        Self::Percent(67),
        Self::Percent(50),
        Self::Percent(33),
        Self::Percent(25),
    ];

    /// The Preferences dialog's list for a game drawing at `current`: the
    /// [`STEPS`](Self::STEPS), and `current` (clamped as
    /// [`percent`](Self::percent) clamps) in its place by size when it is
    /// a percentage not among them. The settings file allows any
    /// percentage, so the dialog shows the one running and a change of
    /// another preference keeps it.
    #[must_use]
    pub fn list(current: Self) -> Vec<Self> {
        let mut list = Self::STEPS.to_vec();
        if let Self::Percent(p) = current {
            let p = p.clamp(Self::MIN, Self::MAX);
            if !list.contains(&Self::Percent(p)) {
                let smaller = |entry: &Self| matches!(*entry, Self::Percent(q) if q < p);
                let at = list.iter().position(smaller).unwrap_or(list.len());
                list.insert(at, Self::Percent(p));
            }
        }
        list
    }

    /// Its entry in the Preferences dialog's list. The list's row has no
    /// caption of its own, so each entry names what it chooses.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Auto => "Render Scale: Auto".to_owned(),
            Self::Percent(p) => format!("Render Scale: {p}%"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_render_scale_is_auto() {
        assert_eq!(RenderScale::default(), RenderScale::Auto);
    }

    const P: fn(u8) -> RenderScale = RenderScale::Percent;

    #[test]
    fn the_list_is_auto_then_the_steps_from_the_backing_size_down() {
        let steps = [RenderScale::Auto, P(100), P(75), P(67), P(50), P(33), P(25)];
        assert_eq!(RenderScale::STEPS, steps);
        assert_eq!(RenderScale::list(RenderScale::Auto), steps);
        for listed in steps {
            assert_eq!(RenderScale::list(listed), steps, "{listed:?}");
        }
    }

    #[test]
    fn a_percent_off_the_steps_is_listed_in_its_place_by_size() {
        assert_eq!(
            RenderScale::list(P(60)),
            [
                RenderScale::Auto,
                P(100),
                P(75),
                P(67),
                P(60),
                P(50),
                P(33),
                P(25),
            ]
        );
        assert_eq!(RenderScale::list(P(99))[1..3], [P(100), P(99)]);
        assert_eq!(RenderScale::list(P(26))[6..], [P(26), P(25)]);
        assert_eq!(RenderScale::list(P(74))[2..4], [P(75), P(74)]);
    }

    #[test]
    fn every_entry_listed_is_a_percent_the_scale_allows() {
        for current in [RenderScale::Auto, P(60), P(0), P(255)] {
            let list = RenderScale::list(current);
            assert_eq!(list[0], RenderScale::Auto);
            for entry in &list[1..] {
                let RenderScale::Percent(p) = *entry else {
                    panic!("{entry:?} after the first");
                };
                assert!((RenderScale::MIN..=RenderScale::MAX).contains(&p), "{p}");
            }
            assert!(
                list[1..].windows(2).all(|pair| match (pair[0], pair[1]) {
                    (RenderScale::Percent(a), RenderScale::Percent(b)) => a > b,
                    _ => false,
                }),
                "descending, none twice: {list:?}"
            );
        }
        assert_eq!(RenderScale::list(P(0)).len(), 7, "clamped onto 25");
        assert_eq!(RenderScale::list(P(255)).len(), 7, "clamped onto 100");
    }

    #[test]
    fn each_is_labelled_render_scale_and_auto_or_its_percentage() {
        assert_eq!(RenderScale::Auto.label(), "Render Scale: Auto");
        assert_eq!(P(100).label(), "Render Scale: 100%");
        assert_eq!(P(67).label(), "Render Scale: 67%");
        assert_eq!(P(25).label(), "Render Scale: 25%");
    }

    #[test]
    fn a_percent_is_clamped_to_25_through_100() {
        for (asked, kept) in [
            (0, 25),
            (24, 25),
            (25, 25),
            (67, 67),
            (100, 100),
            (101, 100),
            (255, 100),
        ] {
            assert_eq!(
                RenderScale::percent(asked),
                RenderScale::Percent(kept),
                "{asked}%"
            );
        }
    }
}
