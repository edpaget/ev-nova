//! The galaxy refresh over the stock data: every `sÿst` `Visibility`
//! parses, a pair of alternate systems swaps as the bit that selects them
//! is set, and Aldebaran's two versions, both visible between Polaris
//! missions 183 and 887, resolve to 141 by default (`colocated_resolution`
//! engine), and wormhole 466 shows only in Spica, which owns it. A swap
//! at Aldebaran keeps the place explored (`explored_by_place` engine).
//! A single jump to Aldebaran from Homeland, selected by place, arrives
//! in 532 as it is, the Con link it was selected from, by default
//! (`arrival_system` engine). Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use std::collections::BTreeSet;

use nova_data::GameData;
use nova_sim::control::{Bit, Gate, NovaBits, PilotFacts};
use nova_sim::gate::{WormholeRule, wormhole_exit};
use nova_sim::pilot::Gender;
use nova_sim::story::{GalaxyOverlay, HiddenMove, OverlaidGalaxy, listers_of};
use nova_sim::{
    CoLocated, Controls, HyperlinkRule, NeverFires, OutfitId, Pilot, PilotCatalog, RuleKey,
    RuleSource, Rulebook, Session, SetExpr, StellarId, SystemId,
};

/// A pilot with only these bits set, as a control-bit test reads it.
#[derive(Debug)]
struct Bits(BTreeSet<u16>);

impl PilotFacts for Bits {
    fn bit(&self, bit: Bit) -> bool {
        self.0.contains(&bit.get())
    }

    fn gender(&self) -> Gender {
        Gender::Male
    }

    fn paid(&self, _days: u16) -> bool {
        true
    }

    fn has_outfit(&self, _outfit: OutfitId) -> bool {
        false
    }

    fn explored(&self, _system: SystemId) -> bool {
        false
    }
}

/// The stock galaxy refreshed for a pilot with `bits` set, a place two
/// visible systems share resolved as `colocated` says.
fn refreshed(data: &GameData, bits: &[u16], colocated: CoLocated) -> OverlaidGalaxy {
    let mut galaxy = OverlaidGalaxy::new(data.star_map(), GalaxyOverlay::default());
    galaxy.set_colocated(colocated);
    let pilot = Bits(bits.iter().copied().collect());
    let gate = Gate {
        control_bits: &NovaBits,
        pilot: &pilot,
    };
    let mut overlay = GalaxyOverlay::default();
    overlay.refresh_visibility(
        galaxy.catalog_systems().collect::<Vec<_>>(),
        &data.system_visibility(),
        &gate,
    );
    galaxy.set_overlay(overlay);
    galaxy
}

/// The owner `galaxy` gives stellar `id`, from the systems listing it.
fn owner(data: &GameData, galaxy: &OverlaidGalaxy, id: i16) -> Option<i16> {
    let listed = data.listed_stellars();
    let stellar = listed
        .iter()
        .find(|stellar| stellar.id == StellarId(id))
        .unwrap_or_else(|| panic!("spöb {id} is listed"));
    galaxy.owner(&stellar.systems).map(|system| system.0)
}

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// Polaris mission 183 sets b305; mission 887 sets b148.
const B305: u16 = 305;
const B148: u16 = 148;

/// Stellars 130, 169 and 170 are listed by both versions of Aldebaran;
/// 430 and 433 by 532 alone.
const SHARED: [i16; 3] = [130, 169, 170];

#[test]
fn every_stock_visibility_parses() {
    let Some(data) = open() else {
        return;
    };
    let tests = data.system_visibility();
    assert_eq!(tests.len(), data.star_map().len(), "one for each system");
    let broken: Vec<i16> = tests
        .iter()
        .filter(|test| test.visibility.tree().is_none())
        .map(|test| test.id.0)
        .collect();
    assert_eq!(broken, [] as [i16; 0]);
}

#[test]
fn aldebaran_swaps_from_141_to_532_once_mission_887_sets_b148() {
    let Some(data) = open() else {
        return;
    };
    let fresh = refreshed(&data, &[], CoLocated::Lowest);
    assert!(fresh.visible(SystemId(141)));
    assert!(!fresh.visible(SystemId(532)));
    for stellar in SHARED.into_iter().chain([148, 171]) {
        assert_eq!(owner(&data, &fresh, stellar), Some(141), "spöb {stellar}");
    }
    for stellar in [430, 433] {
        assert_eq!(owner(&data, &fresh, stellar), None, "spöb {stellar}");
    }
    let swapped = refreshed(&data, &[B305, B148], CoLocated::Lowest);
    assert!(!swapped.visible(SystemId(141)));
    assert!(swapped.visible(SystemId(532)));
    for stellar in SHARED.into_iter().chain([430, 433]) {
        assert_eq!(owner(&data, &swapped, stellar), Some(532), "spöb {stellar}");
    }
    for stellar in [148, 171] {
        assert_eq!(owner(&data, &swapped, stellar), None, "141's own");
    }
    assert_eq!(swapped.same_place(SystemId(141)), Some(SystemId(532)));
}

#[test]
fn with_both_versions_of_aldebaran_visible_141_stands_for_the_place_by_default() {
    let Some(data) = open() else {
        return;
    };
    let both = refreshed(&data, &[B305], CoLocated::Lowest);
    assert!(both.visible(SystemId(141)) && both.visible(SystemId(532)));
    assert_eq!(both.same_place(SystemId(532)), Some(SystemId(141)));
    assert_eq!(
        both.arrival(SystemId(532)),
        HiddenMove {
            to: SystemId(141),
            make_visible: false,
        }
    );
    assert_eq!(both.hidden_move(SystemId(532)), None, "visible: no move");
    assert_eq!(
        both.map().jumps(SystemId(173), HyperlinkRule::Engine),
        [SystemId(141)],
        "173 lists 532 and 141"
    );
    for stellar in SHARED {
        assert_eq!(owner(&data, &both, stellar), Some(141), "spöb {stellar}");
    }
    for stellar in [430, 433] {
        assert_eq!(owner(&data, &both, stellar), None, "532 is shadowed");
    }
}

#[test]
fn by_the_other_reading_both_versions_of_aldebaran_stand_for_themselves() {
    let Some(data) = open() else {
        return;
    };
    let both = refreshed(&data, &[B305], CoLocated::Itself);
    assert_eq!(both.same_place(SystemId(532)), Some(SystemId(532)));
    assert_eq!(both.arrival(SystemId(532)).to, SystemId(532));
    assert_eq!(
        both.map().jumps(SystemId(173), HyperlinkRule::Engine),
        [SystemId(141), SystemId(532)]
    );
    for stellar in SHARED {
        assert_eq!(owner(&data, &both, stellar), Some(141), "the lowest");
    }
    for stellar in [430, 433] {
        assert_eq!(owner(&data, &both, stellar), Some(532), "spöb {stellar}");
    }
}

/// Wormhole 466 is listed by Spica (144) and K-005 (436). K-005 is visible
/// on a fresh pilot (its `Visibility` is `!b330`), but Spica, visible in
/// every state (a blank `Visibility`) and the lower ID, owns it, so the
/// original shows it in Spica alone (`_HandleStellarSystemVisibility`).
#[test]
fn wormhole_466_shows_in_spica_and_not_in_k005() {
    let Some(data) = open() else {
        return;
    };
    let galaxy = refreshed(&data, &[], CoLocated::Lowest);
    let listed = data.listed_stellars();
    assert_eq!(
        listers_of(&listed, StellarId(466)),
        [SystemId(144), SystemId(436)]
    );
    assert!(galaxy.visible(SystemId(436)));
    assert_eq!(owner(&data, &galaxy, 466), Some(144));
    assert!(
        galaxy
            .shown(&listed, SystemId(144))
            .contains(&StellarId(466))
    );
    assert!(
        !galaxy
            .shown(&listed, SystemId(436))
            .contains(&StellarId(466))
    );
}

/// Spica's wormhole leads somewhere: another active wormhole owned by
/// another system than Spica, unlinked, as `_PlayerEnterWormhole` takes
/// its candidates by owner.
#[test]
fn the_spica_wormhole_has_exits() {
    let Some(data) = open() else {
        return;
    };
    let galaxy = refreshed(&data, &[], CoLocated::Lowest);
    let listed = data.listed_stellars();
    let sites = data.gate_sites();
    let spica = SystemId(144);
    let exit = wormhole_exit(
        StellarId(466),
        spica,
        &sites,
        |id| galaxy.holder(id, listers_of(&listed, id), spica),
        WormholeRule::UnlinkedOnly,
        &mut NeverFires,
    )
    .expect("an exit");
    assert_ne!(exit.system, spica);
    assert!(galaxy.visible(exit.system));
    assert!(galaxy.shown(&listed, exit.system).contains(&exit.site.id));
}

/// A new stock pilot reading `rules` that explores 141, then sets b305
/// and b148, which swap 532 in for it at the galaxy's next refresh.
fn swapping(data: &GameData, rules: Rulebook) -> Session {
    let mut session = Session::start(data)
        .expect("the stock first chär starts")
        .with_rules(rules);
    session.run_set(
        &SetExpr::parse(&format!("X141 b{B305} b{B148}")).expect("parses"),
        &mut NeverFires,
    );
    let _ = session.settle_script(data, &mut NeverFires);
    assert!(session.pilot().has_explored(SystemId(141)));
    session
}

#[test]
fn a_swap_at_aldebaran_keeps_the_place_explored() {
    let Some(data) = open() else {
        return;
    };
    // By the engine's refresh points, a load: its refresh swaps 532 in,
    // and its rebuild shares 141's exploration with it.
    let session = swapping(&data, Rulebook::default());
    assert!(session.galaxy().visible(SystemId(141)), "no refresh yet");
    let loaded = Session::fly(&data, session.pilot().clone()).expect("flies");
    assert!(!loaded.galaxy().visible(SystemId(141)));
    assert!(loaded.galaxy().visible(SystemId(532)));
    assert!(loaded.pilot().has_explored(SystemId(532)), "the load");
    // In play, refreshed as each script settles: the next rebuild point,
    // a landing elsewhere, shares it by the engine only.
    for source in RuleSource::ALL {
        let rules = Rulebook::default()
            .with_override(RuleKey::VisibilityRefresh, RuleSource::Bible)
            .with_override(RuleKey::ExploredByPlace, source);
        let mut session = swapping(&data, rules);
        assert!(session.galaxy().visible(SystemId(532)), "refreshed");
        assert!(!session.pilot().has_explored(SystemId(532)), "{source:?}");
        let here = session.system();
        let (system, stellar) = data
            .star_map()
            .into_iter()
            .filter(|system| system.id != here && session.galaxy().visible(system.id))
            .find_map(|system| {
                let sites = data.landing_sites(system.id);
                sites.first().map(|site| (system.id, site.id))
            })
            .expect("a stock system with a stellar");
        session.land_at(&data, system, stellar).expect("lands");
        assert_eq!(
            session.pilot().has_explored(SystemId(532)),
            source == RuleSource::Engine,
            "{source:?}: the landing's rebuild point"
        );
    }
}

/// A stock pilot flying in Homeland (173) with b305 set, so both versions
/// of Aldebaran are visible, reading `arrival_system` as `source`.
fn in_homeland(data: &GameData, source: RuleSource) -> Session {
    let mut pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    pilot.system = SystemId(173);
    pilot.bits_mut().set(Bit::new(B305).expect("a control bit"));
    let session = Session::fly(data, pilot)
        .expect("flies")
        .with_rules(Rulebook::default().with_override(RuleKey::ArrivalSystem, source));
    assert_eq!(session.system(), SystemId(173));
    assert!(session.galaxy().visible(SystemId(141)) && session.galaxy().visible(SystemId(532)));
    session
}

/// Flies out of the no-jump zone, jumps to the system selected, waits out
/// the warp and arrives.
fn jump(session: &mut Session, data: &GameData) -> Option<SystemId> {
    let thrust = Controls {
        thrust: true,
        ..Controls::default()
    };
    for _ in 0..10_000 {
        if session.player().position.length() >= session.stats().jump_distance {
            break;
        }
        session.tick(thrust);
    }
    session.begin_jump().expect("jumps");
    for _ in 0..10_000 {
        if session.warp_end_due() {
            break;
        }
        session.tick(Controls::default());
    }
    assert!(session.warp_end_due(), "the warp ends");
    session.arrive(data, &mut NeverFires)
}

#[test]
fn a_single_jump_to_aldebaran_selected_by_place_arrives_in_532_as_it_is() {
    let Some(data) = open() else {
        return;
    };
    for source in RuleSource::ALL {
        let mut session = in_homeland(&data, source);
        // The cue selects Aldebaran by place: 173's first Con slot standing
        // there, Con1, which names 532 (`_CueNextHyperRouteDest`
        // @0xe30f-0xe344).
        session.plot_course(SystemId(141)).expect("a route");
        assert_eq!(session.course(), [SystemId(141)]);
        let arrived = jump(&mut session, &data);
        let shown: BTreeSet<i16> = session.stellars_shown().map(|stellar| stellar.0).collect();
        match source {
            RuleSource::Engine => {
                assert_eq!(arrived, Some(SystemId(532)), "the raw Con link");
                assert!(shown.contains(&430) && shown.contains(&433), "{shown:?}");
                assert!(
                    SHARED.iter().all(|id| !shown.contains(id)),
                    "141's: {shown:?}"
                );
            }
            RuleSource::Bible => {
                assert_eq!(arrived, Some(SystemId(141)), "resolved by place");
                assert!(SHARED.iter().all(|id| shown.contains(id)), "{shown:?}");
                assert!(!shown.contains(&430) && !shown.contains(&433), "{shown:?}");
            }
        }
        assert_eq!(session.system(), arrived.expect("arrived"), "{source:?}");
        assert_eq!(session.course(), [] as [SystemId; 0], "{source:?}");
    }
}
