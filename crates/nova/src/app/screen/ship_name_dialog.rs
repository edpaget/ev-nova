//! [`AppScreen`]'s Text Input dialog (`DLOG` 3001) asking a ship's name,
//! as both the christening of a new pilot's ship (`christening`) and the
//! rename of a ship captured (`capture_naming`) open it.

use std::rc::Rc;

use nova_sim::shipyard::SHIP_NAME_CHARS;
use nova_view::text::TextMetrics;
use nova_view::ui::text_input::{TEXT_INPUT_DIALOG, TextInputDialog};
use nova_view::ui::{DescriptionSource, DialogResources};

use super::AppScreen;

impl AppScreen {
    /// The Text Input dialog asking a ship's name, of at most
    /// `SHIP_NAME_CHARS`, on `prompt` and `default`: the interface file's,
    /// laid out with the dialogs' metrics, or the built-in one without it,
    /// laid out with `fallback_metrics` (with a warning when the interface
    /// file has none). Opened `by_key`, the text the key that opened it
    /// types goes nowhere, as `_EVTextInputDialog` flushes events as it
    /// opens (@0x55429).
    pub(super) fn ship_name_dialog(
        &mut self,
        prompt: &str,
        default: &str,
        by_key: bool,
        fallback_metrics: impl FnOnce(&Self) -> Rc<dyn TextMetrics>,
    ) -> TextInputDialog {
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(TEXT_INPUT_DIALOG)
                .and_then(|template| {
                    TextInputDialog::new(
                        &template,
                        prompt,
                        default,
                        SHIP_NAME_CHARS,
                        style,
                        Rc::clone(&dialogs.metrics),
                    )
                })
        });
        let mut dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings.push(format!(
                        "nova: using the built-in Text Input dialog: {reason}"
                    ));
                }
                let metrics = fallback_metrics(self);
                TextInputDialog::fallback(prompt, default, SHIP_NAME_CHARS, style, metrics)
            }
        };
        if by_key {
            dialog.flush_typed_key();
        }
        dialog
    }
}
