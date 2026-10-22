//! The session's side of [`target_panel`](crate::target_panel): the facts
//! the target panel's status line and label are decided from, gathered
//! from an NPC, its ship type and the missions.
//!
//! A rescue mission's ship held disabled ([`Session::mission_ship_held`])
//! whose armour is not below its threshold, by the session's
//! [`DisableRule`](crate::DisableRule), shows "Waiting": the original's
//! test (@0x4bab1-0x4bb79) is `_IsDisabled`'s hold and the negation of
//! its armour test.

use super::Session;
use crate::bay::Carrier;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::reserves::shield_points;
use crate::target_panel::{self, StatusFacts, TagFacts, TargetStatus, TargetTag};
use crate::traffic::npc::Npc;

impl Session {
    /// What the target panel's status line shows of `npc` (see
    /// [`target_panel::status`]).
    #[must_use]
    pub fn target_status(&self, npc: &Npc) -> TargetStatus {
        let record = self.statics.ship_record(npc.ship);
        target_panel::status(&StatusFacts {
            reserves: npc.reserves,
            disabled: npc.condition == Condition::Disabled,
            held: self.mission_ship_held(npc),
            below_threshold: self.disable_rule().disabled(npc.reserves.armor, &npc.hull),
            shows_armor: npc.hull.shows_armor,
            hides_status: npc.hull.hides_status,
            person_hides: npc.person.is_some_and(|person| person.invincible),
            type_shield: record.map_or(0, |record| shield_points(record.fields.shield)),
        })
    }

    /// What the label against the target panel's bottom right shows of
    /// `npc`, if anything (see [`target_panel::tag`]): the player leads an
    /// escort hired or captured, a fighter the player launched, and a
    /// mission escort; a fighter the player launched is one whose carrier
    /// is the player.
    #[must_use]
    pub fn target_tag(&self, npc: &Npc) -> Option<TargetTag> {
        target_panel::tag(&TagFacts {
            govt: npc.govt,
            led_by_player: npc.escorts_player(),
            players_fighter: matches!(
                npc.carrier,
                Some(Carrier {
                    ship: ShipRef::Player,
                    ..
                })
            ),
            type_mass: self
                .statics
                .ship_record(npc.ship)
                .map_or(0, |record| record.fields.mass),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtId;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::mission::ships::ShipBehav;
    use crate::session::mission_ships::tests::{goal_catalog, in_flight};
    use crate::testkit::{FAST, ship};
    use crate::traffic::npc::{MissionMark, NpcPerson};
    use crate::{PersonId, ShipFields};

    /// Mission 140's two rescue ships, set up held and disabled.
    fn rescue() -> Session {
        in_flight(&goal_catalog(5, 0))
    }

    fn statuses(session: &Session) -> Vec<TargetStatus> {
        session
            .npcs()
            .iter()
            .map(|npc| session.target_status(npc))
            .collect()
    }

    /// Sets every NPC's armour to `fraction` of what it holds.
    fn armour(session: &mut Session, fraction: f32) {
        for npc in session.space.traffic.npcs_mut() {
            npc.reserves.armor.now = npc.reserves.armor.max * fraction;
        }
    }

    #[test]
    fn a_held_rescue_ship_at_full_armour_is_waiting() {
        let mut session = rescue();
        armour(&mut session, 1.0);
        assert_eq!(statuses(&session), [TargetStatus::Waiting; 2]);
    }

    #[test]
    fn a_held_rescue_ship_below_its_threshold_is_disabled() {
        let mut session = rescue();
        armour(&mut session, 0.5);
        assert_eq!(statuses(&session), [TargetStatus::Waiting; 2]);
        armour(&mut session, 0.3);
        assert_eq!(statuses(&session), [TargetStatus::Disabled; 2]);
    }

    #[test]
    fn a_tough_held_ship_waits_down_to_a_tenth_of_its_armour() {
        let mut session = rescue();
        for npc in session.space.traffic.npcs_mut() {
            npc.hull.tough = true;
            npc.reserves.armor.max = 1000.0;
        }
        armour(&mut session, 0.1);
        assert_eq!(statuses(&session), [TargetStatus::Waiting; 2]);
        armour(&mut session, 0.099);
        assert_eq!(statuses(&session), [TargetStatus::Disabled; 2]);
    }

    #[test]
    fn a_boarded_rescue_ship_or_one_whose_goals_are_met_is_disabled() {
        let mut session = rescue();
        armour(&mut session, 1.0);
        session.space.traffic.npcs_mut()[0].boarded = true;
        assert_eq!(
            statuses(&session),
            [TargetStatus::Disabled, TargetStatus::Waiting]
        );
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.goals_met = true;
        }
        assert_eq!(statuses(&session), [TargetStatus::Disabled; 2]);
    }

    #[test]
    fn a_ship_of_another_goal_shows_its_shield() {
        let session = in_flight(&goal_catalog(2, 0));
        assert_eq!(statuses(&session), [TargetStatus::Shield(100); 2]);
    }

    /// Two intact NPCs, shields down, of ship type 300, of `fields`.
    fn typed(fields: ShipFields) -> Session {
        let mut catalog = goal_catalog(2, 0);
        catalog.ship_records.push(ship(300, fields));
        let mut session = in_flight(&catalog);
        for npc in session.space.traffic.npcs_mut() {
            npc.ship = crate::ShipId(300);
            npc.reserves.shield.now = 0.0;
            npc.condition = Condition::Intact;
        }
        session
    }

    #[test]
    fn shields_down_or_no_shields_by_the_types_shield_points() {
        for (shield, expected) in [
            (30, TargetStatus::ShieldsDown),
            (-1, TargetStatus::ShieldsDown),
            (0, TargetStatus::NoShields),
        ] {
            let session = typed(ShipFields { shield, ..FAST });
            assert_eq!(statuses(&session), [expected; 2], "{shield}");
        }
    }

    #[test]
    fn the_hulls_flags_reach_the_status_line() {
        let mut session = typed(FAST);
        session.space.traffic.npcs_mut()[0].hull.shows_armor = true;
        session.space.traffic.npcs_mut()[1].hull.hides_status = true;
        let armour = session.npcs()[0].reserves.armor;
        assert!(armour.now >= armour.max, "{armour:?}");
        assert_eq!(
            statuses(&session),
            [TargetStatus::Armor(100), TargetStatus::Hidden]
        );
    }

    fn person(invincible: bool) -> NpcPerson {
        NpcPerson {
            id: PersonId(600),
            flags: 0,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible,
            grudge: false,
            quoted: false,
            quoted_at: None,
            zero_fuel: false,
        }
    }

    #[test]
    fn an_invincible_persons_status_line_is_hidden() {
        let mut session = typed(FAST);
        session.space.traffic.npcs_mut()[0].person = Some(person(true));
        session.space.traffic.npcs_mut()[1].person = Some(person(false));
        assert_eq!(
            statuses(&session),
            [TargetStatus::Hidden, TargetStatus::ShieldsDown]
        );
    }

    fn tags(session: &Session) -> Vec<Option<TargetTag>> {
        session
            .npcs()
            .iter()
            .map(|npc| session.target_tag(npc))
            .collect()
    }

    fn duty() -> EscortDuty {
        EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        }
    }

    #[test]
    fn a_ship_with_a_government_shows_its_code_and_one_without_nothing() {
        let mut session = typed(FAST);
        session.space.traffic.npcs_mut()[0].govt = Some(GovtId(128));
        session.space.traffic.npcs_mut()[1].govt = None;
        assert_eq!(tags(&session), [Some(TargetTag::Code(GovtId(128))), None]);
    }

    #[test]
    fn the_players_escort_and_mission_escort_with_no_government_are_escorts() {
        let mut session = typed(FAST);
        let npcs = session.space.traffic.npcs_mut();
        npcs[0].govt = None;
        npcs[0].escort = Some(duty());
        npcs[1].govt = None;
        npcs[1].mission = Some(MissionMark {
            slot: 0,
            behav: ShipBehav::EscortPlayer,
            formation: Some(FormationSlot::default()),
            order: None,
        });
        assert_eq!(tags(&session), [Some(TargetTag::Escort); 2]);
    }

    fn carried_by(ship: ShipRef) -> Carrier {
        Carrier {
            ship,
            window: 0.0,
            reach: 0.0,
        }
    }

    #[test]
    fn the_players_fighter_of_99_tons_is_a_fighter_and_of_100_an_escort() {
        for (mass, expected) in [(99, TargetTag::Fighter), (100, TargetTag::Escort)] {
            let mut session = typed(ShipFields { mass, ..FAST });
            for npc in session.space.traffic.npcs_mut() {
                npc.govt = None;
                npc.escort = Some(duty());
                npc.carrier = Some(carried_by(ShipRef::Player));
            }
            assert_eq!(tags(&session), [Some(expected); 2], "{mass}");
        }
    }

    #[test]
    fn an_escorts_own_fighter_is_not_the_players() {
        let mut session = typed(ShipFields { mass: 10, ..FAST });
        let escort = session.npcs()[0].id;
        let npcs = session.space.traffic.npcs_mut();
        npcs[0].govt = None;
        npcs[0].escort = Some(duty());
        npcs[1].govt = None;
        npcs[1].carrier = Some(carried_by(ShipRef::Npc(escort)));
        assert_eq!(tags(&session), [Some(TargetTag::Escort), None]);
    }
}
