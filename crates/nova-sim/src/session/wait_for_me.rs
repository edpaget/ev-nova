//! The player's fighters' "Wait for me!" (`_HandlePlayer`
//! @0x6b3de-0x6b565): the warning raised as the player's jump is
//! accepted while fighters it launched can't follow it.
//!
//! **When.** On the frame the jump key's checks pass with the player's
//! +0x50 at 0 or below (@0x6b3eb-0x6b3f2): the frame a new jump is
//! accepted, once a jump, as the jump then sets +0x50 to 1.0 to brake
//! (@0x6b6aa) or 2.0 (@0x6b5e9). [`Session::begin_jump`] marks the jump
//! it accepts (not a press during the pre-jump stage), and the next player
//! part takes the mark ([`Session::warn_fighters_left_behind`]): the head
//! of [`Session::tick_traffic`] in the `tick` driver, and
//! [`Session::game_work`] right after the player's move, before the
//! traffic's AI.
//!
//! **The count** (slots 1-63, @0x6b405-0x6b47a): each ship in the system
//! the player leads that is its fighter (AI type 5: a ship the player
//! leads with a [`carrier`](crate::traffic::npc::Npc::carrier)), that
//! can't jump (`_AIShipHasFuelForJump` @0x6b432, [`leave::can_jump`] by
//! [`RuleKey::NpcJumpFuel`]) and whose class lacks `Flags2` 0x0010
//! ([`QUIET_FIGHTER`], @0x6b45c). The last counted, in slot order, gives
//! the class. An escort carrier's fighters are led by the escort, not the
//! player, so none counts.
//!
//! **The line**: the class's name, ":  " (@0x6b4c1), then `STR#` 2002
//! #(160 + `Rand(2)`) for one fighter, #(162 + `Rand(2)`) for more
//! (@0x6b4bf-0x6b4fc), shown for 250 frames whatever it holds
//! ([`SimMessage::WaitForMe`]), and `snd ` 154 at priority 50
//! ([`SimSound::WaitForMe`], @0x6b542-0x6b565).
//!
//! The fighters themselves turn back to dock from the player's brake on
//! (`nova_sim_ai`'s `jump_with_leader` module); those still out on
//! arrival are left as [`RuleKey::FighterRecall`] says.

use super::Session;
use crate::ai::leave;
use crate::chance::Chance;
use crate::message::SimMessage;
use crate::rulebook::RuleKey;
use crate::sound::SimSound;

/// The `STR#` the line's words come from.
const LINES: i16 = 2002;
/// The first of one fighter's two lines, `STR#` 2002 #160 and #161.
const ONE_FIGHTER: usize = 160;
/// The first of several fighters' two lines, `STR#` 2002 #162 and #163.
const FIGHTERS: usize = 162;
/// `shïp` `Flags2` 0x0010: a class whose fighters the warning leaves out
/// (@0x6b45c).
pub const QUIET_FIGHTER: u16 = 0x0010;

impl Session {
    /// The warning, on the first player part after a new jump was
    /// accepted, drawing its line on `chance` (see the module docs).
    pub(super) fn warn_fighters_left_behind(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if !std::mem::take(&mut self.ship.warp.jump_accepted) {
            return;
        }
        let reading = self.setup.rules.source_for(RuleKey::NpcJumpFuel);
        let mut count = 0_u32;
        let mut class = None;
        for npc in self.space.traffic.player_led() {
            if npc.carrier.is_none() || leave::can_jump(npc, reading) {
                continue;
            }
            let record = self.statics.ship_record(npc.ship);
            if record.is_some_and(|record| record.fields.flags2 & QUIET_FIGHTER != 0) {
                continue;
            }
            count += 1;
            class = Some(record.map(|record| record.name.clone()).unwrap_or_default());
        }
        let Some(class) = class else {
            return;
        };
        let first = if count == 1 { ONE_FIGHTER } else { FIGHTERS };
        let pick = chance.below(2) as usize;
        let words = self
            .setup
            .nth_string(LINES, first + pick)
            .unwrap_or_default();
        self.space.say(
            &mut self.out,
            SimMessage::WaitForMe(format!("{class}:  {words}")),
        );
        self.out.sounds.push(SimSound::WaitForMe);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::ai::{Goal, NovaAi};
    use crate::bay::FighterNote;
    use crate::catalog::{CommCatalog, ShipId, ShipRecord};
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::pilot::Escort;
    use crate::session::fighters::tests::{VIPER, VIPER_FIELDS, carrying, fleet, out};
    use crate::testkit::{Draws, FakePilotCatalog, hull, ship};
    use crate::traffic::warp_out::WarpGoal;

    /// A fighter of no fuel.
    const DART: ShipId = ShipId(145);
    /// A fighter of no fuel whose class has `Flags2` 0x0010.
    const QUIET: ShipId = ShipId(146);

    /// `STR#` 2002 #160-163.
    #[derive(Debug)]
    struct Lines;

    impl CommCatalog for Lines {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                LINES => (1..=163)
                    .map(|n| match n {
                        160 => "One A".to_owned(),
                        161 => "One B".to_owned(),
                        162 => "More A".to_owned(),
                        163 => "More B".to_owned(),
                        _ => String::new(),
                    })
                    .collect(),
                _ => Vec::new(),
            }
        }
    }

    /// [`carrying`], with the Dart and the quiet fighter, both of no fuel.
    fn catalog() -> FakePilotCatalog {
        let mut catalog = carrying();
        let empty = crate::handling::ShipFields {
            fuel: 0,
            ..VIPER_FIELDS
        };
        catalog.ship_records.push(ShipRecord {
            escort_type: 0,
            inherent_ai: 4,
            ..ship(145, empty)
        });
        catalog.ship_records.push(ShipRecord {
            escort_type: 0,
            inherent_ai: 4,
            ..ship(
                146,
                crate::handling::ShipFields {
                    flags2: QUIET_FIGHTER,
                    ..empty
                },
            )
        });
        for id in [145, 146] {
            catalog.hulls.push(crate::catalog::HullRecord {
                size: Some(20),
                ..hull(id)
            });
        }
        catalog
    }

    /// A fighter of `ship` out of its bay.
    fn fighter(ship: ShipId) -> Escort {
        Escort { ship, ..out() }
    }

    /// `fighters` flying out of the player's bays, the lines known, out
    /// beyond the no-jump zone and moving, nothing said yet.
    fn flying(catalog: &FakePilotCatalog, fighters: &[ShipId]) -> Session {
        let mut session = fleet(
            catalog,
            0,
            fighters.iter().map(|&ship| fighter(ship)).collect(),
        )
        .with_strings(Rc::new(Lines));
        session
            .plot_course(crate::catalog::SystemId(131))
            .expect("a route");
        crate::testkit::fly_out(&mut session);
        let _ = (session.take_messages(), session.take_sounds());
        session
    }

    /// What the warning says on `session`'s next player part, drawing
    /// `draw`.
    fn warned(session: &mut Session, draw: u32) -> Vec<SimMessage> {
        session.warn_fighters_left_behind(&mut Draws::of(&[draw]));
        session.take_messages()
    }

    fn line(text: &str) -> Vec<SimMessage> {
        vec![SimMessage::WaitForMe(text.to_owned())]
    }

    #[test]
    fn one_fighter_that_cannot_jump_says_160_or_161_by_the_draw_with_snd_154() {
        let catalog = catalog();
        for (draw, text) in [(0, "Ship 145:  One A"), (1, "Ship 145:  One B")] {
            let mut session = flying(&catalog, &[VIPER, DART]);
            session.begin_jump().expect("accepted");
            assert_eq!(warned(&mut session, draw), line(text), "{draw}");
            assert!(session.take_sounds().contains(&SimSound::WaitForMe));
        }
    }

    #[test]
    fn several_say_162_or_163_with_the_last_ones_class() {
        let mut catalog = catalog();
        catalog.ship_records.push(ShipRecord {
            escort_type: 0,
            inherent_ai: 4,
            ..ship(
                147,
                crate::handling::ShipFields {
                    fuel: 0,
                    ..VIPER_FIELDS
                },
            )
        });
        catalog.hulls.push(crate::catalog::HullRecord {
            size: Some(20),
            ..hull(147)
        });
        for (draw, text) in [(0, "Ship 147:  More A"), (1, "Ship 147:  More B")] {
            let mut session = flying(&catalog, &[DART, ShipId(147)]);
            session.begin_jump().expect("accepted");
            assert_eq!(warned(&mut session, draw), line(text), "{draw}");
        }
    }

    #[test]
    fn a_quiet_class_a_fighter_that_can_jump_or_no_new_jump_says_nothing() {
        let catalog = catalog();
        for fighters in [&[QUIET][..], &[VIPER]] {
            let mut session = flying(&catalog, fighters);
            session.begin_jump().expect("accepted");
            assert_eq!(warned(&mut session, 0), [], "{fighters:?}");
            assert!(!session.take_sounds().contains(&SimSound::WaitForMe));
        }
        let mut session = flying(&catalog, &[DART]);
        assert_eq!(warned(&mut session, 0), [], "no jump");
        session.begin_jump().expect("accepted");
        assert_eq!(warned(&mut session, 0).len(), 1);
        session.tick(Controls::default());
        assert!(session.jumping().is_none(), "still braking");
        session.begin_jump().expect("the same jump");
        assert_eq!(warned(&mut session, 0), [], "J again in the brake");
    }

    #[test]
    fn an_escort_carriers_fighter_is_not_the_players() {
        let catalog = catalog();
        let mut session = flying(&catalog, &[]);
        let mut carrier = crate::testkit::npc(0, crate::stats::ShipStats::new(VIPER_FIELDS, &[]));
        carrier.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let carrier = session.space.traffic.add_npc(carrier);
        let mut dart = crate::testkit::npc(0, crate::stats::ShipStats::new(VIPER_FIELDS, &[]));
        dart.ship = DART;
        dart.class_fuel = 0;
        dart.leader = Some(carrier);
        dart.carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Npc(carrier),
            window: 100.0,
            reach: 40.0,
        });
        session.space.traffic.add_npc(dart);
        session.begin_jump().expect("accepted");
        assert_eq!(warned(&mut session, 0), []);
    }

    #[test]
    fn both_drivers_warn_once_a_jump() {
        let catalog = catalog();
        let mut session = flying(&catalog, &[DART]);
        session.begin_jump().expect("accepted");
        for _ in 0..3 {
            session.tick(Controls::default());
            session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        }
        let said = session.take_messages();
        assert_eq!(
            said.iter()
                .filter(|said| matches!(said, SimMessage::WaitForMe(_)))
                .count(),
            1,
            "the tick driver: {said:?}"
        );
        let mut session = flying(&catalog, &[DART]);
        session.begin_jump().expect("accepted");
        for _ in 0..3 {
            crate::session::fighters::tests::game_pass(&mut session, &catalog, &NovaAi::default());
        }
        let said = session.take_messages();
        assert_eq!(
            said.iter()
                .filter(|said| matches!(said, SimMessage::WaitForMe(_)))
                .count(),
            1,
            "the game's work: {said:?}"
        );
    }

    // The fighters on the jump.

    /// The NPC of the fighter of `ship` in `session`.
    fn npc_of(session: &Session, ship: ShipId) -> Option<&crate::traffic::npc::Npc> {
        session.npcs().iter().find(|npc| npc.ship == ship)
    }

    /// Ticks `session` through its jump to arrival.
    fn jump_through(session: &mut Session, catalog: &FakePilotCatalog) {
        session.begin_jump().expect("accepted");
        for _ in 0..2000 {
            session.tick(Controls::default());
            session.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
            session.tick_combat(crate::Rules::default(), &mut NeverFires);
            if session.warp_end_due() {
                break;
            }
        }
        assert!(
            session.arrive(catalog, &mut NeverFires).is_some(),
            "arrived"
        );
    }

    #[test]
    fn a_fighter_short_of_fuel_turns_back_and_docks_if_it_reaches_the_player_in_time() {
        let catalog = catalog();
        let mut session = flying(&catalog, &[DART]);
        let player = session.player().position;
        let id = npc_of(&session, DART).expect("out").id;
        session.space.npc_mut(id).expect("out").state.position = player + Vec2::new(30.0, 0.0);
        jump_through(&mut session, &catalog);
        assert_eq!(session.take_fighter_notes(), [], "docked, none abandoned");
        assert!(session.pilot().escorts().is_empty(), "aboard: none out");
        assert_eq!(npc_of(&session, DART), None);
    }

    #[test]
    fn a_fighter_short_of_fuel_too_far_to_reach_the_player_is_abandoned() {
        let catalog = catalog();
        let mut session = flying(&catalog, &[DART]);
        let player = session.player().position;
        let id = npc_of(&session, DART).expect("out").id;
        session.space.npc_mut(id).expect("out").state.position = player + Vec2::new(20_000.0, 0.0);
        jump_through(&mut session, &catalog);
        assert_eq!(session.take_fighter_notes(), [FighterNote::Abandoned(1)]);
    }

    #[test]
    fn a_players_escort_fighting_at_j_stops_from_the_brake_on() {
        let catalog = catalog();
        let mut session = flying(&catalog, &[VIPER]);
        let raider = session.space.traffic.add_npc(crate::testkit::npc(
            0,
            crate::stats::ShipStats::new(VIPER_FIELDS, &[]),
        ));
        let id = npc_of(&session, VIPER).expect("out").id;
        let viper = session.space.npc_mut(id).expect("out");
        viper.goal = Goal::Attack(ShipRef::Npc(raider));
        viper.target = Some(ShipRef::Npc(raider));
        session.begin_jump().expect("accepted");
        session.tick(Controls::default());
        assert!(session.jumping().is_none(), "braking");
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        let viper = npc_of(&session, VIPER).expect("out");
        assert_eq!(viper.target, None);
        assert_eq!(viper.goal, Goal::Formation { guard: None });
        assert_eq!(viper.warp.goal, Some(WarpGoal::Follow(ShipRef::Player)));
    }
}
