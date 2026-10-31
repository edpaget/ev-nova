//! The keys typed while a map over a screen waits for M's release,
//! replayed to the screen under it as the map closes. All addresses are
//! in `EV Nova.app`'s `EV Nova` (i386).
//!
//! `_DoSystemMap`'s wait reads no events and its closing flush keeps keys
//! (@0x13a2e-0x13a46, @0x13c44-0x13c53), so they reach whoever reads the
//! queue next: [`FlightView::take_map_replay`] hands them over, and
//! `nova_view`'s `map_wait` module says which. Each goes through the
//! router in turn, as the original's next loop reads the queue:
//!
//! - A key that closes a dialog back to another passes the rest on.
//! - A key that opens something that flushes drops the rest, as each
//!   `FlushEvents(0xff7f, 0)` does: [`FlushMark`] tells. So does one that
//!   brings a next text or movie to the front.
//! - Past the last dialog, nothing reads them: flight polls its keys live
//!   (`_IterateInGameEventLoop` drops them, @0x56a6b, @0x56cec).
//!
//! A press's release goes with it, past either end, only so what took
//! the press isn't left holding it.

use nova_view::flight::FlightView;
use nova_view::{Input, Showing};

use super::AppScreen;

/// What says that a key opened something that flushes the queue: the
/// layers open that flushed as they opened, and the texts shown or queued.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FlushMark {
    layers: usize,
    stages: usize,
}

impl FlushMark {
    /// Whether anything opened through a flush since `before`: a layer
    /// more, or a change of text or movie that leaves one showing. A
    /// layer fewer flushes nothing, and neither does the last text closing.
    fn flushed_since(self, before: Self) -> bool {
        self.layers > before.layers || (self.stages != before.stages && self.stages > 0)
    }
}

/// Whether `release` lets go of the key `press` went down on.
fn lets_go_of(release: &Input, press: &Input) -> bool {
    matches!(
        (*release, *press),
        (
            Input::Key { key, pressed: false, .. },
            Input::Key { key: down, pressed: true, .. },
        ) if key == down
    )
}

impl AppScreen {
    /// Where the screens stand for [`FlushMark`]: over the port, what the
    /// spaceport says; over flight, the overlays open, of which the
    /// mission dialogs count for their mission info screen alone, as
    /// their texts count apart.
    fn flush_mark(&self) -> FlushMark {
        let port = self.spaceport_view();
        let missions = self.missions.as_ref();
        let info = missions.is_some_and(|dialogs| dialogs.info().is_some());
        FlushMark {
            layers: self.flushing_overlays()
                + usize::from(info)
                + port.map_or(0, nova_view::spaceport::SpaceportView::flushing_layers),
            stages: missions.map_or(
                0,
                nova_view::ui::mission_dialogs::MissionDialogs::text_stages,
            ) + port.map_or(0, nova_view::spaceport::SpaceportView::text_stages),
        }
    }

    /// Replays what the map that just closed left for the screen under
    /// it ([`FlightView::take_map_replay`]), as the module docs say.
    pub(super) fn replay_map_wait_keys(&mut self) {
        let replay = self
            .flight
            .as_mut()
            .map(FlightView::take_map_replay)
            .unwrap_or_default();
        let mut keys = replay.into_iter();
        while let Some(input) = keys.next() {
            let before = self.flush_mark();
            let _ = self.routed_input(&input);
            if self.showing() == Showing::Flight || self.flush_mark().flushed_since(before) {
                if let Some(release) = keys.find(|queued| lets_go_of(queued, &input)) {
                    let _ = self.routed_input(&release);
                }
                return;
            }
        }
    }
}
