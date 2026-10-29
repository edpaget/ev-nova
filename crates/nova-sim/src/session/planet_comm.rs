//! The planet comm dialog in flight: the session's side of
//! [`planet_comm`](crate::planet_comm), which decides every answer
//! (`_DoPlanetCommDialog` @0x96949-0x97b96 in the `EV Nova` executable).
//!
//! **Who gets it.** Hailing the navigation stellar, with no ship target or
//! with Option held, opens it when the stellar answers
//! ([`landing::answers_hail`](crate::landing::answers_hail)): the comm
//! key's [`Hailed::Stellar`](super::Hailed::Stellar). The dialog open is
//! the [`Chart`]'s, as are the system's bribe roll it keeps until the next
//! arrival, a new pilot, a load or the escape pod's rescue
//! (`_bribeRandom` = -1, `_ResetPlayer` @0x1d7ae, `_HandlePlayer`
//! @0x6bfd3; [`Chart::forget_bribe_roll`]), as both concern the system's
//! stellars.
//!
//! **What it reads.** The stellar as the system lists it now, its
//! government the galaxy overlay's (stellar +0xa); the player's legal
//! record there as landing reads it (`RuleKey::LegalRecords`); the
//! blessing with landing and the IFF scrambler's mark of its government;
//! its domination ([`OverlaidGalaxy::dominated`](crate::story::OverlaidGalaxy::dominated));
//! and its defence ships left and under-attack mark (the traffic's
//! `StellarDefences`). Demand Tribute reads the combat rating and cheat
//! mode, which the engine has none of (rdm:task/cheat-mode-beyond-player-info),
//! so it is off.
//!
//! **What each press does** ([`Session::press_planet_comm`],
//! [`Session::haggle_bribe`]), settled inside the press, as the dialog is
//! modal over the paused flight:
//!
//! - A bribe paid: the dock request is granted at
//!   [`BRIBE_PERMISSION`] for the stellar (@0x97112-0x97125), so the land
//!   key lands; the message line says so ([`SimMessage::BribeCleared`],
//!   @0x97229); the price comes off the cash, which makes no save due; and
//!   the dialog closes.
//! - Release (@0x973a2-0x9755a): the stellar is released on the overlay
//!   and the dock request dropped, its defence ships left reset to its
//!   decoded `DefCount`, the records written, and its `OnRelease` run.
//! - Demand Tribute (@0x9755f-0x979b1): the records written, then each
//!   slap ([`Demand::slaps`]) a conviction of [`Crime::Kill`] against the
//!   stellar's government with its rank pass (`_SlapWithPenalty(.., 3,
//!   -1)` @0x975d0, @0x9773d, @0x9785f). A fight that sends a wave
//!   spends the ships [`wave_on_demand`] says and sends each up through
//!   [`Session::spawn_stellar_defender`] (@0x97814), which marks the
//!   stellar under attack. A domination drops the dock request, takes
//!   person 1150 off the gone (@0x978df), dominates the stellar on the
//!   overlay and runs its `OnDominate`.
//! - The record write ([`record_after_offence`]) changes both of the
//!   pilot's stores, each by its own value: the record in the system, as
//!   the original's `_playerRecord`, and the record with the government
//!   the Bible's reading reads at the stellar (see the `legal` module).
//!
//! **Its sounds**: [`SimSound::PlanetCommOpened`] as it opens,
//! [`SimSound::PlanetCommPressed`] for each press of item 2 or 3,
//! [`SimSound::PlanetCommHaggled`] as the bribe's haggle ends, and
//! [`SimSound::PlanetCommClosed`] as it closes.
//!
//! Left out, as frame-timing and redraw bookkeeping the engine has no use
//! for: `_commDialogMode`, `_commShip`, `_adviceDisasterRandom` (which
//! nothing reads), `_statusRedraw[2]`, the player's +0x4c (never above 0),
//! stellar +0x2a (only saved), `_gSpeedMult`, `FlushEvents`, the cursor,
//! `_RefreshSAT` and the close's repaint of the message line. The
//! `_adviceRandom` = -1 it writes makes the next ship advice pick
//! clock-seeded, which can't be replicated
//! (rdm:phase/play-feel-4/phase-2-planet-comm-dialog's findings).

use super::{Chart, Session};
use crate::SimSound;
use crate::board::BOUNTY_HUNTER;
use crate::catalog::{CommCatalog, LandingSite, StellarId};
use crate::chance::Chance;
use crate::clearance::DockRequest;
use crate::hail::deal::Settled;
use crate::hail::{GREEDY, Haggle};
use crate::landing::StellarFlags;
use crate::legal::{Crime, LegalCode};
use crate::message::SimMessage;
use crate::planet_comm::{
    ALWAYS_DOMINATED, BRIBE_PERMISSION, Bribery, Demand, DemandFacts, Opening, PLANETS_TAKE_BRIBES,
    PlanetComm, PlanetCommItem, PlanetCommView, Pressed, Standing, record_after_offence,
    type_label,
};
use crate::rank::Blessed;
use crate::traffic::stellar_defence::wave_on_demand;

impl Chart {
    /// Forgets the system's bribe roll, so the next planet hail draws it
    /// afresh (`_bribeRandom` = -1: `_ResetPlayer` @0x1d7ae, `_HandlePlayer`
    /// @0x6bfd3).
    pub(super) fn forget_bribe_roll(&mut self) {
        self.bribe_roll = None;
    }
}

impl Session {
    /// Opens the planet comm dialog on `stellar`, one the system shows,
    /// drawing on `chance` and beeping, and gives what it shows, its
    /// replies read from `catalog` (see the module docs); none, opening
    /// nothing, when the system shows no such stellar.
    pub(super) fn open_planet_comm(
        &mut self,
        stellar: StellarId,
        catalog: &(impl CommCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) -> Option<PlanetCommView> {
        let site = self.planet_site(stellar)?;
        let govts = &self.statics.govts;
        let facts = Opening {
            min_status: site.min_status,
            record: self.chart.landing_record(&self.pilot, &self.setup, &site),
            govt: site.govt.map(|govt| Bribery {
                greedy: govts.flag(Some(govt), GREEDY),
                planets_take_bribes: govts.flag(Some(govt), PLANETS_TAKE_BRIBES),
            }),
            blessed: self.blessing().blesses(site.govt, Blessed::Landing),
            iff_marked: site
                .govt
                .is_some_and(|govt| self.space.iff_marked.contains(&govt)),
            cash: self.pilot.cash,
        };
        let dominated = self.chart.galaxy.dominated(&site);
        let comm = PlanetComm::open(
            stellar,
            dominated,
            &facts,
            &mut self.chart.bribe_roll,
            chance,
        );
        self.chart.planet_comm = Some(comm);
        self.out.sounds.push(SimSound::PlanetCommOpened);
        self.planet_comm(catalog)
    }

    /// What the planet comm dialog shows, while it is open, its replies
    /// read from `catalog`.
    #[must_use]
    pub(crate) fn planet_comm(
        &self,
        catalog: &(impl CommCatalog + ?Sized),
    ) -> Option<PlanetCommView> {
        let comm = self.chart.planet_comm?;
        let site = self.planet_site(comm.stellar())?;
        let name = self
            .statics
            .place_names
            .stellars
            .get(&site.id)
            .cloned()
            .unwrap_or_default();
        Some(comm.view(
            &name,
            type_label(site.graphic, catalog),
            self.planet_standing(&site),
            catalog,
        ))
    }

    /// Presses `item` in the planet comm dialog, convicting by `law` and
    /// drawing on `chance` (see the module docs), and gives what it shows
    /// after, its replies read from `catalog`; none once it closes, or
    /// when it is not open or its haggle dialog is.
    pub(crate) fn press_planet_comm(
        &mut self,
        item: PlanetCommItem,
        catalog: &(impl CommCatalog + ?Sized),
        law: &dyn LegalCode,
        chance: &mut dyn Chance,
    ) -> Option<PlanetCommView> {
        let mut comm = self.chart.planet_comm?;
        let site = self.planet_site(comm.stellar())?;
        let defences = self.space.traffic.defences();
        let facts = DemandFacts {
            rating: self.pilot.combat_rating(),
            cheats: false,
            demanded: comm.demanded(),
            defenders_left: defences.left(&site),
            under_attack: defences.under_attack(site.id),
        };
        let pressed = comm.press(item, self.planet_standing(&site), facts, chance)?;
        self.chart.planet_comm = Some(comm);
        match pressed {
            Pressed::Closed => {
                self.close_planet_comm();
                return None;
            }
            Pressed::Said | Pressed::Asked(_) => self.out.sounds.push(SimSound::PlanetCommPressed),
            Pressed::Released => {
                self.out.sounds.push(SimSound::PlanetCommPressed);
                self.release_stellar(&site, chance);
            }
            Pressed::Demanded(demand) => {
                self.out.sounds.push(SimSound::PlanetCommPressed);
                self.demand_tribute(&site, demand, law, chance);
            }
        }
        self.planet_comm(catalog)
    }

    /// Makes `choice` in the planet comm dialog's bribe haggle (see the
    /// module docs), and gives what the dialog shows after, its replies
    /// read from `catalog`: the price lowered while the haggle goes on,
    /// none once a bribe paid closes it, or when no price is asked.
    pub(crate) fn haggle_bribe(
        &mut self,
        choice: Haggle,
        catalog: &(impl CommCatalog + ?Sized),
    ) -> Option<PlanetCommView> {
        let mut comm = self.chart.planet_comm?;
        let settled = comm.haggle(choice, self.pilot.cash, &mut self.chart.bribe_roll)?;
        self.chart.planet_comm = Some(comm);
        if settled == Settled::Lowered {
            return self.planet_comm(catalog);
        }
        // Once, as the haggle dialog returns (@0x970e9).
        self.out.sounds.push(SimSound::PlanetCommHaggled);
        if let Settled::Paid(price) = settled {
            let station = self
                .planet_site(comm.stellar())
                .is_some_and(|site| site.flags & StellarFlags::STATION != 0);
            self.port.dock = Some(DockRequest {
                stellar: comm.stellar(),
                permission: BRIBE_PERMISSION,
            });
            self.space
                .say(&mut self.out, SimMessage::BribeCleared { station });
            self.pilot.cash -= price;
            self.close_planet_comm();
            return None;
        }
        self.planet_comm(catalog)
    }

    /// Closes the planet comm dialog, beeping, when it is open.
    pub(crate) fn close_planet_comm(&mut self) {
        if self.chart.planet_comm.take().is_some() {
            self.out.sounds.push(SimSound::PlanetCommClosed);
        }
    }

    /// `stellar` as the player's system shows it now.
    fn planet_site(&self, stellar: StellarId) -> Option<LandingSite> {
        self.chart
            .sites
            .iter()
            .find(|site| site.id == stellar)
            .copied()
    }

    /// What a press reads of `site`, and the view shows.
    fn planet_standing(&self, site: &LandingSite) -> Standing {
        Standing {
            dominated: self.chart.galaxy.dominated(site),
            always_dominated: site.flags2 & ALWAYS_DOMINATED != 0,
            station: site.flags & StellarFlags::STATION != 0,
            record: self.chart.landing_record(&self.pilot, &self.setup, site),
        }
    }

    /// Release's deeds at `site` (see the module docs), drawing on
    /// `chance`.
    fn release_stellar(&mut self, site: &LandingSite, chance: &mut dyn Chance) {
        self.port.dock = None;
        self.chart
            .write_galaxy(&mut self.pilot, &mut self.out, |overlay| {
                overlay.set_dominated(site.id, false);
            });
        self.space.traffic.defences_mut().reset(site);
        self.write_offence_record(site);
        if let Some(scripts) = self.statics.stellar_scripts.get(&site.id).cloned() {
            self.run_script(&scripts.on_release, chance);
        }
    }

    /// Demand Tribute's deeds at `site` once it came to `demand` (see the
    /// module docs), convicting by `law` and drawing on `chance`.
    fn demand_tribute(
        &mut self,
        site: &LandingSite,
        demand: Demand,
        law: &dyn LegalCode,
        chance: &mut dyn Chance,
    ) {
        self.write_offence_record(site);
        for _ in 0..demand.slaps() {
            self.chart
                .convict_of(&mut self.pilot, &self.statics, law, Crime::Kill, site.govt);
            self.run.lose_ranks_to(
                &mut self.pilot,
                &self.statics,
                &self.setup,
                Crime::Kill,
                site.govt,
            );
        }
        match demand {
            Demand::Laugh | Demand::Fight { send_wave: false } => {}
            Demand::Fight { send_wave: true } => {
                let defences = self.space.traffic.defences_mut();
                let (sent, left) = wave_on_demand(site.def_count, defences.left(site));
                defences.set_left(site, left);
                for _ in 0..sent {
                    self.spawn_stellar_defender(site.id, chance);
                }
            }
            Demand::Dominate => {
                self.port.dock = None;
                self.pilot.gone_persons.remove(&BOUNTY_HUNTER);
                self.chart
                    .write_galaxy(&mut self.pilot, &mut self.out, |overlay| {
                        overlay.set_dominated(site.id, true);
                    });
                if let Some(scripts) = self.statics.stellar_scripts.get(&site.id).cloned() {
                    self.run_script(&scripts.on_dominate, chance);
                }
            }
        }
    }

    /// Writes the record a Release or a Demand Tribute leaves at `site`
    /// into both of the pilot's stores, each by its own value (see the
    /// module docs).
    fn write_offence_record(&mut self, site: &LandingSite) {
        let system = self.pilot.system;
        if let Some(record) =
            record_after_offence(self.pilot.system_record(system), site.min_status)
        {
            self.pilot.set_system_record(system, record);
        }
        let govt = site.govt.or(self.chart.galaxy.govt(system));
        if let Some(govt) = govt
            && let Some(record) =
                record_after_offence(self.pilot.legal_record(govt), site.min_status)
        {
            self.pilot.set_legal_record(govt, record);
        }
    }
}

#[cfg(test)]
pub(in crate::session) mod tests {
    use super::*;
    use crate::catalog::{
        AsteroidCatalog, CombatCatalog, GovtId, GovtRecord, PilotCatalog, StellarScripts, SystemId,
        TrafficCatalog,
    };
    use crate::chance::NeverFires;
    use crate::control::{Bit, Script};
    use crate::govt::Governments;
    use crate::legal::{CrimeMap, NovaLaw};
    use crate::navigation::NavMode;
    use crate::session::Hailed;
    use crate::testkit::{Draws, FakePilotCatalog, catalog, govt};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// What a session in these tests is started over: the fake, or the
    /// fake shared as the app shares its catalog.
    pub(in crate::session) trait Over:
        PilotCatalog + CombatCatalog + AsteroidCatalog + TrafficCatalog + CommCatalog
    {
    }

    impl<T: PilotCatalog + CombatCatalog + AsteroidCatalog + TrafficCatalog + CommCatalog> Over for T {}

    const EARTH: StellarId = StellarId(128);
    const FED: GovtId = GovtId(128);

    /// `STR#` 3000 is "c<n>" and 3002 "s<n>" for each n up to 60, 1100
    /// "[t<n>]" up to 10; stellar 128 is "Earth", of government 128 (the
    /// Federation, whose planets take bribes), `Type` 2; government 129 is
    /// greedy; Earth dominating sets bit 100 and releasing clears it.
    pub(in crate::session) fn planet_catalog() -> FakePilotCatalog {
        let mut base = catalog();
        base.strings = vec![
            (3000, (1..=60).map(|n| format!("c{n}")).collect()),
            (3002, (1..=60).map(|n| format!("s{n}")).collect()),
            (1100, (1..=10).map(|n| format!("[t{n}]")).collect()),
        ];
        base.place_names.stellars.insert(EARTH, "Earth".to_owned());
        base.govts = vec![
            GovtRecord {
                flags: PLANETS_TAKE_BRIBES,
                ..govt(128)
            },
            GovtRecord {
                flags: GREEDY,
                ..govt(129)
            },
        ];
        for (_, sites) in &mut base.sites {
            for site in sites.iter_mut().filter(|site| site.id == EARTH) {
                site.govt = Some(FED);
                site.graphic = 2;
            }
        }
        base.scripts = vec![StellarScripts {
            id: EARTH,
            on_dominate: Script::parse("b100"),
            on_release: Script::parse("!b100"),
        }];
        base
    }

    /// `catalog`'s session in flight, its system populated, with Earth its
    /// nav target and no ship target, holding `cash`, Earth's `MinStatus`
    /// `min_status`.
    pub(in crate::session) fn at_earth(catalog: &impl Over, cash: i64, min_status: i16) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        crate::testkit::work(&mut session, catalog, crate::flight::Controls::default());
        session.chart.target = None;
        session.chart.nav = NavMode::Stellar(Some(EARTH));
        session.pilot.cash = cash;
        session.set_earth(|site| site.min_status = min_status);
        session.take_sounds();
        session.take_save_due();
        session
    }

    impl Session {
        /// Changes Earth as the system shows and lists it.
        fn set_earth(&mut self, change: impl Fn(&mut LandingSite)) {
            for site in self
                .chart
                .sites
                .iter_mut()
                .chain(&mut self.chart.listed)
                .filter(|site| site.id == EARTH)
            {
                change(site);
            }
        }
    }

    /// Hails Earth with the comm key, its variant `variant` and the
    /// bribe roll's draw `roll`: the dialog shown, the draws asked and the
    /// sounds.
    fn hail(
        session: &mut Session,
        catalog: &impl CommCatalog,
        variant: u32,
        roll: u32,
    ) -> (PlanetCommView, Vec<u32>, Vec<SimSound>) {
        let mut chance = Draws::of(&[]).keyed(5, variant).keyed(100, roll);
        let hailed = session
            .hail_key(
                catalog,
                &crate::hail::HailOptions::default(),
                false,
                &mut chance,
            )
            .expect("answered");
        let Hailed::Stellar(view) = hailed else {
            panic!("{hailed:?}");
        };
        (view, chance.asked, session.take_sounds())
    }

    /// Presses `item`, by `law`, the haggle roll's draw `haggle_roll`: the
    /// dialog after and the sounds.
    fn press_by(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        item: PlanetCommItem,
        law: &dyn LegalCode,
        haggle_roll: u32,
    ) -> (Option<PlanetCommView>, Vec<SimSound>) {
        let mut chance = Draws::of(&[haggle_roll]);
        let view = session.press_planet_comm(item, catalog, law, &mut chance);
        (view, session.take_sounds())
    }

    fn press(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        item: PlanetCommItem,
        haggle_roll: u32,
    ) -> (Option<PlanetCommView>, Vec<SimSound>) {
        press_by(session, catalog, item, &NovaLaw::default(), haggle_roll)
    }

    #[test]
    fn hailing_an_inhabited_dockable_stellar_opens_the_planet_comm_dialog_beeping() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 0);
        let (view, asked, sounds) = hail(&mut session, &catalog, 2, 0);
        assert_eq!(
            view,
            PlanetCommView {
                stellar: EARTH,
                name: "Earth".to_owned(),
                type_label: "[t3]".to_owned(),
                reply: "s3Earth.".to_owned(),
                second: crate::planet_comm::PlanetCommButton::Greetings,
                third: Some(crate::planet_comm::PlanetCommButton::DemandTribute),
                status: None,
                asking: None,
            }
        );
        assert_eq!(asked, [5, 1, 100], "the variant, the price and the roll");
        assert_eq!(sounds, [SimSound::PlanetCommOpened], "_beepSnd[4] @0x96b5f");
        assert_eq!(session.planet_comm(&catalog), Some(view));
    }

    #[test]
    fn a_refusing_stellar_offers_its_bribe_and_shows_its_record_line() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 1);
        let (view, _, _) = hail(&mut session, &catalog, 0, 0);
        assert_eq!(view.reply, "c11", "STR# 3000 #(rr + 11)");
        assert_eq!(
            view.second,
            crate::planet_comm::PlanetCommButton::OfferBribe
        );
        assert_eq!(
            view.status,
            Some(crate::planet_comm::PlanetStatus::Forbidden)
        );
        session.pilot.set_system_record(session.pilot.system, -1);
        let view = session.planet_comm(&catalog).expect("open");
        assert_eq!(view.status, Some(crate::planet_comm::PlanetStatus::Hostile));
    }

    #[test]
    fn a_dominated_stellar_offers_release_and_an_always_dominated_one_no_item_three() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 0);
        session
            .chart
            .write_galaxy(&mut session.pilot, &mut session.out, |overlay| {
                overlay.set_dominated(EARTH, true);
            });
        let (view, _, _) = hail(&mut session, &catalog, 0, 0);
        assert_eq!(
            view.third,
            Some(crate::planet_comm::PlanetCommButton::Release)
        );
        let mut session = at_earth(&catalog, 1_000_000, 0);
        session.set_earth(|site| site.flags2 = ALWAYS_DOMINATED);
        let (view, _, _) = hail(&mut session, &catalog, 0, 0);
        assert_eq!(view.third, None);
        assert_eq!(view.status, Some(crate::planet_comm::PlanetStatus::Owned));
    }

    #[test]
    fn the_bribe_roll_is_drawn_once_a_system_and_again_after_an_arrival() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 0);
        let (_, asked, _) = hail(&mut session, &catalog, 0, 31);
        assert_eq!(asked, [5, 1, 100]);
        press(&mut session, &catalog, PlanetCommItem::Close, 0);
        let (_, asked, _) = hail(&mut session, &catalog, 0, 31);
        assert_eq!(asked, [5, 1], "the roll kept");
        session.chart.forget_bribe_roll();
        let (_, asked, _) = hail(&mut session, &catalog, 0, 31);
        assert_eq!(asked, [5, 1, 100], "forgotten, drawn again");
    }

    #[test]
    fn greetings_greets_beeping() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 0);
        hail(&mut session, &catalog, 1, 0);
        let (view, sounds) = press(&mut session, &catalog, PlanetCommItem::Second, 0);
        assert_eq!(view.expect("open").reply, "c47", "STR# 3000 #(rr + 46)");
        assert_eq!(
            sounds,
            [SimSound::PlanetCommPressed],
            "_beepSnd[1] @0x97021"
        );
    }

    /// `catalog`'s session at a refusing Earth (`MinStatus` 1) whose
    /// bribe roll is 31, holding 1,000,000, the dialog open.
    fn bribable(catalog: &FakePilotCatalog) -> Session {
        let mut session = at_earth(catalog, 1_000_000, 1);
        hail(&mut session, catalog, 0, 31);
        session
    }

    #[test]
    fn a_bribe_haggled_down_and_paid_clears_the_player_to_land() {
        let catalog = planet_catalog();
        let mut session = bribable(&catalog);
        let (view, sounds) = press(&mut session, &catalog, PlanetCommItem::Second, 35);
        let view = view.expect("open");
        assert_eq!(view.reply, "s41", "STR# 3002 #(rr + 41)");
        assert_eq!(view.asking, Some(3000));
        assert_eq!(sounds, [SimSound::PlanetCommPressed]);
        let lowered = session.haggle_bribe(Haggle::LowerPrice, &catalog);
        assert_eq!(lowered.expect("open").asking, Some(2200));
        assert_eq!(session.take_sounds(), [], "a won Lower Price plays nothing");
        session.take_messages();
        assert_eq!(
            session.haggle_bribe(Haggle::Accept, &catalog),
            None,
            "closed"
        );
        assert_eq!(
            session.take_sounds(),
            [SimSound::PlanetCommHaggled, SimSound::PlanetCommClosed],
            "@0x970e9, then @0x97aa4"
        );
        assert_eq!(session.pilot.cash, 1_000_000 - 2200);
        assert_eq!(
            session.port.dock,
            Some(DockRequest {
                stellar: EARTH,
                permission: BRIBE_PERMISSION,
            })
        );
        assert_eq!(
            session.take_messages(),
            [SimMessage::BribeCleared { station: false }]
        );
        assert_eq!(session.planet_comm(&catalog), None);
        assert!(!session.take_save_due(), "paying makes no save due");
    }

    #[test]
    fn a_station_bribed_clears_the_player_to_dock() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 1);
        session.set_earth(|site| site.flags |= StellarFlags::STATION);
        hail(&mut session, &catalog, 0, 31);
        press(&mut session, &catalog, PlanetCommItem::Second, 99);
        session.take_messages();
        session.haggle_bribe(Haggle::Accept, &catalog);
        assert_eq!(
            session.take_messages(),
            [SimMessage::BribeCleared { station: true }]
        );
    }

    #[test]
    fn a_declined_bribe_is_refused_until_the_next_arrival() {
        let catalog = planet_catalog();
        let mut session = bribable(&catalog);
        press(&mut session, &catalog, PlanetCommItem::Second, 99);
        let view = session.haggle_bribe(Haggle::LowerPrice, &catalog);
        assert_eq!(view.expect("open").reply, "s31", "STR# 3002 #(rr + 31)");
        assert_eq!(session.take_sounds(), [SimSound::PlanetCommHaggled]);
        assert_eq!(session.chart.bribe_roll, Some(0));
        press(&mut session, &catalog, PlanetCommItem::Close, 0);
        hail(&mut session, &catalog, 0, 99);
        let (view, _) = press(&mut session, &catalog, PlanetCommItem::Second, 0);
        assert_eq!(view.expect("open").reply, "s46", "Yeah, right.");
        assert_eq!(session.port.dock, None);
    }

    #[test]
    fn a_failed_roll_short_cash_or_a_dominated_stellar_takes_no_bribe() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 1);
        hail(&mut session, &catalog, 0, 30);
        let (view, _) = press(&mut session, &catalog, PlanetCommItem::Second, 0);
        assert_eq!(view.expect("open").reply, "s46", "a roll of 30");
        let mut session = at_earth(&catalog, 2999 * 3, 1);
        hail(&mut session, &catalog, 0, 31);
        press(&mut session, &catalog, PlanetCommItem::Second, 99);
        session.pilot.cash = 999;
        let view = session.haggle_bribe(Haggle::Accept, &catalog);
        assert_eq!(view.expect("open").reply, "s21", "short of cash");
        assert_eq!(session.pilot.cash, 999);
        let mut session = at_earth(&catalog, 1_000_000, 1);
        session
            .chart
            .write_galaxy(&mut session.pilot, &mut session.out, |overlay| {
                overlay.set_dominated(EARTH, true);
            });
        hail(&mut session, &catalog, 0, 31);
        let (view, _) = press(&mut session, &catalog, PlanetCommItem::Second, 0);
        assert_eq!(view.expect("open").reply, "s21", "dominated");
    }

    /// Records each crime it is asked about with the victim's government,
    /// and fines government 140 one point a crime.
    #[derive(Debug, Default)]
    struct Slaps {
        seen: RefCell<Vec<(Crime, Option<GovtId>)>>,
    }

    impl LegalCode for Slaps {
        fn penalties(
            &self,
            crime: Crime,
            victim: Option<GovtId>,
            _govts: &Governments,
        ) -> Vec<(GovtId, i32)> {
            self.seen.borrow_mut().push((crime, victim));
            vec![(GovtId(140), -1)]
        }

        fn system_penalties(
            &self,
            _crime: Crime,
            _victim: Option<GovtId>,
            _from: SystemId,
            _map: &dyn CrimeMap,
            _govts: &Governments,
        ) -> Vec<(SystemId, f32)> {
            Vec::new()
        }
    }

    /// [`planet_catalog`] with rank 200 of the Federation, lost to any
    /// crime against it.
    fn ranked() -> FakePilotCatalog {
        let mut catalog = planet_catalog();
        catalog.ranks = vec![crate::catalog::RankRecord {
            affil_govt: Some(FED),
            flags: crate::rank::LOST_TO_CRIME,
            ..crate::testkit::rank(200)
        }];
        catalog
    }

    /// `catalog`'s session at Earth of `MinStatus` 5 with a record of 10
    /// in the system and with the Federation, holding rank 200, the
    /// player's rating `rating`, the dialog open.
    fn demanding(catalog: &impl Over, rating: i64) -> Session {
        let mut session = at_earth(catalog, 1_000_000, 5);
        let system = session.pilot.system;
        session.pilot.set_system_record(system, 10);
        session.pilot.set_legal_record(FED, 10);
        session.pilot.combat_rating = rating;
        session.pilot.ranks.extend([crate::catalog::RankId(200)]);
        hail(&mut session, catalog, 0, 0);
        session
    }

    /// Demands tribute by `slaps`: the dialog after and the sounds.
    fn demand(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        slaps: &Slaps,
    ) -> (Option<PlanetCommView>, Vec<SimSound>) {
        press_by(session, catalog, PlanetCommItem::Third, slaps, 0)
    }

    /// Whether `slaps` saw `count` crimes, each a killing of the
    /// Federation's, taking them.
    fn slapped(slaps: &Slaps, count: usize) {
        assert_eq!(
            slaps.seen.take(),
            vec![(Crime::Kill, Some(FED)); count],
            "_SlapWithPenalty(.., 3, -1)"
        );
    }

    #[test]
    fn a_weak_pilots_demand_is_laughed_off_with_one_slap() {
        let catalog = ranked();
        let mut session = demanding(&catalog, 12_799);
        let slaps = Slaps::default();
        let (view, sounds) = demand(&mut session, &catalog, &slaps);
        let view = view.expect("open");
        assert_eq!(view.reply, "s6", "STR# 3002 #(rr + 6)");
        assert_eq!(sounds, [SimSound::PlanetCommPressed]);
        slapped(&slaps, 1);
        assert_eq!(session.pilot.legal_record(GovtId(140)), -1);
        assert_eq!(
            session.pilot.system_record(session.pilot.system),
            4,
            "MinStatus - 1"
        );
        assert_eq!(session.pilot.legal_record(FED), 4, "both stores");
        assert!(session.pilot.ranks.is_empty(), "the rank lost to the crime");
        assert_eq!(
            view.second,
            crate::planet_comm::PlanetCommButton::OfferBribe,
            "_hatesPlayer set"
        );
    }

    #[test]
    fn a_defended_stellar_fights_sending_its_wave_and_a_second_demand_fights_again() {
        let mut catalog = ranked();
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut().filter(|site| site.id == EARTH) {
                site.defense_dude = Some(crate::catalog::DudeId(140));
                site.def_count = 1082;
            }
        }
        catalog.dudes.push((
            crate::catalog::DudeId(140),
            crate::catalog::DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(crate::catalog::ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        ));
        catalog
            .ship_records
            .push(crate::testkit::ship(129, crate::testkit::FAST));
        let mut session = demanding(&catalog, 12_800);
        let slaps = Slaps::default();
        let (view, _) = demand(&mut session, &catalog, &slaps);
        assert_eq!(view.expect("open").reply, "s11", "STR# 3002 #(rr + 11)");
        slapped(&slaps, 6);
        assert_eq!(
            session.defence_ships_left(EARTH),
            Some(6),
            "8 less a wave of 2"
        );
        let up = session
            .space
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.defends == Some(EARTH))
            .count();
        assert_eq!(up, 2, "the wave sent up");
        assert!(session.space.traffic.defences().under_attack(EARTH));
        assert_eq!(session.pilot.system_record(session.pilot.system), 4);
        let (view, _) = demand(&mut session, &catalog, &slaps);
        assert_eq!(view.expect("open").reply, "s11");
        slapped(&slaps, 1);
        assert_eq!(session.defence_ships_left(EARTH), Some(6), "no wave");
    }

    #[test]
    fn a_demand_once_the_fleet_is_gone_dominates_the_stellar_running_its_on_dominate() {
        let catalog = ranked();
        let mut session = demanding(&catalog, 12_800);
        session.port.dock = Some(DockRequest {
            stellar: EARTH,
            permission: 0,
        });
        session.pilot.gone_persons.insert(BOUNTY_HUNTER);
        let slaps = Slaps::default();
        let (view, _) = demand(&mut session, &catalog, &slaps);
        let view = view.expect("open");
        assert_eq!(view.reply, "s26", "STR# 3002 #26");
        assert_eq!(
            view.third,
            Some(crate::planet_comm::PlanetCommButton::Release)
        );
        slapped(&slaps, 6);
        assert!(
            session
                .chart
                .galaxy
                .dominated(&session.planet_site(EARTH).expect("shown"))
        );
        assert_eq!(session.port.dock, None, "the dock request dropped");
        assert!(!session.pilot.gone(BOUNTY_HUNTER), "person 1150 alive");
        assert!(
            session.control_bit(Bit::new(100).expect("a bit")),
            "OnDominate"
        );
        assert!(session.pilot.ranks.is_empty());
    }

    #[test]
    fn a_session_over_a_shared_catalog_runs_on_dominate() {
        // The app starts its session over an `Rc<GameData>`.
        let catalog = Rc::new(ranked());
        let mut session = demanding(&catalog, 12_800);
        demand(&mut session, &catalog, &Slaps::default());
        assert!(
            session.control_bit(Bit::new(100).expect("a bit")),
            "OnDominate, read through the Rc"
        );
    }

    #[test]
    fn a_station_says_it_agrees_to_pay_tribute() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 0);
        session.set_earth(|site| site.flags |= StellarFlags::STATION);
        session.pilot.combat_rating = 20_000;
        hail(&mut session, &catalog, 0, 0);
        let (view, _) = press(&mut session, &catalog, PlanetCommItem::Third, 0);
        assert_eq!(view.expect("open").reply, "s27");
    }

    #[test]
    fn release_releases_a_dominated_stellar_running_its_on_release() {
        let mut catalog = ranked();
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut().filter(|site| site.id == EARTH) {
                site.def_count = 1082;
            }
        }
        let mut session = demanding(&catalog, 12_800);
        let earth = session.planet_site(EARTH).expect("shown");
        session.space.traffic.defences_mut().set_left(&earth, 0);
        demand(&mut session, &catalog, &Slaps::default());
        assert!(
            session.control_bit(Bit::new(100).expect("a bit")),
            "dominated"
        );
        let system = session.pilot.system;
        session.pilot.set_system_record(system, 7);
        session.port.dock = Some(DockRequest {
            stellar: EARTH,
            permission: 0,
        });
        let slaps = Slaps::default();
        let (view, sounds) = press_by(&mut session, &catalog, PlanetCommItem::Third, &slaps, 0);
        let view = view.expect("open");
        assert_eq!(view.reply, "s36", "STR# 3002 #36");
        assert_eq!(sounds, [SimSound::PlanetCommPressed]);
        assert_eq!(
            view.third,
            Some(crate::planet_comm::PlanetCommButton::DemandTribute)
        );
        slapped(&slaps, 0);
        assert!(
            !session.control_bit(Bit::new(100).expect("a bit")),
            "OnRelease"
        );
        assert!(
            !session
                .chart
                .galaxy
                .dominated(&session.planet_site(EARTH).expect("shown"))
        );
        assert_eq!(session.defence_ships_left(EARTH), Some(8), "the decode");
        assert_eq!(session.pilot.system_record(system), 4, "the record write");
        assert_eq!(session.port.dock, None);
    }

    #[test]
    fn item_three_of_an_always_dominated_stellar_beeps_and_does_nothing_more() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 0);
        session.set_earth(|site| site.flags2 = ALWAYS_DOMINATED);
        hail(&mut session, &catalog, 0, 0);
        let (view, sounds) = press(&mut session, &catalog, PlanetCommItem::Third, 0);
        assert_eq!(view.expect("open").reply, "c6", "No response.");
        assert_eq!(sounds, [SimSound::PlanetCommPressed]);
    }

    #[test]
    fn close_channel_closes_beeping_and_a_later_press_finds_nothing_open() {
        let catalog = planet_catalog();
        let mut session = at_earth(&catalog, 1_000_000, 0);
        hail(&mut session, &catalog, 0, 0);
        let (view, sounds) = press(&mut session, &catalog, PlanetCommItem::Close, 0);
        assert_eq!(view, None);
        assert_eq!(sounds, [SimSound::PlanetCommClosed], "_beepSnd[2] @0x97aa4");
        let (view, sounds) = press(&mut session, &catalog, PlanetCommItem::Second, 0);
        assert_eq!((view, sounds), (None, vec![]));
        assert_eq!(session.haggle_bribe(Haggle::Accept, &catalog), None);
        session.close_planet_comm();
        assert_eq!(session.take_sounds(), [], "closing nothing beeps nothing");
    }

    #[test]
    fn a_hail_opened_again_starts_a_fresh_dialog() {
        let catalog = planet_catalog();
        let mut session = demanding(&catalog, 20_000);
        demand(&mut session, &catalog, &Slaps::default());
        hail(&mut session, &catalog, 0, 0);
        let comm = session.chart.planet_comm.expect("open");
        assert!(!comm.demanded(), "_hasDemandedTribute cleared @0x96ae8");
        let _ = NeverFires;
    }
}
