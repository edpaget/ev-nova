//! The fight's test of point defence's missile branch reading the
//! defender's parent (`_HandleShipPointDefense` @0x39401-0x39426): +0x9a
//! read with no test of how its ship stands.

use super::*;

/// The root of the defender's chain of leads, not in the fight.
const ROOT: ShipRef = ShipRef::Npc(NpcId(4));

#[test]
fn point_defence_engages_a_missile_at_a_parent_that_is_breaking_up() {
    // B's missile at A; C, led by A and of ROOT's fleet, defends it. A has
    // the Quad too, but breaks up, so only C can answer.
    let [lead, firer] = missile_attack(quad());
    let mut wing = Ship::at(3, -30.0, 0.0).armed(quad()).facing(0.0);
    wing.trigger = QUIET;
    wing.fleet = ROOT;
    wing.leader = Some(A);
    let mut ships = [lead, firer, wing];
    let mut combat = Combat::default();
    missile_in_flight(&mut combat, &mut ships);
    combat.take_events();
    ships[0].condition = Condition::Dying {
        ticks_left: 10,
        starting: false,
    };
    phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
    let events = combat.take_events();
    let by_c = |event: &CombatEvent| matches!(event, CombatEvent::Fired { ship, .. } if *ship == C);
    assert!(events.iter().any(by_c), "{events:?}");
}

/// Nova's rule, recording the defenders it is asked about.
#[derive(Debug, Default)]
struct Asked {
    defenders: RefCell<Vec<Side>>,
}

impl PointDefenceRule for Asked {
    fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool {
        self.defenders.borrow_mut().push(defender);
        defence::Allegiance.hostile(defender, firer, govts)
    }
}

#[test]
fn allegiance_reads_the_root_of_a_fighter_led_below_it() {
    // A missile at A, fired by a ship of `fleet`; only C, led by A and of
    // ROOT's fleet, defends it, by the rule. Returns whether C answered,
    // and who the rule was asked about.
    let fought = |fleet: ShipRef| {
        let [mut lead, mut firer] = missile_attack(quad());
        lead.trigger = Trigger::default();
        firer.fleet = fleet;
        let mut wing = Ship::at(3, -30.0, 0.0).armed(quad()).facing(0.0);
        wing.trigger = QUIET;
        wing.fleet = ROOT;
        wing.leader = Some(A);
        let mut ships = [lead, firer, wing];
        let mut combat = Combat::default();
        combat.set_pd_hostility(RuleSource::Bible);
        missile_in_flight(&mut combat, &mut ships);
        combat.take_events();
        let rule = Asked::default();
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        phase::run_by(
            &mut combat,
            &mut ships,
            rules,
            |combat, fighters, arena, chance| {
                combat.defend_npcs(fighters, arena, None, None, chance);
            },
        );
        let answered = combat
            .take_events()
            .iter()
            .any(|event| matches!(event, CombatEvent::Fired { ship, .. } if *ship == C));
        (answered, rule.defenders.take())
    };
    let defender = Side {
        ship: C,
        fleet: ROOT,
        govt: None,
    };
    assert_eq!(fought(ROOT), (false, vec![defender]), "a fleet-mate's");
    assert_eq!(fought(B), (true, vec![defender]), "another fleet's");
    assert_eq!(fought(A), (true, vec![defender]), "its parent's own fleet");
}
