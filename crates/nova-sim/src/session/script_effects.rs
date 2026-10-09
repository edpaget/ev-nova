//! The set operators that move the player and play a sound: `M` and `N`
//! move it to another system, `Q` makes it leave the stellar it is landed
//! on, and `P` plays a sound.
//!
//! **The original.** `_EvalSetExp` (@0x150fc) dispatches on the letter
//! through the table at 0xdd2fc (@0x15cc2):
//!
//! | Operator | Where | What |
//! |---|---|---|
//! | `M` | @0x1570e-0x158d4 | the player's system is set (ship +0x74) and its stellar cleared (+0x6c = -1); the first of the new system's nav slots that is not -1 is found, with no check of landability or gates. Landed, that stellar becomes +0x6c and nothing else changes; in flight, the ship is put at its x/y at rest. With no stellar, nothing more: in flight the position and velocity are kept |
//! | `N` | @0x158d9-0x159ad | as `M`, but the position and velocity are never touched, and `_dontMovePlayerAfterLanding` (0x316951) is set |
//! | both | @0x15807-0x1583e | every active ship whose parent is the player (+0x9a 0: its escorts and fighters out) takes the new system; the others keep the old one, and `_DoPlayGameWork` runs only the ships in the player's system, so they stay behind |
//! | both, landed | | `_dockedPortMissions` is filled with -1, and that is all |
//! | both, in flight | @0x158a4 | the shots, cargo boxes, explosions, smoke and asteroids are killed, the stellar sprites rebuilt, and `_MissionHandlePlayerEnteredNewSystem` runs |
//! | `Q` | @0x15bee-0x15c6c | `_absquatulateStr` (0x1635a0) becomes one of `STR#` `xxx`'s strings, the raw ID, with no range check, drawn as `T` draws (`_GetRandomIndString`: `Rand(count) + 1`); only inside a mission's set expression (`_currentMissionIndexForSetParser` ≤ 15, set by `_EvalCurrentMissionBitSetString` @0x99a6b alone) is it run through `_MungeBriefing`, so a hook's text is shown as it is. A missing or empty list, or an empty pick, leaves it empty (strcpy over an earlier one) |
//!
//! Neither calls `_SetupShipsInSystem` or `_AutoSetExploration`, whose
//! only callers are a new pilot, the main screen, the take-off and
//! `_HandlePlayer`'s death (@0x69114) and hyperspace arrival (@0x6c1f9)
//! paths, so in flight the new system is neither explored nor populated
//! at once. Neither touches `_hyperRoute`, and `_CueNextHyperRouteDest`
//! (@0xe2dc) targets the route's next system only when it is linked from
//! the current one, so a stale route cannot be jumped along until it is
//! replotted.
//!
//! The take-off (`_PlayerLandOnStellar` @0x633ad-0x63400) zeroes the
//! velocity, then: with `_dontMovePlayerAfterLanding` set, clears it and
//! keeps the position, which the landing never moved; otherwise puts the
//! ship at +0x6c's x/y, or at the centre with none. It explores the
//! system (`_AutoSetExploration` @0x634c8), and the spaceport sets it up
//! afterwards (`_SetupShipsInSystem`). A landed move never touches
//! `_dockedPort`, which `_DoPortDialog` stores as it opens (@0x5f93b) and
//! its sub-dialogs read, so the open spaceport serves the stellar it
//! opened on until it is left.
//!
//! **Here.** `Mxxx` and `Nxxx`
//! only queue their move: a set expression runs in the middle of a
//! purchase, a capture or a hook, with no catalog to read the new system
//! from. [`Session::settle_script`] applies the moves in order at a safe
//! point, which whoever runs set expressions calls after each input and
//! tick, before saving; a move to a system that does not exist changes
//! nothing (the original indexes it blindly). Each move makes a save
//! due. In flight a move gives up a jump being prepared.
//!
//! A move enters the system as every way in does, by its rows of the
//! table in the `entry` module ([`Session::enter_system`]): `move_to`
//! only chooses where the ship goes, as below.
//!
//! In flight, the ship is in the new system: its stellars read, the
//! navigation target, a gate's pending entry, the shots and beams, the
//! strikes, the player's target, the boarding and the hail let go; a
//! held `P` sound is kept, and no arrival sounds or is raised. `M`
//! puts it at rest on the system's first stellar, whatever that is (a
//! gate too); in a system with no stellar it stays where it is, moving
//! as it was, unless [`ScriptEffectRules::starless`] puts it at rest at
//! the centre. `N` keeps the position and velocity.
//!
//! Landed, only where the pilot is changes: its system, and by `M` its
//! stellar, with the ship at rest at the first stellar's position (at the
//! centre with none); `N` leaves the pilot on no stellar, the ship where
//! it is. The open spaceport keeps serving the stellar landed on, as
//! [`Session::market`] and the rest read it, until the take-off swaps the
//! new system's stellars in. There is no other reading of that: an `N`
//! leaves no stellar to rebuild the spaceport for. By `M` the pilot's
//! stellar is the first one only when it can be landed on and is not a
//! hypergate or wormhole, otherwise none: the pilot's stellar is what a
//! save keeps as the stellar last landed on, through the flight after
//! the take-off too, and [`Session::fly`] docks a reloaded pilot there,
//! so the original's choice would dock it on a gate or a stellar that
//! cannot be landed on, which nothing else in the engine allows (landing
//! on a gate enters it). Such a pilot reloads in flight at the centre;
//! one saved after a landed `N` does too.
//!
//! Every take-off now explores the system, as the original's does, which
//! changes nothing in a system explored already. It keeps the ship where
//! it touched down, instead of the stellar's centre, after an `N`: by
//! [`ScriptEffectRules::keep_flag`]'s engine reading after any `N` since
//! the last take-off, landed or in flight, as the original's flag stays
//! set; by the other, only after a landed one. The flag is not saved.
//!
//! What else a move does follows [`ScriptEffectRules::arrival`]. By the
//! engine, nothing: the course is kept, and J refuses while its next
//! system is not linked from the new one
//! ([`JumpRefusal::NoDestination`](crate::JumpRefusal)); the system is
//! explored by the next take-off; and in flight, in another system, only
//! the player's escorts and fighters out follow, as they are, the new
//! system's ships coming only with its arrivals over time (see
//! [`Traffic::retarget`](crate::Traffic::retarget)). By the other
//! reading the move is an arrival: the system is explored and the course
//! cleared at once, and in flight the fighters out are kept or abandoned
//! as the fighter rules say and the system is populated
//! ([`Session::populate`]).
//!
//! **`Qxxx`** draws its message when it runs, from
//! the session's string lists ([`Session::with_strings`]; none by
//! default), with a roll of as many sides as the list holds, as `T`
//! draws, so the order of the draws is kept. Nothing moves until it is
//! settled; whoever sees the original's string next acts on it:
//!
//! - Landed, every port dialog's filter (`_BarFilter` @0x47a4b,
//!   `_PortFilter` @0x4f041 and the others) closes on its next event, so
//!   `Q` does not wait for the spaceport to close, it forces it closed;
//!   the take-off then shows the message (`_DisplayComm` @0x635cf) in
//!   place of its usual greeting, with no beep. Here
//!   [`Session::settle_script`] takes off ([`Session::take_off`]) and
//!   gives the message, for whoever shows the spaceport to close it.
//! - In flight, the next frame beeps `_beepSnd[4]` (`_PlayGame`
//!   @0x4654c) and shows the message: here the message, and
//!   [`SimSound::ScriptMessage`].
//!
//! The text is the string as it is: mission text tags wait for missions,
//! as the original munges them only inside one. A blank `Q` (a missing
//! or empty list, or an empty pick) does nothing and cancels an earlier
//! one, unless [`ScriptEffectRules::blank_leave`] makes the player leave
//! all the same, with no message.
//!
//! **`Pxxx`** (@0x15a24) sets `_missionSoundID` to the
//! raw `snd ` ID, with no range check, and only `_PlayGame` reads it,
//! once a flight frame (@0x462fa): when it is not -1 and no mission sound
//! is held (`_missionSnd`), it loads and plays it at the effects volume;
//! it lets the held one go once it has stopped (@0x46358-0x46375); and
//! every frame resets the ID (@0x463a9), as do `_ResetPlayer` (@0x1d909)
//! and the hyperspace arrival (`_HandlePlayer` @0x6c4ff), but not a
//! hypergate or wormhole exit (`_PlayerEnterHypergate` @0x637bf,
//! `_PlayerEnterWormhole` @0x64005, which bypass that arrival). So only
//! the last `P` before a flight frame counts, one while a mission sound
//! still plays is dropped, and the spaceport never reads it, so a landed
//! `P` plays on the first flight frame after the take-off.
//!
//! Here, by [`ScriptEffectRules::sound`]'s engine reading, `P` holds its
//! sound, a later one replacing it, and the session's next flight tick
//! ([`Session::tick`]) sounds it as [`SimSound::Script`], exclusive: the
//! audio side plays it on its one mission channel, unless the last sound
//! played there is still playing. A jump's arrival drops a held sound;
//! a hypergate or wormhole exit does not, so it sounds on the first tick
//! after. None is saved. By the other reading every `P` sounds at once,
//! landed or not, over whatever plays. The session never calls audio: the
//! sound leaves it as an event, drained with [`Session::take_sounds`].
//!
//! Not yet as the original: the mission work its moves do
//! (`_dockedPortMissions`, `_MissionHandlePlayerEnteredNewSystem`) waits
//! for missions, and the first stellar is the first of
//! [`PilotCatalog::landing_sites`] until stellars can be destroyed.

use super::Session;
use super::entry::{Arrival, Entry, Placement, ShipPlacement};
use crate::catalog::{PilotCatalog, SoundId, StellarId, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::geometry::Vec2;
use crate::landing::is_dockable;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::sound::SimSound;

/// The disputed rules of the moving set operators that the session
/// follows, projected from its rule set (see [`Session::with_rules`]):
/// the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptEffectRules {
    /// Where `M` puts a ship in flight in a system with no stellar
    /// ([`RuleKey::MoveStarless`]).
    pub starless: RuleSource,
    /// Whether a move is an arrival ([`RuleKey::MoveArrival`]).
    pub arrival: RuleSource,
    /// Whether an `N` in flight keeps the next take-off's landing
    /// position ([`RuleKey::MoveKeepFlag`]).
    pub keep_flag: RuleSource,
    /// Whether a blank `Q` makes the player leave ([`RuleKey::BlankLeave`]).
    pub blank_leave: RuleSource,
    /// When `P`'s sound plays ([`RuleKey::ScriptSound`]).
    pub sound: RuleSource,
}

impl ScriptEffectRules {
    /// The rules `rulebook` chooses: its [`RuleKey::MoveStarless`],
    /// [`RuleKey::MoveArrival`], [`RuleKey::MoveKeepFlag`],
    /// [`RuleKey::BlankLeave`] and [`RuleKey::ScriptSound`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            starless: rulebook.source_for(RuleKey::MoveStarless),
            arrival: rulebook.source_for(RuleKey::MoveArrival),
            keep_flag: rulebook.source_for(RuleKey::MoveKeepFlag),
            blank_leave: rulebook.source_for(RuleKey::BlankLeave),
            sound: rulebook.source_for(RuleKey::ScriptSound),
        }
    }
}

/// A move a set expression queued, for [`Session::settle_script`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScriptMove {
    /// `M`: to the system, at its first stellar.
    To(SystemId),
    /// `N`: to the system, keeping the position.
    KeepPosition(SystemId),
}

/// What the set expressions left for later: the moves and the leave for
/// [`Session::settle_script`], and the sound for the next flight tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Queued {
    /// The moves, in order.
    pub(super) moves: Vec<ScriptMove>,
    /// The message a `Q` leaves on: none for no `Q` pending.
    leave: Option<String>,
    /// The mission sound a `P` holds to the next flight tick.
    pub(super) sound: Option<SoundId>,
}

/// What [`Session::settle_script`] did.
#[must_use]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    /// The system the last move applied went to, if any did.
    pub moved: Option<SystemId>,
    /// The stellar a `Q` made the ship take off from, if it was landed.
    pub took_off: Option<StellarId>,
    /// The message a `Q` shows, if it has one.
    pub message: Option<String>,
}

impl Session {
    /// The rules the moving set operators follow, projected from the
    /// session's rule set ([`Session::with_rules`]).
    fn script_effect_rules(&self) -> ScriptEffectRules {
        ScriptEffectRules::from_rulebook(&self.rules)
    }

    /// Applies what the set expressions run since the last settling
    /// queued (see the module docs): the moves in order, each read from
    /// `catalog`, populating on `chance` where the rules say, then a `Q`'s
    /// leave. Whoever runs set expressions calls it after each input and
    /// tick, before saving.
    pub fn settle_script(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) -> Settled {
        let mut settled = Settled::default();
        for step in std::mem::take(&mut self.queued.moves) {
            let (system, keep) = match step {
                ScriptMove::To(system) => (system, false),
                ScriptMove::KeepPosition(system) => (system, true),
            };
            if catalog.system_exists(system) {
                self.move_to(system, keep, catalog, chance);
                settled.moved = Some(system);
            }
        }
        if let Some(text) = self.queued.leave.take() {
            settled.message = Some(text).filter(|text| !text.is_empty());
            if self.landed.is_some() {
                settled.took_off = self.take_off();
            } else if settled.message.is_some() {
                self.sounds.push(SimSound::ScriptMessage);
            }
        }
        settled
    }

    /// `P`: plays `sound`, held to the next flight tick or at once, as
    /// [`ScriptEffectRules::sound`] says (see the module docs).
    pub(super) fn play_script_sound(&mut self, sound: SoundId) {
        match self.script_effect_rules().sound {
            RuleSource::Engine => self.queued.sound = Some(sound),
            RuleSource::Bible => self.sounds.push(SimSound::Script {
                sound,
                exclusive: false,
            }),
        }
    }

    /// Sounds the mission sound a `P` holds, if any, as the flight's
    /// tick does.
    pub(super) fn sound_script(&mut self) {
        if let Some(sound) = self.queued.sound.take() {
            self.sounds.push(SimSound::Script {
                sound,
                exclusive: true,
            });
        }
    }

    /// `Q`: draws the message to leave on from `STR#` `list` on `chance`
    /// (see the module docs); a blank one cancels an earlier `Q`, or
    /// leaves with no message by [`ScriptEffectRules::blank_leave`]'s
    /// Bible reading.
    pub(super) fn leave_stellar(&mut self, list: i16, chance: &mut dyn Chance) {
        self.queued.leave = self.pick_string(list, chance).or_else(|| {
            (self.script_effect_rules().blank_leave == RuleSource::Bible).then(String::new)
        });
    }

    /// Moves the player to `system`, keeping the position when `keep`
    /// (`N`) or else at its first stellar (`M`), read from `catalog`, and
    /// enters it as a move (see the module docs): only the placement is
    /// chosen here.
    fn move_to(
        &mut self,
        system: SystemId,
        keep: bool,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        let sites = catalog.landing_sites(system);
        let first = sites
            .first()
            .map(|site| (site.position, is_dockable(site).then_some(site.id)));
        let rules = self.script_effect_rules();
        let landed = self.landed.is_some();
        let ship = match first {
            _ if keep => ShipPlacement::Keep,
            Some((position, _)) => ShipPlacement::AtRest(position),
            None if landed || rules.starless == RuleSource::Bible => {
                ShipPlacement::AtRest(Vec2::ZERO)
            }
            None => ShipPlacement::Keep,
        };
        let stellar = first.and_then(|(_, dock)| dock).filter(|_| landed && !keep);
        let placement = Placement {
            ship,
            stellar,
            hold: keep && (landed || rules.keep_flag == RuleSource::Engine),
        };
        let arrival = Arrival {
            system,
            sites,
            placement,
        };
        self.enter_system(Entry::ScriptMove { landed }, arrival, catalog, chance);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{
        DudeId, DudeRecord, GovtId, HullRecord, LandingSite, ShipId, SoundId, StockWeapon,
        SystemTraffic, WeaponId, WeaponRecord,
    };
    use std::rc::Rc;

    use crate::catalog::CommCatalog;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::control::SetExpr;
    use crate::gate::HYPERGATE;
    use crate::handling::ShipFields;
    use crate::hyperspace::JumpRefusal;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::save;
    use crate::session::SessionEvent;
    use crate::testkit::{
        FAST, FakePilotCatalog, Scripted, begin_jump_now, catalog, fly_out, hull, land_now, planet,
        ship, star, weapon,
    };
    use crate::traffic::npc::NpcId;

    /// A system of `avg` ships of düde 128, ship 129, for govt 140.
    fn traffic(system: i16, avg: i16) -> (SystemId, SystemTraffic) {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        (
            SystemId(system),
            SystemTraffic {
                dude_types,
                avg_ships: avg,
                persons: Default::default(),
            },
        )
    }

    /// [`catalog`] with four systems: 130, the start (planets 128 at
    /// (30, -40) and 129), linked to 131; 131, linked to 132, its first
    /// stellar planet 140 at (100, 200), then 141; 132, with no stellar;
    /// and 133, linked to 130 alone, its first stellar the hypergate 150
    /// at (300, 0), then planet 151. Two ships fly in 130 and three in
    /// 131. Ship 129 has a record; ship 130, a fighter of no fuel, too.
    /// The player's ship carries a blaster, outfit 250 holding it.
    fn moving() -> FakePilotCatalog {
        let base = catalog();
        let gate = LandingSite {
            flags2: HYPERGATE,
            ..planet(150, 300.0, 0.0)
        };
        let dry = ShipFields { fuel: 0, ..FAST };
        FakePilotCatalog {
            systems: [130, 131, 132, 133].map(SystemId).to_vec(),
            sites: vec![
                (SystemId(130), base.sites[0].1.clone()),
                (
                    SystemId(131),
                    vec![planet(140, 100.0, 200.0), planet(141, -50.0, 0.0)],
                ),
                (SystemId(133), vec![gate, planet(151, 0.0, 300.0)]),
            ],
            star_map: vec![
                star(130, (0.0, 0.0), &[131]),
                star(131, (600.0, 0.0), &[132]),
                star(132, (600.0, 600.0), &[]),
                star(133, (-600.0, 0.0), &[130]),
            ],
            traffic: vec![traffic(130, 2), traffic(131, 3)],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ships: vec![(ShipId(128), Ok(FAST)), (ShipId(130), Ok(dry))],
            ship_records: vec![ship(129, FAST), ship(130, dry)],
            weapons: vec![WeaponRecord {
                reload: 2,
                count: 30,
                speed: 2000,
                ..weapon(128)
            }],
            hulls: vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 1,
                    ammo: 0,
                }],
                ..hull(128)
            }],
            // The outfit that holds the blaster, so the pilot owns it.
            outfits: [
                base.outfits.clone(),
                vec![crate::testkit::outfit(
                    250,
                    &[(crate::combat::armament::MOD_WEAPON, 128)],
                )],
            ]
            .concat(),
            ..base
        }
    }

    /// An escort of ship class `ship`, a fighter out when `carried`.
    fn escort(ship: i16, carried: bool) -> Escort {
        Escort {
            ship: ShipId(ship),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried,
            wage: None,
            person: None,
        }
    }

    /// `pilot`'s session in flight in 130 among its traffic, no save due.
    fn flying_with(catalog: &FakePilotCatalog, pilot: Pilot) -> Session {
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut NeverFires);
        session.take_save_due();
        session
    }

    fn flying(catalog: &FakePilotCatalog) -> Session {
        flying_with(catalog, Pilot::new(catalog, "Ada").expect("starts"))
    }

    /// [`flying`], landed on planet 128 from the centre.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = flying(catalog);
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session.take_sounds();
        session
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) -> Settled {
        session.settle_script(catalog, &mut NeverFires)
    }

    /// Runs `text` and settles it.
    fn moved(session: &mut Session, catalog: &FakePilotCatalog, text: &str) {
        run(session, text);
        let _ = settle(session, catalog);
    }

    fn ids(sites: &[LandingSite]) -> Vec<i16> {
        sites.iter().map(|site| site.id.0).collect()
    }

    // The rules.

    #[test]
    fn each_rule_follows_its_own_rulebook_entry() {
        let engine = ScriptEffectRules {
            starless: RuleSource::Engine,
            arrival: RuleSource::Engine,
            keep_flag: RuleSource::Engine,
            blank_leave: RuleSource::Engine,
            sound: RuleSource::Engine,
        };
        assert_eq!(ScriptEffectRules::default(), engine);
        assert_eq!(
            ScriptEffectRules::from_rulebook(&Rulebook::default()),
            engine
        );
        let bible = |key| {
            ScriptEffectRules::from_rulebook(
                &Rulebook::default().with_override(key, RuleSource::Bible),
            )
        };
        assert_eq!(
            bible(RuleKey::MoveStarless),
            ScriptEffectRules {
                starless: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::MoveArrival),
            ScriptEffectRules {
                arrival: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::MoveKeepFlag),
            ScriptEffectRules {
                keep_flag: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::BlankLeave),
            ScriptEffectRules {
                blank_leave: RuleSource::Bible,
                ..engine
            }
        );
        assert_eq!(
            bible(RuleKey::ScriptSound),
            ScriptEffectRules {
                sound: RuleSource::Bible,
                ..engine
            }
        );
    }

    // Through run_set.

    #[test]
    fn m_and_n_are_queued_until_settled() {
        let catalog = moving();
        let mut session = flying(&catalog);
        run(&mut session, "M131 N132");
        assert_eq!(session.system(), SystemId(130), "nothing yet");
        assert_eq!(session.take_script_notes(), []);
        assert!(!session.take_save_due());
        assert_eq!(settle(&mut session, &catalog).moved, Some(SystemId(132)));
        assert_eq!(session.system(), SystemId(132));
        assert!(session.take_save_due());
        assert_eq!(settle(&mut session, &catalog), Settled::default(), "once");
        assert!(!session.take_save_due());
    }

    #[test]
    fn the_moves_apply_in_order() {
        let catalog = moving();
        let mut session = flying(&catalog);
        moved(&mut session, &catalog, "M131 N133");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert_eq!(ids(&session.sites), [150, 151]);
    }

    #[test]
    fn a_move_to_a_system_that_is_not_there_changes_nothing() {
        let catalog = moving();
        let mut session = flying(&catalog);
        let before = session.clone();
        run(&mut session, "M150 N999");
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert_eq!(session, before);
        assert!(!session.take_save_due());
    }

    // In flight.

    #[test]
    fn m_in_flight_puts_the_ship_at_rest_on_the_first_stellar() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        session.player.heading = 1.5;
        run(&mut session, "M131");
        assert_eq!(settle(&mut session, &catalog).moved, Some(SystemId(131)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.player().heading, 1.5, "heading kept");
        assert_eq!(ids(&session.sites), [140, 141]);
        assert_eq!(session.landed(), None);
        assert_eq!(session.pilot().stellar(), None);
        assert!(session.take_save_due());
        moved(&mut session, &catalog, "M133");
        assert_eq!(
            session.player().position,
            Vec2::new(300.0, 0.0),
            "a gate as any stellar"
        );
    }

    /// `pilot` saved and loaded, then flown.
    fn reloaded(catalog: &FakePilotCatalog, pilot: &Pilot) -> Session {
        let saved = save::decode(&save::encode(pilot)).expect("loads");
        Session::fly(catalog, saved).expect("flies")
    }

    #[test]
    fn a_move_in_flight_to_another_system_forgets_the_stellar_last_landed_on() {
        let catalog = moving();
        let mut session = landed(&catalog);
        assert_eq!(session.take_off(), Some(StellarId(128)));
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)), "kept");
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.pilot().stellar(), None);
        let flown = reloaded(&catalog, session.pilot());
        assert_eq!(flown.system(), SystemId(131));
        assert_eq!(flown.landed(), None, "in flight");
        assert_eq!(flown.player().position, Vec2::ZERO, "at the centre");
        moved(&mut session, &catalog, "N130");
        assert_eq!(session.pilot().stellar(), None, "not 128 again");
        let flown = reloaded(&catalog, session.pilot());
        assert_eq!(flown.system(), SystemId(130));
        assert_eq!(flown.landed(), None, "not docked at 128");
        assert_eq!(flown.player().position, Vec2::ZERO);
    }

    #[test]
    fn a_move_in_flight_within_the_system_keeps_the_stellar_last_landed_on() {
        let catalog = moving();
        let mut session = landed(&catalog);
        session.take_off();
        moved(&mut session, &catalog, "N130");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        let flown = reloaded(&catalog, session.pilot());
        assert_eq!(flown.landed(), Some(StellarId(128)), "docked again");
    }

    #[test]
    fn n_in_flight_keeps_the_position_and_velocity() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(10.0, 10.0));
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0));
        assert_eq!(ids(&session.sites), [140, 141]);
    }

    #[test]
    fn m_into_a_system_with_no_stellar_keeps_the_ship_moving_by_the_engine() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "M132");
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.player().position, Vec2::new(10.0, 10.0));
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0));
        assert_eq!(session.sites, []);
    }

    #[test]
    fn m_into_a_system_with_no_stellar_stops_at_the_centre_by_the_bible() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveStarless, RuleSource::Bible);
        let mut session = flying(&catalog).with_rules(rules);
        session.player.position = Vec2::new(10.0, 10.0);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "M132");
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        session.player.velocity = Vec2::new(5.0, -1.0);
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.player().velocity, Vec2::new(5.0, -1.0), "N keeps");
    }

    #[test]
    fn a_move_in_flight_lets_go_of_the_scene() {
        let catalog = moving();
        for text in ["M131", "N130"] {
            let mut session = flying(&catalog);
            session.hold_fire(true, false);
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(session.shots().len(), 1, "{text}");
            session.select_stellar(crate::session::StellarPick::Slot(0));
            session.target = Some(NpcId(0));
            session.gate = Some(StellarId(128));
            session.strikes = session.combat.take_strikes();
            moved(&mut session, &catalog, text);
            assert_eq!(session.shots(), [], "{text}");
            assert_eq!(session.nav_target(), None, "{text}");
            assert_eq!(session.target, None, "{text}");
            assert_eq!(session.gate, None, "{text}");
            assert_eq!(session.strikes, [], "{text}");
        }
    }

    #[test]
    fn a_move_in_flight_gives_up_a_jump_being_prepared() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.player.velocity = Vec2::new(0.0, 5.0);
        session.begin_jump().expect("accepted");
        assert!(session.preparing_jump().is_some());
        session.tick(crate::flight::Controls::default());
        assert!(session.thrusting(), "braking");
        moved(&mut session, &catalog, "N133");
        assert_eq!(session.preparing_jump(), None);
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 0);
    }

    #[test]
    fn a_move_in_flight_with_no_jump_to_give_up_leaves_the_thrust_on() {
        let catalog = moving();
        let mut session = flying(&catalog);
        let thrust = crate::flight::Controls {
            thrust: true,
            ..crate::flight::Controls::default()
        };
        session.tick(thrust);
        session.take_sounds();
        moved(&mut session, &catalog, "N131");
        assert!(session.thrusting());
        assert_eq!(session.take_sounds(), [], "no stop sounded");
    }

    #[test]
    fn by_the_engine_only_the_escorts_and_fighters_out_follow_into_another_system() {
        let catalog = moving();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(129, false), escort(130, true)];
        let mut session = flying_with(&catalog, pilot);
        assert_eq!(session.npcs().len(), 4);
        let fleet: Vec<NpcId> = session.fleet.iter().flatten().copied().collect();
        assert_eq!(fleet.len(), 2);
        moved(&mut session, &catalog, "M131");
        let left: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(left, fleet, "the others stay behind");
        assert_eq!(session.pilot().escorts().len(), 2, "the dry fighter too");
        assert_eq!(session.take_fighter_notes(), []);
        assert_eq!(session.traffic.ship_types(), [ShipId(129)], "131's table");
        for _ in 0..3 {
            session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
        }
        assert!(
            session.npcs().iter().all(|npc| fleet.contains(&npc.id)),
            "no initial population"
        );
    }

    #[test]
    fn by_the_engine_a_move_within_the_system_keeps_its_traffic() {
        let catalog = moving();
        let mut session = flying(&catalog);
        let npcs = session.npcs().to_vec();
        moved(&mut session, &catalog, "N130");
        assert_eq!(session.npcs(), npcs);
    }

    #[test]
    fn by_the_engine_the_system_is_not_explored_and_the_course_kept() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        moved(&mut session, &catalog, "M133");
        assert!(!session.pilot().has_explored(SystemId(133)));
        assert_eq!(session.course(), [SystemId(131), SystemId(132)]);
    }

    #[test]
    fn j_refuses_while_the_courses_next_system_is_not_linked_from_here() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "M133");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        session.plot_course(SystemId(131)).expect("a route by 130");
        assert_eq!(session.course(), [SystemId(130), SystemId(131)]);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(130)));
    }

    #[test]
    fn as_an_arrival_the_move_explores_clears_the_course_and_populates() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![escort(129, false), escort(130, true)];
        let mut session = flying_with(&catalog, pilot).with_rules(rules);
        session.plot_course(SystemId(132)).expect("a route");
        moved(&mut session, &catalog, "M131");
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot().escorts().len(), 1, "the dry fighter left");
        assert_eq!(
            session.take_fighter_notes(),
            [crate::bay::FighterNote::Abandoned(1)]
        );
        assert_eq!(session.npcs().len(), 4, "three ships and the escort");
        assert_eq!(session.fleet.iter().flatten().count(), 1);
    }

    // What a move lets go of and keeps, by either reading of
    // `MoveArrival`.

    const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

    /// The rules with `MoveArrival` read as `reading`.
    fn arrival(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::MoveArrival, reading)
    }

    #[test]
    fn a_move_in_flight_lets_go_of_the_boarding_and_the_hail_by_either_reading() {
        let catalog = moving();
        for reading in READINGS {
            for text in ["M131", "N130"] {
                let mut session = flying(&catalog).with_rules(arrival(reading));
                session.stage_scene(NpcId(0));
                moved(&mut session, &catalog, text);
                session.assert_scene_left(&format!("{text} {reading:?}"));
            }
        }
    }

    #[test]
    fn a_move_keeps_a_held_sound_and_sounds_and_raises_no_arrival() {
        let catalog = moving();
        for reading in READINGS {
            for text in ["M131", "N131"] {
                let case = format!("{text} {reading:?}");
                let mut session = flying(&catalog).with_rules(arrival(reading));
                session.take_sounds();
                session.take_messages();
                run(&mut session, "P300");
                moved(&mut session, &catalog, text);
                assert_eq!(session.take_sounds(), [], "in flight: {case}");
                assert_eq!(session.take_messages(), [], "in flight: {case}");
                session.tick(crate::flight::Controls::default());
                assert_eq!(
                    session.take_sounds(),
                    [mission_sound(300, true)],
                    "in flight: {case}"
                );
                let mut session = landed(&catalog).with_rules(arrival(reading));
                session.take_messages();
                run(&mut session, "P300");
                moved(&mut session, &catalog, text);
                assert_eq!(session.take_sounds(), [], "landed: {case}");
                assert_eq!(session.take_messages(), [], "landed: {case}");
                session.take_off();
                assert_eq!(session.take_sounds(), [SimSound::TookOff], "{case}");
                session.tick(crate::flight::Controls::default());
                assert_eq!(
                    session.take_sounds(),
                    [mission_sound(300, true)],
                    "landed: {case}"
                );
            }
        }
    }

    #[test]
    fn as_an_arrival_a_move_within_the_system_populates_it_afresh_on_the_chance_given() {
        let catalog = moving();
        let mut session = flying(&catalog).with_rules(arrival(RuleSource::Bible));
        let before: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        run(&mut session, "N130");
        let mut chance = crate::testkit::Draws::default();
        let _ = session.settle_script(&catalog, &mut chance);
        let after: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(after.len(), 2);
        assert!(
            after.iter().all(|id| !before.contains(id)),
            "{before:?} {after:?}"
        );
        assert!(!chance.asked.is_empty(), "drawn on the chance given");
    }

    #[test]
    fn a_landed_move_leaves_the_traffic_and_the_fighters_out_by_either_reading() {
        let catalog = moving();
        for reading in READINGS {
            let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
            pilot.escorts = vec![escort(129, false), escort(130, true)];
            let mut session = flying_with(&catalog, pilot).with_rules(arrival(reading));
            land_now(&mut session).expect("lands");
            let npcs = session.npcs().to_vec();
            moved(&mut session, &catalog, "M131");
            assert_eq!(session.npcs(), npcs, "{reading:?}");
            assert_eq!(session.pilot().escorts().len(), 2, "{reading:?}");
            assert_eq!(session.take_fighter_notes(), [], "{reading:?}");
            assert_eq!(
                ids(&session.sites),
                [128, 129],
                "the port's until the take-off: {reading:?}"
            );
            session.take_off();
            assert_eq!(ids(&session.sites), [140, 141], "{reading:?}");
        }
    }

    // Landed.

    #[test]
    fn landed_m_moves_the_pilot_and_leaves_the_spaceport_as_it_is() {
        let catalog = moving();
        let mut session = landed(&catalog);
        assert_eq!(session.landed(), Some(StellarId(128)));
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_eq!(session.pilot().stellar(), Some(StellarId(140)));
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
        assert_eq!(ids(&session.sites), [128, 129], "the port's stellars");
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert!(!session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), [SystemId(131)], "kept");
        assert!(session.take_save_due());
    }

    #[test]
    fn landed_m_onto_a_gate_or_into_a_system_with_no_stellar_leaves_the_pilot_on_none() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M133");
        assert_eq!(session.pilot().stellar(), None, "no docking at a gate");
        assert_eq!(session.player().position, Vec2::new(300.0, 0.0));
        moved(&mut session, &catalog, "M131 M132");
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.landed(), Some(StellarId(128)));
    }

    #[test]
    fn a_pilot_saved_after_a_landed_m_flies_again_docked_at_the_new_stellar() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.system(), SystemId(131));
        assert_eq!(flown.landed(), Some(StellarId(140)));
        assert_eq!(flown.player().position, Vec2::new(100.0, 200.0));
    }

    #[test]
    fn the_take_off_after_a_landed_m_is_in_the_new_system_at_its_first_stellar() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.take_off(), Some(StellarId(128)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(ids(&session.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
        assert!(session.pilot().has_explored(SystemId(131)), "explored");
        session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
        assert_eq!(session.npcs().len(), 3, "131 populated");
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M132");
        session.take_off();
        assert_eq!(session.player().position, Vec2::ZERO, "the centre");
        assert_eq!(session.sites, []);
    }

    #[test]
    fn the_take_off_after_a_landed_n_is_from_where_the_ship_touched_down() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        assert_eq!(session.player().position, Vec2::new(30.0, -40.0), "docked");
        moved(&mut session, &catalog, "N131");
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.landed(), Some(StellarId(128)));
        session.take_off();
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(10.0, -10.0));
        assert_eq!(ids(&session.sites), [140, 141]);
        session.player.position = Vec2::new(-45.0, 0.0);
        land_now(&mut session).expect("lands on 141");
        session.take_off();
        assert_eq!(
            session.player().position,
            Vec2::new(-50.0, 0.0),
            "the flag spent"
        );
    }

    #[test]
    fn a_relocation_after_a_landed_n_takes_off_from_the_stellar_moved_to() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        moved(&mut session, &catalog, "N131");
        session
            .relocate(&catalog, SystemId(131), StellarId(141))
            .expect("moves");
        session.take_off();
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            session.player().position,
            Vec2::new(-50.0, 0.0),
            "141's, not where the ship touched down"
        );
    }

    #[test]
    fn as_an_arrival_a_landed_move_explores_and_clears_the_course_at_once() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut session = landed(&catalog).with_rules(rules);
        session.plot_course(SystemId(131)).expect("a route");
        moved(&mut session, &catalog, "N131");
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.course(), []);
        assert_eq!(session.landed(), Some(StellarId(128)));
    }

    #[test]
    fn a_relocation_after_a_landed_move_takes_off_where_it_docked() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M133");
        session
            .relocate(&catalog, SystemId(131), StellarId(141))
            .expect("moves");
        session.take_off();
        assert_eq!(ids(&session.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(-50.0, 0.0));
    }

    // The take-off's flag.

    /// The ship lands from (10, -10) on planet 128, after `text` ran in
    /// flight under `keep_flag`, and takes off: where it is then.
    fn taken_off_after(text: &str, keep_flag: RuleSource) -> Vec2 {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveKeepFlag, keep_flag);
        let mut session = flying(&catalog).with_rules(rules);
        moved(&mut session, &catalog, text);
        session.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        session.take_off();
        session.player().position
    }

    #[test]
    fn by_the_engine_an_n_in_flight_keeps_the_next_take_offs_landing_position() {
        assert_eq!(
            taken_off_after("N130", RuleSource::Engine),
            Vec2::new(10.0, -10.0)
        );
        assert_eq!(
            taken_off_after("M130", RuleSource::Engine),
            Vec2::new(30.0, -40.0),
            "only N"
        );
    }

    #[test]
    fn by_the_other_reading_an_n_in_flight_leaves_the_take_off_as_any_other() {
        assert_eq!(
            taken_off_after("N130", RuleSource::Bible),
            Vec2::new(30.0, -40.0)
        );
    }

    #[test]
    fn by_the_other_reading_a_landed_n_still_keeps_the_landing_position() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::MoveKeepFlag, RuleSource::Bible);
        let mut session = flying(&catalog).with_rules(rules);
        session.player.position = Vec2::new(10.0, -10.0);
        land_now(&mut session).expect("lands");
        moved(&mut session, &catalog, "N130");
        session.take_off();
        assert_eq!(session.player().position, Vec2::new(10.0, -10.0));
    }

    // P.

    fn mission_sound(id: i16, exclusive: bool) -> SimSound {
        SimSound::Script {
            sound: SoundId(id),
            exclusive,
        }
    }

    #[test]
    fn by_the_engine_p_sounds_on_the_next_flight_tick() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.take_sounds();
        run(&mut session, "P300");
        assert_eq!(session.take_sounds(), [], "held");
        assert!(!session.take_save_due());
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [mission_sound(300, true)]);
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "once");
    }

    #[test]
    fn by_the_engine_only_the_last_p_before_the_tick_sounds() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.take_sounds();
        run(&mut session, "P300 P301");
        run(&mut session, "P302");
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [mission_sound(302, true)]);
    }

    #[test]
    fn by_the_engine_a_landed_p_sounds_on_the_first_tick_after_the_take_off() {
        let catalog = moving();
        let mut session = landed(&catalog);
        run(&mut session, "P300");
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "landed");
        session.take_off();
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [mission_sound(300, true)]);
    }

    #[test]
    fn by_the_engine_a_jumps_arrival_drops_a_held_sound() {
        let catalog = moving();
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        run(&mut session, "P300");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        session.take_sounds();
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn by_the_other_reading_every_p_sounds_at_once_landed_or_not() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::ScriptSound, RuleSource::Bible);
        let mut session = landed(&catalog).with_rules(rules);
        run(&mut session, "P300 P301");
        assert_eq!(
            session.take_sounds(),
            [mission_sound(300, false), mission_sound(301, false)]
        );
        session.take_off();
        session.take_sounds();
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.take_sounds(), [], "nothing held");
    }

    // Q.

    /// String lists by ID.
    #[derive(Debug, Default)]
    struct Lists(Vec<(i16, Vec<&'static str>)>);

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.0
                .iter()
                .find(|(list, _)| *list == id)
                .map_or_else(Vec::new, |(_, strings)| {
                    strings.iter().map(|&string| string.to_owned()).collect()
                })
        }
    }

    /// 25048 holds three messages, the last naming the player; 25049 is
    /// empty; 25050 holds an empty string and one that is not.
    fn told(session: Session) -> Session {
        session.with_strings(Rc::new(Lists(vec![
            (25048, vec!["Go.", "Leave now.", "Off you go, <PSN>."]),
            (25049, vec![]),
            (25050, vec!["", "Shoo."]),
        ])))
    }

    /// Runs `text`, rolling `rolls`, and gives the sides asked.
    fn run_rolling(session: &mut Session, text: &str, rolls: &[u16]) -> Vec<u16> {
        let mut chance = Scripted::rolling(rolls);
        session.run_set(&SetExpr::parse(text).expect("parses"), &mut chance);
        chance.sides_asked
    }

    #[test]
    fn a_landed_q_takes_off_with_a_message_drawn_from_its_list() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        assert_eq!(run_rolling(&mut session, "Q25048", &[2]), [3]);
        assert_eq!(session.landed(), Some(StellarId(128)), "nothing yet");
        assert!(!session.take_save_due());
        let settled = settle(&mut session, &catalog);
        assert_eq!(settled.took_off, Some(StellarId(128)));
        assert_eq!(settled.message.as_deref(), Some("Off you go, <PSN>."));
        assert_eq!(settled.moved, None);
        assert_eq!(session.landed(), None);
        assert_eq!(session.take_sounds(), [SimSound::TookOff], "no beep");
        assert!(session.take_save_due());
        assert_eq!(settle(&mut session, &catalog), Settled::default(), "once");
    }

    #[test]
    fn a_q_in_flight_shows_its_message_with_a_beep() {
        let catalog = moving();
        let mut session = told(flying(&catalog));
        session.take_sounds();
        run_rolling(&mut session, "Q25048", &[0]);
        let settled = settle(&mut session, &catalog);
        assert_eq!(settled.message.as_deref(), Some("Go."));
        assert_eq!(settled.took_off, None);
        assert_eq!(session.take_sounds(), [SimSound::ScriptMessage]);
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn by_the_engine_a_blank_q_does_nothing_and_cancels_an_earlier_one() {
        let catalog = moving();
        for (text, rolls, sides) in [
            ("Q25051", &[][..], &[][..]),
            ("Q25049", &[], &[]),
            ("Q25050", &[0], &[2]),
            ("Q25048 Q25049", &[1], &[3]),
        ] {
            let mut session = told(landed(&catalog));
            assert_eq!(run_rolling(&mut session, text, rolls), sides, "{text}");
            assert_eq!(settle(&mut session, &catalog), Settled::default(), "{text}");
            assert_eq!(session.landed(), Some(StellarId(128)), "{text}");
            assert_eq!(session.take_sounds(), [], "{text}");
            let mut session = told(flying(&catalog));
            session.take_sounds();
            run_rolling(&mut session, text, rolls);
            assert_eq!(settle(&mut session, &catalog), Settled::default(), "{text}");
            assert_eq!(session.take_sounds(), [], "{text}");
        }
    }

    #[test]
    fn by_the_bible_a_blank_q_leaves_all_the_same_with_no_message() {
        let catalog = moving();
        let rules = Rulebook::default().with_override(RuleKey::BlankLeave, RuleSource::Bible);
        let mut session = told(landed(&catalog)).with_rules(rules);
        run_rolling(&mut session, "Q25049", &[]);
        let settled = settle(&mut session, &catalog);
        assert_eq!(settled.took_off, Some(StellarId(128)));
        assert_eq!(settled.message, None);
        assert_eq!(session.landed(), None);
        let mut session = told(flying(&catalog)).with_rules(rules);
        session.take_sounds();
        run_rolling(&mut session, "Q25050", &[0]);
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert_eq!(session.take_sounds(), [], "nothing to show");
        let mut session = told(landed(&catalog)).with_rules(rules);
        run_rolling(&mut session, "Q25048", &[1]);
        let settled = settle(&mut session, &catalog);
        assert_eq!(settled.message.as_deref(), Some("Leave now."), "as usual");
    }

    #[test]
    fn a_landed_move_and_q_take_off_in_the_system_moved_to() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        run_rolling(&mut session, "M131 Q25048", &[0]);
        let settled = settle(&mut session, &catalog);
        assert_eq!(settled.moved, Some(SystemId(131)));
        assert_eq!(settled.took_off, Some(StellarId(128)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(ids(&session.sites), [140, 141]);
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0));
    }

    // Events.

    #[test]
    fn a_move_in_flight_tells_the_system_changed_even_within_the_system() {
        let catalog = moving();
        for text in ["M131", "N130", "M130"] {
            let mut session = flying(&catalog);
            moved(&mut session, &catalog, text);
            assert_eq!(
                session.take_events(),
                [SessionEvent::SystemChanged],
                "{text}"
            );
        }
        let mut session = flying(&catalog);
        moved(&mut session, &catalog, "M999");
        assert_eq!(session.take_events(), [], "no such system");
    }

    #[test]
    fn a_landed_move_to_another_system_tells_it_at_the_take_off() {
        let catalog = moving();
        let mut session = landed(&catalog);
        moved(&mut session, &catalog, "M131");
        assert_eq!(session.take_events(), [], "the stellars wait");
        session.take_off();
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
    }

    #[test]
    fn a_landed_move_within_the_system_or_back_to_it_tells_nothing() {
        let catalog = moving();
        for text in ["N130", "M130", "M131 M130"] {
            let mut session = landed(&catalog);
            moved(&mut session, &catalog, text);
            assert_eq!(session.take_events(), [], "{text}: settled");
            session.take_off();
            assert_eq!(session.take_events(), [], "{text}: taken off");
        }
        let mut session = landed(&catalog);
        session.take_off();
        assert_eq!(session.take_events(), [], "a plain take-off");
    }

    #[test]
    fn a_landed_move_and_q_tell_the_system_changed_as_they_settle() {
        let catalog = moving();
        let mut session = told(landed(&catalog));
        run_rolling(&mut session, "M131 Q25048", &[0]);
        let _ = settle(&mut session, &catalog);
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
    }

    #[test]
    fn without_string_lists_a_q_does_nothing() {
        let catalog = moving();
        let mut session = landed(&catalog);
        assert_eq!(run_rolling(&mut session, "Q25048", &[]), Vec::<u16>::new());
        assert_eq!(settle(&mut session, &catalog), Settled::default());
        assert_eq!(session.landed(), Some(StellarId(128)));
    }
}
