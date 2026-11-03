//! The closes that wait for their key's release, routed: the player info
//! screen's for P (`_DoPlayerInfoDialog` @0x5a6aa-0x5a6d4) and a mission
//! offer's for I (`_OfferOneMission` @0xa35fc-0xa3614). All addresses are
//! in `EV Nova.app`'s `EV Nova` (i386).
//!
//! Each close still acts on its press. Only the dialog's disposal and
//! what follows it wait, so no key acts on a hold or a repeat. The router
//! reads the keys live ([`LiveInput::is_down`](super::live_input::LiveInput::is_down)): the player info
//! screen opens over flight held as P is, and the spaceport is told
//! before each input and tick ([`SpaceportView::set_close_keys`]). An
//! offer over flight answered with I down stays in front, drawn or
//! hidden, until I's release: then it is dropped, the death's fade may
//! begin, what its answering key left is rerouted, and what was typed
//! during the wait is replayed (the `map_wait_keys` module).

use nova_view::Input;
use nova_view::spaceport::CloseKeys;
use nova_view::spaceport::SpaceportView;
use nova_view::ui::mission_info::MISSION_INFO_KEY;
use nova_view::ui::player_info::PLAYER_INFO_KEY;

use super::AppScreen;

impl AppScreen {
    /// Tells the spaceport which of the keys a close waits on are down.
    pub(super) fn tell_close_keys(&mut self) {
        let keys = CloseKeys {
            player_info: self.live.is_down(PLAYER_INFO_KEY),
            mission_info: self.live.is_down(MISSION_INFO_KEY),
        };
        if let Some(spaceport) = &mut self.spaceport {
            spaceport.set_close_keys(keys);
        }
    }

    /// Replays what the spaceport's player info close left
    /// ([`SpaceportView::take_wait_replay`]).
    pub(super) fn replay_spaceport_wait(&mut self) {
        let replay = self
            .spaceport
            .as_mut()
            .map(SpaceportView::take_wait_replay)
            .unwrap_or_default();
        self.replay_wait_keys(replay);
    }

    /// Disposes of the answered offer over flight: at once with I up, the
    /// offer dropped, the death's fade begun if due, and `left`, what its
    /// answering key left, rerouted; with I down, `left` is kept for the
    /// wait's end ([`AppScreen::offer_wait_input`]).
    pub(super) fn dispose_offer(&mut self, left: Vec<Input>) {
        let held = self.live.is_down(MISSION_INFO_KEY);
        let Some(offer) = &mut self.offer else {
            return;
        };
        if offer.dispose(held) {
            self.finish_offer(&left, Vec::new());
        } else {
            self.offer_left = left;
        }
    }

    /// Gives `input` to the offer over flight while its disposal waits:
    /// whether it took it. On I's release the offer goes as
    /// [`AppScreen::dispose_offer`] says, then what was typed during the
    /// wait is replayed.
    pub(super) fn offer_wait_input(&mut self, input: &Input) -> bool {
        let Some(offer) = self.offer.as_mut().filter(|offer| offer.waiting()) else {
            return false;
        };
        let _ = nova_view::Screen::input(offer, input);
        if offer.disposed() {
            let replay = offer.take_replay();
            let left = std::mem::take(&mut self.offer_left);
            self.finish_offer(&left, replay);
        }
        true
    }

    /// Settles the waits that letting go of the keys ended (a wait ended
    /// by losing focus drops its queue): the player info screen over
    /// flight closed ([`AppScreen::drop_closed_player_info`]), and an
    /// offer over flight disposed of
    /// goes as [`AppScreen::dispose_offer`] says.
    pub(super) fn settle_released_waits(&mut self) {
        self.drop_closed_player_info();
        if self
            .offer
            .as_ref()
            .is_some_and(|offer| offer.answered() && offer.disposed())
        {
            let left = std::mem::take(&mut self.offer_left);
            self.finish_offer(&left, Vec::new());
        }
    }

    /// The end of an offer's close: the offer dropped, the death's fade
    /// begun if due, `left` rerouted and `replay` replayed.
    fn finish_offer(&mut self, left: &[Input], replay: Vec<Input>) {
        self.offer = None;
        self.fade_once_dialogs_close();
        for input in left {
            let _ = self.rerouted_input(input);
        }
        self.replay_wait_keys(replay);
    }
}
