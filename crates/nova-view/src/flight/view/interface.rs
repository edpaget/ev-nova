//! [`FlightView`]'s status bar and description reads.

use nova_sim::GovtId;
use nova_sim::InterfaceLoad;

use crate::flight::catalog::StatusBars;
use crate::flight::hud::{self};
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};

use super::{BarFor, FlightView, Then};

impl<C: StatusBars> FlightView<C, crate::flight::SessionFlight<C>> {
    /// Chooses the status bar for the ship's government, as
    /// `_LoadIntfResource` loads it ([`hud::rechoose_status_bar`]), unless
    /// a mission text still shown holds the bar a `C`, `E` or `H` after it
    /// loads ([`FlightView::bar_held`]). Called at every
    /// [`FlightView::catch_up`] and as a text closes.
    ///
    /// The original loads the bar inside the purchase, capture or set
    /// expression. Ours waits for the next `catch_up`, on the next frame or
    /// in the view call that drove the change, which is still before any
    /// frame is drawn: in flight, or docked with no text shown, the player
    /// never sees the old bar on the new ship. Under a docked mission text
    /// queued before the operator, the original shows the old bar
    /// (`_LoadIntfResource` @0x156c7 comes after the text), and so does
    /// ours, until that text's close redraws at the operator
    /// ([`FlightView::close_mission_text`]). A redraw that drops a held
    /// text's redraws ([`FlightView::redraw_status`]) lets go of the hold
    /// at the next `catch_up`, a tick at most, before the next frame.
    pub(super) fn follow_bar(&mut self) {
        if self.bar_held() {
            return;
        }
        if let Ok(session) = &self.sim.session {
            let government = session.interface_government();
            self.choose_bar(government);
        }
    }

    /// Chooses the status bar for `government` ([`hud::rechoose_status_bar`])
    /// when the bar shown was chosen for another: the original returns at
    /// once for the `ïntf` already loaded (@0x73663-0x73672), and asking
    /// only for another government does the same, as a ship type's
    /// government picks its `ïntf` ([`Session::interface_government`]).
    pub(super) fn choose_bar(&mut self, government: Option<GovtId>) {
        if self.bar_government == Some(BarFor(government)) {
            return;
        }
        self.status_bar =
            hud::rechoose_status_bar(&*self.catalog, government, self.status_bar.as_ref().ok());
        self.bar_government = Some(BarFor(government));
    }

    /// The `ïntf` the ship flies under now, as [`hud::interface_for`]
    /// reads it from the session's government; none for a session that
    /// failed. A ship purchase and an outfitter order each compare it
    /// before and after, as `_LoadIntfResource` redraws the whole bar
    /// only for an `ïntf` other than the one loaded (@0x73663-0x73672).
    pub(super) fn interface_now(&self) -> Option<Result<i16, String>> {
        let session = self.sim.session.as_ref().ok()?;
        Some(hud::interface_for(
            &*self.catalog,
            session.interface_government(),
        ))
    }

    /// The `C`, `E` and `H` the session ran while docked since its last
    /// run of docked scripts began ([`Session::take_interface_loads`])
    /// that each loaded another `ïntf`, in order, from the `ïntf` loaded
    /// before them (`before`, [`FlightView::interface_now`]), as
    /// `_LoadIntfResource` compares each with the one loaded at that
    /// moment (`_currentIntf`, @0x73663-0x73672); and the `ïntf` loaded
    /// after them.
    pub(super) fn interface_changes(
        &mut self,
        before: Option<&Result<i16, String>>,
    ) -> (Vec<InterfaceLoad>, Option<Result<i16, String>>) {
        let loads = self.drain().take_interface_loads();
        let mut loaded = before.cloned();
        let mut changes = Vec::new();
        for load in loads {
            let interface = hud::interface_for(&*self.catalog, load.government);
            if loaded.as_ref() != Some(&interface) {
                loaded = Some(interface);
                changes.push(load);
            }
        }
        (changes, loaded)
    }

    /// Runs `run` on the session, as the docked bar sees it: the `ïntf`
    /// loaded before it is noted, and what it ran docked is staged
    /// ([`FlightView::stage_bar`]), the dialog drawing `then` after it. In
    /// flight `run` runs alone.
    pub(super) fn run_docked<T>(&mut self, then: Then, run: impl FnOnce(&mut Self) -> T) -> T {
        if !self.docked() {
            return run(self);
        }
        let before = self.interface_now();
        let done = run(self);
        let (changes, _) = self.interface_changes(before.as_ref());
        self.stage_bar(changes, then);
        done
    }
}

impl<C: DescriptionSource + DescriptionGraphics> FlightView<C, crate::flight::SessionFlight<C>> {
    /// `dësc` `id`, loaded as `_LoadDescription` loads one for a screen
    /// that shows it now: read from the catalog, then taken in by the
    /// session ([`Session::load_description`]), its conditional text
    /// resolved for the pilot, the remembered rank governments reset and
    /// parsed, its movie the last loaded ([`FlightView::loaded_after_movie`]),
    /// and in the `l33t` mode its letters drawn on the flight's chance.
    /// With a session that failed, only its conditional text is resolved,
    /// for a new pilot ([`Gate::FRESH`](nova_sim::control::Gate::FRESH)).
    ///
    /// # Errors
    ///
    /// Why the `dësc` cannot be read; the governments are kept and the
    /// movie emptied, as a missing one's are (@0x71a5e-0x71a77).
    pub fn load_description(&mut self, id: i16) -> Result<String, String> {
        let raw = match self.catalog.description(id) {
            Ok(raw) => raw,
            Err(reason) => {
                if let Ok(session) = &mut self.sim.session {
                    let _ = session.load_description(None, None, &mut self.sim.chance);
                }
                return Err(reason);
            }
        };
        let movie = self.catalog.description_movie(id);
        Ok(match &mut self.sim.session {
            Ok(session) => {
                session.load_description(Some(&raw), movie.as_ref(), &mut self.sim.chance)
            }
            Err(_) => nova_sim::text_parse::conditional_text(
                &raw,
                nova_sim::control::Gate::FRESH.pilot,
                nova_sim::text_parse::ParseRules::default(),
            ),
        })
    }
}
