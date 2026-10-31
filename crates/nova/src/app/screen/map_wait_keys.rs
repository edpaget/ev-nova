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
//! Past either end, the queued release of every press already replayed
//! still goes, only so what took the press isn't left holding it: the
//! router's own Command, a screen's Alt.

use std::collections::HashSet;

use nova_view::flight::FlightView;
use nova_view::{Input, Key, Showing};

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
        let mut down = HashSet::new();
        while let Some(input) = keys.next() {
            let before = self.flush_mark();
            note_down(&mut down, &input);
            let _ = self.routed_input(&input);
            if self.showing() == Showing::Flight || self.flush_mark().flushed_since(before) {
                self.let_go_of_replayed(keys, down);
                return;
            }
        }
    }

    /// Past a stop, the queued release of each key in `down`, the presses
    /// replayed and not yet let go, in order, and nothing else of `rest`:
    /// its live release has been and gone, so nothing else would let go
    /// of it. A press replayed with no release queued is still physically
    /// down, and its release comes live, once. So does a press dropped
    /// here's, whose key the router noted as it was typed.
    fn let_go_of_replayed(&mut self, rest: impl Iterator<Item = Input>, mut down: HashSet<Key>) {
        for input in rest {
            if let Input::Key {
                key,
                pressed: false,
                ..
            } = input
                && down.remove(&key)
            {
                let _ = self.routed_input(&input);
            }
        }
    }
}

/// Notes in `down` a key `input` presses or lets go of.
fn note_down(down: &mut HashSet<Key>, input: &Input) {
    if let Input::Key { key, pressed, .. } = *input {
        if pressed {
            down.insert(key);
        } else {
            down.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(layers: usize, stages: usize) -> FlushMark {
        FlushMark { layers, stages }
    }

    #[test]
    fn a_layer_more_flushed_and_a_layer_fewer_or_the_same_didnt() {
        assert!(mark(2, 0).flushed_since(mark(1, 0)));
        assert!(!mark(1, 0).flushed_since(mark(1, 0)));
        assert!(!mark(0, 0).flushed_since(mark(1, 0)));
    }

    #[test]
    fn a_change_of_text_flushed_unless_it_leaves_none() {
        assert!(mark(0, 1).flushed_since(mark(0, 2)), "the next in front");
        assert!(mark(0, 1).flushed_since(mark(0, 0)), "one shown afresh");
        assert!(!mark(0, 0).flushed_since(mark(0, 1)), "the last closed");
        assert!(!mark(0, 2).flushed_since(mark(0, 2)), "unchanged");
    }
}
