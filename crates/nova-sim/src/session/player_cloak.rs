//! The player's own cloaking device (the key and upkeep are
//! [`player_cloak`](crate::player_cloak); the fade, the gear and the
//! draw are [`cloak`](crate::cloak)).
//!
//! The view holds the cloak key, `cloakKey` (U by default), with
//! [`Session::hold_cloak`] while the player's fate takes
//! [`PlayerCommand::Cloak`](crate::PlayerCommand::Cloak). The player's
//! gear reads the outfits it owns ([`CloakGear::player`]).
//!
//! - **The key and upkeep** (`_HandlePlayer` @0x705b7-0x70827): each pass's
//!   display step runs them after the self-destruct, on both passes of a
//!   double-time frame as `_HandlePlayer` runs on both, so a frame pays the
//!   upkeep twice and the latch lets a held key toggle once. They return at
//!   once in the escape pod (@0x68ddd) and once destroyed (the dead branch,
//!   0x6870d-0x6888c), leaving the latch as it is. The key is taken only
//!   while the player was not disabled at the head of `_HandlePlayer`
//!   (`_IsDisabled`, -0x2cb, which holds while breaking up) and not
//!   jumping (+0x50, the jump under way, the pre-jump included). The
//!   breakup-timer gate (@0x705d6) reads the condition the head read, so
//!   `_IsDisabled` covers it. `_ShipCanCloak` reads
//!   [`RuleKey::CloakReserves`]; the press and the upkeep each ask it, the
//!   upkeep after a zeroing press has zeroed the shield. The key raises
//!   [`SimSound::Cloaked`] (`snd ` 381), [`SimSound::Uncloaked`] (380) or
//!   [`SimSound::Refused`] (153, priority 1); the upkeep's uncloak raises
//!   [`SimSound::Uncloaked`].
//! - **The fade** (`_HandleShipDisplay` @0x2d088-0x2d17e for slot 0): it
//!   steps after the key on each pass, just before the player's display,
//!   in the pod too (`_DoPlayGameWork` @0x43e29), falling at twice the rate
//!   while the player dies. `_PrepareAllSprites` steps it first, for slot
//!   0, before the NPCs' (@0x3a776-0x3a78e;
//!   [`Session::prepare_all_sprites`]), and a gate entry steps it once
//!   whatever the exit, as the player's slot always passes the display's
//!   system test (@0x2b7d8-0x2b7e0).
//! - **Drawn** ([`Session::player_cloak_shown`]): at a full fade the
//!   player is always drawn faint, at
//!   [`SEEN_AT_FULL_FADE`](crate::cloak::SEEN_AT_FULL_FADE) (+0x86 0,
//!   @0x2d199-0x2d1a1); fading, jittered and faint; with no fade, as it is.
//! - **Hits** (`_DamageShip` @0x3bf1f-0x3bf4f): a direct hit decloaks a
//!   cloak that decloaks when hit while it is fully faded or still fading
//!   in, and the player hears [`SimSound::Uncloaked`]. Cloak-when-hit
//!   never applies to the player, whose AI type is 0 (@0x3b459-0x3b461).
//! - **Resets**: only a session's start and the pod's rescue
//!   (`_ResetPlayer` @0x1d457-0x1d462) reset the fade. Landing, taking
//!   off, a jump and the eject keep it, so a player landing still fading
//!   in takes off fading, and a pod that left cloaked keeps its cloak
//!   until the rescue. The cloak is never saved; a pilot loads through
//!   `_ResetPlayer`.

use super::{Outbox, PlayerShip, Session, Setup, Statics};
use crate::cloak::{self, Cloak, CloakCheck, CloakGear, CloakShown, SEEN_AT_FULL_FADE};
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::pilot::Pilot;
use crate::player_cloak::{Pressed, press, upkeep};
use crate::rulebook::RuleKey;
use crate::sound::SimSound;

/// The player's cloak and its key: unsaved (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct PlayerCloak {
    /// The fade, reset only at a session's start and by the rescue
    /// (`_ResetPlayer` @0x1d457-0x1d462).
    pub(super) cloak: Cloak,
    /// Whether the view holds the cloak key.
    pub(super) held: bool,
    /// The key's debounce, `_cloakFlag` (`*0xdedfc`), a global: set by a
    /// taken press, cleared by a frame the key isn't taken.
    pub(super) latch: bool,
}

impl Session {
    /// Holds the cloak key, `cloakKey`, down or lets it go: what the next
    /// pass's key reads. The session takes it only while the player's
    /// fate takes [`PlayerCommand::Cloak`](crate::PlayerCommand::Cloak).
    pub fn hold_cloak(&mut self, held: bool) {
        self.ship.cloak.held = held;
    }

    /// The player's cloak: its fade and the way it is fading.
    #[must_use]
    pub fn player_cloak(&self) -> Cloak {
        self.ship.cloak.cloak
    }

    /// How the player's cloak lets the screen show its ship (see the
    /// module docs).
    #[must_use]
    pub fn player_cloak_shown(&self) -> CloakShown {
        cloak::shown(self.ship.cloak.cloak, SEEN_AT_FULL_FADE)
    }

    /// Sets the player's cloak directly, for
    /// [`fixture::set_player_cloak`](crate::fixture::set_player_cloak).
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn set_player_cloak(&mut self, to: Cloak) {
        self.ship.cloak.cloak = to;
    }

    /// One pass of the cloak key and upkeep, the player
    /// `disabled_at_head` of the pass or not (see the module docs).
    pub(super) fn player_cloak_tick(&mut self, disabled_at_head: bool) {
        if self.pod.is_some() || self.ship.condition == Condition::Destroyed {
            return;
        }
        let jumping = self.mode.jump().is_some();
        self.ship.cloak_tick(
            &mut self.pilot,
            &self.statics,
            &self.setup,
            &mut self.out,
            jumping,
            disabled_at_head,
        );
    }
}

impl PlayerShip {
    /// The player's cloak gear, from its ship type's `Flags2` and the
    /// outfits it owns.
    pub(super) fn cloak_gear(&self, pilot: &Pilot, statics: &Statics) -> CloakGear {
        CloakGear::player(self.fit.fields.flags2, &pilot.outfits, &statics.outfits)
    }

    /// One pass of the cloak key and upkeep, the player `jumping` or not
    /// and `disabled` at the pass's head or not (see the module docs).
    fn cloak_tick(
        &mut self,
        pilot: &mut Pilot,
        statics: &Statics,
        setup: &Setup,
        out: &mut Outbox,
        jumping: bool,
        disabled: bool,
    ) {
        let gear = self.cloak_gear(pilot, statics);
        let reserves = setup.rules.source_for(RuleKey::CloakReserves);
        let can_cloak = |pilot: &Pilot| {
            let check = CloakCheck {
                disabled,
                fuel: pilot.reserves.fuel.now,
                shield: pilot.reserves.shield.now,
                jumping,
            };
            gear.can_cloak(check, reserves)
        };
        let taken = self.cloak.held && !jumping && !disabled;
        let able = can_cloak(pilot);
        let shield = &mut pilot.reserves.shield.now;
        let cloak = &mut self.cloak;
        match press(
            &mut cloak.cloak,
            &mut cloak.latch,
            taken,
            able,
            gear,
            shield,
        ) {
            Some(Pressed::Cloaked { sounds: true }) => out.sounds.push(SimSound::Cloaked),
            Some(Pressed::Uncloaked { sounds: true }) => out.sounds.push(SimSound::Uncloaked),
            Some(Pressed::Refused) => out.sounds.push(SimSound::Refused),
            _ => {}
        }
        let able = can_cloak(pilot);
        let reserves = &mut pilot.reserves;
        let (fuel, shield) = (&mut reserves.fuel.now, &mut reserves.shield.now);
        if upkeep(&mut self.cloak.cloak, gear, able, disabled, fuel, shield) == Some(true) {
            out.sounds.push(SimSound::Uncloaked);
        }
    }

    /// One step of the player's fade, as its slot's `_HandleShipDisplay`
    /// steps it (@0x2d088-0x2d17e): a dying player's falls at twice the
    /// rate.
    pub(super) fn step_cloak_fade(&mut self, pilot: &Pilot, statics: &Statics) {
        let fast = self.cloak_gear(pilot, statics).fast();
        let dying = self.condition.is_dying(pilot.reserves.armor);
        self.cloak.cloak.display_step(fast, dying);
    }

    /// Each direct one of `strikes` on the player, as its cloak takes it
    /// (see the module docs).
    pub(super) fn cloak_hits(
        &mut self,
        pilot: &Pilot,
        statics: &Statics,
        strikes: &[Strike],
        out: &mut Outbox,
    ) {
        let gear = self.cloak_gear(pilot, statics);
        for _ in strikes
            .iter()
            .filter(|strike| strike.direct && strike.ship == ShipRef::Player)
        {
            if cloak::decloak_on_hit(&mut self.cloak.cloak, gear) {
                out.sounds.push(SimSound::Uncloaked);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, SystemId};
    use crate::chance::NeverFires;
    use crate::cloak::{CLOAK_DECLOAKS_WHEN_HIT, CLOAK_JUMPING, FADE_FULL, MOD_CLOAK};
    use crate::combat::Rules;
    use crate::combat::fixture::strike;
    use crate::flight::Controls;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::npc::NpcId;
    use nova_sim_catalog::fixture::outfit;

    /// The cloak `_HandlePlayer` keeps whole: fully faded, holding.
    const FULL: Cloak = Cloak {
        fade: FADE_FULL,
        direction: 0,
    };

    fn at(fade: f32, direction: i8) -> Cloak {
        Cloak { fade, direction }
    }

    /// [`armed`], with outfit 300, a cloak of `ModVal` `mod_val`.
    fn cloaking(mod_val: i16) -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.outfits.push(outfit(300, &[(MOD_CLOAK, mod_val)]));
        catalog
    }

    /// `catalog`'s [`facing_an_npc`], the player owning outfit 300 and
    /// fuel and shield to spare.
    fn owning(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.pilot.outfits.insert(OutfitId(300), 1);
        session.pilot.reserves.fuel.now = 100.0;
        session.pilot.reserves.shield.now = 100.0;
        session
    }

    /// A fight tick, the key held or not: its sounds.
    fn tick(session: &mut Session, held: bool) -> Vec<SimSound> {
        session.take_sounds();
        session.hold_cloak(held);
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.take_sounds()
    }

    /// The cloak sounds among `sounds`.
    fn cloak_sounds(sounds: &[SimSound]) -> Vec<SimSound> {
        sounds
            .iter()
            .copied()
            .filter(|sound| {
                matches!(
                    sound,
                    SimSound::Cloaked | SimSound::Uncloaked | SimSound::Refused
                )
            })
            .collect()
    }

    /// The second pass of a double-time frame, its sounds.
    fn again(session: &mut Session) -> Vec<SimSound> {
        session.take_sounds();
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        session.take_sounds()
    }

    #[test]
    fn the_cloak_key_cloaks_a_player_owning_a_cloak_and_sounds_cloaked() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        assert_eq!(cloak_sounds(&tick(&mut session, true)), [SimSound::Cloaked]);
        assert_eq!(session.player_cloak(), at(0.75, 1), "stepped on that pass");
        assert_eq!(cloak_sounds(&tick(&mut session, true)), [], "held");
        assert_eq!(session.player_cloak(), at(1.5, 1));
    }

    #[test]
    fn the_cloak_key_without_a_cloak_beeps_refused_once_a_press() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        session.pilot.outfits.remove(&OutfitId(300));
        assert_eq!(cloak_sounds(&tick(&mut session, true)), [SimSound::Refused]);
        assert_eq!(cloak_sounds(&tick(&mut session, true)), [], "held");
        assert_eq!(cloak_sounds(&tick(&mut session, false)), [], "let go");
        assert_eq!(cloak_sounds(&tick(&mut session, true)), [SimSound::Refused]);
        assert_eq!(session.player_cloak(), Cloak::default());
    }

    #[test]
    fn the_cloak_key_out_of_fuel_beeps_refused_by_the_engine() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        session.pilot.reserves.fuel.now = 0.0;
        assert_eq!(cloak_sounds(&tick(&mut session, true)), [SimSound::Refused]);
        assert_eq!(session.player_cloak(), Cloak::default());
    }

    #[test]
    fn a_disabled_or_jumping_player_takes_no_cloak_key_and_hears_nothing() {
        let catalog = cloaking(0);
        let mut disabled = owning(&catalog);
        disabled.ship.condition = Condition::Disabled;
        assert_eq!(cloak_sounds(&tick(&mut disabled, true)), []);
        assert_eq!(disabled.player_cloak(), Cloak::default());
        let mut jumping = owning(&catalog);
        jumping.mode = crate::session::Mode::Jumping(crate::session::Jump::PreJump {
            to: SystemId(131),
            bearing: None,
            warp: None,
        });
        assert_eq!(cloak_sounds(&tick(&mut jumping, true)), []);
        assert_eq!(jumping.player_cloak(), Cloak::default());
        assert!(!jumping.ship.cloak.latch, "a gate failing clears the latch");
    }

    #[test]
    fn a_breaking_up_player_takes_no_cloak_key_and_its_upkeep_uncloaks_it() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        session.ship.condition = Condition::Dying {
            ticks_left: 1000,
            starting: false,
        };
        session.ship.cloak.cloak = FULL;
        assert_eq!(
            cloak_sounds(&tick(&mut session, true)),
            [SimSound::Uncloaked]
        );
        assert!(session.player_cloak().direction < 0);
        let mut fresh = owning(&catalog);
        fresh.ship.condition = Condition::Dying {
            ticks_left: 1000,
            starting: false,
        };
        assert_eq!(cloak_sounds(&tick(&mut fresh, true)), []);
        assert_eq!(fresh.player_cloak(), Cloak::default());
    }

    #[test]
    fn a_cloaked_player_out_of_fuel_uncloaks_with_the_uncloak_sound() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        session.ship.cloak.cloak = FULL;
        assert_eq!(cloak_sounds(&tick(&mut session, false)), [], "fuel left");
        assert_eq!(session.player_cloak(), FULL);
        session.pilot.reserves.fuel.now = 0.0;
        assert_eq!(
            cloak_sounds(&tick(&mut session, false)),
            [SimSound::Uncloaked]
        );
        assert_eq!(session.player_cloak(), at(FADE_FULL - 0.75, -1));
    }

    #[test]
    fn a_cloaked_player_keeps_its_cloak_through_a_jump_only_with_flags2_0x0400() {
        let catalog = cloaking(0);
        for (flags2, keeps) in [(0, false), (CLOAK_JUMPING, true)] {
            let mut session = owning(&catalog);
            session.ship.fit.fields.flags2 = flags2;
            session.ship.cloak.cloak = FULL;
            session.mode = crate::session::Mode::Jumping(crate::session::Jump::PreJump {
                to: SystemId(131),
                bearing: None,
                warp: None,
            });
            tick(&mut session, false);
            assert_eq!(session.player_cloak() == FULL, keeps, "{flags2:#x}");
        }
    }

    #[test]
    fn a_zeroing_cloak_zeroes_the_players_shield_as_it_cloaks_and_every_frame_after() {
        // Stock 234's `ModVal` 0x6: zeroes shields, shows on the radar.
        let catalog = cloaking(0x6);
        let mut session = owning(&catalog);
        tick(&mut session, true);
        assert_eq!(session.pilot.reserves.shield.now, 0.0, "as it cloaks");
        while !session.player_cloak().is_cloaked() {
            tick(&mut session, false);
        }
        session.pilot.reserves.shield.now = 40.0;
        tick(&mut session, false);
        assert_eq!(session.pilot.reserves.shield.now, 0.0, "every frame after");
    }

    #[test]
    fn the_cloak_reserves_rule_decides_whether_an_empty_shield_uncloaks_the_player() {
        // Stock 268's `ModVal` 0x080b: a shield usage of 8, no fuel usage.
        let catalog = cloaking(0x080b);
        for (source, keeps) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let rules = Rulebook::default().with_override(RuleKey::CloakReserves, source);
            let mut session = owning(&catalog).with_rules(rules);
            session.ship.cloak.cloak = FULL;
            session.pilot.reserves.shield.now = 0.0;
            let sounds = cloak_sounds(&tick(&mut session, false));
            assert_eq!(session.player_cloak() == FULL, keeps, "{source:?}");
            assert_eq!(sounds.is_empty(), keeps, "{source:?}");
        }
    }

    #[test]
    fn a_direct_hit_uncloaks_a_player_whose_cloak_decloaks_when_hit_and_a_blast_does_not() {
        let hit = strike(ShipRef::Player, ShipRef::Npc(NpcId(0)));
        let blast = Strike {
            direct: false,
            ..hit
        };
        let on_npc = strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        for (mod_val, strikes, uncloaks) in [
            (CLOAK_DECLOAKS_WHEN_HIT, vec![hit], true),
            (CLOAK_DECLOAKS_WHEN_HIT, vec![blast, on_npc], false),
            (0, vec![hit], false),
        ] {
            let catalog = cloaking(mod_val);
            let mut session = owning(&catalog);
            session.ship.cloak.cloak = FULL;
            session.take_sounds();
            let ship = &mut session.ship;
            ship.cloak_hits(&session.pilot, &session.statics, &strikes, &mut session.out);
            let sounds = cloak_sounds(&session.take_sounds());
            let what = format!("{mod_val:#x} {strikes:?}");
            assert_eq!(session.player_cloak().direction < 0, uncloaks, "{what}");
            assert_eq!(sounds == [SimSound::Uncloaked], uncloaks, "{what}");
        }
    }

    #[test]
    fn double_time_toggles_once_a_press_and_pays_the_upkeep_on_both_passes() {
        let catalog = cloaking(0x0030);
        let mut session = owning(&catalog);
        assert_eq!(cloak_sounds(&tick(&mut session, true)), [SimSound::Cloaked]);
        assert_eq!(cloak_sounds(&again(&mut session)), [], "the latch holds");
        assert_eq!(session.player_cloak(), at(1.5, 1), "stepped on both passes");
        session.ship.cloak.cloak = FULL;
        let fuel = session.pilot.reserves.fuel.now;
        tick(&mut session, false);
        again(&mut session);
        let paid = fuel - session.pilot.reserves.fuel.now;
        let rate = 3.0 * crate::cloak::CLOAK_UPKEEP_RATE;
        assert!((paid - 2.0 * rate).abs() < 1e-4, "paid {paid}");
    }

    #[test]
    fn the_players_fade_steps_each_pass_and_falls_twice_as_fast_while_dying() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        session.ship.cloak.cloak = at(10.0, 1);
        tick(&mut session, false);
        assert_eq!(session.player_cloak(), at(10.75, 1));
        again(&mut session);
        assert_eq!(session.player_cloak(), at(11.5, 1));
        // Not cloaked, so the upkeep's uncloak (direction -1) doesn't
        // take it.
        session.ship.cloak.cloak = at(5.0, 1);
        session.ship.condition = Condition::Dying {
            ticks_left: 1000,
            starting: false,
        };
        tick(&mut session, false);
        assert_eq!(
            session.player_cloak(),
            at(5.75, crate::cloak::DYING_DIRECTION)
        );
        tick(&mut session, false);
        assert_eq!(
            session.player_cloak(),
            at(4.25, crate::cloak::DYING_DIRECTION)
        );
        let mut fast = owning(&catalog);
        fast.ship.fit.fields.flags2 = crate::cloak::FAST_FADE;
        fast.ship.cloak.cloak = at(10.0, 1);
        tick(&mut fast, false);
        assert_eq!(fast.player_cloak(), at(11.5, 1), "Flags2 0x0001");
    }

    #[test]
    fn a_player_landing_still_fading_in_takes_off_fading() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        session.ship.cloak.cloak = at(10.0, 1);
        crate::testkit::land_now(&mut session).expect("lands");
        let landed = session.player_cloak();
        assert_eq!(landed.direction, 1, "landing keeps it");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(
            session.player_cloak(),
            at(landed.fade + 1.5, 1),
            "and the take-off's two displays step it"
        );
    }

    #[test]
    fn the_players_fighter_reads_its_cloak_and_device() {
        let read = |session: &mut Session| {
            let mut seen = None;
            let pass = crate::combat::GamePass::Advance { double_time: false };
            session.with_fighters(
                pass,
                Rules::default(),
                &mut NeverFires,
                |_, fighters, _, _| {
                    seen = Some((fighters[0].cloaked, fighters[0].has_cloak));
                },
            );
            seen.expect("the player fights")
        };
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        assert_eq!(read(&mut session), (false, true));
        session.ship.cloak.cloak = FULL;
        assert_eq!(read(&mut session), (true, true));
        session.pilot.outfits.remove(&OutfitId(300));
        assert_eq!(read(&mut session), (true, false));
    }

    #[test]
    fn a_cloaked_player_fires_only_its_weapons_that_fire_cloaked() {
        use crate::combat::weapon::FIRES_CLOAKED;
        let fired = |flags2: u16, cloak: Cloak| {
            let mut catalog = cloaking(0);
            catalog.weapons[0].flags2 = flags2;
            let mut session = owning(&catalog);
            session.ship.cloak.cloak = cloak;
            session.hold_trigger(crate::session::tests::FIRE);
            let pass = crate::combat::GamePass::Advance { double_time: false };
            session.player_fires(pass, Rules::default(), &mut NeverFires);
            session
                .shots()
                .iter()
                .filter(|shot| shot.firer == ShipRef::Player)
                .count()
        };
        assert_eq!(fired(0, Cloak::default()), 1, "uncloaked");
        assert_eq!(fired(0, FULL), 0, "cloaked");
        assert_eq!(fired(FIRES_CLOAKED, FULL), 1, "Flags2 0x4000");
    }

    #[test]
    fn the_players_full_fade_is_drawn_faint() {
        let catalog = cloaking(0);
        let mut session = owning(&catalog);
        assert_eq!(session.player_cloak_shown(), CloakShown::Opaque);
        session.ship.cloak.cloak = FULL;
        assert_eq!(
            session.player_cloak_shown(),
            CloakShown::Translucent(SEEN_AT_FULL_FADE)
        );
        session.ship.cloak.cloak = at(12.0, 1);
        assert_eq!(session.player_cloak_shown(), CloakShown::Fading(12.0));
    }

    // The land key's cloak refusal (`_HandlePlayerDockRequest`
    // @0x66a85-0x66ae9).

    /// `catalog`'s session over planet 128, fully cloaked, its sounds
    /// taken.
    fn cloaked_at_start(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.ship.cloak.cloak = FULL;
        session.take_sounds();
        session
    }

    #[test]
    fn a_cloaked_player_is_refused_landing_with_one_beep_and_requests_no_clearance() {
        use crate::catalog::StellarId;
        use crate::landing::LandingRefusal;
        use crate::session::StellarPick;
        let catalog = nova_sim_catalog::fixture::catalog();
        let mut session = cloaked_at_start(&catalog);
        assert_eq!(session.land(), Err(LandingRefusal::Cloaked));
        assert_eq!(session.nav_target(), Some(StellarId(128)), "selected");
        assert_eq!(session.take_sounds(), [SimSound::Refused], "@0x67e8f");
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert_eq!(session.land(), Err(LandingRefusal::Cloaked));
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
        assert_eq!(session.landed(), None);
        assert_eq!(session.port.dock, None, "no clearance request");
        session.select_stellar(StellarPick::Slot(1));
        assert_eq!(
            session.land(),
            Err(LandingRefusal::Cloaked),
            "too far: the reach comes after (@0x66c23)"
        );
        session.ship.cloak.cloak = at(10.0, 1);
        session.select_stellar(StellarPick::Slot(0));
        assert!(
            session.land().is_ok(),
            "still fading in: it lands as before"
        );
    }

    #[test]
    fn a_cloaked_player_over_a_hostile_stellar_is_refused_as_not_landable_first() {
        use crate::catalog::StellarId;
        use crate::landing::LandingRefusal;
        use crate::session::StellarPick;
        let mut catalog = nova_sim_catalog::fixture::catalog();
        catalog.sites[0].1[0].flags = 0;
        let mut session = cloaked_at_start(&catalog);
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            }),
            "@0x66977 comes first"
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
    }

    #[test]
    fn a_disabled_cloaked_player_is_refused_as_cloaked() {
        use crate::landing::LandingRefusal;
        use crate::session::StellarPick;
        let catalog = nova_sim_catalog::fixture::catalog();
        let mut session = cloaked_at_start(&catalog);
        session.ship.condition = Condition::Disabled;
        session.select_stellar(StellarPick::Slot(0));
        session.take_sounds();
        assert_eq!(
            session.land(),
            Err(LandingRefusal::Cloaked),
            "the cloak (@0x66a8e) before the disabled test (@0x66aee)"
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
    }
}
