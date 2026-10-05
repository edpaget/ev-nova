//! Hailing ships in flight: the session's side of [`hail`](crate::hail).
//!
//! The player hails its target ([`Session::hail`]); the conversation
//! stays open until the player hangs up ([`Session::hang_up`]), lands, or
//! leaves the system, and while it is open the dialog reads it
//! ([`Session::hailing`]). A press of an option ([`Session::answer`]) says
//! its reply, does its deed, and asks its price, drawing the haggle roll
//! then; the player haggles ([`Session::haggle`]) and pays from its cash,
//! which makes no save due. Every reply is said here, in the
//! conversation's variant, read from the caller's [`CommCatalog`].
//!
//! The deeds:
//!
//! - **Spare** (a bribe paid): the ship is [`spared`](Npc::spared) and
//!   becomes a wimpy trader for the rest of its stay (@0x961f8); it leaves
//!   (`_AIMakeShipLeave` @0x7e2b0), jumping out with a jump's fuel, or
//!   else deciding as its idle block does, which lands; its target and
//!   fire command are gone, and so is its provocation.
//! - **Attack** (a bribe declined, `_AIMakeShipAttackPlayer` @0x89c3e):
//!   it attacks the player.
//! - **Release**: the player's escort leaves the fleet once the channel
//!   closes ([`Session::hang_up`]), as the original's dialog does when it
//!   closes (@0x96852): its AI type its `InherentAI`, of no government
//!   still, it leaves the system (`_AIMakeShipLeave`), jumping out with a
//!   jump's fuel, or else deciding as its idle block does. A save is due.
//!   The player's escort is hailed whatever its ship type's government
//!   says, and opens "What can I do for you?".
//! - **Help** (`_AIMakeShipRefuelPlayer` @0x82dbb,
//!   `_AIMakeShipRepairPlayer` @0x82df2): it assists the player
//!   ([`Goal::Assist`]), its provocation gone. Each step
//!   ([`Session::tick_assistance`]) gives the help as
//!   [`assist`](crate::hail::assist) says. Once done it says so
//!   ([`Session::take_comm`]) and decides again; a repair for a player no
//!   longer disabled ends at once, silently. The traffic lets an
//!   assisting ship decide again once anything provokes it.

use super::Session;
use crate::ai::Goal;
use crate::catalog::CommCatalog;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::combat::hull::{Condition, DisableRule};
use crate::hail::{
    Ask, Attitude, CommNote, Conversation, Deed, Dispositions, Haggle, Hail, HailButton,
    HailOptions, HailRefusal, HailView, Help, Reply, Settled, assist, attitude, reply,
};
use crate::hyperspace::JUMP_FUEL;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};

/// A hail under way: the NPC hailed, the conversation, what it last
/// said, and the price it asks while the player haggles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Talk {
    npc: NpcId,
    conversation: Conversation,
    reply: Reply,
    ask: Option<Ask>,
    /// Whether the escort hailed is to be released once the channel
    /// closes.
    release: bool,
}

impl Session {
    /// Hails the player's target with `options` listed, its conversation
    /// rolled on `chance` (see [`hail`](crate::hail)), and gives the comm
    /// dialog's contents, its replies read from `catalog`. Any hail under
    /// way ends.
    ///
    /// # Errors
    ///
    /// The [`HailRefusal`] that applies, drawing nothing:
    /// [`HailRefusal::NoTarget`] with no target, or while landed, jumping
    /// or breaking up; [`HailRefusal::InHyperspace`] for a target still
    /// jumping in; and [`HailRefusal::NoResponse`] for a disabled or mute
    /// one.
    pub fn hail(
        &mut self,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
        chance: &mut dyn Chance,
    ) -> Result<HailView, HailRefusal> {
        self.talk = None;
        let flying = !matches!(
            self.condition,
            Condition::Dying { .. } | Condition::Destroyed
        );
        if self.landed.is_some() || self.jumping.is_some() || !flying {
            return Err(HailRefusal::NoTarget);
        }
        let npc = self.target().ok_or(HailRefusal::NoTarget)?;
        if npc.mode != Mode::Flying {
            return Err(HailRefusal::InHyperspace);
        }
        let dispositions = self.dispositions(npc);
        let escort = npc.escort.is_some();
        if npc.condition != Condition::Intact || (dispositions.mute && !escort) {
            return Err(HailRefusal::NoResponse);
        }
        let conversation =
            Conversation::open(self.pilot.cash, dispositions.greedy, npc.info_types, chance);
        let opening = if escort {
            reply::ESCORT_OPENING
        } else {
            reply::opening(
                !dispositions.untalkative,
                attitude(npc, &self.world().around(self.npcs())),
            )
        };
        self.talk = Some(Talk {
            npc: npc.id,
            conversation,
            reply: Reply::Comm(opening),
            ask: None,
            release: false,
        });
        self.hailing(catalog, options).ok_or(HailRefusal::NoTarget)
    }

    /// What `npc`'s government and ship type make of hails.
    fn dispositions(&self, npc: &Npc) -> Dispositions {
        let inherent = self
            .ship_record(npc.ship)
            .and_then(|record| record.inherent_govt);
        Dispositions::of(npc, inherent, &self.govts)
    }

    /// The hail of `npc` in conversation `talk`, as the options see it.
    fn hail_of<'a>(&self, npc: &'a Npc, talk: &Conversation) -> Hail<'a> {
        let need = assist::need(self.condition, self.pilot.reserves.fuel);
        let around = self.world();
        let around = around.around(self.npcs());
        Hail::new(npc, &around, self.dispositions(npc), talk, need)
    }

    /// The comm dialog's contents while a hail is under way and its ship
    /// is still in the system, `options` listed and the reply read from
    /// `catalog`.
    #[must_use]
    pub fn hailing(
        &self,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
    ) -> Option<HailView> {
        let talk = self.talk?;
        let npc = self.npcs().iter().find(|npc| npc.id == talk.npc)?;
        let hail = self.hail_of(npc, &talk.conversation);
        let buttons = options
            .listed(&hail)
            .iter()
            .map(|option| HailButton {
                label: option.label(),
                key: option.key(),
            })
            .collect();
        Some(HailView {
            npc: npc.id,
            ship: npc.ship,
            reply: talk.reply.say(talk.conversation.variant, catalog),
            comm_name: self
                .ship_record(npc.ship)
                .map(|record| record.comm_name.clone())
                .unwrap_or_default(),
            govt_name: npc
                .govt
                .and_then(|govt| self.govts.get(govt))
                .map(|record| record.comm_name.clone()),
            hostile: hail.attitude == Attitude::Hostile,
            options: buttons,
            asking: talk.conversation.asking(),
            pay_me: true,
        })
    }

    /// Presses the `pick`th option listed (from 0), as `options` lists
    /// them, rolling on `chance`: the ship says its reply, does its deed,
    /// and asks its price, the haggle roll drawn then. Gives the comm
    /// dialog's contents; none, and nothing changes, with no hail under
    /// way, its ship gone, or no such option. While a price is asked,
    /// nothing is pressed.
    pub fn answer(
        &mut self,
        pick: usize,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
        chance: &mut dyn Chance,
    ) -> Option<HailView> {
        let mut talk = self.talk?;
        if talk.ask.is_some() {
            return self.hailing(catalog, options);
        }
        let answer = {
            let npc = self.npcs().iter().find(|npc| npc.id == talk.npc)?;
            let hail = self.hail_of(npc, &talk.conversation);
            options.listed(&hail).get(pick)?.press(&hail, chance)
        };
        if let Some(said) = answer.reply {
            talk.reply = said;
        }
        if let Some(ask) = answer.ask {
            talk.conversation.ask(chance);
            talk.ask = Some(ask);
        }
        self.talk = Some(talk);
        if let Some(deed) = answer.deed {
            self.act(talk.npc, deed);
        }
        self.hailing(catalog, options)
    }

    /// Makes `choice` in the haggle dialog over the price asked: a price
    /// lowered keeps the haggling going; paid, the cash goes and the ship
    /// says and does as its price said; declined or short, likewise. Gives
    /// the comm dialog's contents; none, and nothing changes, with no
    /// price asked.
    pub fn haggle(
        &mut self,
        choice: Haggle,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
    ) -> Option<HailView> {
        let mut talk = self.talk?;
        let ask = talk.ask?;
        let settled = talk.conversation.haggle(choice, self.pilot.cash)?;
        let (said, deed) = match settled {
            Settled::Lowered => (talk.reply, None),
            Settled::Paid(price) => {
                self.pilot.cash -= price;
                ask.paid
            }
            Settled::Declined => ask.declined,
            Settled::Short => (ask.short, None),
        };
        if settled != Settled::Lowered {
            talk.ask = None;
        }
        talk.reply = said;
        self.talk = Some(talk);
        if let Some(deed) = deed {
            self.act(talk.npc, deed);
        }
        self.hailing(catalog, options)
    }

    /// Ends the hail under way, if any; an escort Release was pressed
    /// for is released now.
    pub fn hang_up(&mut self) {
        if let Some(talk) = self.talk.take()
            && talk.release
        {
            self.release(talk.npc);
        }
    }

    /// NPC `id` does `deed` (see the module docs).
    fn act(&mut self, id: NpcId, deed: Deed) {
        if deed == Deed::Release {
            if let Some(talk) = &mut self.talk {
                talk.release = true;
            }
            return;
        }
        let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
            return;
        };
        match deed {
            Deed::Spare => {
                npc.spared = true;
                npc.ai_type = AiType::WimpyTrader;
                npc.goal = if npc.reserves.fuel.now >= JUMP_FUEL {
                    Goal::JumpOut
                } else {
                    Goal::Idle
                };
                npc.target = None;
                npc.trigger = Trigger::default();
                npc.provoked = 0.0;
            }
            Deed::Attack => {
                npc.goal = Goal::Attack(ShipRef::Player);
                npc.target = Some(ShipRef::Player);
            }
            Deed::Release => {}
            Deed::Help(help) => {
                npc.goal = Goal::Assist(help);
                npc.target = Some(ShipRef::Player);
                npc.trigger = Trigger::default();
                npc.provoked = 0.0;
                npc.assisting = 0;
            }
        }
    }

    /// Gives a tick of the help each NPC assisting the player gives (see
    /// [`assist`](crate::hail::assist)), a repair raising the armour until
    /// `rule` no longer has the player disabled. While the ship is landed
    /// or jumping, nothing happens.
    pub fn tick_assistance(&mut self, rule: &dyn DisableRule) {
        if self.landed.is_some() || self.jumping.is_some() {
            return;
        }
        let player = self.player;
        let disabled = self.condition == Condition::Disabled;
        let mut done = Vec::new();
        for npc in self.traffic.npcs_mut() {
            let Goal::Assist(help) = npc.goal else {
                continue;
            };
            if npc.condition != Condition::Intact || npc.mode != Mode::Flying {
                continue;
            }
            let reach = assist::reach(help, npc.stats.handling.turn_rate);
            let within = assist::within(&npc.state, &player, reach);
            let finished = match help {
                Help::Refuel => {
                    within && assist::docked(&npc.state) && refuel(&mut self.pilot.reserves.fuel)
                }
                Help::Repair if !disabled => {
                    stop(npc);
                    continue;
                }
                Help::Repair => {
                    if within {
                        npc.assisting += 1;
                    }
                    npc.assisting >= assist::REPAIR_TICKS && {
                        repair(&mut self.pilot.reserves.armor, &self.hull, rule);
                        true
                    }
                }
            };
            if finished {
                stop(npc);
                done.push((npc.id, npc.ship, help));
            }
        }
        for (from, ship, help) in done {
            self.comm.push(CommNote {
                from,
                comm_name: self
                    .ship_record(ship)
                    .map(|record| record.comm_name.clone())
                    .unwrap_or_default(),
                done: help,
            });
        }
    }

    /// What the NPCs assisting the player have done since this was last
    /// taken, in order; taking it empties the list.
    pub fn take_comm(&mut self) -> Vec<CommNote> {
        std::mem::take(&mut self.comm)
    }
}

/// A tick of refuelling into `fuel`, and whether the refuel is done: it
/// gains [`assist::REFUEL_STEP`] while it holds no more than
/// [`assist::REFUEL_UNTIL`], no further than its most, and is done above
/// that or full.
fn refuel(fuel: &mut crate::reserves::Gauge) -> bool {
    if fuel.now <= assist::REFUEL_UNTIL {
        fuel.now = (fuel.now + assist::REFUEL_STEP).min(fuel.max);
    }
    fuel.now > assist::REFUEL_UNTIL || fuel.now >= fuel.max
}

/// Raises `armor` a step at a time, no further than its most, until
/// `rule` no longer has a ship of `hull` disabled.
fn repair(
    armor: &mut crate::reserves::Gauge,
    hull: &crate::combat::hull::HullSpec,
    rule: &dyn DisableRule,
) {
    // At most the steps up to its most (a step is a point), so a rule
    // that never lets it fly still ends.
    let steps = (armor.max - armor.now).ceil().max(0.0) as u32;
    for _ in 0..steps {
        if !rule.disabled(*armor, hull) {
            return;
        }
        armor.now = (armor.now + assist::REPAIR_STEP).min(armor.max);
    }
}

/// `npc` stops assisting, and decides again.
fn stop(npc: &mut Npc) {
    npc.goal = Goal::Idle;
    npc.target = None;
    npc.assisting = 0;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::ai::NovaAi;
    use crate::catalog::{DudeId, DudeRecord, GovtId, GovtRecord, ShipId, SystemTraffic};
    use crate::catalog::{StellarId, SystemId};
    use crate::combat::Strike;
    use crate::combat::hull::{HullSpec, NovaDisable};
    use crate::geometry::Vec2;
    use crate::govt::XENOPHOBIC;
    use crate::hail::{Answer, BRIBABLE_WARSHIPS, HailOption, MUTE, UNTALKATIVE};
    use crate::handling::ShipFields;
    use crate::reserves::Gauge;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::testkit::{Draws, FAST, FakePilotCatalog, govt, ship};

    const FED: GovtId = GovtId(128);
    const OTHER: GovtId = GovtId(129);
    const PIRATES: GovtId = GovtId(137);
    const HARDLINERS: GovtId = GovtId(138);
    const SILENT: GovtId = GovtId(139);
    const MUTED: GovtId = GovtId(150);

    /// The first variant's words of the groups the tests read; every
    /// other ship comm string is "c<n>".
    const SAID: [(u16, &str); 18] = [
        (1, "Channel open."),
        (11, "What is it you want?"),
        (21, "What can I do for you?"),
        (22, "What can I help you with?"),
        (46, "Nice to meet you."),
        (61, "Yeah, come back when you actually have some money."),
        (66, "Stop wasting my time."),
        (71, "You're not in any trouble."),
        (81, "I'm busy."),
        (91, "You'll have to pay me first."),
        (96, "In your dreams, pal."),
        (101, "A pleasure doing business with you."),
        (116, "You're lucky - I'm in a good mood today."),
        (121, "I'm in a bad mood today, so it's going to cost you."),
        (141, "I'll help you out if you pay me."),
        (146, "Okay, I'm on my way."),
        (151, "What? How dare you! Prepare to die!"),
        (156, "Ha ha ha. What a comedian."),
    ];

    fn comm_strings() -> Vec<String> {
        (1..=200)
            .map(|n| {
                SAID.iter()
                    .find(|(at, _)| *at == n)
                    .map_or_else(|| format!("c{n}"), |(_, said)| (*said).to_owned())
            })
            .collect()
    }

    /// The Federation's hail lines, "f<n>" for each of ten.
    fn fed_hails() -> Vec<String> {
        (1..=10).map(|n| format!("f{n}")).collect()
    }

    /// System 130, the Federation's (128: class 1, `CrimeTol` 6), holds
    /// one ship of düde 128: a Federation warship (AI 3) of ship 129
    /// ("Cruiser", `Maneuver` 20), whose `InfoTypes` is 0x8000. Pirates
    /// (137) are xenophobes who take bribes; hardliners (138) are
    /// xenophobes who take none; 139 says nothing; 150 answers no hail.
    /// `STR#` 3000 holds the replies, 2002 #175 "Greetings.", 7000 the
    /// Federation's hail lines and 7505 specific advice.
    fn hailable() -> FakePilotCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        let mut catalog = FakePilotCatalog {
            govts: vec![
                GovtRecord {
                    crime_tol: 6,
                    classes: [1, -1, -1, -1],
                    comm_name: "Federation".to_owned(),
                    ..govt(128)
                },
                govt(129),
                GovtRecord {
                    flags: XENOPHOBIC | BRIBABLE_WARSHIPS,
                    comm_name: "Pirate".to_owned(),
                    ..govt(137)
                },
                GovtRecord {
                    flags: XENOPHOBIC,
                    ..govt(138)
                },
                GovtRecord {
                    flags2: UNTALKATIVE,
                    ..govt(139)
                },
                GovtRecord {
                    flags: MUTE,
                    ..govt(150)
                },
            ],
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types,
                    avg_ships: 1,
                },
            )],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 3,
                    govt: Some(FED),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0x8000,
                },
            )],
            ship_records: vec![crate::catalog::ShipRecord {
                comm_name: "Cruiser".to_owned(),
                ..ship(
                    129,
                    ShipFields {
                        maneuver: 20,
                        ..FAST
                    },
                )
            }],
            strings: vec![
                (3000, comm_strings()),
                (
                    2002,
                    (1..=200)
                        .map(|n| {
                            if n == 175 {
                                "Greetings.".to_owned()
                            } else {
                                format!("m{n}")
                            }
                        })
                        .collect(),
                ),
                (7000, fed_hails()),
                (7505, vec!["Mind the pirates.".to_owned()]),
                (
                    3001,
                    vec![
                        "Goodbye, captain.".to_owned(),
                        "See you around the galaxy.".to_owned(),
                    ],
                ),
            ],
            ..crate::testkit::catalog()
        };
        catalog.star_map[0].govt = Some(FED);
        catalog
    }

    /// The draws placing the ship at (`x`, `y`), facing up.
    fn placed(x: i32, y: i32) -> Draws {
        Draws::of(&[6, 6, 0, 0, (x + 750) as u32, (y + 750) as u32, 0, 0])
    }

    /// `catalog`'s session with its ship (NPC 0) 300 above the player at
    /// the centre, at rest, targeted; the pilot holds 100,000 credits.
    fn targeting(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut placed(0, -300));
        session.target = Some(NpcId(0));
        session.pilot.cash = 100_000;
        session
    }

    /// NPC 0 of `session`.
    fn npc(session: &mut Session) -> &mut Npc {
        &mut session.traffic.npcs_mut()[0]
    }

    /// A conversation's opening draws: variant `variant`, an even mood,
    /// the price's one draw of none.
    fn opening(variant: u32) -> Vec<u32> {
        vec![variant, 20, 1, 1, 0]
    }

    fn hail_with(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        draws: &[u32],
    ) -> (Result<HailView, HailRefusal>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let hailed = session.hail(catalog, &HailOptions::default(), &mut chance);
        (hailed, chance.asked)
    }

    /// Hails, its conversation in variant 0 and an even mood.
    fn hail(session: &mut Session, catalog: &FakePilotCatalog) -> HailView {
        hail_with(session, catalog, &opening(0))
            .0
            .expect("answered")
    }

    fn labels(view: &HailView) -> Vec<(&str, Option<char>)> {
        view.options
            .iter()
            .map(|button| (button.label.as_str(), button.key))
            .collect()
    }

    // Who can be hailed.

    #[test]
    fn with_no_target_or_while_landed_jumping_or_dying_nothing_is_hailed() {
        let catalog = hailable();
        let mut untargeted = targeting(&catalog);
        untargeted.target = None;
        let mut landed = targeting(&catalog);
        landed.landed = Some(StellarId(128));
        let mut jumping = targeting(&catalog);
        jumping.jumping = Some(SystemId(131));
        let mut dying = targeting(&catalog);
        dying.condition = Condition::Dying { ticks_left: 3 };
        let mut destroyed = targeting(&catalog);
        destroyed.condition = Condition::Destroyed;
        for mut session in [untargeted, landed, jumping, dying, destroyed] {
            let (hailed, asked) = hail_with(&mut session, &catalog, &opening(0));
            assert_eq!(hailed, Err(HailRefusal::NoTarget));
            assert!(asked.is_empty(), "{asked:?}");
            assert!(session.hailing(&catalog, &HailOptions::default()).is_none());
        }
    }

    #[test]
    fn a_disabled_player_can_still_hail() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.condition = Condition::Disabled;
        assert!(hail_with(&mut session, &catalog, &opening(0)).0.is_ok());
    }

    #[test]
    fn a_disabled_or_mute_target_gives_no_response() {
        let catalog = hailable();
        let mut disabled = targeting(&catalog);
        npc(&mut disabled).condition = Condition::Disabled;
        let mut mute = targeting(&catalog);
        npc(&mut mute).govt = Some(MUTED);
        let mut inherited = hailable();
        inherited.ship_records[0].inherent_govt = Some(MUTED);
        let inheriting = targeting(&inherited);
        for (mut session, catalog) in [
            (disabled, &catalog),
            (mute, &catalog),
            (inheriting, &inherited),
        ] {
            let (hailed, asked) = hail_with(&mut session, catalog, &opening(0));
            assert_eq!(hailed, Err(HailRefusal::NoResponse));
            assert!(asked.is_empty(), "{asked:?}");
        }
    }

    #[test]
    fn a_target_still_jumping_in_cannot_be_hailed() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).mode = Mode::JumpingIn { ticks_left: 4 };
        let (hailed, asked) = hail_with(&mut session, &catalog, &opening(0));
        assert_eq!(hailed, Err(HailRefusal::InHyperspace));
        assert!(asked.is_empty());
    }

    // The reply.

    #[test]
    fn a_ship_opens_by_the_players_record_with_its_government() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.pilot.set_legal_record(FED, -6);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.reply, "Channel open.");
        assert_eq!(
            labels(&view),
            [("Greetings", Some('G')), ("Request Assistance", Some('R'))]
        );
        session.pilot.set_legal_record(FED, -7);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.reply, "What is it you want?");
        assert_eq!(
            labels(&view),
            [("Greetings", Some('G')), ("Request Assistance", Some('R'))]
        );
        assert!(!view.hostile);
    }

    #[test]
    fn a_hostile_pirate_lists_beg_for_mercy_and_an_untalkative_ship_greetings_alone() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).govt = Some(PIRATES);
        npc(&mut session).goal = Goal::Attack(ShipRef::Player);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.reply, "What is it you want?");
        assert_eq!(
            labels(&view),
            [("Greetings", Some('G')), ("Beg For Mercy", Some('R'))]
        );
        assert!(view.hostile);
        npc(&mut session).govt = Some(SILENT);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.reply, "Channel open.", "untalkative: no question");
        assert_eq!(labels(&view), [("Greetings", Some('G'))]);
    }

    #[test]
    fn the_view_names_the_ship_and_its_government() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.npc, NpcId(0));
        assert_eq!(view.ship, ShipId(129));
        assert_eq!(view.comm_name, "Cruiser");
        assert_eq!(view.govt_name.as_deref(), Some("Federation"));
        assert_eq!(view.asking, None);
        assert!(view.pay_me);
        npc(&mut session).govt = None;
        assert_eq!(hail(&mut session, &catalog).govt_name, None, "independent");
        assert_eq!(
            session
                .hailing(&catalog, &HailOptions::default())
                .map(|view| view.npc),
            Some(NpcId(0)),
            "it stays open"
        );
    }

    #[test]
    fn every_reply_is_said_in_the_conversations_variant() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let (hailed, asked) = hail_with(&mut session, &catalog, &opening(3));
        assert_eq!(hailed.map(|view| view.reply), Ok("c4".to_owned()));
        assert_eq!(asked, [5, 41, 5, 5, 1], "the conversation's draws");
        session.pilot.set_legal_record(FED, -7);
        let mut chance = Draws::of(&[]);
        let view = session
            .answer(0, &catalog, &HailOptions::default(), &mut chance)
            .expect("open");
        assert_eq!(view.reply, "c69", "Stop wasting my time, variant 3");
    }

    /// The reply to Greetings from `session`'s ship, hailed in `variant`.
    fn greeted(session: &mut Session, catalog: &FakePilotCatalog, variant: u32) -> String {
        hail_with(session, catalog, &opening(variant))
            .0
            .expect("answered");
        let mut chance = Draws::of(&[]);
        let view = session
            .answer(0, catalog, &HailOptions::default(), &mut chance)
            .expect("open");
        assert!(chance.asked.is_empty(), "Greetings draws nothing");
        view.reply
    }

    #[test]
    fn a_friendly_warship_greets_with_its_governments_hail() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        assert_eq!(greeted(&mut session, &catalog, 1), "f7");
        assert_eq!(greeted(&mut session, &catalog, 0), "f6");
        npc(&mut session).ai_type = AiType::WimpyTrader;
        assert_eq!(greeted(&mut session, &catalog, 1), "f2");
    }

    #[test]
    fn a_hail_line_passed_over_or_missing_is_nice_to_meet_you() {
        let long = "x".repeat(42);
        for line in ["*", "*Federation", long.as_str()] {
            let mut catalog = hailable();
            catalog.strings[2].1[6] = line.to_owned();
            let mut session = targeting(&catalog);
            assert_eq!(greeted(&mut session, &catalog, 1), "c47", "{line:?}");
        }
        let mut short = hailable();
        short.strings[2].1.truncate(3);
        let mut session = targeting(&short);
        assert_eq!(greeted(&mut session, &short, 1), "c47", "a missing string");
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).govt = Some(OTHER);
        assert_eq!(greeted(&mut session, &catalog, 1), "c47", "a missing list");
        let mut brief = hailable();
        brief.strings[2].1[6] = "Hi".to_owned();
        let mut session = targeting(&brief);
        assert_eq!(greeted(&mut session, &brief, 1), "Hi", "two characters");
    }

    #[test]
    fn by_the_other_reading_a_long_hail_line_is_shown() {
        let long = "x".repeat(42);
        let mut catalog = hailable();
        catalog.strings[2].1[6] = long.clone();
        let mut session = targeting(&catalog);
        let options = HailOptions::nova(
            &Rulebook::default().with_override(RuleKey::LongAdvice, RuleSource::Bible),
        );
        let mut chance = Draws::of(&opening(1));
        session
            .hail(&catalog, &options, &mut chance)
            .expect("answered");
        let view = session
            .answer(0, &catalog, &options, &mut chance)
            .expect("open");
        assert_eq!(view.reply, long);
    }

    #[test]
    fn specific_advice_is_its_lists_first_string_or_greetings() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).info_types = 0x4005;
        assert_eq!(greeted(&mut session, &catalog, 2), "Mind the pirates.");
        let mut blank = hailable();
        blank.strings[3].1[0] = String::new();
        let mut session = targeting(&blank);
        npc(&mut session).info_types = 0x4005;
        assert_eq!(greeted(&mut session, &blank, 2), "Greetings.");
        let mut empty = hailable();
        empty.strings[3].1.clear();
        let mut session = targeting(&empty);
        npc(&mut session).info_types = 0x4005;
        assert_eq!(greeted(&mut session, &empty, 2), "Greetings.");
    }

    #[test]
    fn greetings_twice_says_the_same_and_rolls_nothing_more() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).info_types = 0xC000;
        let mut chance = Draws::of(&[1, 20, 1, 1, 0, 1]);
        session
            .hail(&catalog, &HailOptions::default(), &mut chance)
            .expect("answered");
        assert_eq!(chance.asked, [5, 41, 5, 5, 1, 2], "the advice drawn once");
        let mut again = Draws::of(&[]);
        for _ in 0..2 {
            let view = session
                .answer(0, &catalog, &HailOptions::default(), &mut again)
                .expect("open");
            assert_eq!(view.reply, "f7");
        }
        assert!(again.asked.is_empty());
    }

    // Answers and haggling.

    /// The pirates' warship (NPC 0, AI 3), 300 above the player at the
    /// centre and attacking it after a tick of Nova's AI, targeted.
    fn pirate(catalog: &FakePilotCatalog, govt: GovtId) -> Session {
        let mut session = targeting(catalog);
        npc(&mut session).govt = Some(govt);
        session.tick_traffic(catalog, &NovaAi::default(), &mut crate::NeverFires);
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Player));
        session
    }

    /// Presses the `pick`th option, the haggle roll (if any) `roll`.
    fn press(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        pick: usize,
        roll: u32,
    ) -> HailView {
        let mut chance = Draws::of(&[roll]);
        session
            .answer(pick, catalog, &HailOptions::default(), &mut chance)
            .expect("open")
    }

    fn haggle(session: &mut Session, catalog: &FakePilotCatalog, choice: Haggle) -> HailView {
        session
            .haggle(choice, catalog, &HailOptions::default())
            .expect("open")
    }

    /// Ticks Nova's traffic for `ticks`, checking `check` on NPC 0 after
    /// each.
    fn decide(session: &mut Session, catalog: &FakePilotCatalog, ticks: u32, check: impl Fn(&Npc)) {
        for _ in 0..ticks {
            session.tick_traffic(catalog, &NovaAi::default(), &mut crate::NeverFires);
            check(&session.npcs()[0]);
        }
    }

    #[test]
    fn a_bribe_paid_spares_the_player_for_the_rest_of_the_ships_stay() {
        let catalog = hailable();
        let mut session = pirate(&catalog, PIRATES);
        assert_eq!(hail(&mut session, &catalog).reply, "What is it you want?");
        let mut chance = Draws::of(&[99]);
        let view = session
            .answer(1, &catalog, &HailOptions::default(), &mut chance)
            .expect("open");
        assert_eq!(
            chance.asked,
            [100],
            "the haggle roll, once, as the price is asked"
        );
        assert_eq!(view.reply, "You'll have to pay me first.");
        assert_eq!(view.asking, Some(3000));
        let view = haggle(&mut session, &catalog, Haggle::Accept);
        assert_eq!(view.reply, "A pleasure doing business with you.");
        assert_eq!(view.asking, None);
        assert_eq!(session.pilot().cash, 97_000);
        assert!(!session.take_save_due(), "paying makes no save due");
        let spared = &session.npcs()[0];
        assert!(spared.spared);
        assert_eq!(spared.ai_type, AiType::WimpyTrader);
        assert_eq!(spared.goal, Goal::JumpOut);
        assert_eq!(spared.target, None);
        assert_eq!(spared.trigger, Trigger::default());
        decide(&mut session, &catalog, 3, |npc| {
            assert!(!npc.goal.fights(), "{:?}", npc.goal);
            assert_ne!(npc.target, Some(ShipRef::Player));
        });
        session.strikes.push(Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by: ShipRef::Player,
            damage: 5.0,
            downed: None,
        });
        decide(&mut session, &catalog, 1, |npc| {
            assert_eq!(npc.goal, Goal::Flee(ShipRef::Player), "hit, it flees");
        });
    }

    #[test]
    fn a_spared_ship_without_a_jumps_fuel_decides_as_its_idle_block_does() {
        let catalog = hailable();
        let mut session = pirate(&catalog, PIRATES);
        npc(&mut session).reserves.fuel.now = 50.0;
        hail(&mut session, &catalog);
        press(&mut session, &catalog, 1, 99);
        haggle(&mut session, &catalog, Haggle::Accept);
        assert_eq!(session.npcs()[0].goal, Goal::Idle);
    }

    #[test]
    fn a_price_the_player_cannot_pay_is_refused_and_the_ship_keeps_attacking() {
        let catalog = hailable();
        let mut session = pirate(&catalog, PIRATES);
        session.pilot.cash = 500;
        hail(&mut session, &catalog);
        assert_eq!(press(&mut session, &catalog, 1, 99).asking, Some(1000));
        let view = haggle(&mut session, &catalog, Haggle::Accept);
        assert_eq!(
            view.reply,
            "Yeah, come back when you actually have some money."
        );
        assert_eq!(session.pilot().cash, 500);
        decide(&mut session, &catalog, 3, |npc| {
            assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
            assert!(!npc.spared);
        });
    }

    #[test]
    fn a_ship_that_takes_no_bribes_refuses_and_keeps_attacking() {
        let catalog = hailable();
        let mut session = pirate(&catalog, HARDLINERS);
        hail(&mut session, &catalog);
        let mut chance = Draws::of(&[]);
        let view = session
            .answer(1, &catalog, &HailOptions::default(), &mut chance)
            .expect("open");
        assert_eq!(view.reply, "In your dreams, pal.");
        assert_eq!(view.asking, None);
        assert!(chance.asked.is_empty(), "no price, no roll");
        assert_eq!(
            session.haggle(Haggle::Accept, &catalog, &HailOptions::default()),
            None
        );
        decide(&mut session, &catalog, 3, |npc| {
            assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
        });
    }

    #[test]
    fn a_declined_bribe_brings_an_attack_and_a_higher_price() {
        let catalog = hailable();
        let mut session = pirate(&catalog, PIRATES);
        hail(&mut session, &catalog);
        press(&mut session, &catalog, 1, 36);
        npc(&mut session).goal = Goal::Idle;
        npc(&mut session).target = None;
        let view = haggle(&mut session, &catalog, Haggle::LowerPrice);
        assert_eq!(view.reply, "What? How dare you! Prepare to die!");
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Player));
        assert_eq!(session.npcs()[0].target, Some(ShipRef::Player));
        assert_eq!(session.pilot().cash, 100_000);
        assert_eq!(press(&mut session, &catalog, 1, 99).asking, Some(4000));
    }

    #[test]
    fn a_won_haggle_lowers_the_price_once() {
        let catalog = hailable();
        let mut session = pirate(&catalog, PIRATES);
        hail(&mut session, &catalog);
        press(&mut session, &catalog, 1, 35);
        let view = haggle(&mut session, &catalog, Haggle::LowerPrice);
        assert_eq!(view.asking, Some(2200));
        assert_eq!(view.reply, "You'll have to pay me first.", "unchanged");
        let mut chance = Draws::of(&[]);
        assert_eq!(
            session
                .answer(0, &catalog, &HailOptions::default(), &mut chance)
                .map(|view| view.reply),
            Some("You'll have to pay me first.".to_owned()),
            "no press while haggling"
        );
        haggle(&mut session, &catalog, Haggle::Accept);
        assert_eq!(session.pilot().cash, 97_800);
    }

    #[test]
    fn requested_help_paid_for_brings_the_ship_over() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.pilot.reserves.fuel.now = 40.0;
        hail(&mut session, &catalog);
        let view = press(&mut session, &catalog, 1, 99);
        assert_eq!(view.reply, "I'll help you out if you pay me.");
        assert_eq!(view.asking, Some(3000));
        let view = haggle(&mut session, &catalog, Haggle::Accept);
        assert_eq!(view.reply, "Okay, I'm on my way.");
        assert_eq!(session.pilot().cash, 97_000);
        let helper = &session.npcs()[0];
        assert_eq!(helper.goal, Goal::Assist(Help::Refuel));
        assert_eq!(helper.target, Some(ShipRef::Player));
        let view = press(&mut session, &catalog, 1, 99);
        assert_eq!(view.reply, "Okay, I'm on my way.", "already on its way");
    }

    #[test]
    fn requested_help_declined_is_laughed_off() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.pilot.reserves.fuel.now = 40.0;
        hail(&mut session, &catalog);
        press(&mut session, &catalog, 1, 99);
        let view = haggle(&mut session, &catalog, Haggle::LowerPrice);
        assert_eq!(view.reply, "Ha ha ha. What a comedian.");
        assert_eq!(session.npcs()[0].goal, Goal::Idle);
    }

    #[test]
    fn a_player_with_a_jumps_fuel_is_not_in_any_trouble() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.pilot.reserves.fuel.now = 100.0;
        hail(&mut session, &catalog);
        assert_eq!(
            press(&mut session, &catalog, 1, 99).reply,
            "You're not in any trouble."
        );
    }

    /// An option for every ship that makes it attack the player, saying
    /// `STR#` 9000 #1.
    #[derive(Debug)]
    struct Taunt;

    impl HailOption for Taunt {
        fn label(&self) -> String {
            "Taunt".to_owned()
        }

        fn key(&self) -> Option<char> {
            Some('T')
        }

        fn applies(&self, _hail: &Hail) -> bool {
            true
        }

        fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
            Answer {
                deed: Some(Deed::Attack),
                ..Answer::say(Reply::Line {
                    list: 9000,
                    index: 1,
                })
            }
        }
    }

    #[test]
    fn an_option_registered_at_the_edge_is_listed_and_done() {
        let mut catalog = hailable();
        catalog
            .strings
            .push((9000, vec!["Your mother was a tug.".to_owned()]));
        let options = HailOptions::default().with(Rc::new(Taunt));
        let mut session = targeting(&catalog);
        let mut chance = Draws::of(&opening(0));
        let view = session
            .hail(&catalog, &options, &mut chance)
            .expect("answered");
        assert_eq!(
            labels(&view),
            [
                ("Greetings", Some('G')),
                ("Request Assistance", Some('R')),
                ("Taunt", Some('T'))
            ]
        );
        let view = session
            .answer(2, &catalog, &options, &mut chance)
            .expect("open");
        assert_eq!(view.reply, "Your mother was a tug.");
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Player));
    }

    #[test]
    fn hanging_up_ends_the_hail_and_a_press_after_does_nothing() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        hail(&mut session, &catalog);
        session.hang_up();
        assert!(session.hailing(&catalog, &HailOptions::default()).is_none());
        let mut chance = Draws::of(&[]);
        assert!(
            session
                .answer(0, &catalog, &HailOptions::default(), &mut chance)
                .is_none()
        );
        assert!(
            session
                .haggle(Haggle::Accept, &catalog, &HailOptions::default())
                .is_none()
        );
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn landing_leaving_or_losing_the_ship_ends_the_hail() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        hail(&mut session, &catalog);
        session.traffic.remove(NpcId(0));
        assert!(session.hailing(&catalog, &HailOptions::default()).is_none());
        let mut session = targeting(&catalog);
        hail(&mut session, &catalog);
        session.populate(&catalog, &mut placed(0, -300));
        assert!(session.talk.is_none(), "populated afresh");
        let mut session = targeting(&catalog);
        hail(&mut session, &catalog);
        session.player.position = Vec2::new(30.0, -40.0);
        session.land().expect("lands");
        assert!(session.talk.is_none(), "landed");
    }

    #[test]
    fn a_press_beyond_the_listed_options_does_nothing() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        hail(&mut session, &catalog);
        let mut chance = Draws::of(&[]);
        assert!(
            session
                .answer(2, &catalog, &HailOptions::default(), &mut chance)
                .is_none()
        );
        assert_eq!(
            session
                .hailing(&catalog, &HailOptions::default())
                .map(|view| view.reply),
            Some("Channel open.".to_owned())
        );
    }

    // Assistance.

    /// `catalog`'s session with its ship (NPC 0, `Maneuver` 20, so a
    /// refuel's reach of 120 and a repair's of 290) assisting the player
    /// with `help` from (`x`, 0), at rest; the player at the centre.
    fn helped(catalog: &FakePilotCatalog, help: Help, x: f32) -> Session {
        let mut session = targeting(catalog);
        let helper = npc(&mut session);
        helper.goal = Goal::Assist(help);
        helper.target = Some(ShipRef::Player);
        helper.state.position = Vec2::new(x, 0.0);
        session
    }

    #[test]
    fn a_refuelling_ship_docked_at_its_reach_gives_a_unit_a_tick() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 120.0);
        session.pilot.reserves.fuel.now = 40.0;
        session.tick_assistance(&NovaDisable);
        assert_eq!(session.reserves().fuel.now, 41.0);
        let mut far = helped(&catalog, Help::Refuel, 121.0);
        far.pilot.reserves.fuel.now = 40.0;
        far.tick_assistance(&NovaDisable);
        assert_eq!(far.reserves().fuel.now, 40.0, "beyond its reach");
        let mut moving = helped(&catalog, Help::Refuel, 120.0);
        moving.pilot.reserves.fuel.now = 40.0;
        npc(&mut moving).state.velocity = Vec2::new(0.0, 0.35);
        moving.tick_assistance(&NovaDisable);
        assert_eq!(moving.reserves().fuel.now, 40.0, "not slow enough");
    }

    #[test]
    fn a_refuel_stops_above_a_jumps_fuel_and_says_so() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 0.0);
        session.pilot.reserves.fuel.now = 40.0;
        for tick in 1..=61 {
            assert!(session.take_comm().is_empty(), "tick {tick}");
            session.tick_assistance(&NovaDisable);
        }
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert_eq!(
            session.take_comm(),
            [CommNote {
                from: NpcId(0),
                comm_name: "Cruiser".to_owned(),
                done: Help::Refuel,
            }]
        );
        let helper = &session.npcs()[0];
        assert_eq!(helper.goal, Goal::Idle, "it decides again");
        assert_eq!(helper.target, None);
        session.tick_assistance(&NovaDisable);
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert!(session.take_comm().is_empty());
    }

    #[test]
    fn a_player_already_above_a_jumps_fuel_gains_nothing_and_the_refuel_is_done() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 0.0);
        session.pilot.reserves.fuel.now = 150.0;
        session.tick_assistance(&NovaDisable);
        assert_eq!(session.reserves().fuel.now, 150.0);
        assert_eq!(session.take_comm().len(), 1);
    }

    #[test]
    fn a_refuel_stops_at_the_tanks_most() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 0.0);
        session.pilot.reserves.fuel = Gauge {
            now: 79.5,
            max: 80.0,
        };
        session.tick_assistance(&NovaDisable);
        assert_eq!(session.reserves().fuel.now, 80.0);
        assert_eq!(session.take_comm().len(), 1);
    }

    /// Records the armour it is asked about, disabled below 20.
    #[derive(Debug, Default)]
    struct Below20 {
        asked: RefCell<Vec<f32>>,
    }

    impl DisableRule for Below20 {
        fn disabled(&self, armor: Gauge, _hull: &HullSpec) -> bool {
            self.asked.borrow_mut().push(armor.now);
            armor.now < 20.0
        }
    }

    /// [`helped`] repairing from within its reach, the player disabled
    /// with 10 armour of 45.
    fn repairing(catalog: &FakePilotCatalog) -> Session {
        let mut session = helped(catalog, Help::Repair, 290.0);
        session.condition = Condition::Disabled;
        session.pilot.reserves.armor.now = 10.0;
        session
    }

    #[test]
    fn a_repair_raises_the_armour_after_a_hundred_ticks_within_reach() {
        let catalog = hailable();
        let mut session = repairing(&catalog);
        let rule = Below20::default();
        for _ in 0..99 {
            session.tick_assistance(&rule);
        }
        assert_eq!(session.reserves().armor.now, 10.0);
        assert!(rule.asked.borrow().is_empty());
        assert!(session.take_comm().is_empty());
        session.tick_assistance(&rule);
        assert_eq!(session.reserves().armor.now, 20.0);
        assert_eq!(
            *rule.asked.borrow(),
            (10..=20).map(|a| a as f32).collect::<Vec<_>>()
        );
        assert_eq!(
            session.take_comm(),
            [CommNote {
                from: NpcId(0),
                comm_name: "Cruiser".to_owned(),
                done: Help::Repair,
            }]
        );
        assert_eq!(session.npcs()[0].goal, Goal::Idle);
        assert_eq!(session.npcs()[0].assisting, 0);
    }

    /// Has every ship disabled, whatever its armour.
    #[derive(Debug)]
    struct Always;

    impl DisableRule for Always {
        fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
            true
        }
    }

    #[test]
    fn a_repair_raises_the_armour_no_further_than_its_most() {
        let catalog = hailable();
        let mut session = repairing(&catalog);
        for _ in 0..100 {
            session.tick_assistance(&Always);
        }
        assert_eq!(session.reserves().armor.now, 45.0, "full, and done");
        assert_eq!(session.take_comm().len(), 1);
    }

    #[test]
    fn a_repair_by_novas_rule_raises_the_armour_to_the_least_that_flies() {
        let catalog = hailable();
        let mut session = repairing(&catalog);
        for _ in 0..100 {
            session.tick_assistance(&NovaDisable);
        }
        let armor = session.reserves().armor;
        assert_eq!(armor.now, 15.0, "a third of 45, a point at a time");
        assert!(!NovaDisable.disabled(armor, &session.hull()));
        session.tick_combat(crate::combat::Rules::default(), &mut crate::NeverFires);
        assert_eq!(session.player_condition(), Condition::Intact);
    }

    #[test]
    fn a_repair_counts_only_the_ticks_within_reach() {
        let catalog = hailable();
        let mut session = repairing(&catalog);
        npc(&mut session).state.position = Vec2::new(291.0, 0.0);
        for _ in 0..200 {
            session.tick_assistance(&NovaDisable);
        }
        assert_eq!(session.reserves().armor.now, 10.0);
        assert_eq!(session.npcs()[0].assisting, 0);
    }

    #[test]
    fn a_repair_for_a_player_no_longer_disabled_ends_at_once_unsaid() {
        let catalog = hailable();
        let mut session = repairing(&catalog);
        session.condition = Condition::Intact;
        session.tick_assistance(&NovaDisable);
        assert_eq!(session.npcs()[0].goal, Goal::Idle);
        assert!(session.take_comm().is_empty());
        assert_eq!(session.reserves().armor.now, 10.0);
    }

    #[test]
    fn nothing_is_given_while_landed_jumping_or_by_a_ship_not_intact() {
        let catalog = hailable();
        let mut landed = helped(&catalog, Help::Refuel, 0.0);
        landed.landed = Some(StellarId(128));
        let mut jumping = helped(&catalog, Help::Refuel, 0.0);
        jumping.jumping = Some(SystemId(131));
        let mut disabled = helped(&catalog, Help::Refuel, 0.0);
        npc(&mut disabled).condition = Condition::Disabled;
        for mut session in [landed, jumping, disabled] {
            session.pilot.reserves.fuel.now = 40.0;
            session.tick_assistance(&NovaDisable);
            assert_eq!(session.reserves().fuel.now, 40.0);
        }
    }

    #[test]
    fn a_strike_from_the_player_ends_the_help() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 300.0);
        session.pilot.reserves.fuel.now = 40.0;
        decide(&mut session, &catalog, 5, |npc| {
            assert_eq!(npc.goal, Goal::Assist(Help::Refuel));
        });
        session.strikes.push(Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by: ShipRef::Player,
            damage: 5.0,
            downed: None,
        });
        decide(&mut session, &catalog, 1, |npc| {
            assert!(!matches!(npc.goal, Goal::Assist(_)), "{:?}", npc.goal);
        });
    }

    // Hailing an escort.

    /// [`targeting`]'s ship, the Federation cruiser, joined to the fleet
    /// where it is (as a capture does), and targeted.
    fn escorting(catalog: &FakePilotCatalog) -> Session {
        let mut session = targeting(catalog);
        session.join_fleet(NpcId(0));
        session.target = Some(NpcId(0));
        session.take_save_due();
        session
    }

    #[test]
    fn an_escort_answers_what_can_i_do_for_you_with_release_alone() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.reply, "What can I do for you?");
        assert_eq!(labels(&view), [("Release", Some('R'))]);
        assert!(!view.hostile);
        assert_eq!(view.govt_name, None, "of no government");
        let (view, _) = hail_with(&mut session, &catalog, &opening(1));
        assert_eq!(view.expect("answered").reply, "What can I help you with?");
    }

    #[test]
    fn a_carried_fighter_answers_as_an_escort_with_no_release() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        session.pilot.escorts[0].carried = true;
        if let Some(npc) = session.npc_mut(NpcId(0)) {
            npc.carrier = Some(crate::bay::Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            });
        }
        let view = hail(&mut session, &catalog);
        assert_eq!(view.reply, "What can I do for you?");
        assert_eq!(labels(&view), []);
    }

    #[test]
    fn an_escort_of_a_mute_ship_type_is_hailed_all_the_same() {
        let mut catalog = hailable();
        catalog.ship_records[0].inherent_govt = Some(MUTED);
        let mut session = escorting(&catalog);
        assert!(hail_with(&mut session, &catalog, &opening(0)).0.is_ok());
    }

    #[test]
    fn release_says_goodbye_and_the_escort_leaves_when_the_channel_closes() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        hail(&mut session, &catalog);
        let view = session
            .answer(0, &catalog, &HailOptions::default(), &mut Draws::of(&[]))
            .expect("answered");
        assert_eq!(view.reply, "Goodbye, captain.");
        assert!(session.is_escort(NpcId(0)), "not yet");
        assert_eq!(session.pilot().escorts().len(), 1);
        assert!(!session.take_save_due());
        session.hang_up();
        assert_eq!(session.pilot().escorts(), []);
        assert!(!session.is_escort(NpcId(0)));
        let released = npc(&mut session).clone();
        assert_eq!(released.escort, None);
        assert_eq!(released.ai_type, AiType::WimpyTrader, "its InherentAI");
        assert_eq!(released.goal, Goal::JumpOut, "with a jump's fuel");
        assert_eq!(released.target, None);
        assert_eq!(released.govt, None);
        assert!(session.take_save_due());
    }

    #[test]
    fn released_without_a_jumps_fuel_it_decides_again() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        npc(&mut session).reserves.fuel.now = 99.0;
        hail(&mut session, &catalog);
        session.answer(0, &catalog, &HailOptions::default(), &mut Draws::of(&[]));
        session.hang_up();
        assert_eq!(npc(&mut session).goal, Goal::Idle);
    }

    #[test]
    fn hanging_up_on_an_escort_without_a_press_changes_nothing() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        hail(&mut session, &catalog);
        let before = (session.pilot().clone(), npc(&mut session).clone());
        session.hang_up();
        assert_eq!((session.pilot().clone(), npc(&mut session).clone()), before);
        assert!(session.is_escort(NpcId(0)));
        assert!(!session.take_save_due());
        session.hang_up();
        assert!(session.is_escort(NpcId(0)), "nothing to hang up");
    }

    #[test]
    fn a_traffic_ship_is_never_offered_release() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let view = hail(&mut session, &catalog);
        assert!(labels(&view).iter().all(|(label, _)| *label != "Release"));
        assert_eq!(view.options.len(), 2, "{view:?}");
    }
}
