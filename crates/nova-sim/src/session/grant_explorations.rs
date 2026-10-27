//! A map grant's explorations run at once, wherever the grant runs
//! (crons-news-ranks-fidelity phase 43): `_GrantOutfitItem`'s map arm
//! (@0x44d9b) explores synchronously, its `OnExplore`s before the rest of
//! the grant and before its caller's next step, for each of its three
//! callers: `G` (`_EvalSetExp` @0x15444), the outfitter's buy
//! (`_DoOutfitDialog` @0x5c21d) and the plunder (`_DoPlunderDialog`
//! @0x93216). See [`Session::grant_outfit`].

use std::rc::Rc;

use super::crons::tests::scripted;
use super::jump_exploration::tests::{Explorations, watching_explored};
use super::nebulae::tests::nebulous;
use super::script_effects::Settled;
use super::story::{NoCatalog, SettlingCatalog};
use super::tests::{ACE_DRAWS, alongside_ace, board_drawing, buy, granting, outfitted, outfitting};
use crate::board::{Boarding, NovaBoarding};
use crate::catalog::{NebulaRecord, OutfitId, OutfitRecord, StellarId, SystemId};
use crate::chance::NeverFires;
use crate::control::{Bit, Script, SetExpr, Test};
use crate::outfit_effects::{MAP, PAINT, Rgb15};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::session::Session;
use crate::story::{
    Consumer, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::testkit::{FakePilotCatalog, nebula, outfit};

fn bit(n: u16) -> Bit {
    Bit::new(n).expect("a bit")
}

fn on(session: &Session, n: u16) -> bool {
    session.pilot().control_bit(bit(n))
}

fn run(session: &mut Session, script: &str) {
    session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
}

/// Nebula 128 around 131 alone, always active, running `on_explore`.
fn around_131(on_explore: &str) -> NebulaRecord {
    nebula(128, (500, -100, 200, 200), "", on_explore)
}

/// [`nebulous`] with [`around_131`] running `on_explore`, and outfit 300,
/// a map of 1 jump: from 130, 131.
fn mapped(on_explore: &str) -> FakePilotCatalog {
    FakePilotCatalog {
        outfits: vec![outfit(300, &[(MAP, 1)])],
        ..nebulous(vec![around_131(on_explore)])
    }
}

/// `catalog`'s new pilot flown by `source`'s reading of
/// [`RuleKey::StoryReentry`], its load settled.
fn flown(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
    let rules = Rulebook::default().with_override(RuleKey::StoryReentry, source);
    let mut session = Session::fly(catalog, Pilot::new(catalog, "").expect("starts"))
        .expect("flies")
        .with_rules(rules);
    let _ = session.settle_script(catalog, &mut NeverFires);
    session
}

/// Whether 131's exploration is raised and waits for a settling.
fn explored_131_raised(session: &Session) -> bool {
    session.story_raised().contains(&StoryEvent::Explored {
        system: SystemId(131),
    })
}

#[test]
fn a_grants_on_explore_runs_before_the_scripts_next_operator() {
    let catalog = mapped("!b9");
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = flown(&catalog, source);
        run(&mut session, "G300 b9");
        assert!(on(&session, 9), "{source:?}: the OnExplore's !b9, then b9");
        assert!(!explored_131_raised(&session), "{source:?}: none left");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(on(&session, 9), "{source:?}: nothing ran later");
    }
}

#[test]
fn a_cron_on_a_day_reads_a_bit_an_earlier_crons_map_grant_set() {
    // Crön 128 maps 131, whose OnExplore sets b9; crön 129, after it in
    // the pass, is enabled on b9.
    let mut reads_b9 = scripted(129, 1, "b10", "");
    reads_b9.enable_on = Test::parse("b9");
    let catalog = FakePilotCatalog {
        crons: vec![scripted(128, 1, "G300", ""), reads_b9],
        ..mapped("b9")
    };
    let mut session = flown(&catalog, RuleSource::Engine);
    session.let_days_pass(1, &mut NeverFires);
    assert!(on(&session, 9), "crön 128's map's OnExplore");
    assert!(on(&session, 10), "crön 129 started that day");
}

#[test]
fn the_outfitters_map_runs_its_on_explore_before_the_on_purchase() {
    // `_DoOutfitDialog` grants (@0x5c21d) before the `OnPurchase`
    // (@0x5c2cb).
    let mut catalog = outfitting();
    catalog.nebulae = vec![around_131("!b9 A131")];
    catalog.outfits.push(OutfitRecord {
        on_purchase: Script::parse("b9 A999"),
        ..outfit(310, &[(MAP, 1)])
    });
    let seen = Rc::new(Explorations::default());
    let mut session = outfitted(&catalog).with_set_op_observer(seen.clone());
    session
        .outfit(buy(OutfitId(310)), &mut NeverFires)
        .expect("bought");
    assert_eq!(seen.systems(), [SystemId(131), SystemId(999)]);
    assert!(
        on(&session, 9),
        "the OnExplore's !b9, then the OnPurchase's b9"
    );
    assert!(!explored_131_raised(&session));
}

#[test]
fn a_plundered_map_runs_its_on_explore_before_the_dialog_opens() {
    // `_DoPlunderDialog` grants (@0x93216) before the dialog shows.
    let mut catalog = granting();
    catalog.nebulae = vec![around_131("b9")];
    for record in &mut catalog.outfits {
        if record.id == OutfitId(200) {
            record.mods = outfit(200, &[(MAP, 1)]).mods;
        }
    }
    let mut session = alongside_ace(&catalog);
    let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
    assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
    assert!(on(&session, 9), "the OnExplore ran");
    assert!(!explored_131_raised(&session));
}

#[test]
fn a_grants_on_explore_runs_before_its_paint() {
    // The map arm explores before the paint is written (@0x44f9a-0x44fd1),
    // so the outer grant's paint is the one left.
    let outer = (3 << 10) | (5 << 5) | 7;
    let inner = (1 << 10) | (2 << 5) | 4;
    let catalog = FakePilotCatalog {
        outfits: vec![
            outfit(300, &[(MAP, 1), (PAINT, outer)]),
            outfit(301, &[(PAINT, inner)]),
        ],
        ..nebulous(vec![around_131("G301")])
    };
    let mut session = flown(&catalog, RuleSource::Engine);
    run(&mut session, "G300");
    let _ = session.settle_script(&catalog, &mut NeverFires);
    assert_eq!(session.pilot().paint(), Some(Rgb15 { r: 3, g: 5, b: 7 }));
}

/// `session` with a fixture subscription to `kind` running `script`.
fn subscribed(session: &mut Session, record: i16, kind: StoryEventKind, script: &str) {
    session
        .register_story(Subscription::new(
            SubscriptionId {
                consumer: Consumer::Fixture,
                record,
                hook: 0,
            },
            EventFilter::Any(kind),
            Script::parse(script),
        ))
        .expect("registers");
}

#[test]
fn an_on_explore_inside_a_dispatch_keeps_the_outer_queue() {
    // A landing's subscription grants the map; the take-off raised after
    // the landing runs `A999` once the landing's script has settled. The
    // OnExplore's `A131` runs inside the grant, before it, by either
    // reading.
    let catalog = mapped("A131");
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let mut session = flown(&catalog, source);
        subscribed(&mut session, 1, StoryEventKind::Landed, "G300");
        subscribed(&mut session, 2, StoryEventKind::TookOff, "A999");
        let seen = Rc::new(Explorations::default());
        let mut session = session.with_set_op_observer(seen.clone());
        let stellar = StellarId(128);
        session.run.raise(StoryEvent::Landed { stellar });
        session.run.raise(StoryEvent::TookOff { stellar });
        let _ = session.settle_script(&catalog, &mut NeverFires);
        let order: Vec<i16> = seen.systems().iter().map(|system| system.0).collect();
        assert_eq!(order, [131, 999], "{source:?}");
        assert_eq!(session.story_raised(), [], "{source:?}");
    }
}

#[test]
fn a_grant_outside_a_dispatch_leaves_the_events_raised_before_it_queued() {
    let catalog = mapped("b9");
    let (mut session, seen) = watching_explored(flown(&catalog, RuleSource::Engine), &[131]);
    let took_off = StoryEvent::TookOff {
        stellar: StellarId(128),
    };
    session.run.raise(took_off);
    run(&mut session, "G300");
    assert_eq!(seen.systems().len(), 1, "131's exploration, now");
    assert!(on(&session, 9));
    assert_eq!(session.story_raised(), [took_off], "kept, in order");
}

/// A session to hand [`NoCatalog`].
fn plain() -> Session {
    flown(&mapped(""), RuleSource::Engine)
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "the offer roll")]
fn no_catalog_fails_a_debug_build_that_rolls_the_offers() {
    NoCatalog.roll_landing_offers(&mut plain(), StellarId(128), &mut NeverFires);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "the re-offer")]
fn no_catalog_fails_a_debug_build_that_reoffers() {
    NoCatalog.reoffer(&mut plain(), &mut NeverFires);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "the moves")]
fn no_catalog_fails_a_debug_build_that_settles_the_moves() {
    NoCatalog.settle_moves(&mut plain(), &mut NeverFires, &mut Settled::default());
}
