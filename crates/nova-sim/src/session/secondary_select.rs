//! `_HandlePlayer`'s secondary-weapon block (@0x6abcc-0x6ae20): the
//! let-go of a selection that runs out, the Weapon Select key and the
//! Weapon Safety key. Which secondary the select key picks, and which
//! selection running out lets go of, are
//! [`select_secondary`] and [`empty_lets_go`]'s.
//!
//! The view presses Weapon Select, `weapSelect` (W by default), with
//! [`Session::press_weapon_select`], `backwards` with Option down, while
//! the player's fate takes
//! [`PlayerCommand::SelectWeapon`](crate::PlayerCommand::SelectWeapon); it
//! holds Weapon Safety, `weapSafe` (S by default, keycode 0x01 in
//! `_loadKeys` @0xcdde9-0xcde0c), with [`Session::hold_weapon_safety`]
//! while it takes
//! [`PlayerCommand::DeselectWeapon`](crate::PlayerCommand::DeselectWeapon).
//!
//! Each pass, after the player's fire ([`Combat::fire_player`]), in this
//! order:
//!
//! 1. **The let-go** (@0x6abcc-0x6ac0e). With a secondary selected (+0x72
//!    not -1, @0x6abd7) that [`empty_lets_go`] by
//!    [`RuleKey::EmptySecondary`], and for which `_WeaponHasAmmo` fails
//!    ([`Armament::has_ammo`] for the player, cloaked or not, with its
//!    class's `KeyCarried`), the selection becomes none (@0x6ac08). It is
//!    silent. It runs whether the fire key is held or not. A selection the
//!    ship no longer carries ([`RuleKey::StaleSecondary`]) is checked on
//!    its record, as the original reads the weapon table by the slot;
//!    with no record nothing happens.
//! 2. **Weapon Select** (@0x6ac12-0x6adbf). A press waiting picks the next
//!    secondary ([`select_secondary`]) and raises
//!    [`SimSound::SecondarySelected`] (`_beepSnd[2]`, `snd ` 152, priority
//!    1, @0x6acce-0x6acf1), a re-pick of the only candidate too; with no
//!    candidate it raises [`SimSound::Refused`] (`_beepSnd[3]`, `snd `
//!    153, priority 1, @0x6ad90-0x6adb3) and leaves the selection as it
//!    is. The original's latch (`*0xdee54`, @0x6ac32-0x6ac45, cleared with
//!    the key up @0x6adba) keeps a held W to one step a press, which the
//!    press already is: double time's second pass runs the block again,
//!    and finds no press.
//! 3. **Weapon Safety** (@0x6adc2-0x6ae1b). Every pass the key is held
//!    with a secondary selected (@0x6adda), the selection becomes none
//!    (@0x6adee) and [`SimSound::SecondarySelected`] is raised
//!    (@0x6adf8-0x6ae1b). It has no latch. With nothing selected it does
//!    nothing. So W pressed with S held selects and lets go in one pass,
//!    beeping twice.
//!
//! The block runs on every `_HandlePlayer` pass but its early returns:
//! destroyed (@0x68853, @0x6888c) and the escape pod's countdown
//! (@0x68ddd), when a press waiting is dropped, as the original never
//! polls it. So it runs while disabled, while breaking up, and through a
//! jump's pre-jump and warp: the jump state +0x50 gates only the fire
//! keys (@0x6aaf0, @0x6ab75) and the disabled flag only the fire. Landed,
//! no pass runs. It runs on both passes of a double-time frame, as
//! `_HandlePlayer` does ([`Session::player_fires`] on either).
//!
//! The original runs the block before the reloads' countdown (@0x6ae25),
//! which the engine counts inside the fire: they share no state, so the
//! order is never seen.
//!
//! The secondary fire key fires the selection as each step finds it
//! ([`PlayerShip::held_trigger`]), as the original reads +0x72 as it fires
//! (@0x6ab9b): a let-go in one pass fires nothing on later steps of the
//! same frame, double time's second pass included.
//!
//! The status bar's redraw flag (`*0x3b506c`+3, @0x6ac0e, @0x6acca,
//! @0x6adf4) is left out: the engine draws the status bar every frame.
//!
//! [`Combat::fire_player`]: crate::combat::Combat::fire_player
//! [`Armament::has_ammo`]: crate::combat::armament::Armament::has_ammo
//! [`RuleKey::EmptySecondary`]: crate::rulebook::RuleKey::EmptySecondary
//! [`RuleKey::StaleSecondary`]: crate::rulebook::RuleKey::StaleSecondary

use super::{Outbox, PlayerShip, Session, Setup, Statics};
use crate::catalog::WeaponId;
use crate::combat::armament::{
    Mount, OutfitRounds, Shooter, Trigger, empty_lets_go, select_secondary,
};
use crate::combat::hull::Condition;
use crate::pilot::Pilot;
use crate::rulebook::RuleKey;
use crate::sound::SimSound;

/// The player's secondary-weapon keys as the block reads them: unsaved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct WeaponKeys {
    /// Whether the secondary fire key is held, which fires the secondary
    /// selected as each step finds it ([`PlayerShip::held_trigger`]).
    pub(super) fire_secondary: bool,
    /// A Weapon Select press waiting for the next pass's block,
    /// `Some(backwards)`.
    pub(super) select_press: Option<bool>,
    /// Whether the Weapon Safety key is held.
    pub(super) safety_held: bool,
}

impl Session {
    /// Presses Weapon Select, `backwards` with Option down: the next
    /// pass's secondary block takes it (see the module docs). Option is
    /// read at the press, where the original reads it at the pass
    /// (`QuickKeyCheck(0x3a)` @0x6acf6): the press is taken on the next
    /// pass, before any frame is drawn, so the difference is never seen.
    pub(crate) fn press_weapon_select(&mut self, backwards: bool) {
        self.ship.weapon_keys.select_press = Some(backwards);
    }

    /// Holds the Weapon Safety key down or lets it go: what each pass's
    /// secondary block reads (see the module docs).
    pub(crate) fn hold_weapon_safety(&mut self, held: bool) {
        self.ship.weapon_keys.safety_held = held;
    }

    /// Selects the next secondary weapon, or the one before it when
    /// `backwards`, at once, with its sound, as a Weapon Select press does
    /// on the next pass: for the tests, as no production code calls it.
    #[cfg(test)]
    pub(crate) fn select_secondary(&mut self, backwards: bool) {
        self.ship
            .select_secondary(&mut self.pilot, &self.statics, &mut self.out, backwards);
    }

    /// The pass's secondary block, unless the player is destroyed or in
    /// the escape pod, when a press waiting is dropped (see the module
    /// docs).
    pub(super) fn secondary_block(&mut self) {
        if self.pod.is_none() && self.ship.condition != Condition::Destroyed {
            self.ship
                .secondary_block(&mut self.pilot, &self.statics, &self.setup, &mut self.out);
        } else {
            self.ship.weapon_keys.select_press = None;
        }
    }
}

impl PlayerShip {
    /// Holds `trigger`, the player's fire command, until another is held:
    /// its secondary as it is, not the selection.
    pub(super) fn hold_trigger(&mut self, trigger: Trigger) {
        self.trigger = trigger;
        self.weapon_keys.fire_secondary = false;
    }

    /// Holds the player's fire keys: the `primary` trigger, and the
    /// `secondary` one on the secondary weapon selected as each step finds
    /// it ([`PlayerShip::held_trigger`]).
    pub(super) fn hold_fire(&mut self, primary: bool, secondary: bool) {
        self.trigger = Trigger {
            primary,
            ..Trigger::default()
        };
        self.weapon_keys.fire_secondary = secondary;
    }

    /// The fire command the player holds as a step fires it: with the
    /// secondary fire key held, on the selection now.
    pub(super) fn held_trigger(&self) -> Trigger {
        if self.weapon_keys.fire_secondary {
            Trigger {
                secondary: self.secondary,
                ..self.trigger
            }
        } else {
            self.trigger
        }
    }

    /// Selects the next secondary weapon, `backwards` or not, with its
    /// sound, the pilot's rounds and fuel paying (see the module docs).
    fn select_secondary(
        &mut self,
        pilot: &mut Pilot,
        statics: &Statics,
        out: &mut Outbox,
        backwards: bool,
    ) {
        let fuel = pilot.reserves.fuel;
        let rounds = OutfitRounds {
            owned: &mut pilot.outfits,
            sources: &statics.ammo_outfits,
        };
        let shooter = self.shooter();
        let armament = &self.fit.armament;
        let picked = select_secondary(armament.mounts(), self.secondary, backwards, |mount| {
            armament.has_ammo(mount, &rounds, fuel, shooter)
        });
        match picked {
            Some(weapon) => {
                self.secondary = Some(weapon);
                out.sounds.push(SimSound::SecondarySelected);
            }
            None => out.sounds.push(SimSound::Refused),
        }
    }

    /// The player as `_WeaponHasAmmo` asks of it: no NPC, cloaked or not,
    /// with its class's `KeyCarried`.
    fn shooter(&self) -> Shooter {
        Shooter {
            npc: false,
            cloaked: self.cloak.cloak.is_cloaked(),
            key_carried: self.fit.hull.key_carried,
            folds: None,
        }
    }

    /// The selection as the let-go reads it: the mount carrying it, or a
    /// mount of none on its record when the ship no longer carries it;
    /// none with nothing selected or no record.
    fn selected_mount(&self, statics: &Statics, selected: WeaponId) -> Option<Mount> {
        let carried = self
            .fit
            .armament
            .mounts()
            .iter()
            .find(|mount| mount.spec.id == selected);
        carried.copied().or_else(|| {
            statics.arsenal.weapon(selected).map(|spec| Mount {
                spec: *spec,
                count: 0,
                reload: 0.0,
                burst: 0,
            })
        })
    }

    /// The secondary block's three parts (see the module docs).
    fn secondary_block(
        &mut self,
        pilot: &mut Pilot,
        statics: &Statics,
        setup: &Setup,
        out: &mut Outbox,
    ) {
        let source = setup.rules.source_for(RuleKey::EmptySecondary);
        if let Some(mount) = self
            .secondary
            .and_then(|selected| self.selected_mount(statics, selected))
        {
            let fuel = pilot.reserves.fuel;
            let rounds = OutfitRounds {
                owned: &mut pilot.outfits,
                sources: &statics.ammo_outfits,
            };
            if empty_lets_go(&mount.spec, source)
                && !self
                    .fit
                    .armament
                    .has_ammo(&mount, &rounds, fuel, self.shooter())
            {
                self.secondary = None;
            }
        }
        if let Some(backwards) = self.weapon_keys.select_press.take() {
            self.select_secondary(pilot, statics, out, backwards);
        }
        if self.weapon_keys.safety_held && self.secondary.is_some() {
            self.secondary = None;
            out.sounds.push(SimSound::SecondarySelected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, ShipId, SystemId, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::cloak::{Cloak, FADE_FULL};
    use crate::combat::Rules;
    use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use crate::combat::weapon::{
        AI_WONT_USE, FIRES_CLOAKED, HIDE_WHEN_EMPTY, NEEDS_KEY_SHIP, PERSISTENT_SMOKE, SECONDARY,
    };
    use crate::flight::Controls;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::tests::{MISSILE, ROCKET, TORCH, fired_by_the_player, with_secondaries};
    use crate::session::{Jump, Mode};
    use crate::testkit::work;
    use crate::testkit::{FakePilotCatalog, catalog};
    use nova_sim_catalog::fixture::outfit;

    /// The rockets' ammunition outfit, which brings 3 rounds by default.
    const ROCKETS: OutfitId = OutfitId(310);

    /// [`with_secondaries`], the rockets ([`ROCKET`]) of `flags` and
    /// `flags2` past their own.
    fn rockets(flags: u16, flags2: u16) -> FakePilotCatalog {
        let mut catalog = with_secondaries(catalog());
        catalog.weapons[1].flags |= flags;
        catalog.weapons[1].flags2 |= flags2;
        catalog
    }

    /// `catalog`'s session by `source`'s reading of
    /// [`RuleKey::EmptySecondary`], the rockets selected, holding `rounds`
    /// of them.
    fn armed(catalog: &FakePilotCatalog, source: RuleSource, rounds: u16) -> Session {
        let rules = Rulebook::default().with_override(RuleKey::EmptySecondary, source);
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        session.select_secondary(false);
        assert_eq!(session.secondary(), Some(ROCKET));
        set_rounds(&mut session, rounds);
        session.take_sounds();
        session
    }

    /// The pilot holding `rounds` rockets.
    fn set_rounds(session: &mut Session, rounds: u16) {
        session.pilot.outfits.remove(&ROCKETS);
        if rounds > 0 {
            session.pilot.outfits.insert(ROCKETS, rounds);
        }
    }

    /// A fight tick: what the player fired.
    fn tick(session: &mut Session) -> Vec<WeaponId> {
        work(session, &crate::testkit::catalog(), Controls::default());
        fired_by_the_player(&session.take_combat_events())
    }

    /// Double time's second pass: what the player fired.
    fn again(session: &mut Session) -> Vec<WeaponId> {
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        fired_by_the_player(&session.take_combat_events())
    }

    /// The secondary block's sounds among `session`'s.
    fn beeps(session: &mut Session) -> Vec<SimSound> {
        session
            .take_sounds()
            .into_iter()
            .filter(|sound| matches!(sound, SimSound::SecondarySelected | SimSound::Refused))
            .collect()
    }

    // The let-go.

    #[test]
    fn a_persistent_smoke_secondary_firing_its_last_round_is_let_go_that_pass_by_the_engine() {
        let catalog = rockets(PERSISTENT_SMOKE, 0);
        let mut session = armed(&catalog, RuleSource::Engine, 1);
        session.hold_fire(false, true);
        assert_eq!(tick(&mut session), [ROCKET]);
        assert_eq!(session.secondary(), None, "@0x6ac08");
        assert_eq!(beeps(&mut session), [], "silent");
        let mut kept = armed(&catalog, RuleSource::Bible, 1);
        kept.hold_fire(false, true);
        assert_eq!(tick(&mut kept), [ROCKET]);
        assert_eq!(kept.secondary(), Some(ROCKET), "not hidden when empty");
    }

    #[test]
    fn a_secondary_hidden_when_empty_running_out_is_let_go_by_the_other_reading() {
        let catalog = rockets(0, HIDE_WHEN_EMPTY);
        let mut session = armed(&catalog, RuleSource::Bible, 1);
        session.hold_fire(false, true);
        assert_eq!(tick(&mut session), [ROCKET]);
        assert_eq!(session.secondary(), None);
        assert_eq!(beeps(&mut session), [], "silent");
        let mut kept = armed(&catalog, RuleSource::Engine, 1);
        kept.hold_fire(false, true);
        assert_eq!(tick(&mut kept), [ROCKET]);
        assert_eq!(kept.secondary(), Some(ROCKET), "no Flags 0x0800");
    }

    #[test]
    fn the_let_go_runs_without_the_fire_key() {
        let catalog = rockets(PERSISTENT_SMOKE, 0);
        let mut session = armed(&catalog, RuleSource::Engine, 2);
        assert_eq!(tick(&mut session), []);
        assert_eq!(session.secondary(), Some(ROCKET), "rounds left");
        set_rounds(&mut session, 0);
        assert_eq!(tick(&mut session), []);
        assert_eq!(session.secondary(), None);
    }

    #[test]
    fn a_cloaked_player_lets_go_of_a_secondary_that_cannot_fire_cloaked() {
        let cloaked = Cloak {
            fade: FADE_FULL,
            direction: 0,
        };
        let catalog = rockets(PERSISTENT_SMOKE, 0);
        let mut session = armed(&catalog, RuleSource::Engine, 3);
        session.ship.cloak.cloak = cloaked;
        tick(&mut session);
        assert_eq!(session.secondary(), None, "_WeaponHasAmmo @0xb9fc");
        let catalog = rockets(PERSISTENT_SMOKE, FIRES_CLOAKED);
        let mut session = armed(&catalog, RuleSource::Engine, 3);
        session.ship.cloak.cloak = cloaked;
        tick(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET), "Flags2 0x4000");
    }

    #[test]
    fn the_player_is_no_npc_to_the_let_go() {
        let catalog = rockets(PERSISTENT_SMOKE, AI_WONT_USE);
        let mut session = armed(&catalog, RuleSource::Engine, 3);
        tick(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET), "@0xb963-0xb993");
    }

    /// [`rockets`] needing its class's `KeyCarried` aboard, ship 128
    /// carrying a bay (143) of ship 150, whose rounds outfit 315 holds,
    /// and a `KeyCarried` of `key`.
    fn keyed(key: Option<ShipId>) -> FakePilotCatalog {
        let mut catalog = rockets(PERSISTENT_SMOKE, NEEDS_KEY_SHIP);
        catalog.weapons.push(WeaponRecord {
            id: WeaponId(143),
            flags: SECONDARY,
            guidance: 99,
            ammo_type: 150,
            ..catalog.weapons[0]
        });
        catalog.outfits.push(outfit(315, &[(MOD_AMMO, 143)]));
        catalog.outfits.push(outfit(316, &[(MOD_WEAPON, 143)]));
        let hull = &mut catalog.hulls[0];
        hull.weapons.push(crate::catalog::StockWeapon {
            weapon: WeaponId(143),
            count: 1,
            ammo: 0,
        });
        hull.key_carried = key;
        catalog
    }

    #[test]
    fn the_let_go_reads_the_class_key_carried() {
        let carrying = keyed(Some(ShipId(150)));
        let mut session = armed(&carrying, RuleSource::Engine, 3);
        session.pilot.outfits.insert(OutfitId(315), 1);
        tick(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET), "its key aboard");
        session.pilot.outfits.remove(&OutfitId(315));
        tick(&mut session);
        assert_eq!(session.secondary(), None, "its key gone");
        let keyless = keyed(None);
        let mut session = armed(&keyless, RuleSource::Engine, 3);
        session.pilot.outfits.insert(OutfitId(315), 1);
        tick(&mut session);
        assert_eq!(session.secondary(), None, "no KeyCarried");
    }

    #[test]
    fn the_let_go_runs_while_disabled_breaking_up_and_jumping() {
        let catalog = rockets(PERSISTENT_SMOKE, 0);
        let downed = [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 20,
                starting: false,
            },
        ];
        for condition in downed {
            let mut session = armed(&catalog, RuleSource::Engine, 0);
            session.ship.condition = condition;
            tick(&mut session);
            assert_eq!(session.secondary(), None, "{condition:?}");
        }
        let mut jumping = armed(&catalog, RuleSource::Engine, 0);
        jumping.mode = Mode::Jumping(Jump::PreJump {
            to: SystemId(131),
            bearing: None,
            warp: None,
        });
        tick(&mut jumping);
        assert_eq!(jumping.secondary(), None, "+0x50 gates only the fire");
    }

    #[test]
    fn the_block_is_skipped_when_destroyed_or_in_the_pod() {
        let catalog = rockets(PERSISTENT_SMOKE, 0);
        let mut destroyed = armed(&catalog, RuleSource::Engine, 0);
        destroyed.ship.condition = Condition::Destroyed;
        destroyed.press_weapon_select(false);
        destroyed.hold_weapon_safety(true);
        tick(&mut destroyed);
        assert_eq!(destroyed.secondary(), Some(ROCKET), "@0x68853");
        assert_eq!(beeps(&mut destroyed), []);
        let mut pod = armed(&catalog, RuleSource::Engine, 0);
        pod.pod = Some(100);
        pod.press_weapon_select(false);
        tick(&mut pod);
        assert_eq!(pod.secondary(), Some(ROCKET), "@0x68ddd");
        assert_eq!(beeps(&mut pod), []);
        pod.pod = None;
        tick(&mut pod);
        assert_eq!(beeps(&mut pod), [], "the press was dropped");
    }

    #[test]
    fn a_stale_selection_is_checked_on_its_record() {
        // Ship 128 carries no rockets here: the selection is stale.
        let mut catalog = rockets(PERSISTENT_SMOKE, 0);
        catalog.hulls[0]
            .weapons
            .retain(|stock| stock.weapon != ROCKET);
        let rules = Rulebook::default().with_override(RuleKey::EmptySecondary, RuleSource::Engine);
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        session.ship.secondary = Some(ROCKET);
        set_rounds(&mut session, 1);
        tick(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET), "its rounds held");
        set_rounds(&mut session, 0);
        tick(&mut session);
        assert_eq!(session.secondary(), None, "none of its rounds");
        session.ship.secondary = Some(WeaponId(199));
        tick(&mut session);
        assert_eq!(session.secondary(), Some(WeaponId(199)), "no record");
    }

    // Weapon Select.

    #[test]
    fn a_select_press_waits_for_the_pass_which_fires_the_old_selection_first() {
        let catalog = with_secondaries(catalog());
        let mut session = armed(&catalog, RuleSource::Engine, 3);
        session.hold_fire(false, true);
        session.press_weapon_select(false);
        assert_eq!(session.secondary(), Some(ROCKET), "until the pass");
        assert_eq!(tick(&mut session), [ROCKET]);
        assert_eq!(session.secondary(), Some(MISSILE));
        assert_eq!(beeps(&mut session), [SimSound::SecondarySelected]);
        assert_eq!(tick(&mut session), [MISSILE]);
        assert_eq!(beeps(&mut session), [], "one step a press");
    }

    #[test]
    fn a_select_press_goes_in_weap_id_order_and_backwards_with_option() {
        // Ship 129 mounts the torch (142) before the missiles (141).
        let mut catalog = with_secondaries(catalog());
        catalog.hulls[0].weapons = catalog.hulls[1].weapons.clone();
        let mut session = Session::start(&catalog).expect("starts");
        let mut picked = Vec::new();
        for backwards in [false, false, false, true] {
            session.press_weapon_select(backwards);
            tick(&mut session);
            picked.push(session.secondary());
        }
        assert_eq!(
            picked,
            [Some(MISSILE), Some(TORCH), Some(MISSILE), Some(TORCH)]
        );
    }

    #[test]
    fn the_only_candidate_is_picked_again_with_its_beep() {
        let mut catalog = with_secondaries(catalog());
        catalog.hulls[0]
            .weapons
            .retain(|stock| stock.weapon == ROCKET);
        let mut session = Session::start(&catalog).expect("starts");
        session.press_weapon_select(false);
        tick(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET));
        assert_eq!(beeps(&mut session), [SimSound::SecondarySelected]);
        session.press_weapon_select(true);
        tick(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET));
        assert_eq!(beeps(&mut session), [SimSound::SecondarySelected]);
    }

    #[test]
    fn a_select_press_with_no_candidate_is_refused_and_changes_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.press_weapon_select(false);
        tick(&mut session);
        assert_eq!(session.secondary(), None);
        assert_eq!(beeps(&mut session), [SimSound::Refused], "@0x6ad90");
        let mut hidden = rockets(0, HIDE_WHEN_EMPTY);
        hidden.hulls[0]
            .weapons
            .retain(|stock| stock.weapon == ROCKET);
        let mut stale = armed(&hidden, RuleSource::Engine, 0);
        stale.press_weapon_select(false);
        tick(&mut stale);
        assert_eq!(stale.secondary(), Some(ROCKET), "kept");
        assert_eq!(beeps(&mut stale), [SimSound::Refused]);
    }

    #[test]
    fn one_press_steps_once_through_both_passes_of_double_time() {
        let catalog = with_secondaries(catalog());
        let mut session = Session::start(&catalog).expect("starts");
        session.press_weapon_select(false);
        tick(&mut session);
        again(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET));
        assert_eq!(beeps(&mut session), [SimSound::SecondarySelected]);
    }

    #[test]
    fn the_second_pass_of_double_time_takes_a_press_too() {
        let catalog = with_secondaries(catalog());
        let mut session = Session::start(&catalog).expect("starts");
        session.press_weapon_select(false);
        again(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET));
    }

    // Weapon Safety.

    #[test]
    fn the_safety_lets_go_on_the_next_pass_with_one_beep_and_then_holds_silent() {
        let catalog = with_secondaries(catalog());
        let mut session = armed(&catalog, RuleSource::Engine, 3);
        session.hold_weapon_safety(true);
        assert_eq!(session.secondary(), Some(ROCKET), "until the pass");
        tick(&mut session);
        assert_eq!(session.secondary(), None);
        assert_eq!(beeps(&mut session), [SimSound::SecondarySelected]);
        tick(&mut session);
        assert_eq!(beeps(&mut session), [], "held, none selected");
        session.hold_weapon_safety(false);
        session.select_secondary(false);
        session.take_sounds();
        tick(&mut session);
        assert_eq!(session.secondary(), Some(ROCKET), "let go of");
    }

    #[test]
    fn the_safety_with_nothing_selected_is_silent() {
        let mut session = Session::start(&with_secondaries(catalog())).expect("starts");
        session.hold_weapon_safety(true);
        tick(&mut session);
        assert_eq!(session.secondary(), None);
        assert_eq!(beeps(&mut session), []);
    }

    #[test]
    fn the_safety_with_the_fire_held_fires_once_and_no_later_step_fires() {
        let catalog = with_secondaries(catalog());
        let mut session = armed(&catalog, RuleSource::Engine, 3);
        session.hold_fire(false, true);
        session.hold_weapon_safety(true);
        assert_eq!(tick(&mut session), [ROCKET]);
        assert_eq!(session.secondary(), None);
        assert_eq!(again(&mut session), [], "the selection read live");
        assert_eq!(tick(&mut session), []);
    }

    #[test]
    fn a_select_press_with_the_safety_held_selects_and_lets_go_in_one_pass() {
        let catalog = with_secondaries(catalog());
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_weapon_safety(true);
        session.press_weapon_select(false);
        tick(&mut session);
        assert_eq!(session.secondary(), None);
        assert_eq!(
            beeps(&mut session),
            [SimSound::SecondarySelected, SimSound::SecondarySelected]
        );
    }

    // The trigger.

    #[test]
    fn a_trigger_held_whole_keeps_its_own_secondary() {
        let catalog = with_secondaries(catalog());
        let mut session = armed(&catalog, RuleSource::Engine, 3);
        session.hold_fire(false, true);
        session.hold_trigger(Trigger {
            secondary: Some(MISSILE),
            ..Trigger::default()
        });
        assert_eq!(tick(&mut session), [MISSILE]);
        session.hold_fire(false, false);
        assert_eq!(tick(&mut session), []);
    }

    #[test]
    fn the_secondary_fire_key_fires_the_selection_made_after_it_was_held() {
        let catalog = with_secondaries(catalog());
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_fire(false, true);
        session.select_secondary(false);
        assert!(matches!(
            session.take_sounds().as_slice(),
            [SimSound::SecondarySelected]
        ));
        assert_eq!(tick(&mut session), [ROCKET]);
    }
}
