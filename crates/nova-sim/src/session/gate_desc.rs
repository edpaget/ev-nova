//! [`Session::gate_entry_desc`]: which `dësc` a hypergate's or wormhole's
//! entry loads for its movie, as `_PlayQTMovieFromDesc` (@0x55f11) is
//! called by `_PlayerEnterHypergate` (@0x639c6-0x639d8) and
//! `_PlayerEnterWormhole` (@0x6426b-0x6427d), asked by the view before it
//! sends the entry ([`ScreenAsk::GateEntryDesc`]).
//!
//! Both callers reach the movie only past their refusals: a hypergate's
//! cancel or a pick no link leads to goes to the refusal at @0x6392c, and
//! a wormhole with no exit to @0x6461c, each returning before the movie.
//! So the answer is `None` wherever the entry would be refused, and the
//! view then sends the entry at once.
//!
//! The ID is `player+0x6c` + 128. A wormhole's `+0x6c` is still the
//! wormhole landed on, so it loads its own `dësc`. A hypergate's entry sets
//! `+0x6c` to -1 (@0x637f2) and `_DoSystemMap` leaves the picked system's
//! index there (@0x63887-0x63897, matched against the links @0x638fb), so
//! by [`RuleKey::HypergateMovieDesc`]'s engine reading it loads the `dësc`
//! numbered as the picked `sÿst`, and by the other reading the gate's own.
//!
//! The refusals are the entry's own, read without acting: the pending
//! gate ([`Session::pending_gate`]) and the hypergate's route
//! ([`Session::hypergate_route`]), which [`Session::enter_hypergate`]
//! reads too, and [`gate::wormhole_leads_anywhere`], which the wormhole's
//! exit reads too.
//!
//! [`ScreenAsk::GateEntryDesc`]: crate::flight_frame::ScreenAsk::GateEntryDesc

use super::Session;
use crate::PilotCatalog;
use crate::catalog::StellarId;
use crate::flight_frame::GateEntry;
use crate::gate::{self, GateKind};
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// The `dësc` ID `entry`'s movie loads, read from `catalog`'s gate
    /// sites without changing anything or rolling: `None` when no entry of
    /// that kind is pending, or when the entry would be refused (a
    /// hypergate without links, no pick or a pick no link leads to, a
    /// wormhole that leads nowhere, [`gate::wormhole_leads_anywhere`]). A
    /// wormhole's is its own `spöb` ID; a hypergate's is as
    /// [`RuleKey::HypergateMovieDesc`] reads (see the module docs).
    #[must_use]
    pub fn gate_entry_desc(
        &self,
        entry: GateEntry,
        catalog: &(impl PilotCatalog + ?Sized),
    ) -> Option<i16> {
        let kind = match entry {
            GateEntry::Hypergate(_) => GateKind::Hypergate,
            GateEntry::Wormhole => GateKind::Wormhole,
        };
        let here = self.pending_gate(kind)?;
        let sites = catalog.gate_sites();
        match entry {
            GateEntry::Hypergate(to) => {
                let (to, _) = self.hypergate_route(here, to, &sites).ok()?;
                Some(
                    match self.setup.rules.source_for(RuleKey::HypergateMovieDesc) {
                        RuleSource::Engine => to.0,
                        RuleSource::Bible => here.0,
                    },
                )
            }
            GateEntry::Wormhole => {
                let player = self.pilot.system;
                let owner_of = |id: StellarId| {
                    self.chart
                        .galaxy
                        .holder(id, self.statics.listers(id), player)
                };
                gate::wormhole_leads_anywhere(here, player, &sites, owner_of).then_some(here.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;
    use crate::gate::GateRefusal;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{at_gate, at_wormhole, gate_site, gated, wormholes};
    use crate::testkit::FakePilotCatalog;
    use nova_sim_base::fixture::Scripted;

    const WORMHOLE: u16 = 0x2000;

    #[test]
    fn a_hypergate_pick_a_link_leads_to_loads_the_picked_systems_desc_by_the_engine() {
        let catalog = gated();
        let session = at_gate(&catalog);
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(131))), &catalog),
            Some(131),
            "player+0x6c, the pick, + 128 (@0x639cc)"
        );
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(132))), &catalog),
            Some(132)
        );
    }

    #[test]
    fn by_the_other_reading_a_hypergate_loads_its_own_desc() {
        let catalog = gated();
        let session = at_gate(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::HypergateMovieDesc, RuleSource::Bible),
        );
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(131))), &catalog),
            Some(300)
        );
    }

    #[test]
    fn a_hypergate_entry_that_would_be_refused_loads_nothing() {
        let catalog = gated();
        let session = at_gate(&catalog);
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(None), &catalog),
            None,
            "a cancel (@0x6392c)"
        );
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(140))), &catalog),
            None,
            "a pick no link leads to (@0x6392c)"
        );
        assert_eq!(
            session.gate_entry_desc(GateEntry::Wormhole, &catalog),
            None,
            "not a wormhole"
        );
        let unlinked = FakePilotCatalog {
            gates: vec![gate_site(300, (0.0, 0.0), 0x1200, &[], 120)],
            ..gated()
        };
        let session = at_gate(&unlinked);
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(131))), &unlinked),
            None,
            "a hypergate without links"
        );
    }

    #[test]
    fn with_no_entry_pending_nothing_loads() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(131))), &catalog),
            None
        );
    }

    #[test]
    fn asking_changes_nothing_and_the_entry_still_goes_through() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        let _ = session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(131))), &catalog);
        let mut chance = Scripted::default();
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut chance),
            Ok(SystemId(131))
        );
        let holes = wormholes();
        let mut session = at_wormhole(&holes);
        assert_eq!(
            session.gate_entry_desc(GateEntry::Wormhole, &holes),
            Some(400)
        );
        assert_eq!(
            session.enter_wormhole(&holes, &mut Scripted::rolling(&[0])),
            Ok(SystemId(131))
        );
    }

    #[test]
    fn a_wormhole_with_an_exit_loads_its_own_desc_linked_or_not() {
        let holes = wormholes();
        let session = at_wormhole(&holes);
        assert_eq!(
            session.gate_entry_desc(GateEntry::Wormhole, &holes),
            Some(400)
        );
        assert_eq!(
            session.gate_entry_desc(GateEntry::Hypergate(Some(SystemId(131))), &holes),
            None,
            "not a hypergate"
        );
        let linked = FakePilotCatalog {
            gates: vec![
                gate_site(400, (0.0, 0.0), 0x2200, &[410], 120),
                gate_site(410, (-300.0, 40.0), WORMHOLE, &[], 0),
            ],
            ..wormholes()
        };
        let session = at_wormhole(&linked);
        assert_eq!(
            session.gate_entry_desc(GateEntry::Wormhole, &linked),
            Some(400)
        );
    }

    #[test]
    fn a_wormhole_with_nowhere_to_lead_loads_nothing() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(400, (0.0, 0.0), WORMHOLE, &[], 0)],
            ..wormholes()
        };
        let session = at_wormhole(&catalog);
        assert_eq!(session.gate_entry_desc(GateEntry::Wormhole, &catalog), None);
    }
}
