//! [`Pen`]: sets lines of text in one font into a draw list, placed by
//! their baselines as the original places them: a line's top is its size
//! above its baseline. The HUD's panels share it.

use crate::text::TextMetrics;
use crate::{Color, DrawList, Font, Point};

/// Sets lines of text in one font into a draw list, measured by the
/// metrics, if any.
pub(super) struct Pen<'a, 'b> {
    /// Where the text goes.
    pub(super) list: &'a mut DrawList,
    /// The font every line is set in.
    pub(super) font: Font,
    /// Measures the lines, for centring and setting against the right.
    pub(super) metrics: Option<&'b dyn TextMetrics>,
}

impl Pen<'_, '_> {
    /// How wide `text` is at `size`: nothing without metrics.
    pub(super) fn width(&self, text: &str, size: f32) -> f32 {
        self.metrics
            .map_or(0.0, |metrics| metrics.width(self.font, size, text))
    }

    /// Sets `text` from `x` on `baseline` at `size` in `color`, and gives
    /// where it ends.
    pub(super) fn at(&mut self, text: &str, x: f32, baseline: f32, size: f32, color: Color) -> f32 {
        let top = baseline - size;
        self.list
            .text_in(self.font, text, Point::new(x, top), size, None, color);
        x + self.width(text, size)
    }

    /// Sets `text` centred between `left` and `right` on `baseline` at
    /// `size` in `color`.
    pub(super) fn centred(
        &mut self,
        text: &str,
        left: f32,
        right: f32,
        baseline: f32,
        size: f32,
        color: Color,
    ) {
        let x = left + (right - left - self.width(text, size)) / 2.0;
        let x = if self.metrics.is_some() { x } else { left };
        self.at(text, x, baseline, size, color);
    }

    /// Sets `text` centred between `left` and `right` on `baseline` at
    /// `size` in `color`, as `_DrawCenteredText` (@0x263d) does:
    /// `MoveTo(trunc((left + right) / 2) − trunc(StringWidth / 2),
    /// baseline)`, each half truncated toward zero, then `DrawString`,
    /// which never wraps. Without metrics it starts at `left`.
    pub(super) fn centred_text(
        &mut self,
        text: &str,
        left: f32,
        right: f32,
        baseline: f32,
        size: f32,
        color: Color,
    ) {
        let x = if self.metrics.is_some() {
            f32::midpoint(left, right).trunc() - (self.width(text, size) / 2.0).trunc()
        } else {
            left
        };
        self.at(text, x, baseline, size, color);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    /// Where `centred_text` puts `text` between `left` and `right` on
    /// baseline 40 at Geneva 14 (7 a character in [`MonoMetrics`]).
    fn centred_at(
        text: &str,
        left: f32,
        right: f32,
        metrics: Option<&dyn TextMetrics>,
    ) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        let mut pen = Pen {
            list: &mut list,
            font: Font::Geneva,
            metrics,
        };
        pen.centred_text(text, left, right, 40.0, 14.0, Color::DIM);
        list.iter().cloned().collect()
    }

    fn text_at(text: &str, x: f32) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: Point::new(x, 26.0),
            size: 14.0,
            wrap_width: None,
            color: Color::DIM,
        }
    }

    #[test]
    fn centred_text_truncates_both_halves_as_draw_centered_text_does() {
        // (10 + 111) / 2 = 60.5 → 60; 7 / 2 = 3.5 → 3: 57.
        assert_eq!(
            centred_at("a", 10.0, 111.0, Some(&MonoMetrics)),
            [text_at("a", 57.0)]
        );
        // (10 + 110) / 2 = 60; 21 / 2 = 10.5 → 10: 50.
        assert_eq!(
            centred_at("abc", 10.0, 110.0, Some(&MonoMetrics)),
            [text_at("abc", 50.0)]
        );
    }

    #[test]
    fn centred_text_truncates_toward_zero_left_of_the_origin() {
        // (−11 + 0) / 2 = −5.5 → −5; 7 / 2 = 3.5 → 3: −8.
        assert_eq!(
            centred_at("a", -11.0, 0.0, Some(&MonoMetrics)),
            [text_at("a", -8.0)]
        );
    }

    #[test]
    fn without_metrics_centred_text_starts_at_the_left() {
        assert_eq!(centred_at("abc", 10.0, 111.0, None), [text_at("abc", 10.0)]);
    }
}
