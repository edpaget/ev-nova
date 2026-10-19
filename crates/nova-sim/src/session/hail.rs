//! Hailing ships in flight: the session's side of [`hail`](crate::hail).
//!
//! The comm key hails through [`Session::hail_key`] (or
//! [`Session::open_hail`], Option not held). With no ship target, or
//! with Option held, it hails the navigation stellar: one that answers
//! ([`landing::answers_hail`]) gives
//! [`Hailed::Stellar`](super::Hailed::Stellar), where the original opens
//! its comm dialog (`_DoPlanetCommDialog` @0x6204b, not built), and any
//! other says "No response."; with none there is nothing to hail. Else
//! it hails the ship target: a person whose `LinkMission` is available
//! offers it in place of the comm dialog, and no conversation opens (see
//! the `person_offers` module). Any other target, or a person who may
//! join under [`RuleKey::PersonJoin`]'s other reading, opens the comm
//! dialog.
//!
//! The player hails its target ([`Session::hail`]); the conversation
//! stays open until the player hangs up ([`Session::hang_up`]), lands, or
//! leaves the system, and while it is open the dialog reads it
//! ([`Session::hailing`]). A press of an option ([`Session::answer`]) says
//! its reply, does its deed, and asks its price, drawing the haggle roll
//! then; the player haggles ([`Session::haggle`]) and pays from its cash,
//! which makes no save due. Every reply is said here, in the
//! conversation's variant, read from the caller's [`CommCatalog`], and
//! filled with no mission as the dialog shows it (`_CommDialogUpdate`
//! @0x907f7-0x9080b); a specific advice line is parsed as the press picks
//! it (`_LoadAdvice` @0x919ca).
//!
//! The deeds:
//!
//! - **Spare** (a bribe paid): the ship is [`spared`](Npc::spared) and
//!   becomes a wimpy trader for the rest of its stay (@0x961f8); it leaves
//!   (`_AIMakeShipLeave` @0x7e2b0), jumping out with a jump's fuel, or
//!   else deciding as its idle block does, which lands; its target and
//!   fire command are gone, and so is its provocation.
//! - **Attack** (a bribe declined, `_AIMakeShipAttackPlayer` @0x89c3e):
//!   it attacks the player, a person of `Flags` 0x0010 not already
//!   threatening it saying its hail quote at once
//!   ([`Traffic::turn_with_quote`](crate::traffic::Traffic::turn_with_quote)).
//! - **Release**: the player's escort leaves the fleet once the channel
//!   closes ([`Session::hang_up`]), as the original's dialog does when it
//!   closes (@0x96852), having taken its share of the fleet's cargo and
//!   reset as it went (see the `escorts` module): its AI type its
//!   `InherentAI`, of no government
//!   still, it leaves the system (`_AIMakeShipLeave`), jumping out with a
//!   jump's fuel, or else deciding as its idle block does. A save is due.
//!   The player's escort is hailed whatever its ship type's government
//!   says, and opens "What can I do for you?".
//! - **Join** (Use As Escort, under
//!   [`RuleKey::PersonJoin`](crate::RuleKey::PersonJoin)'s other reading):
//!   the person hailed joins the fleet at once as itself
//!   ([`Escort::person`](crate::Escort)), with no wage and its reserves as
//!   they are, of no government, its AI type its `InherentAI`, keeping
//!   formation; every other NPC lets it go, and so does the player's
//!   target. A save is due, and the hail stays open as the escort's. A
//!   hail lists it ([`Hail::joins`](crate::Hail)) for a person whose
//!   record allows it ([`offers_to_join`]) and whose `LinkMission` is
//!   available from a ship (`_HandlePlayerCommunication` @0x6220d) while
//!   the fleet has room, fewer than [`MAX_ESCORTS`] escorts not counting
//!   the carried fighters.
//! - **Help** (`_AIMakeShipRefuelPlayer` @0x82dbb,
//!   `_AIMakeShipRepairPlayer` @0x82df2): it assists the player
//!   ([`Goal::Assist`]), its provocation gone. Each step a refuel
//!   ([`Session::tick_assistance`]) gives the help as
//!   [`assist`](crate::hail::assist) says; once done it says so
//!   ([`Session::take_comm`]) with `snd ` 151 and decides again. A repair
//!   is the traffic's: the ship docks with the player as a pirate does and
//!   repairs it once its dock's wait is done (see
//!   [`repair_dock`](crate::traffic::repair_dock) and the session's
//!   `repair_dock` module). The traffic lets an assisting ship decide
//!   again once anything provokes it.
//! - **Defend** (battle help, `_AIFightEnemyThreatToPlayer` @0x82c80):
//!   it attacks the threat to the player picked
//!   ([`Goal::Attack`], its target set, @0x82d9b-0x82d9f), and calls for
//!   reinforcements, forced past their strength test
//!   (`_AICallForReinforcements(ship, 1)` @0x9636d; see the
//!   `reinforcements` module).

use super::Session;
use super::missions::TextSource;
use crate::ai::Goal;
use crate::board::MAX_ESCORTS;
use crate::catalog::{CommCatalog, MissionId, ShipId, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::combat::hull::{Condition, ESCAPE_POD_SHIP};
use crate::hail::{
    Ask, Attitude, CommNote, Conversation, Deed, Dispositions, EscortStatus, Haggle, Hail,
    HailButton, HailOptions, HailRefusal, HailView, Help, MissionHail, Reply, Settled, assist,
    attitude, reply,
};
use crate::landing;
use crate::mission::goals::ShipGoal;
use crate::mission::ships::ShipBehav;
use crate::person::{COMM_QUOTES, offers_to_join};
use crate::rank;
use crate::rulebook::{RuleKey, RuleSource};
use crate::sound::SimSound;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};

/// What the comm key hails ([`Session::hail_target`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HailTarget {
    /// The ship target.
    Ship(NpcId),
    /// The navigation stellar.
    Stellar(StellarId),
}

/// A hail under way: the NPC hailed, the conversation, what it last
/// said, and the price it asks while the player haggles.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Talk {
    npc: NpcId,
    conversation: Conversation,
    /// What it said just before `reply` in the last press, if anything.
    first: Option<Reply>,
    reply: Reply,
    /// The words of `reply` when it is a specific advice line, parsed as
    /// it was said (`_LoadAdvice` @0x919ca; [`Reply::say_parsed`]).
    said: Option<String>,
    ask: Option<Ask>,
    /// Whether the escort hailed is to be released once the channel
    /// closes.
    release: bool,
}

#[cfg(test)]
impl Talk {
    /// A hail of `npc` just opened, its conversation rolled on
    /// [`NeverFires`](crate::chance::NeverFires), for the session's tests
    /// that only need one under way.
    pub(super) fn opened(npc: NpcId) -> Self {
        Self {
            npc,
            conversation: Conversation::open(0, false, 0, &mut crate::chance::NeverFires),
            first: None,
            reply: Reply::Comm(1),
            said: None,
            ask: None,
            release: false,
        }
    }
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
    /// [`HailRefusal::NoTarget`] with no ship target (the comm key hails
    /// the navigation stellar then, [`Session::hail_key`]), or while
    /// landed, breaking up, or while a jump is under way, the pre-jump
    /// stage included (@0x61f87); [`HailRefusal::InHyperspace`] for a
    /// target still jumping in, unless it is disabled; and [`HailRefusal::NoResponse`]
    /// for a disabled or mute one, a mute one still jumping in being in
    /// hyperspace (@0x62089-0x6209c, @0x62610-0x62676). Each beeps
    /// [`SimSound::Refused`].
    pub fn hail(
        &mut self,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
        chance: &mut dyn Chance,
    ) -> Result<HailView, HailRefusal> {
        self.talk = None;
        self.beep_unanswered(self.answering().map(|npc| npc.id))?;
        let npc = self.answering()?;
        let dispositions = self.dispositions(npc);
        let escort = npc.escort.is_some();
        let conversation =
            Conversation::open(self.pilot.cash, dispositions.greedy, npc.info_types, chance);
        let opening = if escort {
            Reply::Comm(reply::ESCORT_OPENING)
        } else {
            reply::opening(
                !dispositions.untalkative,
                attitude(
                    npc,
                    &self.world().around(self.npcs()),
                    self.mission_hail(npc),
                ),
                self.mission_hail(npc),
            )
        };
        let opening = match Self::quote_of(npc) {
            Some(index) if self.rules.source_for(RuleKey::CommQuote) == RuleSource::Bible => {
                Reply::Line {
                    list: COMM_QUOTES,
                    index,
                }
            }
            _ => opening,
        };
        self.talk = Some(Talk {
            npc: npc.id,
            conversation,
            first: None,
            reply: opening,
            said: None,
            ask: None,
            release: false,
        });
        self.hailing(catalog, options).ok_or(HailRefusal::NoTarget)
    }

    /// `answered`, beeping [`SimSound::Refused`] once with its
    /// refusal, if any, as every refusal of `_HandlePlayerCommunication`
    /// plays `_beepSnd[3]` (@0x626cf, @0x6206c, @0x6261c, @0x6264e).
    pub(super) fn beep_unanswered<T>(
        &mut self,
        answered: Result<T, HailRefusal>,
    ) -> Result<T, HailRefusal> {
        if answered.is_err() {
            self.sounds.push(SimSound::Refused);
        }
        answered
    }

    /// What the comm key hails, Option held as `option` says, drawing
    /// nothing, in `_HandlePlayerCommunication`'s order (@0x61f48-0x6209c):
    /// [`HailRefusal::NoTarget`] while landed, jumping (the pre-jump stage
    /// included) or breaking up; then, with no ship target or Option held (`_QuickKeyCheck(0x3a)`
    /// @0x61f91-0x61fa6), the navigation stellar
    /// ([`Session::nav_target`]) when it is shown in the system (its
    /// active byte, +0x44 @0x62009) and [answers](landing::answers_hail),
    /// else [`HailRefusal::NoResponse`] (@0x6261c), and
    /// [`HailRefusal::NoTarget`] with none (@0x6206c); else the ship
    /// target as [`Session::answering`] says.
    pub(super) fn hail_target(&self, option: bool) -> Result<HailTarget, HailRefusal> {
        self.player_hails()?;
        if self.target().is_some() && !option {
            return self.answering().map(|npc| HailTarget::Ship(npc.id));
        }
        let stellar = self.nav_target().ok_or(HailRefusal::NoTarget)?;
        self.sites
            .iter()
            .find(|site| site.id == stellar)
            .filter(|site| landing::answers_hail(site))
            .map(|_| HailTarget::Stellar(stellar))
            .ok_or(HailRefusal::NoResponse)
    }

    /// Whether the player may hail at all: [`HailRefusal::NoTarget`]
    /// while landed, jumping (+0x50 @0x61f87, which spans the pre-jump's
    /// brake) or breaking up (`_IsDying` @0x61f75). The original refuses a cloaked player first
    /// (`_ShipIsCloaked` @0x61f65); the player has no cloak yet.
    fn player_hails(&self) -> Result<(), HailRefusal> {
        let flying = !matches!(
            self.condition,
            Condition::Dying { .. } | Condition::Destroyed
        );
        if self.landed.is_some() || self.jump.is_some() || !flying {
            return Err(HailRefusal::NoTarget);
        }
        Ok(())
    }

    /// The player's ship target, when it answers a hail, drawing nothing:
    /// otherwise [`HailRefusal::NoTarget`] with no ship target, or while
    /// landed, jumping (the pre-jump stage included) or breaking up; [`HailRefusal::InHyperspace`] for a
    /// target still jumping in, unless it is disabled; and
    /// [`HailRefusal::NoResponse`] for a disabled one, jumping in or not,
    /// or a mute one that has arrived, an escort answering whatever its
    /// government says but not in an escape pod ([`ESCAPE_POD_SHIP`],
    /// class 0x2ff, mute whoever leads it, @0x62145).
    pub(super) fn answering(&self) -> Result<&Npc, HailRefusal> {
        self.player_hails()?;
        let npc = self.target().ok_or(HailRefusal::NoTarget)?;
        // `_HandlePlayerCommunication` refuses a target whose hyperspace
        // state (+0x50) is above 0 (@0x62089-0x6209c) before it reads the
        // government: "No response." if it is disabled (`_IsDisabled`
        // @0x62610-0x62644), else "entering hyperspace" (@0x6264e-0x62676).
        // `_AIMakeShipStopJumping` zeroes it before a leaderless ship's
        // wait (+0x4c), which the hail does not read, so a ship holding
        // its course after its glide ([`Mode::Coasting`]) answers.
        if !npc.arrived() {
            return Err(if npc.condition == Condition::Disabled {
                HailRefusal::NoResponse
            } else {
                HailRefusal::InHyperspace
            });
        }
        // The escape pod's class is mute whoever leads it: the class test
        // (@0x62145) follows the escort shortcut (@0x620a2-0x620bb).
        let mute = self.dispositions(npc).mute;
        if npc.ship == ESCAPE_POD_SHIP
            || npc.condition != Condition::Intact
            || (mute && npc.escort.is_none())
        {
            return Err(HailRefusal::NoResponse);
        }
        Ok(npc)
    }

    /// What `npc` is as a ship the player leads, as `_CommDialogUpdate`'s
    /// status block reads it (@0x90688-0x906ff): none for a ship it does
    /// not lead (+0x9a not 0) or once Release is pressed in this hail
    /// (`releasing`, the flag at 0xdef2c); else "Escort", or "Hired
    /// Escort" with its wage for a hired fleet escort (+0xbb). Whether it
    /// is led is [`Npc::escorts_player`], the relation
    /// [`Session::player_led`] filters on, asked of the ship in hand; a
    /// mission escort is not of the fleet, so it has no wage.
    fn escort_status(&self, npc: &Npc, releasing: bool) -> Option<EscortStatus> {
        if releasing || !npc.escorts_player() {
            return None;
        }
        let wage = self
            .fleet
            .iter()
            .position(|&placed| placed == Some(npc.id))
            .and_then(|index| self.pilot.escorts.get(index))
            .filter(|escort| escort.hired())
            .map(|escort| self.paid_wage(escort));
        Some(EscortStatus { wage })
    }

    /// The mission of `npc`, when it is a mission ship: its slot's
    /// mission's `ShipGoal` and its mark's `ShipBehav`. A mark whose slot
    /// no longer holds a mission (until the AI next thinks) reads as a
    /// mission of no goal and of its own behaviour, as the original reads
    /// a mark of an inactive slot: "Channel open." whatever its attitude
    /// (@0x95be4), and no refusal to be bribed (@0x9612d).
    pub(super) fn mission_hail(&self, npc: &Npc) -> Option<MissionHail> {
        let mark = npc.mission?;
        Some(match self.pilot.missions.get(mark.slot) {
            Some(active) => MissionHail {
                goal: ShipGoal::from_code(self.lifecycle_record(active.mission).ships.goal),
                behav: mark.behav,
            },
            None => MissionHail {
                goal: ShipGoal::None,
                behav: ShipBehav::Own,
            },
        })
    }

    /// What `npc`'s government and ship type make of hails, the pilot's
    /// ranks reaching it by the session's `rank_govt_reach` rule.
    fn dispositions(&self, npc: &Npc) -> Dispositions {
        let inherent = self
            .ship_record(npc.ship)
            .and_then(|record| record.inherent_govt);
        let favour = rank::favour(
            &self.ranks,
            &self.pilot.ranks,
            npc.govt,
            &self.govts,
            self.rules.source_for(RuleKey::RankGovtReach),
        );
        Dispositions::of(npc, inherent, &self.govts, favour)
    }

    /// The hail of `npc` in conversation `talk`, as the options see it.
    pub(super) fn hail_of<'a>(&'a self, npc: &'a Npc, talk: &Conversation) -> Hail<'a> {
        let need = assist::need(self.condition, self.pilot.reserves.fuel);
        let around = self.world();
        let around = around.around(self.npcs());
        let quote = Self::quote_of(npc)
            .filter(|_| self.rules.source_for(RuleKey::CommQuote) == RuleSource::Engine);
        Hail {
            quote,
            joins: self.joins(npc),
            ..Hail::new(
                npc,
                &around,
                self.dispositions(npc),
                talk,
                need,
                self.mission_hail(npc),
            )
        }
    }

    /// Whether `npc` is a person who may join the player's fleet: its
    /// record allows it ([`offers_to_join`]), its `LinkMission` is
    /// available from a ship (@0x6220d), and the fleet has room, fewer
    /// than [`MAX_ESCORTS`] not counting the carried fighters.
    fn joins(&self, npc: &Npc) -> bool {
        npc.person
            .and_then(|person| self.traffic.person(person.id))
            .is_some_and(|person| {
                offers_to_join(&person.record)
                    && person
                        .record
                        .link_mission
                        .is_some_and(|mission| self.ship_mission_available(MissionId(mission)))
            })
            && self.pilot.escort_count() < MAX_ESCORTS
    }

    /// The comm dialog's contents while a hail is under way and its ship
    /// is still in the system, `options` listed and the reply read from
    /// `catalog`. What the ship says is filled with no mission
    /// ([`Session::plain_text`]), as `_CommDialogUpdate` munges the comm
    /// text with index -1 on each update (@0x907f7-0x9080b): its mission
    /// tags read as [`RuleKey::MissionlessTags`] says, its crossed rank
    /// tags the governments remembered, and the `l33t` munge applies. The
    /// escort status is not filled (@0x90688-0x906ff). This changes
    /// nothing, as the original's update does not.
    #[must_use]
    pub fn hailing(
        &self,
        catalog: &(impl CommCatalog + ?Sized),
        options: &HailOptions,
    ) -> Option<HailView> {
        let talk = self.talk.as_ref()?;
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
        let say = |reply: Reply| {
            self.plain_text(&reply.say(
                talk.conversation.variant,
                catalog,
                self.pilot.ship_name().unwrap_or_default(),
            ))
        };
        let reply = match &talk.said {
            Some(said) => self.plain_text(said),
            None => say(talk.reply),
        };
        Some(HailView {
            npc: npc.id,
            ship: npc.ship,
            first: talk.first.map(say),
            reply,
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
            escort: self.escort_status(npc, talk.release),
            portrait: npc.person.and_then(|person| person.portrait),
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
        let mut talk = self.talk.clone()?;
        if talk.ask.is_some() {
            return self.hailing(catalog, options);
        }
        let answer = {
            let npc = self.npcs().iter().find(|npc| npc.id == talk.npc)?;
            let hail = self.hail_of(npc, &talk.conversation);
            options.listed(&hail).get(pick)?.press(&hail, chance)
        };
        talk.first = answer.first;
        if let Some(said) = answer.reply {
            talk.reply = said;
            talk.said = self.take_advice(said, talk.conversation.variant, catalog, chance);
        }
        if let Some(ask) = answer.ask {
            talk.conversation.ask(chance);
            talk.ask = Some(ask);
        }
        let npc = talk.npc;
        self.talk = Some(talk);
        if let Some(deed) = answer.deed {
            self.act(npc, deed);
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
        let mut talk = self.talk.clone()?;
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
        talk.first = None;
        talk.reply = said;
        talk.said = None;
        let npc = talk.npc;
        self.talk = Some(talk);
        if let Some(deed) = deed {
            self.act(npc, deed);
        }
        self.hailing(catalog, options)
    }

    /// The words of `reply` when it is a specific advice line, taken in as
    /// `_LoadAdvice` takes one in as the press picks it (@0x919a2-0x919ca):
    /// a line that is not blank is parsed ([`TextSource::Advice`]: its
    /// conditional text resolved, its rank governments scanned onto those
    /// remembered, with no reset), before the "Nice to meet you." check;
    /// none for any other reply, whose words [`Reply::say`] reads.
    fn take_advice(
        &mut self,
        reply: Reply,
        variant: u8,
        catalog: &(impl CommCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) -> Option<String> {
        if !matches!(
            reply,
            Reply::Advice {
                greet_if_blank: true,
                ..
            }
        ) {
            return None;
        }
        let ship_name = self.pilot.ship_name().unwrap_or_default().to_owned();
        Some(reply.say_parsed(variant, catalog, &ship_name, |line| {
            self.take_in(TextSource::Advice, line, chance)
        }))
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
        if deed == Deed::Join {
            self.enlist_person(id);
            return;
        }
        if deed == Deed::Attack {
            // `_AIMakeShipAttackPlayer` (@0x9621f) asks its quote's gate
            // before it sets the goal (@0x89c50-0x89c9b).
            let view = self.world().gate_view();
            self.traffic.turn_with_quote(id, view);
        }
        let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
            return;
        };
        match deed {
            Deed::Spare => {
                npc.spared = true;
                npc.ai_type = AiType::WimpyTrader;
                make_leave(npc);
            }
            Deed::Attack => {
                npc.goal = Goal::Attack(ShipRef::Player);
                npc.target = Some(ShipRef::Player);
            }
            Deed::Release | Deed::Join => {}
            Deed::Defend(threat) => {
                npc.goal = Goal::Attack(ShipRef::Npc(threat));
                npc.target = Some(ShipRef::Npc(threat));
                self.call_for_battle_help_reinforcements(id);
            }
            Deed::Help(help) => {
                npc.goal = Goal::Assist(help);
                npc.target = Some(ShipRef::Player);
                npc.trigger = Trigger::default();
                npc.provoked = 0.0;
                // +0x4c = -1.0 and sub 0 (@0x82de9, @0x82e20): a wait
                // running is cleared and any docking dropped. Its +0x50,
                // the hyperspace state, is 0 already for a ship hailed in
                // flight.
                npc.ai_wait = 0;
                npc.docking = None;
            }
        }
    }

    /// NPC `id` calls for reinforcements, forced past its strength test:
    /// `_AICallForReinforcements(ship, 1)`, which battle help makes
    /// (`_DoCommDialog` @0x9636d), after its goal is set (see the
    /// `reinforcements` module).
    fn call_for_battle_help_reinforcements(&mut self, id: NpcId) {
        // Between frames, after the frame's odds pass: the cached odds.
        self.call_for_reinforcements(id, true, None);
    }

    /// Gives a tick of the refuel each NPC refuelling the player gives (see
    /// [`assist`](crate::hail::assist)), saying "Energy transfer complete"
    /// with `snd ` 151 once it is done. While the ship is landed or
    /// jumping, nothing happens. A repair is the traffic's (see the
    /// `repair_dock` module).
    pub fn tick_assistance(&mut self) {
        if self.landed.is_some() || self.jumping().is_some() {
            return;
        }
        let player = self.player;
        let mut done = Vec::new();
        for npc in self.traffic.npcs_mut() {
            if npc.goal != Goal::Assist(Help::Refuel) {
                continue;
            }
            // Not while it jumps in or comes out of a gate, nor while it
            // holds its course after (`Mode::Coasting`): the help is given
            // in `_HighLevelAIHandler`, which returns at once while the
            // wait +0x4c runs, refuel (goal 9) included (@0x8d4b4-0x8d4d2).
            if npc.condition != Condition::Intact || npc.mode != Mode::Flying {
                continue;
            }
            let reach = assist::reach(Help::Refuel, npc.stats.handling.turn_rate);
            if assist::within(&npc.state, &player, reach)
                && assist::docked(&npc.state)
                && refuel(&mut self.pilot.reserves.fuel)
            {
                stop(npc);
                done.push((npc.id, npc.ship));
            }
        }
        for (from, ship) in done {
            // `_beepSnd[1]` (@0x8f585-0x8f5b0), with the line.
            self.say_help_done(from, ship, Help::Refuel, SimSound::EnergyTransferComplete);
        }
    }

    /// NPC `from`, of ship type `ship`, says it has `done` its help
    /// ("`CommName`:  `done`, `ship name`.", a [`CommNote`]), holding the
    /// message line for the note's ticks, with `sound`.
    pub(super) fn say_help_done(&mut self, from: NpcId, ship: ShipId, done: Help, sound: SimSound) {
        let note = CommNote {
            from,
            comm_name: self
                .ship_record(ship)
                .map(|record| record.comm_name.clone())
                .unwrap_or_default(),
            ship_name: self.pilot.ship_name().unwrap_or_default().to_owned(),
            done,
        };
        self.comm_line.show(note.ticks());
        self.comm.push(note);
        self.sounds.push(sound);
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

/// `npc` leaves the system (`_AIMakeShipLeave` @0x7e2b0-0x7e2e7): it
/// jumps out (state 2), whatever its fuel, as the original tests none;
/// its target, fire command and provocation are gone. A warship that
/// cannot jump lands or stops on its next think
/// (`nova_sim::ai::warship`); the other types keep leaving.
pub(super) fn make_leave(npc: &mut Npc) {
    npc.goal = Goal::JumpOut;
    npc.target = None;
    npc.trigger = Trigger::default();
    npc.provoked = 0.0;
}

/// `npc` stops assisting, and decides again, its travel stellar
/// cleared.
fn stop(npc: &mut Npc) {
    npc.goal = Goal::Idle;
    npc.target = None;
    // +0x6c -1 (@0x8f6ce): the travel stellar a pirate reads.
    npc.travel_set = false;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::ai::NovaAi;
    use crate::catalog::{DudeId, DudeRecord, GovtId, GovtRecord, ShipId, SystemTraffic};
    use crate::catalog::{RankId, RankRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::Strike;
    use crate::combat::hull::HullSpec;
    use crate::geometry::Vec2;
    use crate::govt::XENOPHOBIC;
    use crate::hail::{Answer, BRIBABLE_WARSHIPS, HailOption, MUTE, UNTALKATIVE};
    use crate::handling::ShipFields;
    use crate::landing::StellarFlags;
    use crate::reserves::Gauge;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::testkit::{Draws, FAST, FakePilotCatalog, govt, land_now, ship};

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
                    persons: Default::default(),
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
        // Setup starts it at its top speed: brought to rest.
        npc(&mut session).state.velocity = Vec2::ZERO;
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
        // Boxed: six sessions are too large for the stack together.
        let mut untargeted = Box::new(targeting(&catalog));
        untargeted.target = None;
        let mut landed = Box::new(targeting(&catalog));
        landed.landed = Some(StellarId(128));
        let mut jumping = Box::new(targeting(&catalog));
        jumping.jump = Some(crate::session::Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        let mut braking = Box::new(targeting(&catalog));
        braking.jump = Some(crate::session::Jump::PreJump {
            to: SystemId(131),
            bearing: None,
            warp: None,
        });
        let mut dying = Box::new(targeting(&catalog));
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let mut destroyed = Box::new(targeting(&catalog));
        destroyed.condition = Condition::Destroyed;
        for mut session in [untargeted, landed, jumping, braking, dying, destroyed] {
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
            (Box::new(disabled), &catalog),
            (Box::new(mute), &catalog),
            (Box::new(inheriting), &inherited),
        ] {
            let (hailed, asked) = hail_with(&mut session, catalog, &opening(0));
            assert_eq!(hailed, Err(HailRefusal::NoResponse));
            assert!(asked.is_empty(), "{asked:?}");
        }
    }

    #[test]
    fn a_target_emerging_from_a_gate_cannot_be_hailed_and_is_let_go() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).mode = Mode::Emerging { ticks_left: 4 };
        let (hailed, asked) = hail_with(&mut session, &catalog, &opening(0));
        assert!(hailed.is_err(), "{hailed:?}");
        assert!(asked.is_empty());
        session.clear_lost_target();
        assert_eq!(
            session.target, None,
            "as the original lets it go (@0x686a4)"
        );
    }

    #[test]
    fn a_target_still_jumping_in_cannot_be_hailed() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).mode = Mode::JumpingIn { tick: 0 };
        let (hailed, asked) = hail_with(&mut session, &catalog, &opening(0));
        assert_eq!(hailed, Err(HailRefusal::InHyperspace));
        assert!(asked.is_empty());
    }

    #[test]
    fn a_disabled_target_still_jumping_in_gives_no_response() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).mode = Mode::JumpingIn { tick: 0 };
        npc(&mut session).condition = Condition::Disabled;
        let (hailed, asked) = hail_with(&mut session, &catalog, &opening(0));
        assert_eq!(hailed, Err(HailRefusal::NoResponse), "@0x62610-0x62644");
        assert!(asked.is_empty(), "{asked:?}");
    }

    #[test]
    fn a_mute_target_still_jumping_in_is_entering_hyperspace() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).mode = Mode::JumpingIn { tick: 0 };
        npc(&mut session).govt = Some(MUTED);
        let (hailed, _) = hail_with(&mut session, &catalog, &opening(0));
        assert_eq!(
            hailed,
            Err(HailRefusal::InHyperspace),
            "the government is read after the hyperspace state"
        );
    }

    /// `catalog`'s targeted ship flying the escape pod, `shïp` 895.
    fn in_a_pod(catalog: &FakePilotCatalog) -> Session {
        let mut session = targeting(catalog);
        npc(&mut session).ship = crate::combat::hull::ESCAPE_POD_SHIP;
        session
    }

    #[test]
    fn an_escape_pod_gives_no_response_and_beeps_once() {
        let catalog = hailable();
        let mut session = in_a_pod(&catalog);
        session.take_sounds();
        let hailed = open_hail_with(&mut session, &catalog);
        assert_eq!(hailed, Err(HailRefusal::NoResponse), "@0x62145");
        assert_eq!(session.take_sounds(), [crate::SimSound::Refused]);
    }

    #[test]
    fn an_escape_pod_escorting_the_player_still_gives_no_response() {
        let catalog = hailable();
        let mut session = in_a_pod(&catalog);
        npc(&mut session).escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let (hailed, asked) = hail_with(&mut session, &catalog, &opening(0));
        assert_eq!(
            hailed,
            Err(HailRefusal::NoResponse),
            "the class test follows the escort shortcut (@0x620a2-0x620bb)"
        );
        assert!(asked.is_empty(), "{asked:?}");
    }

    #[test]
    fn an_escape_pod_still_jumping_in_is_entering_hyperspace() {
        let catalog = hailable();
        let mut session = in_a_pod(&catalog);
        npc(&mut session).mode = Mode::JumpingIn { tick: 0 };
        let (hailed, _) = hail_with(&mut session, &catalog, &opening(0));
        assert_eq!(
            hailed,
            Err(HailRefusal::InHyperspace),
            "the arrival test comes first (@0x62098)"
        );
    }

    /// Hails through the comm key, its conversation in variant 0.
    fn open_hail_with(
        session: &mut Session,
        catalog: &FakePilotCatalog,
    ) -> Result<super::super::Hailed, HailRefusal> {
        session.open_hail(
            catalog,
            &HailOptions::default(),
            &mut Draws::of(&opening(0)),
        )
    }

    // Hailing the navigation stellar (@0x61f91-0x6206c).

    /// Presses the comm key, Option held as `option` says, its
    /// conversation in variant 0: what it opened, the draws asked and
    /// the sounds made.
    fn key_with(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        option: bool,
    ) -> (
        Result<super::super::Hailed, HailRefusal>,
        Vec<u32>,
        Vec<crate::SimSound>,
    ) {
        session.take_sounds();
        let mut chance = Draws::of(&opening(0));
        let hailed = session.hail_key(catalog, &HailOptions::default(), option, &mut chance);
        (hailed, chance.asked, session.take_sounds())
    }

    /// `catalog`'s session with nothing targeted but planet 128, which
    /// the system shows, its navigation target, its `Flags` `flags` and
    /// `Flags2` `flags2`.
    fn at_stellar(catalog: &FakePilotCatalog, flags: u32, flags2: u16) -> Session {
        let mut session = targeting(catalog);
        session.target = None;
        session.nav = crate::navigation::NavMode::Stellar(Some(StellarId(128)));
        let site = session
            .sites
            .iter_mut()
            .find(|site| site.id == StellarId(128))
            .expect("shown");
        site.flags = flags;
        site.flags2 = flags2;
        session
    }

    #[test]
    fn with_no_ship_target_a_dockable_inhabited_nav_stellar_answers_silently() {
        let catalog = hailable();
        let mut session = at_stellar(&catalog, StellarFlags::CAN_LAND, 0);
        session.talk = Some(Talk::opened(NpcId(0)));
        let (hailed, asked, sounds) = key_with(&mut session, &catalog, false);
        assert_eq!(
            hailed,
            Ok(super::super::Hailed::Stellar(StellarId(128))),
            "_DoPlanetCommDialog's seam (@0x6204b)"
        );
        assert!(asked.is_empty(), "{asked:?}");
        assert_eq!(sounds, [], "the dialog path plays nothing");
        assert!(
            session.hailing(&catalog, &HailOptions::default()).is_none(),
            "the hail under way ends, and none opens"
        );
    }

    #[test]
    fn with_no_ship_target_a_nav_stellar_that_cannot_answer_gives_no_response() {
        use crate::gate::{HYPERGATE, WORMHOLE};
        let catalog = hailable();
        let land = StellarFlags::CAN_LAND;
        let mut hidden = Box::new(at_stellar(&catalog, land, 0));
        hidden.sites.retain(|site| site.id != StellarId(128));
        for (mut session, why) in [
            (
                Box::new(at_stellar(&catalog, land | StellarFlags::UNINHABITED, 0)),
                "uninhabited",
            ),
            (Box::new(at_stellar(&catalog, 0, 0)), "not landable"),
            (
                Box::new(at_stellar(&catalog, land, HYPERGATE)),
                "a hypergate",
            ),
            (Box::new(at_stellar(&catalog, land, WORMHOLE)), "a wormhole"),
            (hidden, "not active (+0x44 @0x62009)"),
        ] {
            let (hailed, asked, sounds) = key_with(&mut session, &catalog, false);
            assert_eq!(hailed, Err(HailRefusal::NoResponse), "{why}: @0x6261c");
            assert!(asked.is_empty(), "{why}: {asked:?}");
            assert_eq!(sounds, [crate::SimSound::Refused], "{why}");
        }
    }

    #[test]
    fn with_no_ship_target_and_no_nav_stellar_there_is_nothing_to_hail() {
        let catalog = hailable();
        let mut session = at_stellar(&catalog, StellarFlags::CAN_LAND, 0);
        session.nav = crate::navigation::NavMode::Off;
        let (hailed, asked, sounds) = key_with(&mut session, &catalog, false);
        assert_eq!(hailed, Err(HailRefusal::NoTarget), "@0x6206c");
        assert!(asked.is_empty(), "{asked:?}");
        assert_eq!(sounds, [crate::SimSound::Refused]);
    }

    #[test]
    fn option_held_hails_the_nav_stellar_over_the_ship_target() {
        let catalog = hailable();
        let mut session = at_stellar(&catalog, StellarFlags::CAN_LAND, 0);
        session.target = Some(NpcId(0));
        let (hailed, asked, sounds) = key_with(&mut session, &catalog, true);
        assert_eq!(hailed, Ok(super::super::Hailed::Stellar(StellarId(128))));
        assert!(asked.is_empty(), "the ship is not hailed: {asked:?}");
        assert_eq!(sounds, []);
        assert!(session.hailing(&catalog, &HailOptions::default()).is_none());
    }

    #[test]
    fn option_held_with_a_nav_stellar_that_cannot_answer_gives_no_response_over_the_ship() {
        let catalog = hailable();
        let flags = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        let mut session = at_stellar(&catalog, flags, 0);
        session.target = Some(NpcId(0));
        let (hailed, asked, sounds) = key_with(&mut session, &catalog, true);
        assert_eq!(hailed, Err(HailRefusal::NoResponse), "@0x6261c");
        assert!(asked.is_empty(), "the ship is not hailed: {asked:?}");
        assert_eq!(sounds, [crate::SimSound::Refused]);
        assert!(session.hailing(&catalog, &HailOptions::default()).is_none());
    }

    #[test]
    fn option_held_with_no_nav_stellar_has_nothing_to_hail_whatever_the_ship_target() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let (hailed, asked, sounds) = key_with(&mut session, &catalog, true);
        assert_eq!(hailed, Err(HailRefusal::NoTarget), "@0x6206c");
        assert!(asked.is_empty(), "{asked:?}");
        assert_eq!(sounds, [crate::SimSound::Refused]);
    }

    #[test]
    fn option_held_while_landed_jumping_or_dying_hails_nothing() {
        let catalog = hailable();
        let mut landed = Box::new(at_stellar(&catalog, StellarFlags::CAN_LAND, 0));
        landed.landed = Some(StellarId(128));
        let mut jumping = Box::new(at_stellar(&catalog, StellarFlags::CAN_LAND, 0));
        jumping.jump = Some(crate::session::Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        let mut dying = Box::new(at_stellar(&catalog, StellarFlags::CAN_LAND, 0));
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        for mut session in [landed, jumping, dying] {
            session.target = Some(NpcId(0));
            for option in [false, true] {
                let (hailed, asked, sounds) = key_with(&mut session, &catalog, option);
                assert_eq!(hailed, Err(HailRefusal::NoTarget), "option {option}");
                assert!(asked.is_empty(), "{asked:?}");
                assert_eq!(sounds, [crate::SimSound::Refused]);
            }
        }
    }

    #[test]
    fn every_refusal_beeps_once() {
        let catalog = hailable();
        let mut untargeted = Box::new(targeting(&catalog));
        untargeted.target = None;
        let mut disabled = Box::new(targeting(&catalog));
        npc(&mut disabled).condition = Condition::Disabled;
        let mut arriving = Box::new(targeting(&catalog));
        npc(&mut arriving).mode = Mode::JumpingIn { tick: 0 };
        let mut jumping = Box::new(targeting(&catalog));
        jumping.jump = Some(crate::session::Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        for mut session in [untargeted, disabled, arriving, jumping] {
            session.take_sounds();
            let hailed = open_hail_with(&mut session, &catalog);
            assert!(hailed.is_err(), "{hailed:?}");
            assert_eq!(
                session.take_sounds(),
                [crate::SimSound::Refused],
                "_beepSnd[3]"
            );
        }
    }

    #[test]
    fn a_refusal_through_hail_beeps_too() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.target = None;
        session.take_sounds();
        assert!(hail_with(&mut session, &catalog, &opening(0)).0.is_err());
        assert_eq!(session.take_sounds(), [crate::SimSound::Refused]);
    }

    #[test]
    fn an_answered_hail_makes_no_sound() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.take_sounds();
        let hailed = open_hail_with(&mut session, &catalog);
        assert!(
            matches!(hailed, Ok(super::super::Hailed::Comm(_))),
            "{hailed:?}"
        );
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_target_holding_its_course_after_its_glide_answers_a_hail() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        npc(&mut session).mode = Mode::Coasting { ticks_left: 30 };
        let (hailed, _) = hail_with(&mut session, &catalog, &opening(0));
        assert!(hailed.is_ok(), "{hailed:?}");
    }

    // The reply.

    #[test]
    fn a_ship_opens_by_the_players_record_with_its_government() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        crate::session::tests::with_record(&mut session, FED, -6);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.reply, "Channel open.");
        assert_eq!(
            labels(&view),
            [("Greetings", Some('G')), ("Request Assistance", Some('R'))]
        );
        crate::session::tests::with_record(&mut session, FED, -7);
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
        crate::session::tests::with_record(&mut session, FED, -7);
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

    // The comm text's tags (`_CommDialogUpdate` @0x907f7-0x9080b) and the
    // specific advice's parse (`_LoadAdvice` @0x919ca).

    /// [`hailable`] with `STR#` 7505's first line `advice`.
    fn advising(advice: &str) -> FakePilotCatalog {
        let mut catalog = hailable();
        catalog.strings[3].1[0] = advice.to_owned();
        catalog
    }

    /// [`targeting`], its ship giving specific advice (`STR#` 7505), the
    /// pilot "Stock".
    fn advised(catalog: &FakePilotCatalog) -> Session {
        let mut session = targeting(catalog);
        npc(&mut session).info_types = 0x4005;
        session.pilot.name = "Stock".to_owned();
        session
    }

    #[test]
    fn the_comm_text_is_filled_with_no_mission() {
        let catalog = advising("Hello, <PN>. Off to <DST>?");
        let mut session = advised(&catalog);
        assert_eq!(
            greeted(&mut session, &catalog, 2),
            "Hello, Stock. Off to [Error]?"
        );
        let rules = Rulebook::default().with_override(RuleKey::MissionlessTags, RuleSource::Bible);
        let mut session = advised(&catalog).with_rules(rules);
        assert_eq!(
            greeted(&mut session, &catalog, 2),
            "Hello, Stock. Off to <DST>?",
            "by the other reading, the mission tags as written"
        );
    }

    #[test]
    fn the_opening_line_is_filled_too() {
        let mut catalog = hailable();
        catalog.strings[0].1[3] = "Hail, <PN>.".to_owned();
        let mut session = targeting(&catalog);
        session.pilot.name = "Stock".to_owned();
        let (hailed, _) = hail_with(&mut session, &catalog, &opening(3));
        assert_eq!(hailed.map(|view| view.reply), Ok("Hail, Stock.".to_owned()));
    }

    #[test]
    fn specific_advice_reads_its_conditional_text_by_the_pilots_gender() {
        let catalog = advising("Fly safe, {G \"sir\" \"ma'am\"}.");
        let mut session = advised(&catalog);
        assert_eq!(greeted(&mut session, &catalog, 2), "Fly safe, sir.");
        let mut session = advised(&catalog);
        session.pilot.gender = crate::pilot::Gender::Female;
        assert_eq!(greeted(&mut session, &catalog, 2), "Fly safe, ma'am.");
    }

    #[test]
    fn specific_advice_emptied_by_its_conditional_text_is_nice_to_meet_you() {
        let catalog = advising("{b7 \"Secret.\"}");
        let mut session = advised(&catalog);
        assert_eq!(
            greeted(&mut session, &catalog, 2),
            "c48",
            "parsed before the check (@0x91aa9)"
        );
    }

    #[test]
    fn specific_advice_parses_its_rank_governments_with_no_reset() {
        let catalog = advising("Hail the <PRK128>.");
        let mut session = advised(&catalog);
        let _ = session.take_in(TextSource::Quote, "<SRK129>!", &mut NeverFires);
        let before = session.plain_text("<SRK129> <PRK128>");
        assert_eq!(before, "captain <PRK128>");
        let _ = greeted(&mut session, &catalog, 2);
        assert_eq!(
            session.plain_text("<SRK129> <PRK128>"),
            "captain captain",
            "the <PRK128> remembered beside the <SRK129> kept"
        );
        let plain = advising("No ranks here.");
        let mut session = advised(&plain);
        let _ = session.take_in(TextSource::Quote, "<SRK129>!", &mut NeverFires);
        let _ = greeted(&mut session, &plain, 2);
        assert_eq!(session.plain_text("<SRK129>"), "captain", "kept");
    }

    #[test]
    fn with_leet_mode_the_comm_text_is_munged() {
        let catalog = advising("Hello, <PN>.");
        let mut session = advised(&catalog);
        session.leet = true;
        assert_eq!(greeted(&mut session, &catalog, 2), "H377o, Stox0r.");
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
    /// centre and attacking it after a tick of Nova's AI, targeted. It
    /// carries a harmless gun of unlimited ammunition, as a warship out
    /// of ammunition takes no target (@0x89ff4).
    fn pirate(catalog: &FakePilotCatalog, govt: GovtId) -> Session {
        let mut session = targeting(catalog);
        npc(&mut session).govt = Some(govt);
        npc(&mut session).armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&crate::testkit::weapon(127)),
            1,
        )]);
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
    fn a_bribed_ship_with_an_empty_tank_leaves_all_the_same() {
        // `_AIMakeShipLeave` (@0x7e2b0) sets state 2 with no fuel test.
        let catalog = hailable();
        let mut session = pirate(&catalog, PIRATES);
        npc(&mut session).reserves.fuel.now = 0.0;
        hail(&mut session, &catalog);
        session
            .answer(1, &catalog, &HailOptions::default(), &mut Draws::of(&[99]))
            .expect("open");
        haggle(&mut session, &catalog, Haggle::Accept);
        let spared = &session.npcs()[0];
        assert!(spared.spared);
        assert_eq!(spared.goal, Goal::JumpOut);
        assert_eq!(spared.target, None);
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
            by: Some(ShipRef::Player),
            damage: 5.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
            nominal: crate::combat::Nominal::default(),
            weight: 0.0,
            answered: true,
        });
        decide(&mut session, &catalog, 1, |npc| {
            assert_eq!(npc.goal, Goal::Flee(ShipRef::Player), "hit, it flees");
        });
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
    fn a_person_declined_its_bribe_says_its_attack_quote() {
        // `_DoCommDialog` calls `_AIMakeShipAttackPlayer` (@0x9621f), whose
        // gate reads the ship before it sets goal 4 (@0x89c50-0x89c9b).
        let declined = |attacking: bool| {
            let catalog = hailable();
            let mut session = pirate(&catalog, PIRATES);
            hail(&mut session, &catalog);
            press(&mut session, &catalog, 1, 36);
            let ship = npc(&mut session);
            ship.person = Some(crate::traffic::npc::NpcPerson {
                id: crate::catalog::PersonId(151),
                flags: crate::person::QUOTE_ON_ATTACK,
                coward: 0,
                comm_quote: -1,
                hail_quote: 1,
                mission: false,
                portrait: None,
                invincible: false,
                grudge: false,
                quoted: false,
                quoted_at: None,
                zero_fuel: false,
            });
            if !attacking {
                ship.goal = Goal::Idle;
                ship.target = None;
            }
            let view = haggle(&mut session, &catalog, Haggle::LowerPrice);
            assert_eq!(view.reply, "What? How dare you! Prepare to die!");
            session.traffic.take_attack_quotes()
        };
        assert_eq!(declined(false), [NpcId(0)]);
        assert_eq!(declined(true), [], "already attacking the player");
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
    fn a_ship_asked_for_help_drops_its_ai_wait_and_docking() {
        // `_AIMakeShipRepairPlayer` @0x82df2 and `_AIMakeShipRefuelPlayer`
        // @0x82dbb: +0x4c = -1.0, sub 0.
        let catalog = hailable();
        let mut session = targeting(&catalog);
        session.pilot.reserves.fuel.now = 40.0;
        let ship = npc(&mut session);
        ship.ai_wait = 50;
        ship.docking = Some(crate::traffic::plunder_dock::Docking {
            ship: ShipRef::Npc(NpcId(7)),
            sub: crate::traffic::plunder_dock::DockSub::Close,
        });
        hail(&mut session, &catalog);
        press(&mut session, &catalog, 1, 99);
        haggle(&mut session, &catalog, Haggle::Accept);
        let helper = &session.npcs()[0];
        assert_eq!(helper.goal, Goal::Assist(Help::Refuel));
        assert_eq!((helper.ai_wait, helper.docking), (0, None));
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

    /// `hailable`, its government 129 allied with the Federation, with
    /// rank 128 of 129 carrying `flags`, active on the pilot of a session
    /// as `targeting` makes it.
    fn ranked(flags: u16) -> (FakePilotCatalog, Session) {
        let mut catalog = hailable();
        catalog.govts[1].allies = [1, -1, -1, -1];
        catalog.ranks = vec![RankRecord {
            affil_govt: Some(OTHER),
            flags,
            ..crate::testkit::rank(128)
        }];
        let mut session = targeting(&catalog);
        session.pilot.ranks.insert(RankId(128));
        (catalog, session)
    }

    #[test]
    fn a_rank_allied_with_the_ships_government_gives_it_free_help_by_the_engine_only() {
        let (catalog, mut session) = ranked(crate::rank::HELPS_FREE);
        session.pilot.reserves.fuel.now = 40.0;
        hail(&mut session, &catalog);
        let view = press(&mut session, &catalog, 1, 99);
        assert_eq!(view.reply, "Okay, I'm on my way.");
        assert_eq!(view.asking, None);
        assert_eq!(session.pilot().cash, 100_000);
        assert_eq!(session.npcs()[0].goal, Goal::Assist(Help::Refuel));
        let (catalog, mut session) = ranked(crate::rank::HELPS_FREE);
        session.rules =
            Rulebook::default().with_override(RuleKey::RankGovtReach, RuleSource::Bible);
        session.pilot.reserves.fuel.now = 40.0;
        hail(&mut session, &catalog);
        let view = press(&mut session, &catalog, 1, 99);
        assert_eq!(view.reply, "I'll help you out if you pay me.");
        assert_eq!(view.asking, Some(3000));
        assert_eq!(session.npcs()[0].goal, Goal::Idle);
    }

    #[test]
    fn battle_help_sets_the_ship_attacking_the_threat_and_takes_no_cash() {
        let mut catalog = hailable();
        catalog.ranks = vec![RankRecord {
            affil_govt: Some(FED),
            flags: crate::rank::HELPS_IN_BATTLE,
            ..crate::testkit::rank(128)
        }];
        let mut session = targeting(&catalog);
        session.pilot.ranks.insert(RankId(128));
        let pirate = Npc {
            govt: Some(PIRATES),
            goal: Goal::Attack(ShipRef::Player),
            ..session.npcs()[0].clone()
        };
        let pirate = session.traffic.add_npc(pirate);
        hail(&mut session, &catalog);
        let view = press(&mut session, &catalog, 1, 0);
        assert_eq!(view.reply, "Okay, I'm on my way.");
        assert_eq!(view.asking, None);
        assert_eq!(session.pilot().cash, 100_000);
        let helper = &session.npcs()[0];
        assert_eq!(helper.goal, Goal::Attack(ShipRef::Npc(pirate)));
        assert_eq!(helper.target, Some(ShipRef::Npc(pirate)));
        let view = press(&mut session, &catalog, 1, 0);
        assert_eq!(view.reply, "I'm busy.");
    }

    #[test]
    fn battle_help_calls_the_systems_reinforcements_past_their_strength_test() {
        let mut catalog = hailable();
        catalog.govts[2].enemies = [1, -1, -1, -1];
        catalog.ranks = vec![RankRecord {
            affil_govt: Some(FED),
            flags: crate::rank::HELPS_IN_BATTLE,
            ..crate::testkit::rank(128)
        }];
        catalog.reinforcements = vec![(
            SystemId(130),
            crate::catalog::SystemReinforcements {
                fleet: crate::catalog::FleetId(140),
                time: 900,
                interval: 2,
            },
        )];
        catalog.fleets = vec![crate::catalog::FleetRecord {
            id: crate::catalog::FleetId(140),
            lead: Some(ShipId(129)),
            escorts: Vec::new(),
            govt: Some(FED),
            link_syst: 133,
            appear_on: crate::control::Test::default(),
            quote: None,
            flags: 0,
        }];
        let mut session = targeting(&catalog);
        session.pilot.ranks.insert(RankId(128));
        npc(&mut session).hull.strength = 100.0;
        let pirate = Npc {
            govt: Some(PIRATES),
            goal: Goal::Attack(ShipRef::Player),
            hull: HullSpec {
                strength: 20.0,
                ..HullSpec::default()
            },
            ..session.npcs()[0].clone()
        };
        session.traffic.add_npc(pirate);
        // Its odds as the frame's pass left them: the pirate's 20 over
        // its 100 (see `odds_cache`).
        let hailed = session.npcs()[0].id;
        session.traffic.cache_odds(hailed, 0.2);
        assert_eq!(session.reinforcements_coming(), None);
        hail(&mut session, &catalog);
        press(&mut session, &catalog, 1, 0);
        assert_eq!(
            session.reinforcements_coming(),
            Some(crate::ai::reinforcements::Coming { govt: Some(FED) }),
            "odds of 0.2, below half of 1, yet called (@0x9636d)"
        );
    }

    #[test]
    fn free_help_says_its_price_first_then_is_on_its_way_and_takes_no_cash() {
        let mut catalog = hailable();
        catalog.govts[0].flags2 = crate::hail::FREE_HELP;
        let mut session = targeting(&catalog);
        session.pilot.reserves.fuel.now = 40.0;
        let view = hail(&mut session, &catalog);
        assert_eq!(view.first, None);
        let view = press(&mut session, &catalog, 1, 99);
        assert_eq!(
            view.first.as_deref(),
            Some("I'll help you out if you pay me.")
        );
        assert_eq!(view.reply, "Okay, I'm on my way.");
        assert_eq!(view.asking, None);
        assert_eq!(session.pilot().cash, 100_000);
        assert_eq!(session.npcs()[0].goal, Goal::Assist(Help::Refuel));
        let view = press(&mut session, &catalog, 0, 99);
        assert_eq!(view.first, None, "Greetings says one line");
        assert_eq!(
            session
                .hailing(&catalog, &HailOptions::default())
                .and_then(|view| view.first),
            None
        );
    }

    /// An option that says "Nice to meet you." before asking a price
    /// with "What is it you want?".
    #[derive(Debug)]
    struct Haggler;

    impl HailOption for Haggler {
        fn label(&self) -> String {
            "Haggle".to_owned()
        }

        fn applies(&self, _hail: &Hail) -> bool {
            true
        }

        fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
            Answer {
                first: Some(Reply::Comm(reply::NICE_TO_MEET_YOU)),
                ask: Some(Ask {
                    paid: (Reply::Comm(reply::ON_MY_WAY), None),
                    declined: (Reply::Comm(reply::COMEDIAN), None),
                    short: Reply::Comm(reply::NO_MONEY),
                }),
                ..Answer::say(Reply::Comm(reply::WHAT_IS_IT))
            }
        }
    }

    #[test]
    fn a_haggle_clears_the_line_said_first() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        let options = HailOptions::empty().with(Rc::new(Haggler));
        session
            .hail(&catalog, &options, &mut Draws::of(&opening(0)))
            .expect("answered");
        let view = session
            .answer(0, &catalog, &options, &mut Draws::of(&[99]))
            .expect("open");
        assert_eq!(view.first.as_deref(), Some("Nice to meet you."));
        assert_eq!(view.reply, "What is it you want?");
        let view = session
            .haggle(Haggle::LowerPrice, &catalog, &options)
            .expect("open");
        assert_eq!(view.reply, "Ha ha ha. What a comedian.");
        assert_eq!(view.first, None);
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
    fn a_pirate_taunted_in_its_count_wait_attacks_once_its_wait_is_out() {
        // `_DoCommDialog`'s attack (`_AIMakeShipAttackPlayer` @0x9621f)
        // writes no +0x4c, and nor does its bribe (`_AIMakeShipLeave`
        // @0x961eb, @0x7e2b0-0x7e2e7): every deed's goal keeps the wait,
        // as the count lives in the AI wait, not the goal.
        let mut catalog = hailable();
        catalog
            .strings
            .push((9000, vec!["Your mother was a tug.".to_owned()]));
        let options = HailOptions::default().with(Rc::new(Taunt));
        let mut session = targeting(&catalog);
        // Armed, as a warship out of ammunition takes no target (@0x89ff4).
        npc(&mut session).armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&crate::testkit::weapon(127)),
            1,
        )]);
        crate::testkit::count_waiting(npc(&mut session), 50);
        let mut chance = Draws::of(&opening(0));
        let view = session
            .hail(&catalog, &options, &mut chance)
            .expect("answered");
        let taunt = labels(&view)
            .iter()
            .position(|(label, _)| *label == "Taunt")
            .expect("listed");
        session
            .answer(taunt, &catalog, &options, &mut chance)
            .expect("open");
        let pirate = &session.npcs()[0];
        assert_eq!(pirate.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(pirate.ai_wait, 50, "the wait outlives goal 0x16");
        assert!(pirate.wait_running());
        // Through the session's traffic, it holds still through the wait,
        // its fire gated (`_HandleShip` @0x34a54-0x34a72)...
        let at = pirate.state;
        for tick in 1..50 {
            decide(&mut session, &catalog, 1, |pirate| {
                assert_eq!(pirate.goal, Goal::Attack(ShipRef::Player), "tick {tick}");
                assert_eq!(pirate.state, at, "still on tick {tick}");
                assert!(
                    !crate::ai::fire::gated(pirate).primary,
                    "it fires nothing on tick {tick}"
                );
            });
        }
        decide(&mut session, &catalog, 1, |pirate| {
            assert_eq!(pirate.state, at, "still on the wait's last tick");
            assert_eq!(pirate.ai_wait, 0, "the wait is out");
        });
        // ...then, the next tick, turns to its fight with the player below.
        decide(&mut session, &catalog, 1, |pirate| {
            assert_eq!(pirate.goal, Goal::Attack(ShipRef::Player));
            assert_ne!(pirate.state.heading, at.heading, "it turns to fight");
        });
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
        land_now(&mut session).expect("lands");
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
        // Bound for a stellar before it was asked: its travel stellar set.
        helper.travel_set = true;
        session
    }

    #[test]
    fn a_refuelling_ship_docked_at_its_reach_gives_a_unit_a_tick() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 120.0);
        session.pilot.reserves.fuel.now = 40.0;
        session.tick_assistance();
        assert_eq!(session.reserves().fuel.now, 41.0);
        let mut far = helped(&catalog, Help::Refuel, 121.0);
        far.pilot.reserves.fuel.now = 40.0;
        far.tick_assistance();
        assert_eq!(far.reserves().fuel.now, 40.0, "beyond its reach");
        let mut moving = helped(&catalog, Help::Refuel, 120.0);
        moving.pilot.reserves.fuel.now = 40.0;
        npc(&mut moving).state.velocity = Vec2::new(0.0, 0.35);
        moving.tick_assistance();
        assert_eq!(moving.reserves().fuel.now, 40.0, "not slow enough");
    }

    #[test]
    fn a_ship_holding_its_course_after_its_glide_gives_no_help_until_it_flies() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 120.0);
        session.pilot.reserves.fuel.now = 40.0;
        npc(&mut session).mode = Mode::Coasting { ticks_left: 3 };
        session.tick_assistance();
        assert_eq!(session.reserves().fuel.now, 40.0);
        npc(&mut session).mode = Mode::Flying;
        session.tick_assistance();
        assert_eq!(session.reserves().fuel.now, 41.0);
    }

    #[test]
    fn a_refuel_stops_above_a_jumps_fuel_and_says_so() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 0.0);
        session.pilot.reserves.fuel.now = 40.0;
        session.pilot.ship_name = Some("Kestrel".to_owned());
        let _ = session.take_sounds();
        for tick in 1..=61 {
            assert!(session.take_comm().is_empty(), "tick {tick}");
            session.tick_assistance();
        }
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert_eq!(
            session.take_comm(),
            [CommNote {
                from: NpcId(0),
                comm_name: "Cruiser".to_owned(),
                ship_name: "Kestrel".to_owned(),
                done: Help::Refuel,
            }]
        );
        assert_eq!(session.take_sounds(), [SimSound::EnergyTransferComplete]);
        let helper = &session.npcs()[0];
        assert_eq!(helper.goal, Goal::Idle, "it decides again");
        assert_eq!(helper.target, None);
        assert!(!helper.travel_set, "+0x6c -1, @0x8f6ce");
        session.tick_assistance();
        assert_eq!(session.reserves().fuel.now, 101.0);
        assert!(session.take_comm().is_empty());
    }

    #[test]
    fn a_finished_help_holds_the_message_line_for_its_note() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 0.0);
        session.pilot.reserves.fuel.now = 150.0;
        assert!(session.comm_line().free_for_calls(), "free before");
        session.tick_assistance();
        let [note] = session.take_comm().try_into().expect("one note");
        for tick in 1..note.ticks() {
            session.count_down_comm_line();
            assert!(!session.comm_line().free_for_calls(), "{tick} counted");
        }
        session.count_down_comm_line();
        assert!(session.comm_line().free_for_calls(), "the note counted");
    }

    #[test]
    fn a_player_already_above_a_jumps_fuel_gains_nothing_and_the_refuel_is_done() {
        let catalog = hailable();
        let mut session = helped(&catalog, Help::Refuel, 0.0);
        session.pilot.reserves.fuel.now = 150.0;
        session.tick_assistance();
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
        session.tick_assistance();
        assert_eq!(session.reserves().fuel.now, 80.0);
        assert_eq!(session.take_comm().len(), 1);
    }

    #[test]
    fn nothing_is_given_while_landed_jumping_or_by_a_ship_not_intact() {
        let catalog = hailable();
        let mut landed = helped(&catalog, Help::Refuel, 0.0);
        landed.landed = Some(StellarId(128));
        let mut jumping = helped(&catalog, Help::Refuel, 0.0);
        jumping.jump = Some(crate::session::Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        let mut disabled = helped(&catalog, Help::Refuel, 0.0);
        npc(&mut disabled).condition = Condition::Disabled;
        // One at a time: three sessions make too large an array.
        let given_nothing = |mut session: Session| {
            session.pilot.reserves.fuel.now = 40.0;
            session.tick_assistance();
            assert_eq!(session.reserves().fuel.now, 40.0);
        };
        given_nothing(landed);
        given_nothing(jumping);
        given_nothing(disabled);
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
            by: Some(ShipRef::Player),
            damage: 5.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
            nominal: crate::combat::Nominal::default(),
            weight: 0.0,
            answered: true,
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
    fn hailing_a_hired_escort_shows_its_daily_pay() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        session.pilot.escorts[0].wage = Some(100);
        let view = hail(&mut session, &catalog);
        assert_eq!(
            view.escort,
            Some(crate::hail::EscortStatus { wage: Some(100) })
        );
        assert_eq!(labels(&view), [("Release", Some('R'))]);
    }

    #[test]
    fn hailing_a_captured_escort_shows_it_is_an_escort_with_no_pay() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        let view = hail(&mut session, &catalog);
        assert_eq!(view.escort, Some(crate::hail::EscortStatus { wage: None }));
        assert_eq!(labels(&view), [("Release", Some('R'))]);
    }

    #[test]
    fn hailing_a_ship_of_the_traffic_shows_no_escort_status() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        assert_eq!(hail(&mut session, &catalog).escort, None);
    }

    /// [`targeting`]'s ship marked as a ship of the mission in slot 0
    /// that escorts the player, in the player's formation when
    /// `formation` holds one.
    fn mission_escort(
        catalog: &FakePilotCatalog,
        formation: Option<crate::escort::FormationSlot>,
    ) -> Session {
        let mut session = targeting(catalog);
        npc(&mut session).mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: ShipBehav::EscortPlayer,
            formation,
            order: None,
        });
        session
    }

    #[test]
    fn hailing_a_mission_escort_shows_it_is_an_escort_with_no_pay() {
        let catalog = hailable();
        let mut session = mission_escort(&catalog, Some(crate::escort::FormationSlot::default()));
        assert_eq!(npc(&mut session).escort, None, "not of the fleet");
        assert!(session.player_led().any(|led| led.id == NpcId(0)));
        let view = hail(&mut session, &catalog);
        assert_eq!(view.escort, Some(crate::hail::EscortStatus { wage: None }));
    }

    #[test]
    fn a_mission_ship_not_in_the_formation_shows_no_escort_status() {
        let catalog = hailable();
        let mut session = mission_escort(&catalog, None);
        assert!(!session.player_led().any(|led| led.id == NpcId(0)));
        assert_eq!(hail(&mut session, &catalog).escort, None);
    }

    #[test]
    fn once_release_is_pressed_the_escort_shows_no_status() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        hail(&mut session, &catalog);
        let view = session
            .answer(0, &catalog, &HailOptions::default(), &mut Draws::of(&[]))
            .expect("answered");
        assert_eq!(view.escort, None, "release pending (0xdef2c)");
        assert!(!view.hostile);
        assert!(session.is_escort(NpcId(0)), "not yet released");
        let view = hail(&mut session, &catalog);
        assert_eq!(
            view.escort,
            Some(crate::hail::EscortStatus { wage: None }),
            "the flag is cleared as the dialog opens"
        );
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
        assert_eq!(view.escort, Some(crate::hail::EscortStatus { wage: None }));
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
        assert_eq!(released.goal, Goal::JumpOut);
        assert_eq!(released.target, None);
        assert_eq!(released.govt, None);
        assert!(session.take_save_due());
    }

    /// Releases the escort [`escorting`] leads, by `source` of
    /// `ReleaseCargoShare`, holding 30 food and 8 tons of mission cargo,
    /// its latches cleared.
    fn released(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        use crate::market::Good;
        let mut session = escorting(catalog).with_rules(
            crate::rulebook::Rulebook::default()
                .with_override(crate::rulebook::RuleKey::ReleaseCargoShare, source),
        );
        session.pilot.cargo = std::iter::once((Good::Commodity(0), 30)).collect();
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 8));
        crate::session::precalced::tests::convicted(&mut session);
        hail(&mut session, catalog);
        session.answer(0, catalog, &HailOptions::default(), &mut Draws::of(&[]));
        session.hang_up();
        session
    }

    #[test]
    fn a_released_trader_takes_its_share_and_resets_still_in_the_fleet() {
        use crate::market::Good;
        use crate::scan::Latches;
        let catalog = hailable();
        for source in RuleSource::ALL {
            let session = released(&catalog, source);
            assert_eq!(session.pilot().escorts(), [], "{source:?}");
            assert_eq!(session.scan_latches, Latches::ARMED, "{source:?}: @0x96864");
            assert_eq!(
                session.pilot().held(Good::Commodity(0)),
                15,
                "{source:?}: 20 of 40, then no trim with it still in the fleet"
            );
        }
    }

    #[test]
    fn a_released_warship_takes_a_share_by_the_engine_and_none_by_the_other_reading() {
        use crate::market::Good;
        use crate::scan::Latches;
        let mut catalog = hailable();
        catalog.ship_records[0].inherent_ai = 4;
        let session = released(&catalog, RuleSource::Engine);
        assert_eq!(session.scan_latches, Latches::ARMED);
        assert_eq!(
            session.pilot().held(Good::Commodity(0)),
            0,
            "its 20 over the player's 20: everything"
        );
        let session = released(&catalog, RuleSource::Bible);
        assert_ne!(session.scan_latches, Latches::ARMED, "no call");
        assert_eq!(session.pilot().held(Good::Commodity(0)), 30);
    }

    #[test]
    fn released_with_an_empty_tank_it_leaves_all_the_same() {
        let catalog = hailable();
        let mut session = escorting(&catalog);
        npc(&mut session).reserves.fuel.now = 0.0;
        hail(&mut session, &catalog);
        session.answer(0, &catalog, &HailOptions::default(), &mut Draws::of(&[]));
        session.hang_up();
        assert_eq!(npc(&mut session).goal, Goal::JumpOut);
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

    // Persons joining the fleet.

    /// [`hailable`] with "Merchant" (`përs` 600, anywhere, of `Flags`
    /// 0x0040), linked to mission 130, offered from a ship, whose one
    /// special ship escorts the player: it may join.
    fn merchants() -> FakePilotCatalog {
        let mut catalog = hailable();
        catalog.persons = vec![crate::catalog::PersonRecord {
            name: "Merchant".to_owned(),
            flags: crate::person::REPLACED_BY_MISSION_SHIP,
            link_mission: Some(130),
            mission_ship: Some(crate::catalog::MissionShip {
                count: 1,
                goal: 3,
                ..crate::catalog::MissionShip::NONE
            }),
            ..crate::testkit::person(600, 129)
        }];
        catalog.missions = vec![crate::catalog::MissionRecord {
            avail_loc: 2,
            ..crate::testkit::mission(130)
        }];
        catalog
    }

    /// [`targeting`] Merchant, flying the Federation ship (NPC 0), worn,
    /// its mission resolved as it spawned.
    fn targeting_merchant(catalog: &FakePilotCatalog) -> Session {
        let mut session = targeting(catalog);
        let record = catalog.persons[0].clone();
        let merchant = npc(&mut session);
        merchant.person = Some(crate::traffic::npc::NpcPerson::of(&record, false));
        merchant.reserves.shield.now = 7.0;
        merchant.reserves.armor.now = 20.0;
        merchant.goal = Goal::Attack(ShipRef::Npc(NpcId(5)));
        merchant.leader = Some(NpcId(5));
        session.resolve_person_offers(None, &mut crate::NeverFires);
        session
    }

    /// Nova's options with `person_join` by `source`.
    fn joining_options(source: RuleSource) -> HailOptions {
        HailOptions::nova(&Rulebook::default().with_override(RuleKey::PersonJoin, source))
    }

    /// The comm dialog's labels hailing NPC 0 with `options`.
    fn hailed_labels(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        options: &HailOptions,
    ) -> Vec<String> {
        let view = session
            .hail(catalog, options, &mut Draws::of(&opening(0)))
            .expect("answered");
        view.options
            .into_iter()
            .map(|button| button.label)
            .collect()
    }

    /// An escort of ship 129, a carried fighter when `carried`.
    fn escort_of(carried: bool) -> crate::pilot::Escort {
        crate::pilot::Escort {
            ship: ShipId(129),
            reserves: crate::reserves::Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried,
            wage: None,
            person: None,
        }
    }

    #[test]
    fn a_person_who_may_join_lists_use_as_escort_while_the_fleet_has_room() {
        let catalog = merchants();
        let bible = joining_options(RuleSource::Bible);
        let mut session = targeting_merchant(&catalog);
        assert_eq!(
            hailed_labels(&mut session, &catalog, &bible),
            ["Greetings", "Request Assistance", "Use As Escort"]
        );
        assert_eq!(
            hailed_labels(&mut session, &catalog, &joining_options(RuleSource::Engine)),
            ["Greetings", "Request Assistance"],
            "by the engine"
        );
        session.pilot.escorts = vec![escort_of(false); 5];
        session.pilot.escorts.extend([escort_of(true); 3]);
        assert!(
            hailed_labels(&mut session, &catalog, &bible).contains(&"Use As Escort".to_owned()),
            "fighters do not count"
        );
        session.pilot.escorts.push(escort_of(false));
        assert_eq!(
            hailed_labels(&mut session, &catalog, &bible),
            ["Greetings", "Request Assistance"],
            "six escorts fill the fleet"
        );
        let mut plain = targeting(&catalog);
        assert_eq!(
            hailed_labels(&mut plain, &catalog, &bible),
            ["Greetings", "Request Assistance"],
            "a ship no person flies"
        );
        let mut barred = merchants();
        barred.persons[0].flags |= crate::person::MISSION_ON_BOARD;
        let mut session = targeting_merchant(&barred);
        assert_eq!(
            hailed_labels(&mut session, &barred, &bible),
            ["Greetings", "Request Assistance"],
            "a person whose record does not allow it"
        );
        let mut session = targeting_merchant(&catalog);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(130, 0));
        assert_eq!(
            hailed_labels(&mut session, &catalog, &bible),
            ["Greetings", "Request Assistance"],
            "a person whose mission is not available"
        );
    }

    /// What hailing Merchant through the hail entry point opens, with
    /// `options`.
    fn opened(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        options: &HailOptions,
    ) -> Vec<String> {
        match session.open_hail(catalog, options, &mut Draws::of(&opening(0))) {
            Ok(super::super::Hailed::Comm(view)) => view
                .options
                .into_iter()
                .map(|button| button.label)
                .collect(),
            Ok(super::super::Hailed::Offer(_)) => vec!["the offer".to_owned()],
            hailed => panic!("{hailed:?}"),
        }
    }

    #[test]
    fn by_the_engine_a_person_who_may_join_offers_its_mission() {
        let catalog = merchants();
        let mut session = targeting_merchant(&catalog);
        assert_eq!(
            opened(&mut session, &catalog, &joining_options(RuleSource::Engine)),
            ["the offer"]
        );
    }

    #[test]
    fn by_the_other_reading_a_person_who_may_join_opens_the_comm_dialog_to_join() {
        let catalog = merchants();
        let bible = joining_options(RuleSource::Bible);
        let mut session = targeting_merchant(&catalog);
        assert_eq!(
            opened(&mut session, &catalog, &bible),
            ["Greetings", "Request Assistance", "Use As Escort"]
        );
        session
            .answer(2, &catalog, &bible, &mut Draws::of(&[]))
            .expect("open");
        assert!(session.is_escort(NpcId(0)), "joined");
        let mut full = targeting_merchant(&catalog);
        full.pilot.escorts = vec![escort_of(false); 6];
        assert_eq!(
            opened(&mut full, &catalog, &bible),
            ["the offer"],
            "with the fleet full"
        );
    }

    #[test]
    fn by_the_other_reading_a_person_whose_mission_is_unavailable_lists_no_way_to_join() {
        let mut catalog = merchants();
        catalog.missions[0].avail_random = 50;
        let mut session = targeting_merchant(&catalog);
        session.offers.set_roll(crate::catalog::MissionId(130), 51);
        assert_eq!(
            opened(&mut session, &catalog, &joining_options(RuleSource::Bible)),
            ["Greetings", "Request Assistance"]
        );
    }

    #[test]
    fn pressing_use_as_escort_makes_the_person_an_escort_at_once() {
        let catalog = merchants();
        let bible = joining_options(RuleSource::Bible);
        let mut session = targeting_merchant(&catalog);
        let mut other = crate::testkit::npc(9, session.npcs()[0].stats);
        other.target = Some(ShipRef::Npc(NpcId(0)));
        other.goal = Goal::Attack(ShipRef::Npc(NpcId(0)));
        let other = session.traffic.add_npc(other);
        session.take_save_due();
        session
            .hail(&catalog, &bible, &mut Draws::of(&opening(0)))
            .expect("answered");
        let reserves = session.npcs()[0].reserves;
        let mut chance = Draws::of(&[]);
        let view = session
            .answer(2, &catalog, &bible, &mut chance)
            .expect("open");
        assert!(chance.asked.is_empty(), "nothing drawn");
        assert_eq!(view.reply, "Okay, I'm on my way.");
        assert_eq!(
            view.options
                .iter()
                .map(|button| button.label.as_str())
                .collect::<Vec<_>>(),
            ["Release"],
            "the hail stays open, with the escort's options"
        );
        assert_eq!(
            session.pilot().escorts(),
            [crate::pilot::Escort {
                ship: ShipId(129),
                reserves,
                order: None,
                carried: false,
                wage: None,
                person: Some(crate::catalog::PersonId(600)),
            }],
            "its reserves as they were, not halved"
        );
        let merchant = &session.npcs()[0];
        assert_eq!(
            merchant.person.map(|person| person.id),
            Some(crate::catalog::PersonId(600))
        );
        assert_eq!(merchant.govt, None);
        assert_eq!(merchant.ai_type, AiType::WimpyTrader, "its InherentAI");
        assert_eq!(merchant.goal, Goal::Formation { guard: None });
        assert_eq!(merchant.leader, None);
        assert_eq!(merchant.condition, Condition::Intact);
        assert!(merchant.escort.is_some());
        assert!(session.is_escort(NpcId(0)));
        assert_eq!(session.target, None, "the player's target cleared");
        let other = session.npc(other).expect("there");
        assert_eq!((other.target, other.goal), (None, Goal::Idle), "let go");
        assert!(session.take_save_due());
        assert!(
            !session.pilot().gone(crate::catalog::PersonId(600)),
            "alive"
        );
    }
}
