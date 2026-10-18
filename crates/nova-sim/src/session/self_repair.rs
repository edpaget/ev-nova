//! Disabled ships repairing themselves in flight, and former escorts
//! rejoining the fleet as they do: the session's side of
//! [`self_repair`](crate::self_repair), `_HandleShip`'s disabled branch
//! (@0x33a3b-0x33bde).
//!
//! **Each fight tick**, after the escorts lost to it have left the fleet
//! ([`Session::lose_escorts`]), every NPC that is not disabled forgets
//! what it was as an escort ([`Npc::former`](crate::traffic::npc::Npc::former), @0x33bdb), and every
//! disabled NPC whose class carries a repair system
//! ([`Npc::self_repair`](crate::traffic::npc::Npc::self_repair)) draws `Rand(500)`. On 0, with armour above 0
//! and not breaking up, its armour goes to
//! [`repaired_armor`](crate::self_repair::repaired_armor) and its
//! condition is decided again at once, so no frame shows it disabled: it
//! is intact unless the fight's [`DisableRule`](crate::combat::hull::DisableRule)
//! still disables it, or it is a mission's ship held disabled (as
//! `_IsDisabled` holds it).
//!
//! **Rejoining** (@0x33afc-0x33bd9). A repaired ship that remembers being
//! the player's escort rejoins the fleet as what it was
//! ([`FormerEscort::rejoined`](crate::self_repair::FormerEscort::rejoined)):
//! its record back at the end of the fleet with its reserves, its
//! shield none (+0x54), its own target let go (+0x6c, +0x70), keeping
//! formation by its standing order as it was, and a fighter carried by
//! the player again. It keeps its berth, the formation re-forms, and a
//! save is due. The player's target is kept, though the original redraws
//! the target panel when it is this ship (@0x33ad9-0x33af5; the engine
//! draws the panel every frame). The player is told "Fighter repaired." or
//! "Escort repaired." ([`SimMessage::FighterRepaired`](crate::message::SimMessage::FighterRepaired),
//! [`SimMessage::EscortRepaired`](crate::message::SimMessage::EscortRepaired)) with `_beepSnd[4]`
//! ([`SimSound::EscortRepaired`]). As in the original, the fleet's size
//! ([`MAX_ESCORTS`](crate::board::MAX_ESCORTS)) is not checked. A ship
//! the rule still disables once repaired (a class that is not tough
//! holding more than 33,333 armour) rejoins all the same, as the
//! original's does, but [`Session::lose_escorts`] loses it again at the
//! next fight tick where the original keeps it led and disabled: a
//! divergence filed with
//! rdm:phase/player-led-ships/phase-26-warp-out-motion-and-disabled-drag.
//!
//! A repaired ship that remembers nothing (traffic carrying the gear, or
//! a disabled mission escort, which never left its formation) only gets
//! its armour back; a mission escort resumes as one.
//!
//! The original repairs in its ships' turn of the frame, after the damage
//! that disabled them; here it is the fight tick's last step for the
//! NPCs, before the frame is drawn.

use super::Session;
use crate::chance::Chance;
use crate::combat::Rules;
use crate::combat::hull::Condition;
use crate::self_repair::{
    FormerEscort, REPAIR_ODDS, can_self_repair, rejoin_message, repaired_armor,
};
use crate::sound::SimSound;
use crate::traffic::npc::NpcId;

impl Session {
    /// Each disabled NPC with the gear may repair itself, and a former
    /// escort among them rejoin, drawing on `chance`; every other NPC
    /// forgets being an escort (see the module docs).
    pub(super) fn self_repair(&mut self, rules: Rules, chance: &mut (impl Chance + ?Sized)) {
        let mut disabled = Vec::new();
        for npc in self.traffic.npcs_mut() {
            match npc.condition {
                Condition::Intact => npc.former = None,
                Condition::Disabled if npc.self_repair => disabled.push(npc.id),
                _ => {}
            }
        }
        for id in disabled {
            let held = self.npc(id).is_some_and(|npc| self.mission_ship_held(npc));
            let Some(npc) = self.npc_mut(id) else {
                continue;
            };
            // The gear is tested before the draw, so that a ship without
            // it draws nothing.
            let drawn = chance.below(REPAIR_ODDS);
            if !can_self_repair(drawn, npc.condition, npc.reserves.armor.now, true) {
                continue;
            }
            npc.reserves.armor.now = repaired_armor(npc.reserves.armor.max, npc.hull.tough);
            let still = held || rules.disable.disabled(npc.reserves.armor, &npc.hull);
            npc.condition = if still {
                Condition::Disabled
            } else {
                Condition::Intact
            };
            if let Some(former) = npc.former.take() {
                self.rejoin(id, former);
            }
        }
    }

    /// Former escort NPC `id`, repaired, rejoins the fleet as `former`
    /// says (see the module docs).
    fn rejoin(&mut self, id: NpcId, former: FormerEscort) {
        let Some(npc) = self.npc_mut(id) else {
            return;
        };
        npc.reserves.shield.now = 0.0;
        let record = former.rejoined(npc.reserves);
        self.enlist(id, record);
        self.say(rejoin_message(former.kind()));
        self.sounds.push(SimSound::EscortRepaired);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Peaceful;
    use crate::bay::{Carrier, dock_window};
    use crate::catalog::{HullRecord, OutfitId, ShipId, ShipRecord, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::escort::EscortOrder;
    use crate::message::SimMessage;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::{Gauge, Reserves};
    use crate::self_repair::{FormerKind, MOD_REPAIR};
    use crate::testkit::{Draws, FAST, FakePilotCatalog, catalog, hull, jump, outfit, ship};

    /// The warship escort's type, carrying Repair Droids, 50 pixels
    /// across, of maneuver 30.
    const WARSHIP: ShipId = ShipId(150);
    /// The freighter's, carrying them, tough.
    const FREIGHTER: ShipId = ShipId(151);
    /// A ship type without them.
    const BARE: ShipId = ShipId(152);
    /// Repair Droids.
    const DROIDS: OutfitId = OutfitId(437);

    fn repairing() -> FakePilotCatalog {
        let droids = vec![(DROIDS, 1)];
        FakePilotCatalog {
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    escort_type: 2,
                    inherent_ai: 3,
                    defaults: droids.clone(),
                    fields: crate::handling::ShipFields {
                        maneuver: 30,
                        ..FAST
                    },
                    ..ship(150, FAST)
                },
                ShipRecord {
                    escort_type: 3,
                    inherent_ai: 1,
                    defaults: droids,
                    ..ship(151, FAST)
                },
                ship(152, FAST),
            ],
            outfits: vec![outfit(437, &[(MOD_REPAIR, 0)])],
            hulls: vec![
                HullRecord {
                    size: Some(40),
                    ..hull(128)
                },
                HullRecord {
                    size: Some(50),
                    ..hull(150)
                },
                HullRecord {
                    size: Some(30),
                    flags: crate::combat::hull::TOUGH,
                    ..hull(151)
                },
                hull(152),
            ],
            ..catalog()
        }
    }

    /// An escort of `ship`, full.
    fn escort(ship: ShipId) -> Escort {
        Escort {
            ship,
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    /// A warship hired at 50 a day, defending; a captured freighter; a
    /// carried warship fighter; and a captured bare ship.
    fn fleet() -> Vec<Escort> {
        vec![
            Escort {
                wage: Some(50),
                order: Some(EscortOrder::Defend),
                ..escort(WARSHIP)
            },
            escort(FREIGHTER),
            Escort {
                carried: true,
                ..escort(WARSHIP)
            },
            escort(BARE),
        ]
    }

    /// [`fleet`] flying, placed, the warship ordered to defend again
    /// (entering the system reset it), with no message, sound or save due.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = fleet();
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
        session.pilot.escorts[0].order = Some(EscortOrder::Defend);
        let _ = (session.take_messages(), session.take_sounds());
        session.take_save_due();
        session
    }

    /// The NPCs of the fleet, in fleet order.
    fn placed(session: &Session) -> Vec<NpcId> {
        session.fleet.iter().flatten().copied().collect()
    }

    /// Escort NPC `id` is disabled (armour 3 of 45, below a tenth) and
    /// the fight ticks, drawing nothing: it leaves the fleet.
    fn disable(session: &mut Session, id: NpcId) {
        session.npc_mut(id).expect("there").reserves.armor.now = 3.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(!session.is_escort(id));
        let _ = (session.take_messages(), session.take_sounds());
        session.take_save_due();
    }

    /// The fight ticks on `draws`; what it asked.
    fn fight(session: &mut Session, draws: &[u32]) -> Vec<u32> {
        let mut chance = Draws::of(draws);
        session.tick_combat(Rules::default(), &mut chance);
        chance.asked
    }

    #[test]
    fn a_hired_escort_that_repairs_itself_rejoins_hired_with_its_wage_in_formation() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let hired = placed(&session)[0];
        disable(&mut session, hired);
        session.target = Some(hired);
        assert_eq!(fight(&mut session, &[0]), [REPAIR_ODDS]);
        let armor = repaired_armor(45.0, false);
        let reserves = Reserves {
            shield: Gauge {
                now: 0.0,
                max: 30.0,
            },
            armor: Gauge {
                now: armor,
                max: 45.0,
            },
            fuel: Gauge {
                now: 300.0,
                max: 300.0,
            },
        };
        assert_eq!(
            session.pilot().escorts().last(),
            Some(&Escort {
                reserves,
                ..fleet()[0]
            }),
            "hired at its wage, defending"
        );
        assert!(session.is_escort(hired));
        let npc = session.npc(hired).expect("here");
        assert_eq!(npc.condition, Condition::Intact);
        assert_eq!(npc.reserves, reserves);
        assert_eq!(npc.former, None);
        assert_eq!(npc.leader, None);
        assert_eq!(npc.target, None);
        assert_eq!(npc.goal, crate::ai::Goal::Formation { guard: None });
        assert_eq!(
            npc.escort.map(|duty| (duty.order, duty.formation.ships)),
            Some((Some(EscortOrder::Defend), 5)),
            "in the formation of all four and the player"
        );
        assert_eq!(npc.carrier, None);
        assert_eq!(session.target, Some(hired), "the player's target is kept");
        assert_eq!(session.take_messages(), [SimMessage::EscortRepaired]);
        assert_eq!(session.take_sounds(), [SimSound::EscortRepaired]);
        assert!(session.take_save_due());
        assert!(
            crate::save::encode(session.pilot()).contains("\"defend\""),
            "saved with its order"
        );
    }

    #[test]
    fn a_captured_escort_rejoins_unhired() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let captured = placed(&session)[1];
        disable(&mut session, captured);
        // A carrier of its own, unlike the one the player would give it.
        let elsewhere = Some(Carrier {
            ship: ShipRef::Player,
            window: 7.0,
            reach: 9.0,
        });
        session.npc_mut(captured).expect("here").carrier = elsewhere;
        assert_eq!(fight(&mut session, &[0]), [REPAIR_ODDS]);
        assert_eq!(
            session.npc(captured).expect("here").carrier,
            elsewhere,
            "an escort not carried keeps whatever carrier it had"
        );
        let record = session.pilot().escorts().last().expect("rejoined");
        assert_eq!(
            (record.ship, record.wage, record.carried),
            (FREIGHTER, None, false)
        );
        assert_eq!(
            record.reserves.armor.now,
            repaired_armor(45.0, true),
            "a tough hull: a tenth and one"
        );
        assert!((record.reserves.armor.now - 5.5).abs() < 1e-4);
        assert_eq!(session.take_messages(), [SimMessage::EscortRepaired]);
    }

    #[test]
    fn a_fighter_rejoins_as_a_fighter_carried_by_the_player() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let fighter = placed(&session)[2];
        disable(&mut session, fighter);
        session.npc_mut(fighter).expect("here").carrier = None;
        assert_eq!(fight(&mut session, &[0]), [REPAIR_ODDS]);
        let record = session.pilot().escorts().last().expect("rejoined");
        assert!(record.carried);
        assert_eq!(record.wage, None);
        let npc = session.npc(fighter).expect("here");
        assert_eq!(
            npc.carrier,
            Some(Carrier {
                ship: ShipRef::Player,
                window: dock_window(30),
                reach: 40.0,
            }),
            "it can dock again"
        );
        assert_eq!(session.take_messages(), [SimMessage::FighterRepaired]);
        assert_eq!(session.take_sounds(), [SimSound::EscortRepaired]);
    }

    #[test]
    fn a_draw_other_than_none_repairs_nothing() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let hired = placed(&session)[0];
        disable(&mut session, hired);
        assert_eq!(fight(&mut session, &[1]), [REPAIR_ODDS]);
        let npc = session.npc(hired).expect("here");
        assert_eq!(npc.condition, Condition::Disabled);
        assert_eq!(npc.reserves.armor.now, 3.0);
        assert!(npc.former.is_some(), "still remembers");
        assert!(!session.is_escort(hired));
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.take_sounds(), []);
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_ship_without_the_gear_draws_nothing_and_stays_disabled() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let bare = placed(&session)[3];
        disable(&mut session, bare);
        assert_eq!(fight(&mut session, &[0]), Vec::<u32>::new());
        let npc = session.npc(bare).expect("here");
        assert_eq!(npc.condition, Condition::Disabled);
        assert!(!session.is_escort(bare));
    }

    #[test]
    fn traffic_with_the_gear_repairs_itself_and_joins_nothing() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let hired = placed(&session)[0];
        disable(&mut session, hired);
        session.npc_mut(hired).expect("here").former = None;
        assert_eq!(fight(&mut session, &[0]), [REPAIR_ODDS]);
        let npc = session.npc(hired).expect("here");
        assert_eq!(npc.condition, Condition::Intact);
        assert_eq!(npc.reserves.armor.now, repaired_armor(45.0, false));
        assert_eq!(npc.escort, None);
        assert!(!session.is_escort(hired));
        assert_eq!(session.pilot().escorts().len(), 3);
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_ship_the_rule_still_disables_stays_disabled_as_it_rejoins() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let hired = placed(&session)[0];
        disable(&mut session, hired);
        let npc = session.npc_mut(hired).expect("here");
        npc.reserves.armor.max = 40_000.0;
        npc.reserves.armor.now = 100.0;
        session.self_repair(Rules::default(), &mut Draws::of(&[0]));
        let npc = session.npc(hired).expect("here");
        assert_eq!(npc.reserves.armor.now, repaired_armor(40_000.0, false));
        assert_eq!(npc.condition, Condition::Disabled, "below a third");
        assert!(session.is_escort(hired), "rejoins all the same");
    }

    #[test]
    fn a_ship_breaking_up_or_with_no_armour_never_repairs_itself() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let hired = placed(&session)[0];
        disable(&mut session, hired);
        session.npc_mut(hired).expect("here").reserves.armor.now = 0.0;
        session.self_repair(Rules::default(), &mut Draws::of(&[0]));
        assert!(!session.is_escort(hired));
        assert_eq!(session.npc(hired).expect("here").reserves.armor.now, 0.0);
    }

    #[test]
    fn a_former_escort_intact_again_forgets_and_never_rejoins() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let hired = placed(&session)[0];
        disable(&mut session, hired);
        session.npc_mut(hired).expect("here").reserves.armor.now = 45.0;
        assert_eq!(fight(&mut session, &[]), Vec::<u32>::new());
        let npc = session.npc(hired).expect("here");
        assert_eq!(npc.condition, Condition::Intact);
        assert_eq!(npc.former, None);
        session.npc_mut(hired).expect("here").reserves.armor.now = 3.0;
        fight(&mut session, &[]);
        assert_eq!(fight(&mut session, &[0]), [REPAIR_ODDS]);
        assert!(!session.is_escort(hired), "no longer remembers");
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn a_rejoined_escort_follows_the_player_through_a_jump() {
        let catalog = repairing();
        let mut session = flying(&catalog);
        let hired = placed(&session)[0];
        disable(&mut session, hired);
        fight(&mut session, &[0]);
        session.pilot.set_cash(10_000);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let index = session
            .pilot()
            .escorts()
            .iter()
            .position(|record| record.wage == Some(50))
            .expect("still in the fleet");
        let npc = session.fleet[index].and_then(|id| session.npc(id));
        assert_eq!(
            npc.map(|npc| (npc.ship, npc.escort.is_some())),
            Some((WARSHIP, true)),
            "placed in the new system"
        );
        let kinds: Vec<FormerKind> = session
            .pilot()
            .escorts()
            .iter()
            .map(FormerKind::of)
            .collect();
        assert_eq!(
            kinds,
            [
                FormerKind::Captured,
                FormerKind::Fighter,
                FormerKind::Captured,
                FormerKind::Hired
            ]
        );
    }
}
