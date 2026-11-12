//! The stellars' animation frames over the stock data (rendering phase
//! 11): no stock deadly stellar animates, so the deadly touch by the frame
//! shown is plug-in only; and a stock wormhole steps through a real
//! session as the sim's rule steps it, the wiring over real data (the
//! frame sequence itself is `nova_sim_port::stellar_frames`' to pin).
//! Skips, passing, when `NOVA_DATA` is unset.

use std::num::NonZeroU16;

use nova_data::GameData;
use nova_sim::ai::NovaAi;
use nova_sim::deadly_stellar::DEADLY_STELLAR;
use nova_sim::flight::Controls;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::pilot::Pilot;
use nova_sim::stellar_frames::{StellarAnimation, StellarFrames, is_animated};
use nova_sim::{LandingSite, NeverFires, PilotCatalog, Rules, Session, StellarId, SystemId};

use crate::common;

/// Spica, which owns wormhole 466.
const SPICA: SystemId = SystemId(144);
/// The wormhole Spica owns.
const WORMHOLE: StellarId = StellarId(466);

/// `count` as a frame count, 1 for none.
fn frames(count: u16) -> NonZeroU16 {
    NonZeroU16::new(count).unwrap_or(NonZeroU16::MIN)
}

/// The frames `site` animates over whole and destroyed: its `Type`
/// sheet's, and its `DeadType` sheet's when it has one.
fn counts(site: &LandingSite) -> (NonZeroU16, NonZeroU16) {
    let whole = frames(site.animation.type_sheet_frames);
    let dead = if site.dead_graphic.is_some() {
        frames(site.animation.dead_sheet_frames)
    } else {
        whole
    };
    (whole, dead)
}

#[test]
fn no_stock_deadly_stellar_animates() {
    let Some(data) = common::opened() else {
        return;
    };
    let mut deadly = 0;
    for system in data.star_map() {
        for site in data.landing_sites(system.id) {
            if site.flags2 & DEADLY_STELLAR == 0 {
                continue;
            }
            deadly += 1;
            let anim = StellarAnimation::of(&site);
            let (whole, dead) = counts(&site);
            assert!(!is_animated(anim, false, whole), "spöb {} whole", site.id.0);
            assert!(
                !is_animated(anim, true, dead),
                "spöb {} destroyed",
                site.id.0
            );
        }
    }
    assert_eq!(deadly, 0, "no stock spöb has Flags2 0x0100");
}

/// A new stock pilot flying in `system`, at its centre.
fn flying_in(data: &GameData, system: SystemId) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::Value::Null;
    save["system"] = serde_json::json!(system.0);
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, there).expect("flies");
    assert_eq!(session.system(), system);
    session
}

/// The frame `session` reports `stellar` shows, and the gain its next
/// pass runs at.
fn reported(session: &Session, stellar: StellarId) -> (u16, nova_sim::SpeedMult) {
    let mut frame = FlightFrame::default();
    session.fill_frame(&mut frame);
    let shown = frame.stellar(stellar).expect("listed");
    assert!(shown.shown, "shown");
    (shown.frame, frame.speed_mult)
}

#[test]
fn a_stock_wormhole_steps_through_the_session_as_its_rule_steps_it() {
    let Some(data) = common::opened() else {
        return;
    };
    let site = data
        .landing_sites(SPICA)
        .into_iter()
        .find(|site| site.id == WORMHOLE)
        .expect("Spica lists 466");
    let anim = StellarAnimation::of(&site);
    let (count, _) = counts(&site);
    assert!(is_animated(anim, false, count), "the wormhole animates");
    let mut session = flying_in(&data, SPICA);
    let mut oracle = StellarFrames::new();
    let behaviour = NovaAi::default();
    // Over its whole cycle, and a little more.
    let passes =
        usize::from(count.get()) * usize::try_from(anim.delay.max(1)).expect("a delay") + 8;
    let mut turned = 0;
    for pass in 0..passes {
        let (before, gain) = reported(&session, WORMHOLE);
        let rescued = session.game_work(
            Controls::default(),
            &data,
            &behaviour,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
        oracle.step_by(gain, anim, false, count, count, &mut NeverFires);
        let (after, _) = reported(&session, WORMHOLE);
        assert_eq!(after, oracle.shown(), "pass {pass}");
        turned += usize::from(after != before);
    }
    assert!(
        turned >= usize::from(count.get()),
        "it went round: {turned}"
    );
}
