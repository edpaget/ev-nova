//! Which hits the player answers for: its own, and those of the ships it
//! leads directly ([`Space::players_hit`]). Both the conviction
//! (`Session::punish`) and the kill credit ([`Space::credit_kills`]) of a
//! phase's strikes read it, as `_DamageShip`'s crime and kill block
//! (@0x3ad4f-0x3ae53) gates both on one test of the shooter.
//!
//! **The gate** (@0x3ad4f-0x3ad7b): the hit's source slot is the
//! player's (0), or a ship's (1-63) whose leader +0x9a is the player (0):
//! a hired or captured escort, a fighter the player launched, or a
//! mission escort, the engine's [`Npc::leader_ref`] being the player. It
//! reads +0x9a once, as the hit lands, and walks no further: a fighter
//! launched by the player's escort is led by its carrier
//! (`_LaunchFighter` @0x3d866-0x3d870), so its hits are no crime of the
//! player's and no kill of its own, though `_ShipsShareParent`'s root is
//! the player. Every kind of hit passes its shooter's slot (shots,
//! blasts, beams, missile locks and the death shockwave), as each
//! [`Strike::by`] carries it.
//!
//! **An escort lost to a hit** keeps counting up to and including the
//! strike that loses it: the escort block clears its +0x9a (@0x3afdd)
//! after the crime block in the same `_DamageShip` call. The engine has
//! already taken it out of the fleet (`Session::lose_escorts`) when the
//! phase's strikes settle, so it reads the [`EscortLoss`] the fight
//! recorded.
//!
//! **Left out**: the target gate's +0x8c == -1 (@0x3ad81), which spares a
//! stellar's defence ship, waits on stellar defence fleets
//! (rdm:task/planet-domination); its +0xc8d0 <= 0x3fe (@0x3ad8f), which
//! spares the shareware nag ship, can never apply here.
//!
//! [`Npc::leader_ref`]: crate::traffic::npc::Npc::leader_ref

use super::Space;
use crate::combat::{EscortLoss, ShipRef, Strike};

impl Space {
    /// Whether the player answers for `strike`, the `index`th of a
    /// phase's strikes, as its own hit (`_DamageShip` @0x3ad4f-0x3ad7b):
    /// the player struck, or a ship it leads directly
    /// ([`Npc::leader_ref`]) as the hit landed. A shooter among `losses`
    /// lost by this strike or a later one was still the player's then; one
    /// lost before it, or no longer here, is not. A strike by no ship is
    /// no one's.
    ///
    /// [`Npc::leader_ref`]: crate::traffic::npc::Npc::leader_ref
    pub(super) fn players_hit(&self, index: usize, strike: &Strike, losses: &[EscortLoss]) -> bool {
        match strike.by {
            None => false,
            Some(ShipRef::Player) => true,
            Some(ShipRef::Npc(by)) => {
                let lost_since = losses
                    .iter()
                    .any(|loss| loss.ship == ShipRef::Npc(by) && loss.strike >= index);
                lost_since
                    || self
                        .traffic
                        .npcs()
                        .iter()
                        .any(|npc| npc.id == by && npc.leader_ref() == Some(ShipRef::Player))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::ai::NovaAi;
    use crate::catalog::{GovtId, ShipId, WeaponId};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::combat::{Downed, EscortLoss, Rules, ShipRef, Sortie, Strike};
    use crate::escort::FormationSlot;
    use crate::flight::Controls;
    use crate::legal::Crime;
    use crate::mission::ships::ShipBehav;
    use crate::pilot::Escort;
    use crate::session::escort_loss::an_escorts_shot_into_the_bystander;
    use crate::session::tests::{Witness, alongside, boardable, lawful};
    use crate::session::{NpcBefore, Session};
    use crate::traffic::npc::{MissionMark, NpcId};

    /// A row of strikes to settle: its name, and what sets it up in a
    /// session, giving the strikes and the escorts they lost.
    type Row<'a> = (
        &'a str,
        &'a dyn Fn(&mut Session) -> (Vec<Strike>, Vec<EscortLoss>),
    );

    /// A strike on `ship` by `by` that leaves it `downed`.
    fn strike(ship: NpcId, by: ShipRef, downed: Downed) -> Strike {
        Strike {
            ship: ShipRef::Npc(ship),
            by: Some(by),
            damage: 9.0,
            downed: Some(downed),
            direct: true,
            intended: true,
            lock: false,
            nominal: crate::combat::Nominal::default(),
            weight: 0.0,
            answered: true,
        }
    }

    /// NPC `id` as it was before the tick: `condition`, of `govt`,
    /// unmarked.
    fn was(id: NpcId, condition: Condition, govt: i16) -> NpcBefore {
        NpcBefore {
            id,
            condition,
            govt: Some(GovtId(govt)),
            marked: false,
        }
    }

    /// Enlists [`alongside`]'s police, NPC 1, as the player's escort.
    fn enlist_the_police(session: &mut Session) {
        let police = &session.npcs()[1];
        let record = Escort {
            ship: police.ship,
            reserves: police.reserves,
            order: None,
            carried: false,
            wage: None,
            person: None,
            mark: None,
        };
        session.enlist(NpcId(1), record);
        assert_eq!(session.npcs()[1].leader_ref(), Some(ShipRef::Player));
    }

    /// Takes the escort NPC 1 out of the fleet as a hit that disabled it
    /// does: disabled, then lost by `Session::lose_escorts`.
    fn lose_the_escort(session: &mut Session) {
        session.space.npc_mut(NpcId(1)).expect("there").condition = Condition::Disabled;
        session.lose_escorts(
            &[was(NpcId(1), Condition::Intact, 141)],
            &BTreeSet::from([NpcId(1)]),
        );
        assert_ne!(session.npcs()[1].leader_ref(), Some(ShipRef::Player));
    }

    /// The crimes `strikes` convicted the player of, as they settle with
    /// `losses` against `was`, and the combat rating they left.
    fn settled(
        session: &mut Session,
        strikes: Vec<Strike>,
        losses: Vec<EscortLoss>,
        was: &[NpcBefore],
    ) -> (Vec<(Crime, Option<GovtId>)>, i64) {
        let witness = Witness::default();
        session.settle_strikes((strikes, losses), was, &witness, &mut NeverFires);
        (witness.seen.take(), session.pilot().combat_rating())
    }

    #[test]
    fn a_hired_escorts_shot_that_disables_a_lawful_ship_convicts_the_player_of_disabling_it() {
        // 45 armour, disabled below 33%, 14.85; the blaster takes 5.
        let catalog = lawful();
        let mut session = an_escorts_shot_into_the_bystander(&catalog, 16.0);
        let govt = session.npcs()[1].govt;
        assert!(govt.is_some());
        let witness = Witness::default();
        session.game_work(
            Controls::default(),
            &catalog,
            &NovaAi::default(),
            Rules {
                law: &witness,
                ..Rules::default()
            },
            &mut NeverFires,
        );
        assert_eq!(
            session.space.npc(NpcId(1)).expect("there").condition,
            Condition::Disabled
        );
        assert_eq!(witness.seen.take(), [(Crime::Disable, govt)]);
        assert_eq!(session.pilot().combat_rating(), 0, "no kill");
    }

    #[test]
    fn a_fighter_the_player_launched_that_kills_convicts_the_player_and_credits_the_kill() {
        let catalog = boardable();
        for (condition, crimes) in [
            (Condition::Disabled, vec![Crime::Kill]),
            (Condition::Intact, vec![Crime::Disable, Crime::Kill]),
        ] {
            let mut session = alongside(&catalog);
            let fighter = session.launch_fighters(
                &[Sortie {
                    carrier: ShipRef::Player,
                    bay: WeaponId(149),
                    ship: ShipId(130),
                    from: *session.player(),
                    heading: 0.0,
                    target: None,
                }],
                &mut NeverFires,
            )[0];
            assert_eq!(
                session.space.npc(fighter).expect("there").leader_ref(),
                Some(ShipRef::Player)
            );
            let (seen, rating) = settled(
                &mut session,
                vec![strike(NpcId(0), ShipRef::Npc(fighter), Downed::BreakingUp)],
                vec![],
                &[was(NpcId(0), condition, 140)],
            );
            let crimes: Vec<_> = crimes
                .into_iter()
                .map(|crime| (crime, Some(GovtId(140))))
                .collect();
            assert_eq!(seen, crimes, "{condition:?}");
            assert_eq!(rating, 2, "10 / 5, {condition:?}");
        }
    }

    #[test]
    fn a_mission_escorts_hit_that_disables_a_ship_convicts_the_player() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.space.npc_mut(NpcId(1)).expect("there").mission = Some(MissionMark {
            slot: 0,
            behav: ShipBehav::EscortPlayer,
            formation: Some(FormationSlot::default()),
            order: None,
        });
        assert!(session.npcs()[1].mission_escort());
        let (seen, _) = settled(
            &mut session,
            vec![strike(NpcId(0), ShipRef::Npc(NpcId(1)), Downed::Disabled)],
            vec![],
            &[was(NpcId(0), Condition::Intact, 140)],
        );
        assert_eq!(seen, [(Crime::Disable, Some(GovtId(140)))]);
    }

    #[test]
    fn an_escort_convicts_the_player_up_to_the_strike_that_loses_it() {
        // `_DamageShip`'s crime block reads the shooter's +0x9a (@0x3ad72)
        // before its escort block clears it (@0x3afdd).
        let catalog = boardable();
        let seen_when_lost_by = |strike_index: usize| {
            let mut session = alongside(&catalog);
            let mut b = session.npcs()[0].clone();
            b.state.position.x += 3000.0;
            let b = session.space.traffic.add_npc(b);
            enlist_the_police(&mut session);
            lose_the_escort(&mut session);
            let escort = ShipRef::Npc(NpcId(1));
            settled(
                &mut session,
                vec![
                    strike(NpcId(0), escort, Downed::Disabled),
                    strike(b, escort, Downed::Disabled),
                ],
                vec![EscortLoss {
                    ship: escort,
                    strike: strike_index,
                }],
                &[
                    was(NpcId(0), Condition::Intact, 140),
                    was(b, Condition::Intact, 142),
                ],
            )
            .0
        };
        assert_eq!(
            seen_when_lost_by(0),
            [(Crime::Disable, Some(GovtId(140)))],
            "lost by the first"
        );
        assert_eq!(
            seen_when_lost_by(1),
            [
                (Crime::Disable, Some(GovtId(140))),
                (Crime::Disable, Some(GovtId(142)))
            ],
            "lost by the second"
        );
    }

    #[test]
    fn a_strike_by_a_ship_the_player_does_not_lead_convicts_no_one_and_credits_nothing() {
        let catalog = boardable();
        let kill = |by| strike(NpcId(0), by, Downed::BreakingUp);
        let traffic = |session: &mut Session| {
            assert_eq!(session.npcs()[1].leader_ref(), None);
            (vec![kill(ShipRef::Npc(NpcId(1)))], vec![])
        };
        let escorts_fighter = |session: &mut Session| {
            enlist_the_police(session);
            let from = session.npcs()[1].state;
            let fighter = session
                .launch_fighter(
                    &Sortie {
                        carrier: ShipRef::Npc(NpcId(1)),
                        bay: WeaponId(149),
                        ship: ShipId(130),
                        from,
                        heading: 0.0,
                        target: None,
                    },
                    &mut NeverFires,
                )
                .expect("launched");
            assert_eq!(
                session.space.npc(fighter).expect("there").leader_ref(),
                Some(ShipRef::Npc(NpcId(1))),
                "led by its carrier"
            );
            (vec![kill(ShipRef::Npc(fighter))], vec![])
        };
        let lost_before = |session: &mut Session| {
            enlist_the_police(session);
            lose_the_escort(session);
            let mut loss = kill(ShipRef::Player);
            loss.ship = ShipRef::Npc(NpcId(1));
            loss.by = None;
            loss.downed = Some(Downed::Disabled);
            (
                vec![loss, kill(ShipRef::Npc(NpcId(1)))],
                vec![EscortLoss {
                    ship: ShipRef::Npc(NpcId(1)),
                    strike: 0,
                }],
            )
        };
        let rows: [Row; 3] = [
            ("a traffic ship", &traffic),
            ("a fighter of the player's escort", &escorts_fighter),
            ("an escort lost before the strike", &lost_before),
        ];
        for (name, row) in rows {
            let mut session = alongside(&catalog);
            let (strikes, losses) = row(&mut session);
            let (seen, rating) = settled(
                &mut session,
                strikes,
                losses,
                &[
                    was(NpcId(0), Condition::Intact, 140),
                    was(NpcId(1), Condition::Intact, 141),
                ],
            );
            assert_eq!(seen, [], "{name}");
            assert_eq!(rating, 0, "{name}");
        }
    }
}
