//! Tests that a load reads the rules it is given from its first read of
//! the galaxy on: `Session::load` sets the reading on the galaxy as it
//! builds it and takes the whole rule set before its first rebuild, so
//! every read of the load, and everything the load writes to the pilot, is
//! the chosen reading's. None of these calls `Session::with_rules`. A bare
//! `Session::fly` then `Session::with_rules` stays the case of the module
//! that reads each rule.

use crate::catalog::{StellarId, SystemId};
use crate::hyperspace::HyperlinkRule;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::session::Session;
use crate::session::kept_rescues::tests::by;
use crate::session::tests::alternates;
use crate::testkit::{FakePilotCatalog, hiding_bit, planet, star};

fn ids(systems: &[i16]) -> Vec<SystemId> {
    systems.iter().copied().map(SystemId).collect()
}

fn rules(key: RuleKey, source: RuleSource) -> Rulebook {
    Rulebook::default().with_override(key, source)
}

/// `catalog`'s new pilot.
fn pilot(catalog: &FakePilotCatalog) -> Pilot {
    Pilot::new(catalog, "").expect("a pilot")
}

/// `pilot` loaded over `catalog` by `rules`.
fn loaded(catalog: &FakePilotCatalog, pilot: Pilot, rules: &Rulebook) -> Session {
    Session::load(catalog, pilot, rules).expect("loads")
}

#[test]
fn a_new_pilots_session_reads_the_rules_it_is_given() {
    let catalog = alternates();
    for source in RuleSource::ALL {
        let rules = rules(RuleKey::ColocatedResolution, source);
        let session = Session::start_ruled(&catalog, &rules).expect("starts");
        assert_eq!(session.rules(), &rules, "{source:?}");
        assert_eq!(
            session
                .star_map()
                .jumps(SystemId(130), HyperlinkRule::Engine),
            by(source, ids(&[131]), ids(&[131, 134])),
            "{source:?}"
        );
    }
}

#[test]
fn a_load_holds_the_rulebook_it_is_given() {
    let catalog = alternates();
    let every = RuleKey::ALL
        .into_iter()
        .fold(Rulebook::default(), |book, key| {
            book.with_override(key, RuleSource::Bible)
        });
    assert_eq!(
        loaded(&catalog, pilot(&catalog), &every).rules(),
        &every,
        "the load's own, with no setup after"
    );
}

#[test]
fn a_loads_links_and_known_set_follow_the_colocated_rule_it_is_given() {
    // 130, explored, lists 134 too: a Con slot is known through its place.
    let mut catalog = alternates();
    catalog
        .star_map
        .iter_mut()
        .find(|system| system.id == SystemId(130))
        .expect("130 on the map")
        .links
        .push(SystemId(134));
    for source in RuleSource::ALL {
        let rules = rules(RuleKey::ColocatedResolution, source);
        let session = loaded(&catalog, pilot(&catalog), &rules);
        assert_eq!(
            session
                .star_map()
                .jumps(SystemId(130), HyperlinkRule::Engine),
            by(source, ids(&[131]), ids(&[131, 134])),
            "{source:?}"
        );
        assert_eq!(
            session.is_known(SystemId(134)),
            by(source, false, true),
            "the load's rebuild read that map: {source:?}"
        );
    }
}

#[test]
fn a_load_resumes_in_the_owner_the_colocated_rule_it_is_given_says() {
    // Stellar 160 is listed by 134, which 131 shadows by the engine, and
    // by 135, elsewhere.
    let mut catalog = alternates();
    catalog.star_map.push(star(135, (900.0, 900.0), &[]));
    catalog.systems.push(SystemId(135));
    for (system, sites) in &mut catalog.sites {
        if *system == SystemId(134) {
            sites.push(planet(160, 0.0, 0.0));
        }
    }
    catalog
        .sites
        .push((SystemId(135), vec![planet(160, 0.0, 0.0)]));
    for source in RuleSource::ALL {
        let mut last_landed = pilot(&catalog);
        last_landed.stellar = Some(StellarId(160));
        let rules = rules(RuleKey::ColocatedResolution, source);
        let session = loaded(&catalog, last_landed, &rules);
        assert_eq!(
            session.system(),
            by(source, SystemId(135), SystemId(134)),
            "{source:?}"
        );
        assert!(!session.load_repaired(), "{source:?}");
    }
}

#[test]
fn a_load_shares_explored_levels_by_place_only_as_the_rule_it_is_given_says() {
    let catalog = alternates();
    for source in RuleSource::ALL {
        let mut explorer = pilot(&catalog);
        explorer.explore(SystemId(131));
        let rules = rules(RuleKey::ExploredByPlace, source);
        let session = loaded(&catalog, explorer, &rules);
        assert_eq!(
            session.pilot().has_explored(SystemId(134)),
            by(source, true, false),
            "131 heads 134's place: {source:?}"
        );
    }
}

#[test]
fn a_load_rewrites_a_kept_course_only_as_the_rule_it_is_given_says() {
    let catalog = alternates();
    for source in RuleSource::ALL {
        // Saved bound through 131, hidden, with 134 at its place.
        let mut bound = pilot(&catalog);
        bound.course = ids(&[131, 132]);
        bound.course_start = Some(SystemId(130));
        bound.bits_mut().set(hiding_bit(131));
        bound.galaxy.set_visible(SystemId(131), Some(false));
        let rules = rules(RuleKey::RouteRewrite, source)
            .with_override(RuleKey::LoadedCourse, RuleSource::Bible);
        let session = loaded(&catalog, bound, &rules);
        assert_eq!(
            session.course(),
            by(source, ids(&[134, 132]), ids(&[131, 132])),
            "{source:?}"
        );
    }
}
