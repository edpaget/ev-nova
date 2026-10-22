//! Cloaking: how a ship fades into its cloak and out again, what makes an
//! NPC cloak or uncloak, and whether the player sees a cloaked ship.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova` each ship keeps a fade level
//! (+0x64, a float) and a fade direction (+0xc8d8).
//!
//! - **Cloaked** ([`Cloak::is_cloaked`], `_ShipIsCloaked` @0x255f): the
//!   fade is above [`CLOAKED_RISING`] (16) with the direction at or above
//!   0, or above [`CLOAKED_FALLING`] (8) with it below 0. Its 24 test
//!   (@0xdd040) is subsumed.
//! - **The fade** ([`Cloak::step`], `_HandleShipDisplay`
//!   @0x2d088-0x2d164, once a frame, right after the ship's own
//!   `_HandleShip` and so after its pick; the traffic's `step_fades`,
//!   which the session runs after the NPCs' step): with a direction it moves by
//!   [`FADE_RATE`] (0.75) a tick that way, or [`FAST_FADE_RATE`] (1.5)
//!   for a ship type whose `Flags2` has [`FAST_FADE`] (shïp +0x9ea bit
//!   0x1, @0x2d0aa). Going down it stops at 0, and going up at
//!   [`FADE_FULL`] (32), the direction then 0. With no direction a fade
//!   between 0 and 32 drifts down by [`DRIFT`] (1) a tick. A ship starts
//!   at 0, with no direction. `_PrepareAllSprites` (@0x3a776-0x3a78e)
//!   steps it too, for every ship, before the first frame of an entry
//!   (`nova_sim::Session::prepare_all_sprites`): twice at a take-off or
//!   gate exit, once at a jump's arrival.
//! - **`_DoShipCloak`** ([`Cloak::cloak`], @0xde73): nothing when cloaked;
//!   else a fade at or below 0 starts fading in, and a fade going down
//!   stops (direction 0, so it drifts down). **`_DoShipUncloak`**
//!   ([`Cloak::uncloak`], @0xdf55): nothing unless cloaked; then a fade
//!   above 0 starts fading out (its other branch, stopping a fade at 0
//!   going in, cannot follow: a cloaked fade is above 8). So a ship
//!   cloaking from 0 reads cloaked only past 16, and an uncloak asked
//!   before then does nothing.
//! - **The sounds**: `_DoShipCloak` plays [`CLOAK_SOUND`] (381,
//!   `cloakSnd[1]`, @0xdec0-0xdef3) when the fade was not already going
//!   in, and `_DoShipUncloak` [`UNCLOAK_SOUND`] (380, `cloakSnd[0]`,
//!   @0xdf9e-0xdfd0) when it was not already going out, each at priority
//!   8 and only for the player (+0x86 0). [`Cloak::cloak`] and
//!   [`Cloak::uncloak`] say whether it would play; no NPC's cloak ever
//!   sounds.
//! - **Dying** ([`Cloak::display_step`], @0x2d169-0x2d17e): after the
//!   step, a dying ship (`_IsDying`) with any fade left takes direction
//!   [`DYING_DIRECTION`] (-2), whatever it was, so it falls at twice the
//!   rate from the next step.
//! - **Drawn** ([`CloakShown`]): with no fade a ship is drawn as it is;
//!   fading, every ship is drawn jittered and faint, whoever leads it
//!   (@0x2d2c0-0x2d648); fully faded, at [`full_fade_translucency`]
//!   (@0x2d187-0x2d2bb). The view's `cloak_fade` module draws each.
//!
//! What an NPC's ship type gives ([`CloakGear`]) comes from its default
//! items and its `Flags2`: a cloak (an outfit with a `ModType` 17 mod,
//! `_HasCloak` @0x95f2), whether it decloaks when hit (that mod's
//! `ModVal` bit 0x8, `_ShipDecloaksWhenHit` @0x3f68), whether it shows on
//! the radar while cloaked (bit 0x2, `_ShipCloakVisibleOnRadar` @0x40f8),
//! and the AI-cloak bits of `Flags2` (shïp +0x9ea, loaded from the
//! resource's offset 0x62, `_LoadObjectData` @0x7a1f2).
//!
//! # When an NPC cloaks
//!
//! Each think, `_AIHandleCloaking` (@0x833e1, [`think`]) uncloaks a ship
//! with no cloak or a disabled one (@0x833f3-0x8341d), and otherwise
//! cloaks it while one of its ship type's AI-cloak conditions holds and
//! uncloaks it while none does (@0x83682, @0x83694). The conditions OR
//! together, in the original's order:
//!
//! - [`CLOAK_RELOADING`] (`Flags2` 0x0100, @0x8345d-0x834b3): while a
//!   weapon it carries reloads past its `Reload`
//!   ([`Situation::reloading`]; the Nova Bible reads it as a burst
//!   reload, a rulebook choice the caller makes).
//! - [`CLOAK_RETREATING`] (`Flags2` 0x0200, @0x834b7-0x834e3): while it
//!   retreats (AI state 3, [`Situation::retreating`]), or as a miner
//!   chasing its rock (state 0x10, order 0x13, [`MineOrder::Chase`]).
//! - [`CLOAK_JUMPING`] (`Flags2` 0x0400, @0x834e6-0x8350b): through its
//!   own jump (sub-goal 4), a hyperspace arrival or gate burst (goal 8),
//!   or a jump with its parent (goal 0xb) ([`Situation::jumping`]).
//! - [`CLOAK_TRAVELLING`] (`Flags2` 0x0800, @0x8350f-0x83542): while it
//!   lands, waits, enters a gate, inspects or flies in formation (goals
//!   1, 6, 0x14, 7 and 0xa, [`Situation::travelling`]).
//! - [`CLOAK_PRESSING`] (`Flags2` 0x1000, @0x83546-0x835fd): once
//!   cloaked, while it attacks a ship more than [`PRESS_RANGE`] (165)
//!   away on either axis ([`Situation::attacking`]); closer, it uncloaks
//!   to fight. Once cloaked, too, while it mines on any order but
//!   [`MineOrder::Close`] (@0x835dd-0x835fd): this keeps a miner's cloak.
//! - [`CLOAK_IDLE`] (`Flags2` 0x2000, shïp +0x9eb bit 0x20,
//!   @0x83605-0x83630): while it idles (goal 0, [`Situation::idle`]; the
//!   Bible says "when docking"). No stock ship carries it.
//! - [`CLOAK_WHEN_HIT`] (`Flags2` 0x4000, `_DamageShip` @0x3bb95-0x3bbdd,
//!   [`hit`]): hit by a ship it is not attacking (`_IsShipFightingMe`
//!   @0x827fa), while it can cloak.
//!
//! Whatever those say, a ship of AI type above 4 keeping close formation
//! (sub-goal 0xc) with a cloaked parent cloaks
//! ([`Situation::close_to_cloaked_parent`], @0x83633-0x8366d). The think
//! raises no sound: its cloak and uncloak sound only for the player.
//!
//! Every frame, too, a cloaked ship that can no longer cloak
//! ([`CloakGear::can_cloak`], `_ShipCanCloak` @0x553c: not disabled, a
//! cloak, and the reserves its cloak needs by
//! [`RuleKey::CloakReserves`](crate::RuleKey::CloakReserves), fuel alone
//! by the engine; and for the player, not jumping unless its `Flags2` has
//! [`CLOAK_JUMPING`]) uncloaks (`_HandleShip` @0x3413d-0x3415f,
//! [`hold`]), and a weapon's hit uncloaks a ship whose cloak decloaks when
//! hit while it is fully faded or still fading in (`_DamageShip`
//! @0x3bf1f-0x3bf4f, [`hit`], [`decloak_on_hit`]).
//!
//! # Upkeep
//!
//! A cloak's first `ModType` 17 mod among a ship type's default items, in
//! slot order ([`CloakGear::with_upkeep`]), sets its shield usage
//! (`_CloakShieldUsage` @0x4033, `ModVal` >> 8 & 0xf) and whether
//! cloaking zeroes the shield (`_CloakZeroesShields` @0x3e9d, bit
//! [`CLOAK_ZEROES_SHIELDS`]). `_DoShipCloak` on a ship not yet cloaked
//! zeroes the shield of a zeroing cloak ([`cloak_ship`], @0xdf1f-0xdf3d),
//! however it cloaks: a think, a hit, or set up cloaked. A think uncloaks
//! a ship whose cloak costs shield with none left, before the AI-cloak
//! conditions (@0x83449-0x83452); and every frame a ship cloaked at its
//! start pays usage x [`CLOAK_UPKEEP_RATE`] while its shield is at least
//! its usage ([`drain_shield`], `_HandleShip` @0x341af-0x34204), after
//! the hold, so even a ship the hold uncloaks pays that frame. Stock: the
//! Polaris Cloaking Organs 268 (usage 8) and 269 (4) zero nothing.
//!
//! # The player's cloak
//!
//! The player's gear reads the outfits it owns, in outfit order
//! ([`CloakGear::player`]): the first owned cloak gives its usages, its
//! zeroing and its decloak, and the last one the mod `_ShipCanCloak`
//! reads. Its key and upkeep are [`player_cloak`](crate::player_cloak)'s,
//! held through `nova_sim::Session::hold_cloak`; at a full fade it is
//! always drawn faint ([`shown`] with [`SEEN_AT_FULL_FADE`],
//! @0x2d199-0x2d1a1), and it alone hears its cloak's sounds. NPCs see it
//! by [`visible`] with the player the target (`nova_sim_ai::ai::sight`):
//! a cloaked player is no target, scan or hail for an NPC that can't see
//! it, and point defence and homing shots ask [`seen`].
//!
//! # Not modelled
//!
//! The think's 30-59 frame delay after a ship's hyperspace glide
//! (`_HandleShip` @0x34d74); a cloak's fuel upkeep (the uncloak @0x83422-0x8343a and the drain @0x34164-0x341ac),
//! which no NPC pays (`_CloakFuelUsage` @0x3e25 is 0 for one,
//! @0x3e31-0x3e39); `_DoShipCloak`'s `_statusRedraw[1]` (@0xdf35-0xdf4b),
//! player-only, as the HUD is drawn every frame; area cloaks
//! (rdm:phase/cloaking-fidelity/phase-10-area-cloaks).
//!
//! Left out of `_HandleShipDisplay`'s drawing (@0x2d088-0x2d72e): the
//! 8-bit depth, its hull at translucency 0x10 (@0x2d341-0x2d34c) and its
//! weapon glow hidden (@0x2d667-0x2d681), as the view draws 16-bit only;
//! the ship's paint (`_GetShipPaintColor` @0x7d0c-0x7da7), as the engine
//! draws none (rdm:task/ship-paint-tint-and-capture): the factors take
//! paint as an argument and the view passes unpainted (32, 32, 32); and
//! the step's frame-time factor (the double at 0x3b511c, @0x2d0d2-0x2d0df),
//! as the engine's tick already models frame time.
//!
//! # Seeing a cloaked ship
//!
//! `_ShipVisibleToShip(target, observer)` (@0x971c, [`visible`]): a ship
//! emerging from a hypergate is unseen; an uncloaked one is seen; a
//! cloaked one is seen by an observer escorting it that has a cloak
//! itself (@0x9788-0x97a2, never the player, which has no parent), and
//! otherwise only by an observer with a cloak scanner on screen
//! ([`Scanner::on_screen`], `_ShipCanSeeCloakedShips` mode 0 @0x41a9, a
//! `ModType` 30 mod with `ModVal` bit 0x2) within [`SIGHT_RANGE`] (200)
//! on both axes (@0x97a4-0x97f8). No stock outfit has a `ModType` 30
//! mod, so the stock player never sees a cloaked ship. The player's
//! targeting and radar ask the cloak itself rather than this test (see
//! [`Scanner`]).
//!
//! A mission's observe goal on a ship that can cloak is met only while it
//! is also on screen ([`on_screen`], `_MissionObjectivesCheck`
//! @0x9ea81-0x9eb09).

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

/// The fade above which a ship fading in, or holding, is cloaked.
pub const CLOAKED_RISING: f32 = 16.0;
/// The fade above which a ship fading out is still cloaked.
pub const CLOAKED_FALLING: f32 = 8.0;
/// The fade of a ship fully cloaked, where fading in stops.
pub const FADE_FULL: f32 = 32.0;
/// How far the fade moves a tick.
pub const FADE_RATE: f32 = 0.75;
/// How far the fade moves a tick for a ship type with [`FAST_FADE`].
pub const FAST_FADE_RATE: f32 = 1.5;
/// How far a fade with no direction drifts down a tick.
pub const DRIFT: f32 = 1.0;
/// How near, on each axis, a cloaked ship is seen by a ship with a
/// cloak scanner.
pub const SIGHT_RANGE: f32 = 200.0;
/// How far, on either axis, a cloaked ship pressing an attack
/// ([`CLOAK_PRESSING`]) keeps its cloak.
pub const PRESS_RANGE: f32 = 165.0;
/// The share of a ship's sprite, on each side, it must be inside the view
/// to be on screen.
pub const ON_SCREEN_INSET: f32 = 0.75;

/// The fade direction `_HandleShipDisplay` gives a dying ship with any
/// fade left (@0x2d17e): it falls at twice the rate.
pub const DYING_DIRECTION: i8 = -2;
/// The `snd ` `_DoShipCloak` plays as the player cloaks (`cloakSnd[1]`,
/// @0xdec0-0xdef3), at priority 8.
pub const CLOAK_SOUND: i16 = 381;
/// The `snd ` `_DoShipUncloak` plays as the player uncloaks
/// (`cloakSnd[0]`, @0xdf9e-0xdfd0), at priority 8.
pub const UNCLOAK_SOUND: i16 = 380;

/// The `ModType` of a cloaking device.
pub const MOD_CLOAK: i16 = 17;
/// The `ModType` of a cloak scanner.
pub const MOD_SCANNER: i16 = 30;
/// A cloak's `ModVal` bit: it shows on the radar while cloaked.
pub const CLOAK_ON_RADAR: i16 = 0x0002;
/// A cloak's `ModVal` bit: cloaking zeroes the shield
/// (`_CloakZeroesShields` @0x3edb).
pub const CLOAK_ZEROES_SHIELDS: i16 = 0x0004;
/// What a cloak's shield usage costs a tick while cloaked, per unit of
/// usage (the double at 0xdd600, `_HandleShip` @0x341af-0x34204; its
/// frame-time factor, 0x3b511c, the engine's tick models).
pub const CLOAK_UPKEEP_RATE: f32 = 0.03333;
/// A cloak's `ModVal` bit: a hit uncloaks it.
pub const CLOAK_DECLOAKS_WHEN_HIT: i16 = 0x0008;
/// A scanner's `ModVal` bit: cloaked ships show on the radar.
pub const SCAN_RADAR: i16 = 0x0001;
/// A scanner's `ModVal` bit: cloaked ships show on screen, within
/// [`SIGHT_RANGE`].
pub const SCAN_SCREEN: i16 = 0x0002;
/// A scanner's `ModVal` bit: ships of an [`UNTARGETABLE`] type can be
/// targeted (`_ShipCanTargetUntargetableShips` @0x238f).
pub const SCAN_UNTARGETABLE: i16 = 0x0004;
/// A scanner's `ModVal` bit: cloaked ships can be targeted.
pub const SCAN_TARGET: i16 = 0x0008;

/// The `Flags2` bit that fades a ship type's cloak at [`FAST_FADE_RATE`].
pub const FAST_FADE: u16 = 0x0001;
/// The `Flags2` bit that makes a ship type untargetable without a
/// scanner's [`SCAN_UNTARGETABLE`] (see [`CloakGear::untargetable`]).
pub const UNTARGETABLE: u16 = 0x0004;
/// The `Flags2` bit that cloaks a ship while one of its weapons reloads
/// past its `Reload` ([`Situation::reloading`]).
pub const CLOAK_RELOADING: u16 = 0x0100;
/// The `Flags2` bit that cloaks a ship while it retreats, or while it
/// chases its rock as a miner.
pub const CLOAK_RETREATING: u16 = 0x0200;
/// The `Flags2` bit that cloaks a ship through a jump
/// ([`Situation::jumping`]).
pub const CLOAK_JUMPING: u16 = 0x0400;
/// The `Flags2` bit that cloaks a ship while it travels: lands, waits,
/// enters a gate, inspects or flies in formation
/// ([`Situation::travelling`]).
pub const CLOAK_TRAVELLING: u16 = 0x0800;
/// The `Flags2` bit (shïp +0x9eb bit 0x20) that cloaks a ship while it
/// idles ([`Situation::idle`]).
pub const CLOAK_IDLE: u16 = 0x2000;
/// The `Flags2` bit that keeps a ship cloaked while it presses an attack
/// from beyond [`PRESS_RANGE`], or while it mines as a miner on any
/// [`MineOrder`] but [`MineOrder::Close`].
pub const CLOAK_PRESSING: u16 = 0x1000;
/// The `Flags2` bit that cloaks a ship hit by a ship it is not
/// attacking.
pub const CLOAK_WHEN_HIT: u16 = 0x4000;

/// The steps of a sprite's translucency: a level of this or more is not
/// drawn (`_BlitPixieRLETranslucentDrawProc` @0xbb5b6-0xbb5d8 draws
/// nothing above 0x3f).
pub const TRANSLUCENCY_STEPS: u8 = 0x40;
/// The translucency, in 64ths, of a fully cloaked ship the player still
/// sees: one it leads, or any ship while it scans for cloaked ships on
/// screen (`_HandleShipDisplay` @0x2d1a3-0x2d1cd).
pub const SEEN_AT_FULL_FADE: u8 = 0x1e;

/// How a ship's cloak lets the player's screen show it
/// (`_HandleShipDisplay` @0x2d169-0x2d648).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CloakShown {
    /// Drawn as it is: no fade (at or below 0, @0x2d169-0x2d16c).
    Opaque,
    /// Fading, at this fade, above 0 and below [`FADE_FULL`]: drawn
    /// jittered and faint, whoever leads it (@0x2d2c0-0x2d648).
    Fading(f32),
    /// Fully faded and drawn translucent, at this level in 64ths: always
    /// below [`TRANSLUCENCY_STEPS`] (@0x2d187-0x2d1cd).
    Translucent(u8),
    /// Fully faded and its hull not drawn (translucency
    /// [`TRANSLUCENCY_STEPS`]); its weapon glow, when lit, still is.
    Hidden,
}

/// A fully cloaked ship's translucency, in 64ths (`_HandleShipDisplay`
/// @0x2d187-0x2d1cd): [`SEEN_AT_FULL_FADE`] when the player leads it
/// (`led`) or scans for cloaked ships on screen (`scanner`), otherwise
/// [`TRANSLUCENCY_STEPS`], not drawn.
#[must_use]
pub fn full_fade_translucency(led: bool, scanner: bool) -> u8 {
    if led || scanner {
        SEEN_AT_FULL_FADE
    } else {
        TRANSLUCENCY_STEPS
    }
}

/// A ship's cloak: its fade level and the way it is fading.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cloak {
    /// The fade level, 0 (seen) to [`FADE_FULL`].
    pub fade: f32,
    /// The way the fade moves: 1 in, -1 out ([`DYING_DIRECTION`], -2,
    /// for a dying ship), 0 neither.
    pub direction: i8,
}

impl Cloak {
    /// Whether it is cloaked (see the module docs).
    #[must_use]
    pub fn is_cloaked(self) -> bool {
        if self.direction < 0 {
            self.fade > CLOAKED_FALLING
        } else {
            self.fade > CLOAKED_RISING
        }
    }

    /// Starts cloaking, as `_DoShipCloak` does (see the module docs);
    /// and whether the cloak's sound would play: when it was not cloaked
    /// and not already fading in (`jg` @0xded2). The original plays it
    /// only for the player ([`CLOAK_SOUND`]); the caller raises it.
    pub fn cloak(&mut self) -> bool {
        if self.is_cloaked() {
            return false;
        }
        let sounds = self.direction <= 0;
        if self.fade <= 0.0 {
            self.direction = 1;
        } else if self.direction < 0 {
            self.direction = 0;
        }
        sounds
    }

    /// Starts uncloaking, as `_DoShipUncloak` does (see the module docs);
    /// and whether the decloak sound would play: when it was cloaked and
    /// not already fading out (`js` @0xdfb0). The original plays it only
    /// for the player ([`UNCLOAK_SOUND`]); the caller raises it.
    pub fn uncloak(&mut self) -> bool {
        // The original sets direction -1 for a fade above 0, else stops a
        // fade going in (@0xdfda-0xdff8); a cloaked ship's fade is above 8,
        // so only the first can happen.
        if !self.is_cloaked() {
            return false;
        }
        let sounds = self.direction >= 0;
        self.direction = -1;
        sounds
    }

    /// One frame of `_HandleShipDisplay`'s fade: [`Cloak::step`], then,
    /// for a `dying` ship (`_IsDying`) with any fade left, the direction
    /// [`DYING_DIRECTION`] whatever it was (@0x2d169-0x2d17e), so the
    /// next step falls at twice the rate.
    pub fn display_step(&mut self, fast: bool, dying: bool) {
        self.step(fast);
        if dying && self.fade > 0.0 {
            self.direction = DYING_DIRECTION;
        }
    }

    /// Whether it is fully faded ([`FADE_FULL`]).
    #[must_use]
    pub fn full(self) -> bool {
        self.fade >= FADE_FULL
    }

    /// One tick of the fade, at [`FAST_FADE_RATE`] when `fast`, else
    /// [`FADE_RATE`] (see the module docs).
    pub fn step(&mut self, fast: bool) {
        let rate = if fast { FAST_FADE_RATE } else { FADE_RATE };
        // Matching on the direction's sign, rather than testing it for 0
        // and again for each clamp, leaves no `< 0` or `> 0` whose `<= 0`
        // or `>= 0` mutant chooses the same.
        match self.direction.cmp(&0) {
            Ordering::Equal => {
                if self.fade > 0.0 && self.fade < FADE_FULL {
                    self.fade -= DRIFT;
                }
            }
            Ordering::Less => {
                self.fade += f32::from(self.direction) * rate;
                if self.fade <= 0.0 {
                    self.fade = 0.0;
                    self.direction = 0;
                }
            }
            Ordering::Greater => {
                self.fade += f32::from(self.direction) * rate;
                if self.fade >= FADE_FULL {
                    self.fade = FADE_FULL;
                    self.direction = 0;
                }
            }
        }
    }
}

/// Whether a ship type of `flags2` is untargetable ([`UNTARGETABLE`]):
/// only a searcher whose scanner has [`SCAN_UNTARGETABLE`] targets it.
#[must_use]
pub fn untargetable(flags2: u16) -> bool {
    flags2 & UNTARGETABLE != 0
}

/// What a ship type gives its cloak, and its sight of other ships': from
/// its default items and `Flags2`.
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloakGear {
    /// Whether it has a cloak (a `ModType` 17 mod).
    pub device: bool,
    /// Whether a hit uncloaks it ([`CLOAK_DECLOAKS_WHEN_HIT`]): the bit of
    /// the first cloak mod, of the first owned cloak outfit for the player
    /// or the first cloak default-item slot for an NPC
    /// (`_ShipDecloaksWhenHit` @0x3f71-0x3faf, @0x3fcd-0x4027); every
    /// cloak mod's bit, any of them set, for a gear built by
    /// [`CloakGear::of`] alone.
    pub decloaks_when_hit: bool,
    /// Whether it shows on the radar while cloaked ([`CLOAK_ON_RADAR`]).
    pub on_radar: bool,
    /// Its `Flags2`, whose AI-cloak bits, [`FAST_FADE`] and
    /// [`UNTARGETABLE`] it reads.
    pub flags2: u16,
    /// The shield its cloak costs, a tick, while cloaked
    /// (`_CloakShieldUsage` @0x4033: `(ModVal >> 8) & 0xf` of the first
    /// cloak, see [`CloakGear::with_upkeep`] and [`CloakGear::player`]),
    /// times [`CLOAK_UPKEEP_RATE`].
    pub shield_usage: u8,
    /// The fuel its cloak costs, a tick, while cloaked, times
    /// [`CLOAK_UPKEEP_RATE`]: the player's first owned cloak's
    /// `(ModVal >> 4) & 0xf` (`_CloakFuelUsage` @0x3e25-0x3e73). It is
    /// always 0 for an NPC (@0x3e31-0x3e39).
    pub fuel_usage: u8,
    /// Whether cloaking zeroes its shield (`_CloakZeroesShields` @0x3e9d:
    /// the first cloak's `ModVal` bit [`CLOAK_ZEROES_SHIELDS`]).
    pub zeroes_shields: bool,
    /// The `ModVal` of the cloak mod `_ShipCanCloak` reads, by the Bible's
    /// reading of [`RuleKey::CloakReserves`](crate::RuleKey::CloakReserves):
    /// the first cloak mod of the **last** owned cloak outfit, in outfit
    /// order (the player, @0x556b-0x55ca), or of the last default-item slot
    /// holding a cloak (an NPC, @0x55cc-0x5640). `None` with no cloak.
    pub can_cloak_mod: Option<i16>,
    /// What the cloak scanners among its default items give: an NPC's
    /// own scanner (`_ShipCanTargetUntargetableShips` @0x2402 and
    /// `_ShipCanTargetCloakedShips` @0x24d9 read its ship type's default
    /// items, where the player's read the outfits it owns).
    pub scanner: Scanner,
}

impl CloakGear {
    /// The gear of a ship type of `flags2` carrying `items` of `outfits`:
    /// any cloak among them sets its `ModVal` bits (a stock ship carries
    /// one).
    #[must_use]
    pub fn of(flags2: u16, items: &BTreeMap<OutfitId, u16>, outfits: &[OutfitRecord]) -> Self {
        let mut gear = Self {
            flags2,
            scanner: Scanner::of(items, outfits),
            ..Self::default()
        };
        for mod_val in mods(items, outfits, MOD_CLOAK) {
            gear.device = true;
            gear.decloaks_when_hit |= mod_val & CLOAK_DECLOAKS_WHEN_HIT != 0;
            gear.on_radar |= mod_val & CLOAK_ON_RADAR != 0;
        }
        gear
    }

    /// The gear with its cloak's upkeep read from `slots`, a ship type's
    /// default items in slot order: the first slot holding any (count
    /// above 0) whose outfit has a cloak mod, and that outfit's first
    /// cloak mod, decides (`_CloakShieldUsage` @0x40a7-0x40ef,
    /// `_CloakZeroesShields` @0x3f02-0x3f5c). An NPC's cloak costs no fuel
    /// (`_CloakFuelUsage` @0x3e31-0x3e39).
    ///
    /// The first slot's cloak mod also decides whether a hit decloaks it
    /// (`_ShipDecloaksWhenHit`'s NPC branch @0x3fcd-0x4027), and the last
    /// slot's sets [`CloakGear::can_cloak_mod`] (`_ShipCanCloak`
    /// @0x55cc-0x5640).
    #[must_use]
    pub fn with_upkeep(self, slots: &[(OutfitId, u16)], outfits: &[OutfitRecord]) -> Self {
        let cloaks: Vec<i16> = slots
            .iter()
            .filter(|&&(_, count)| count > 0)
            .filter_map(|&(id, _)| outfits.iter().find(|record| record.id == id))
            .filter_map(first_cloak_mod)
            .collect();
        let (Some(&first), Some(&last)) = (cloaks.first(), cloaks.last()) else {
            return self;
        };
        Self {
            shield_usage: shield_usage(first),
            zeroes_shields: first & CLOAK_ZEROES_SHIELDS != 0,
            decloaks_when_hit: first & CLOAK_DECLOAKS_WHEN_HIT != 0,
            can_cloak_mod: Some(last),
            ..self
        }
    }

    /// The player's gear: its ship type's `flags2` and the outfits it
    /// owns of `outfits`, walked in outfit order, which is by ID (the
    /// original's 512 owned-outfit slots). Any owned cloak is a device
    /// (`_HasCloak` @0x95fd-0x9647). The **first** owned cloak outfit's
    /// first cloak mod gives its usages, its zeroing and its decloak
    /// (`_CloakShieldUsage` @0x4046, `_CloakFuelUsage` @0x3e25,
    /// `_CloakZeroesShields` @0x3eb0, `_ShipDecloaksWhenHit`
    /// @0x3f71-0x3faf, which answers from it without looking further,
    /// @0x3fad), and its radar bit, which nothing reads for the player;
    /// the **last** one's sets [`CloakGear::can_cloak_mod`]
    /// (`_ShipCanCloak` @0x556b-0x55ca).
    #[must_use]
    pub fn player(flags2: u16, owned: &BTreeMap<OutfitId, u16>, outfits: &[OutfitRecord]) -> Self {
        let gear = Self {
            flags2,
            scanner: Scanner::of(owned, outfits),
            ..Self::default()
        };
        let cloaks: Vec<i16> = owned
            .iter()
            .filter(|&(_, &count)| count > 0)
            .filter_map(|(&id, _)| outfits.iter().find(|record| record.id == id))
            .filter_map(first_cloak_mod)
            .collect();
        let (Some(&first), Some(&last)) = (cloaks.first(), cloaks.last()) else {
            return gear;
        };
        Self {
            device: true,
            decloaks_when_hit: first & CLOAK_DECLOAKS_WHEN_HIT != 0,
            on_radar: first & CLOAK_ON_RADAR != 0,
            shield_usage: shield_usage(first),
            fuel_usage: nibble(first, 4),
            zeroes_shields: first & CLOAK_ZEROES_SHIELDS != 0,
            can_cloak_mod: Some(last),
            ..gear
        }
    }

    /// Whether its fade moves at [`FAST_FADE_RATE`].
    #[must_use]
    pub fn fast(self) -> bool {
        self.flags2 & FAST_FADE != 0
    }

    /// Whether a ship of it is untargetable ([`untargetable`]).
    #[must_use]
    pub fn untargetable(self) -> bool {
        untargetable(self.flags2)
    }

    /// Whether its `Flags2` has the `bit`.
    fn has(self, bit: u16) -> bool {
        self.flags2 & bit != 0
    }

    /// Whether a ship of it can cloak, as `_ShipCanCloak` (@0x553c-0x56b0)
    /// asks under `reserves` ([`RuleKey::CloakReserves`](crate::RuleKey::CloakReserves)):
    ///
    /// 1. a disabled ship can't (@0x554b);
    /// 2. nor one with no cloak (@0x5642);
    /// 3. a fuel nibble with fuel at or below 0 can't, nor a shield nibble
    ///    with shield at or below 0 (@0x5648-0x566a). By the engine the
    ///    nibbles come from the `ModType` word, [`MOD_CLOAK`] (0x11), so
    ///    every cloak needs fuel and none shield; by the Bible from
    ///    [`CloakGear::can_cloak_mod`];
    /// 4. a ship `jumping` (the player's jump timer +0x50 above 0, never
    ///    set for an NPC) can't unless its `Flags2` has [`CLOAK_JUMPING`]
    ///    (shïp +0x9eb bit 0x04, @0x566c-0x569e).
    #[must_use]
    pub fn can_cloak(self, check: CloakCheck, reserves: RuleSource) -> bool {
        if check.disabled || !self.device {
            return false;
        }
        let word = match reserves {
            RuleSource::Engine => MOD_CLOAK,
            RuleSource::Bible => self.can_cloak_mod.unwrap_or(0),
        };
        if nibble(word, 4) > 0 && check.fuel <= 0.0 {
            return false;
        }
        if nibble(word, 8) > 0 && check.shield <= 0.0 {
            return false;
        }
        !check.jumping || self.has(CLOAK_JUMPING)
    }
}

/// What `_ShipCanCloak` asks of a ship besides its gear, for
/// [`CloakGear::can_cloak`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CloakCheck {
    /// Whether it is disabled (`_IsDisabled`).
    pub disabled: bool,
    /// Its fuel (+0x38).
    pub fuel: f32,
    /// Its shield (+0x54).
    pub shield: f32,
    /// Whether its jump timer (+0x50) runs: only ever the player's.
    pub jumping: bool,
}

/// The first `ModType` 17 mod's `ModVal` of `record`, if it has one.
fn first_cloak_mod(record: &OutfitRecord) -> Option<i16> {
    record
        .mods
        .iter()
        .find(|&&(kind, _)| kind == MOD_CLOAK)
        .map(|&(_, mod_val)| mod_val)
}

/// The nibble of `word` at bit `shift`.
fn nibble(word: i16, shift: u32) -> u8 {
    // Masked to a nibble, so it fits.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let nibble = ((word >> shift) & 0xf) as u8;
    nibble
}

/// A cloak mod's shield usage, `(ModVal >> 8) & 0xf`.
fn shield_usage(mod_val: i16) -> u8 {
    nibble(mod_val, 8)
}

/// How a ship with `cloak` is drawn when a full fade draws it at
/// `full_level` (`_HandleShipDisplay` @0x2d169-0x2d648): at a full fade
/// [`CloakShown::Translucent`] below [`TRANSLUCENCY_STEPS`], else
/// [`CloakShown::Hidden`]; with any fade, [`CloakShown::Fading`]; with
/// none, [`CloakShown::Opaque`]. The player's full fade is always
/// [`SEEN_AT_FULL_FADE`] (+0x86 0, @0x2d199-0x2d1a1 → 0x2d1be), and an
/// NPC's [`full_fade_translucency`].
#[must_use]
pub fn shown(cloak: Cloak, full_level: u8) -> CloakShown {
    if cloak.full() {
        if full_level < TRANSLUCENCY_STEPS {
            CloakShown::Translucent(full_level)
        } else {
            CloakShown::Hidden
        }
    } else if cloak.fade > 0.0 {
        CloakShown::Fading(cloak.fade)
    } else {
        CloakShown::Opaque
    }
}

/// What a ship's cloak scanners give: a `ModType` 30 mod's `ModVal`
/// bits, over every outfit the player owns or every default item of an
/// NPC's ship type ([`CloakGear::scanner`]).
// Each bit is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scanner {
    /// Cloaked ships within [`SIGHT_RANGE`] are seen ([`SCAN_SCREEN`]).
    pub on_screen: bool,
    /// Cloaked ships show on the radar ([`SCAN_RADAR`]).
    pub on_radar: bool,
    /// Cloaked ships can be targeted ([`SCAN_TARGET`]).
    pub targets: bool,
    /// Ships of an [`UNTARGETABLE`] type can be targeted
    /// ([`SCAN_UNTARGETABLE`]).
    pub untargetable: bool,
}

impl Scanner {
    /// No scanner at all: [`Scanner::default`], for a `const`.
    pub const NONE: Self = Self {
        on_screen: false,
        on_radar: false,
        targets: false,
        untargetable: false,
    };

    /// What `owned` of `outfits` give.
    #[must_use]
    pub fn of(owned: &BTreeMap<OutfitId, u16>, outfits: &[OutfitRecord]) -> Self {
        let mut scanner = Self::default();
        for mod_val in mods(owned, outfits, MOD_SCANNER) {
            scanner.on_screen |= mod_val & SCAN_SCREEN != 0;
            scanner.on_radar |= mod_val & SCAN_RADAR != 0;
            scanner.targets |= mod_val & SCAN_TARGET != 0;
            scanner.untargetable |= mod_val & SCAN_UNTARGETABLE != 0;
        }
        scanner
    }
}

/// The `ModVal` of each mod of `mod_type` among the outfits of `outfits`
/// that `owned` holds.
fn mods<'a>(
    owned: &'a BTreeMap<OutfitId, u16>,
    outfits: &'a [OutfitRecord],
    mod_type: i16,
) -> impl Iterator<Item = i16> + 'a {
    outfits
        .iter()
        .filter(|record| owned.get(&record.id).is_some_and(|&count| count > 0))
        .flat_map(|record| record.mods)
        .filter(move |&(kind, _)| kind == mod_type)
        .map(|(_, mod_val)| mod_val)
}

/// Where a ship looks from, for [`visible`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Observer {
    /// Where it is.
    pub at: Vec2,
    /// Whether it has a cloak scanner on screen ([`Scanner::on_screen`]).
    pub scanner: bool,
    /// Whether it escorts the target and has a cloak itself.
    pub cloaked_escort: bool,
}

/// Whether a ship at `at` with `cloak`, `emerging` from a hypergate or
/// not, is seen by `observer` (see the module docs).
#[must_use]
pub fn visible(cloak: Cloak, at: Vec2, emerging: bool, observer: Observer) -> bool {
    seen(cloak.is_cloaked(), at, emerging, observer)
}

/// Whether a ship at `at`, `cloaked` or not and `emerging` from a
/// hypergate or not, is seen by `observer`: [`visible`]'s rule
/// (`_ShipVisibleToShip` @0x971c-0x9809), for a ship known only by
/// whether it is cloaked, as point defence knows its quarry (see
/// [`defence::choose_ship`](crate::combat::defence::choose_ship)) and a
/// homing shot's firer its target (see
/// [`projectile`](crate::combat::projectile)).
#[must_use]
pub fn seen(cloaked: bool, at: Vec2, emerging: bool, observer: Observer) -> bool {
    if emerging {
        return false;
    }
    if !cloaked || observer.cloaked_escort {
        return true;
    }
    observer.scanner
        && (observer.at.x - at.x).abs() <= SIGHT_RANGE
        && (observer.at.y - at.y).abs() <= SIGHT_RANGE
}

/// What an NPC's think knows of itself, for [`think`].
#[allow(clippy::struct_excessive_bools)] // Each is one condition the think reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Situation {
    /// Whether it is disabled.
    pub disabled: bool,
    /// Whether it retreats (AI state 3: here, fleeing).
    pub retreating: bool,
    /// Where the ship it attacks is from it, if it attacks one.
    pub attacking: Option<Vec2>,
    /// Whether one of the weapons it carries reloads past its `Reload`
    /// (`Flags2` 0x0100, as the caller reads it under the rulebook).
    pub reloading: bool,
    /// Whether it jumps: its own jump (sub-goal 4), a hyperspace arrival
    /// or gate burst (goal 8), or a jump with its parent (goal 0xb).
    pub jumping: bool,
    /// Whether it lands, waits, enters a gate, inspects or flies in
    /// formation (goals 1, 6, 0x14, 7 and 0xa).
    pub travelling: bool,
    /// Whether it idles (goal 0).
    pub idle: bool,
    /// Its mining order, if it mines (goal 0x10).
    pub mining: Option<MineOrder>,
    /// Whether it keeps close formation (sub-goal 0xc) with a cloaked
    /// parent, its AI type above 4.
    pub close_to_cloaked_parent: bool,
}

/// A miner's order (goal 0x10's sub-goal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineOrder {
    /// Order 1: stop, as with no rock in slot 0.
    Stop,
    /// Order 0x13: chase the rock in slot 0.
    Chase,
    /// Order 0x14: inside the rock's box.
    Close,
}

/// `cloak` and `shield` after `_DoShipCloak` on a ship of `gear`
/// (@0xde73): [`Cloak::cloak`], then, when it was not cloaked before and
/// the gear [`zeroes_shields`](CloakGear::zeroes_shields), a shield above
/// 0 goes to 0 (@0xdf1f-0xdf3d), even for a ship already fading in.
/// Returns [`Cloak::cloak`]'s sound condition.
pub fn cloak_ship(cloak: &mut Cloak, gear: CloakGear, shield: &mut f32) -> bool {
    let was_cloaked = cloak.is_cloaked();
    let sounds = cloak.cloak();
    if !was_cloaked && gear.zeroes_shields {
        // A shield at or below 0 stays as it is (@0xdf1f's `> 0` test),
        // so only one above 0 changes.
        *shield = shield.min(0.0);
    }
    sounds
}

/// `cloak` and `shield` after a think of a ship of `gear` in `situation`
/// (`_AIHandleCloaking`, see the module docs): a ship whose cloak costs
/// shield with none left uncloaks before the conditions are asked
/// (@0x83449-0x83452).
pub fn think(cloak: &mut Cloak, gear: CloakGear, situation: Situation, shield: &mut f32) {
    if !gear.device || situation.disabled {
        cloak.uncloak();
        return;
    }
    if gear.shield_usage > 0 && *shield <= 0.0 {
        cloak.uncloak();
        return;
    }
    let mining = situation.mining;
    let reloading = gear.has(CLOAK_RELOADING) && situation.reloading;
    let retreating =
        gear.has(CLOAK_RETREATING) && (situation.retreating || mining == Some(MineOrder::Chase));
    let jumping = gear.has(CLOAK_JUMPING) && situation.jumping;
    let travelling = gear.has(CLOAK_TRAVELLING) && situation.travelling;
    let cloaked = cloak.is_cloaked();
    let pressing = gear.has(CLOAK_PRESSING)
        && cloaked
        && (situation
            .attacking
            .is_some_and(|off| off.x.abs() > PRESS_RANGE || off.y.abs() > PRESS_RANGE)
            || mining.is_some_and(|order| order != MineOrder::Close));
    let idle = gear.has(CLOAK_IDLE) && situation.idle;
    let flag = reloading || retreating || jumping || travelling || pressing || idle;
    // The parent rule (@0x83633-0x8366d) cloaks whatever the flag says.
    if situation.close_to_cloaked_parent || flag {
        cloak_ship(cloak, gear, shield);
    } else {
        cloak.uncloak();
    }
}

/// `shield` after a frame's cloak upkeep (`_HandleShip`
/// @0x341af-0x34204) on a ship `cloaked` at the frame's start whose cloak
/// costs `usage`: while the shield is at least `usage`, it pays
/// `usage` x [`CLOAK_UPKEEP_RATE`]. The original floors the result at 0,
/// which a shield at least `usage` never reaches; so the drain alone
/// never empties it.
pub fn drain_shield(shield: &mut f32, usage: u8, cloaked: bool) {
    let usage = f32::from(usage);
    // A usage of 0 costs nothing, so its `> 0` test (@0x341b6) is subsumed.
    if cloaked && *shield >= usage {
        *shield -= usage * CLOAK_UPKEEP_RATE;
    }
}

/// `cloak` after a frame's check that a ship still cloaked `can_cloak`
/// (`_HandleShip` @0x3413d-0x3415f): one that can no longer uncloaks.
pub fn hold(cloak: &mut Cloak, can_cloak: bool) {
    if !can_cloak {
        cloak.uncloak();
    }
}

/// `cloak` and `shield` after a weapon's hit on a ship of `gear` that `can_cloak`,
/// `attacking` the ship that hit it or not (see the module docs); and
/// whether cloak-when-hit's conditions held, whether or not the ship was
/// cloaked already. `_DamageShip` forces the ship's burst reload on those
/// conditions alone (`_AIForceBurstReload` @0x3bbd5), before it cloaks
/// (@0x3bbdd).
pub fn hit(
    cloak: &mut Cloak,
    gear: CloakGear,
    can_cloak: bool,
    attacking: bool,
    shield: &mut f32,
) -> bool {
    let cloaks = gear.has(CLOAK_WHEN_HIT) && can_cloak && !attacking;
    if cloaks {
        cloak_ship(cloak, gear, shield);
    }
    decloak_on_hit(cloak, gear);
    cloaks
}

/// `cloak` after a direct hit's decloak on a ship of `gear` (`_DamageShip`
/// @0x3bf1f-0x3bf4f): a cloak that decloaks when hit, fully faded or still
/// fading in, uncloaks ([`Cloak::uncloak`]). Returns [`Cloak::uncloak`]'s
/// sound condition, which only the player's cloak raises.
pub fn decloak_on_hit(cloak: &mut Cloak, gear: CloakGear) -> bool {
    gear.decloaks_when_hit && (cloak.full() || cloak.direction > 0) && cloak.uncloak()
}

/// A rectangle in world space: x grows right and y down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Area {
    /// Its left edge.
    pub left: f32,
    /// Its top edge.
    pub top: f32,
    /// Its right edge.
    pub right: f32,
    /// Its bottom edge.
    pub bottom: f32,
}

impl Area {
    /// The area `width` x `height` centred on `at`.
    #[must_use]
    pub fn centred(at: Vec2, width: f32, height: f32) -> Self {
        Self {
            left: at.x - width / 2.0,
            top: at.y - height / 2.0,
            right: at.x + width / 2.0,
            bottom: at.y + height / 2.0,
        }
    }

    /// The area moved by `by`.
    #[must_use]
    pub fn offset(self, by: Vec2) -> Self {
        Self {
            left: self.left + by.x,
            top: self.top + by.y,
            right: self.right + by.x,
            bottom: self.bottom + by.y,
        }
    }

    /// Whether this area and `other` strictly overlap, so two that only
    /// touch along an edge do not. A zero-size area (a point or a line)
    /// overlaps one it lies strictly inside.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
}

/// The play area the session assumes when no view has said what it
/// shows: the original's smallest screen, 800 x 600, less its 194-pixel
/// status bar, centred on the player.
pub const DEFAULT_PLAY_AREA: Area = Area {
    left: -303.0,
    top: -300.0,
    right: 303.0,
    bottom: 300.0,
};

/// Whether a ship at `at` whose sprite is `width` x `height` is on
/// screen in `view`: its sprite meets `view` inset by
/// trunc([`ON_SCREEN_INSET`] x its size) on each side (`_InsetRect` and
/// `_SectRect` @0x9ea81-0x9eb09). In effect its centre is a quarter of
/// its sprite inside each edge. A ship with no sprite (0 x 0) is on
/// screen while its centre is inside `view`.
#[must_use]
pub fn on_screen(view: Area, at: Vec2, width: f32, height: f32) -> bool {
    let dx = (width * ON_SCREEN_INSET).trunc();
    let dy = (height * ON_SCREEN_INSET).trunc();
    let inset = Area {
        left: view.left + dx,
        top: view.top + dy,
        right: view.right - dx,
        bottom: view.bottom - dy,
    };
    // `_InsetRect` empties a rect it turns inside out.
    inset.left < inset.right
        && inset.top < inset.bottom
        && Area::centred(at, width, height).overlaps(inset)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use nova_sim_catalog::fixture::outfit;

    fn at(fade: f32, direction: i8) -> Cloak {
        Cloak { fade, direction }
    }

    #[test]
    fn the_full_fade_hides_only_ships_the_player_neither_leads_nor_scans() {
        assert_eq!(full_fade_translucency(true, true), SEEN_AT_FULL_FADE);
        assert_eq!(full_fade_translucency(true, false), SEEN_AT_FULL_FADE);
        assert_eq!(full_fade_translucency(false, true), SEEN_AT_FULL_FADE);
        assert_eq!(full_fade_translucency(false, false), TRANSLUCENCY_STEPS);
        assert_eq!((SEEN_AT_FULL_FADE, TRANSLUCENCY_STEPS), (0x1e, 0x40));
    }

    #[test]
    fn a_ship_is_cloaked_past_16_holding_or_fading_in_and_past_8_fading_out() {
        for direction in [0, 1] {
            assert!(!at(16.0, direction).is_cloaked(), "{direction}");
            assert!(at(16.5, direction).is_cloaked(), "{direction}");
            assert!(at(FADE_FULL, direction).is_cloaked(), "{direction}");
        }
        for direction in [-1, -2] {
            assert!(!at(8.0, direction).is_cloaked(), "{direction}");
            assert!(at(8.5, direction).is_cloaked(), "{direction}");
            assert!(at(30.0, direction).is_cloaked(), "{direction}");
        }
        assert!(!Cloak::default().is_cloaked());
    }

    #[test]
    fn cloaking_from_nothing_fades_in_and_reads_cloaked_only_past_16() {
        let mut cloak = Cloak::default();
        cloak.cloak();
        assert_eq!(cloak, at(0.0, 1));
        let mut ticks = 0;
        while !cloak.is_cloaked() {
            cloak.step(false);
            ticks += 1;
        }
        assert_eq!(ticks, 22, "16 / 0.75, then one more");
        assert_eq!(cloak.fade, 16.5);
    }

    #[test]
    fn the_fade_stops_at_32_going_in_and_at_0_going_out() {
        let mut cloak = at(31.5, 1);
        cloak.step(false);
        assert_eq!(cloak, at(FADE_FULL, 0));
        cloak.step(false);
        assert_eq!(cloak, at(FADE_FULL, 0), "full and holding");
        let mut cloak = at(0.5, -1);
        cloak.step(false);
        assert_eq!(cloak, at(0.0, 0));
        cloak.step(false);
        assert_eq!(cloak, at(0.0, 0), "seen and holding");
        let mut onto_0 = at(0.75, -1);
        onto_0.step(false);
        assert_eq!(onto_0, at(0.0, 0), "stops on reaching 0");
        let mut onto_32 = at(31.25, 1);
        onto_32.step(false);
        assert_eq!(onto_32, at(FADE_FULL, 0), "stops on reaching 32");
    }

    #[test]
    fn a_fast_fading_ship_moves_twice_as_far_a_tick() {
        let mut slow = at(10.0, 1);
        slow.step(false);
        assert_eq!(slow, at(10.75, 1));
        let mut fast = at(10.0, 1);
        fast.step(true);
        assert_eq!(fast, at(11.5, 1));
        let mut out = at(10.0, -1);
        out.step(true);
        assert_eq!(out, at(8.5, -1));
    }

    #[test]
    fn a_fade_with_no_direction_drifts_down_by_1_until_it_is_gone() {
        let mut cloak = at(1.5, 0);
        cloak.step(false);
        assert_eq!(cloak, at(0.5, 0));
        cloak.step(false);
        assert_eq!(cloak, at(-0.5, 0), "a drift is not stopped at 0");
        cloak.step(false);
        assert_eq!(cloak, at(-0.5, 0), "nothing to drift");
        let mut seen = Cloak::default();
        seen.step(false);
        assert_eq!(seen, Cloak::default(), "nothing to drift at 0");
        let mut full = at(FADE_FULL, 0);
        full.step(true);
        assert_eq!(full, at(FADE_FULL, 0), "a full cloak holds");
    }

    #[test]
    fn with_no_direction_a_full_cloak_holds_at_32_and_an_uncloaked_ship_at_0() {
        let mut full = at(FADE_FULL, 0);
        let mut seen = Cloak::default();
        for _ in 0..40 {
            full.step(false);
            seen.step(false);
        }
        assert_eq!(full, at(FADE_FULL, 0));
        assert_eq!(seen, Cloak::default());
    }

    #[test]
    fn cloaking_does_nothing_when_cloaked_and_stops_a_fade_going_out() {
        let mut cloaked = at(20.0, -1);
        cloaked.cloak();
        assert_eq!(cloaked, at(20.0, -1));
        let mut going = at(5.0, -1);
        going.cloak();
        assert_eq!(going, at(5.0, 0));
        let mut rising = at(5.0, 1);
        rising.cloak();
        assert_eq!(rising, at(5.0, 1));
    }

    #[test]
    fn cloaking_sounds_only_from_rest_or_fading_out_and_not_when_cloaked() {
        // `_DoShipCloak` plays snd 381 only for a direction at or below 0
        // (`jg` @0xded2), after the cloaked early return.
        assert!(at(0.0, 0).cloak(), "from rest");
        assert!(at(5.0, -1).cloak(), "fading out, no longer cloaked");
        assert!(at(5.0, -2).cloak(), "dying, no longer cloaked");
        assert!(!at(10.0, 1).cloak(), "already rising");
        assert!(!at(20.0, -1).cloak(), "cloaked");
        assert!(!at(FADE_FULL, 0).cloak(), "full");
    }

    #[test]
    fn uncloaking_sounds_only_when_cloaked_and_not_already_falling() {
        // `_DoShipUncloak` plays snd 380 only for a direction at or above
        // 0 (`js` @0xdfb0), after the not-cloaked early return.
        assert!(at(FADE_FULL, 0).uncloak(), "holding");
        assert!(at(20.0, 1).uncloak(), "rising");
        assert!(!at(20.0, -1).uncloak(), "falling");
        assert!(!at(20.0, -2).uncloak(), "dying");
        assert!(!at(10.0, 1).uncloak(), "not cloaked");
        assert!(!at(0.0, 0).uncloak(), "seen");
    }

    #[test]
    fn a_dying_ship_with_any_fade_left_falls_at_twice_the_rate() {
        // `_HandleShipDisplay` sets -2 after the step (@0x2d169-0x2d17e).
        for start in [at(10.0, 1), at(FADE_FULL, 0), at(20.0, -1)] {
            let mut cloak = start;
            cloak.display_step(false, true);
            let mut plain = start;
            plain.step(false);
            assert_eq!(cloak, at(plain.fade, -2), "{start:?} moved the old way");
            cloak.display_step(false, true);
            assert_eq!(cloak, at(plain.fade - 1.5, -2), "{start:?} then 2 x 0.75");
        }
        let mut fast = at(20.0, -2);
        fast.display_step(true, true);
        assert_eq!(fast, at(17.0, -2), "2 x 1.5");
        let mut gone = Cloak::default();
        gone.display_step(false, true);
        assert_eq!(gone, Cloak::default(), "no fade, left alone");
        let mut emptied = at(1.0, -2);
        emptied.display_step(false, true);
        assert_eq!(emptied, Cloak::default(), "faded out on this step");
        let mut alive = at(10.0, 1);
        alive.display_step(true, false);
        assert_eq!(alive, at(11.5, 1), "not dying, a plain step");
    }

    #[test]
    fn uncloaking_does_nothing_unless_cloaked_and_then_fades_out() {
        let mut early = at(10.0, 1);
        early.uncloak();
        assert_eq!(early, at(10.0, 1), "not cloaked yet");
        let mut cloaked = at(20.0, 1);
        cloaked.uncloak();
        assert_eq!(cloaked, at(20.0, -1));
        let mut full = at(FADE_FULL, 0);
        full.uncloak();
        assert_eq!(full, at(FADE_FULL, -1));
    }

    /// A cloak of `mod_val`, outfit 300.
    fn cloak_outfit(mod_val: i16) -> OutfitRecord {
        outfit(300, &[(MOD_CLOAK, mod_val)])
    }

    fn one(id: i16) -> BTreeMap<OutfitId, u16> {
        [(OutfitId(id), 1)].into()
    }

    #[test]
    fn a_ship_type_carrying_a_cloak_has_its_modval_bits() {
        let outfits = [outfit(299, &[(1, 5)]), cloak_outfit(0x080b)];
        let zephyr = CloakGear::of(0x5282, &one(300), &outfits);
        assert_eq!(
            zephyr,
            CloakGear {
                device: true,
                decloaks_when_hit: true,
                on_radar: true,
                flags2: 0x5282,
                shield_usage: 0,
                fuel_usage: 0,
                zeroes_shields: false,
                can_cloak_mod: None,
                scanner: Scanner::default(),
            },
            "the upkeep needs the slot order (`with_upkeep`)"
        );
        assert!(!zephyr.fast());
        let plain = CloakGear::of(0x0001, &one(300), &[cloak_outfit(0)]);
        assert!(plain.device && !plain.decloaks_when_hit && !plain.on_radar);
        assert!(plain.fast());
        let none = CloakGear::of(0, &one(299), &outfits);
        assert!(!none.device && !none.decloaks_when_hit && !none.on_radar);
        let unowned = CloakGear::of(0, &BTreeMap::new(), &outfits);
        assert!(!unowned.device);
        let in_a_later_slot = outfit(301, &[(1, 1), (MOD_CLOAK, 0x0008)]);
        let later = CloakGear::of(0, &one(301), std::slice::from_ref(&in_a_later_slot));
        assert!(later.device && later.decloaks_when_hit);
        let twice = outfit(302, &[(MOD_CLOAK, 0x000a), (MOD_CLOAK, 0x000a)]);
        let both = CloakGear::of(0, &one(302), &[twice]);
        assert!(
            both.decloaks_when_hit && both.on_radar,
            "a bit set twice is set"
        );
        let none_held: BTreeMap<OutfitId, u16> = [(OutfitId(301), 0)].into();
        assert!(!CloakGear::of(0, &none_held, &[in_a_later_slot]).device);
    }

    /// Plenty of fuel and shield, not disabled and not jumping.
    const READY: CloakCheck = CloakCheck {
        disabled: false,
        fuel: 100.0,
        shield: 100.0,
        jumping: false,
    };

    const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

    /// The outfits `owned` holds, each of `(id, count)`.
    fn owned(items: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        items.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    /// The player's gear owning one cloak, outfit 300, of `mod_val`.
    fn players(mod_val: i16) -> CloakGear {
        CloakGear::player(0, &one(300), &[cloak_outfit(mod_val)])
    }

    #[test]
    fn a_disabled_ship_or_one_with_no_cloak_never_can_cloak() {
        let gear = players(0);
        let disabled = CloakCheck {
            disabled: true,
            ..READY
        };
        for reserves in READINGS {
            assert!(gear.can_cloak(READY, reserves), "{reserves:?}");
            assert!(
                !gear.can_cloak(disabled, reserves),
                "{reserves:?}: disabled"
            );
            assert!(
                !CloakGear::default().can_cloak(READY, reserves),
                "{reserves:?}: no cloak"
            );
        }
    }

    #[test]
    fn by_the_engine_every_cloak_needs_fuel_and_none_needs_shield() {
        // `_ShipCanCloak` reads the `ModType` word 0x11 (@0x558d): a fuel
        // nibble of 1 and a shield nibble of 0, whatever the `ModVal`.
        for mod_val in [0x080b, 0x0000, 0x0f00] {
            let gear = players(mod_val);
            let no_shield = CloakCheck {
                shield: 0.0,
                ..READY
            };
            let no_fuel = CloakCheck { fuel: 0.0, ..READY };
            assert!(
                gear.can_cloak(no_shield, RuleSource::Engine),
                "{mod_val:#x}"
            );
            assert!(!gear.can_cloak(no_fuel, RuleSource::Engine), "{mod_val:#x}");
        }
    }

    #[test]
    fn by_the_bible_a_cloak_needs_fuel_only_with_a_fuel_usage_and_shield_only_with_a_shield_usage()
    {
        let no_shield = CloakCheck {
            shield: 0.0,
            ..READY
        };
        let less_shield = CloakCheck {
            shield: -1.0,
            ..READY
        };
        let no_fuel = CloakCheck { fuel: 0.0, ..READY };
        let less_fuel = CloakCheck {
            fuel: -1.0,
            ..READY
        };
        let some = CloakCheck {
            fuel: 0.1,
            shield: 0.1,
            ..READY
        };
        let bible = RuleSource::Bible;
        for (mod_val, at_no_shield, at_no_fuel) in [
            (0x080b, false, true),
            (0x0010, true, false),
            (0x0000, true, true),
            (0x0f10, false, false),
        ] {
            let gear = players(mod_val);
            assert_eq!(
                gear.can_cloak(no_shield, bible),
                at_no_shield,
                "{mod_val:#x}"
            );
            assert_eq!(
                gear.can_cloak(less_shield, bible),
                at_no_shield,
                "{mod_val:#x}"
            );
            assert_eq!(gear.can_cloak(no_fuel, bible), at_no_fuel, "{mod_val:#x}");
            assert_eq!(gear.can_cloak(less_fuel, bible), at_no_fuel, "{mod_val:#x}");
            assert!(
                gear.can_cloak(some, bible),
                "{mod_val:#x}: a little of each"
            );
        }
    }

    #[test]
    fn can_cloak_reads_the_last_owned_cloak_outfit_and_its_first_cloak_mod() {
        let outfits = [
            outfit(268, &[(MOD_CLOAK, 0x080b)]),
            outfit(300, &[(1, 1), (MOD_CLOAK, 0x0010), (MOD_CLOAK, 0x0800)]),
            outfit(301, &[(1, 1)]),
            outfit(302, &[(MOD_CLOAK, 0x0800)]),
        ];
        let gear = CloakGear::player(0, &owned(&[(268, 1), (300, 1), (301, 1)]), &outfits);
        assert_eq!(gear.can_cloak_mod, Some(0x0010));
        let no_shield = CloakCheck {
            shield: 0.0,
            ..READY
        };
        let no_fuel = CloakCheck { fuel: 0.0, ..READY };
        assert!(
            gear.can_cloak(no_shield, RuleSource::Bible),
            "300's first cloak mod asks no shield"
        );
        assert!(!gear.can_cloak(no_fuel, RuleSource::Bible));
        let unheld = CloakGear::player(0, &owned(&[(300, 1), (302, 0)]), &outfits);
        assert_eq!(
            unheld.can_cloak_mod,
            Some(0x0010),
            "a count of 0 is unowned"
        );
        let reversed = [outfits[1].clone(), outfits[0].clone()];
        let by_id = CloakGear::player(0, &owned(&[(268, 1), (300, 1)]), &reversed);
        assert_eq!(by_id.can_cloak_mod, Some(0x0010), "outfit order is by ID");
        assert_eq!(
            CloakGear::player(0, &owned(&[(301, 1)]), &outfits).can_cloak_mod,
            None
        );
    }

    #[test]
    fn an_npcs_can_cloak_reads_its_last_cloak_slot() {
        let outfits = [
            outfit(268, &[(MOD_CLOAK, 0x080b)]),
            outfit(269, &[(1, 1), (MOD_CLOAK, 0x0010), (MOD_CLOAK, 0x0800)]),
            outfit(270, &[(1, 1)]),
        ];
        let gear = |items: &[(i16, u16)]| {
            let held: BTreeMap<OutfitId, u16> = owned(items);
            CloakGear::of(0, &held, &outfits).with_upkeep(&slots(items), &outfits)
        };
        let no_shield = CloakCheck {
            shield: 0.0,
            ..READY
        };
        let bible = RuleSource::Bible;
        let last_asks_fuel = gear(&[(268, 1), (269, 1), (270, 1)]);
        assert_eq!(last_asks_fuel.can_cloak_mod, Some(0x0010));
        assert!(last_asks_fuel.can_cloak(no_shield, bible));
        let last_asks_shield = gear(&[(269, 1), (268, 1)]);
        assert_eq!(last_asks_shield.can_cloak_mod, Some(0x080b));
        assert!(!last_asks_shield.can_cloak(no_shield, bible));
        assert_eq!(
            gear(&[(269, 1), (268, 0)]).can_cloak_mod,
            Some(0x0010),
            "a count-0 slot is skipped"
        );
    }

    #[test]
    fn a_jumping_player_keeps_its_cloak_only_with_flags2_0x0400() {
        let jumping = CloakCheck {
            jumping: true,
            ..READY
        };
        let through = CloakGear::player(CLOAK_JUMPING, &one(300), &[cloak_outfit(0)]);
        for reserves in READINGS {
            assert!(!players(0).can_cloak(jumping, reserves), "{reserves:?}");
            assert!(through.can_cloak(jumping, reserves), "{reserves:?}: 0x0400");
            assert!(players(0).can_cloak(READY, reserves), "{reserves:?}: still");
        }
    }

    #[test]
    fn the_players_gear_reads_its_usages_and_decloak_from_the_first_owned_cloak_outfit() {
        let outfits = [
            outfit(260, &[(MOD_SCANNER, SCAN_SCREEN)]),
            outfit(300, &[(1, 1), (MOD_CLOAK, 0x0a3e), (MOD_CLOAK, 0x0000)]),
            outfit(301, &[(MOD_CLOAK, 0x0000)]),
        ];
        let gear = CloakGear::player(0x0401, &owned(&[(260, 1), (300, 1), (301, 2)]), &outfits);
        assert_eq!(
            gear,
            CloakGear {
                device: true,
                decloaks_when_hit: true,
                on_radar: true,
                flags2: 0x0401,
                shield_usage: 0xa,
                fuel_usage: 3,
                zeroes_shields: true,
                can_cloak_mod: Some(0x0000),
                scanner: Scanner {
                    on_screen: true,
                    ..Scanner::default()
                },
            }
        );
        let reversed = [outfits[2].clone(), outfits[1].clone()];
        let later = CloakGear::player(0, &owned(&[(300, 1), (301, 1)]), &reversed);
        assert_eq!(
            (later.shield_usage, later.fuel_usage, later.zeroes_shields),
            (0xa, 3, true),
            "outfit 300 comes first by ID, whatever the records' order"
        );
        let plain = CloakGear::player(0, &owned(&[(301, 1)]), &outfits);
        assert!(plain.device);
        assert!(!plain.decloaks_when_hit && !plain.zeroes_shields && !plain.on_radar);
        let none = CloakGear::player(0, &owned(&[(260, 1)]), &outfits);
        assert!(!none.device);
        assert!(none.scanner.on_screen);
    }

    #[test]
    fn the_players_fuel_usage_is_its_first_cloaks_fuel_nibble_and_an_npcs_is_none() {
        assert_eq!(players(0x0030).fuel_usage, 3);
        assert_eq!(players(0x00f0).fuel_usage, 0xf);
        assert_eq!(players(0x0f0f).fuel_usage, 0);
        let outfits = [cloak_outfit(0x00f0)];
        let npc = CloakGear::of(0, &one(300), &outfits).with_upkeep(&slots(&[(300, 1)]), &outfits);
        assert_eq!(npc.fuel_usage, 0, "`_CloakFuelUsage` @0x3e31-0x3e39");
    }

    #[test]
    fn an_npcs_decloak_when_hit_is_its_first_cloak_slots_bit() {
        let outfits = [
            outfit(268, &[(MOD_CLOAK, 0x0000)]),
            outfit(269, &[(MOD_CLOAK, CLOAK_DECLOAKS_WHEN_HIT)]),
        ];
        let gear = |items: &[(i16, u16)]| {
            CloakGear::of(0, &owned(items), &outfits).with_upkeep(&slots(items), &outfits)
        };
        assert!(!gear(&[(268, 1), (269, 1)]).decloaks_when_hit);
        assert!(gear(&[(269, 1), (268, 1)]).decloaks_when_hit);
        assert!(gear(&[(268, 0), (269, 1)]).decloaks_when_hit);
    }

    #[test]
    fn the_players_full_fade_is_always_drawn_faint_and_a_fade_splits_as_an_npcs() {
        let full = at(FADE_FULL, 0);
        assert_eq!(
            shown(full, SEEN_AT_FULL_FADE),
            CloakShown::Translucent(SEEN_AT_FULL_FADE)
        );
        assert_eq!(shown(full, 0x3f), CloakShown::Translucent(0x3f));
        assert_eq!(shown(full, TRANSLUCENCY_STEPS), CloakShown::Hidden);
        assert_eq!(
            shown(at(31.9, 1), SEEN_AT_FULL_FADE),
            CloakShown::Fading(31.9)
        );
        assert_eq!(
            shown(at(0.5, -2), TRANSLUCENCY_STEPS),
            CloakShown::Fading(0.5)
        );
        assert_eq!(shown(at(0.0, 0), SEEN_AT_FULL_FADE), CloakShown::Opaque);
        assert_eq!(shown(at(-0.5, 0), SEEN_AT_FULL_FADE), CloakShown::Opaque);
    }

    #[test]
    fn a_direct_hit_decloaks_a_full_or_fading_in_cloak_that_decloaks_when_hit_and_says_whether_it_sounds()
     {
        let gear = players(CLOAK_DECLOAKS_WHEN_HIT);
        let mut full = at(FADE_FULL, 0);
        assert!(decloak_on_hit(&mut full, gear));
        assert_eq!(full, at(FADE_FULL, -1));
        let mut rising = at(20.0, 1);
        assert!(decloak_on_hit(&mut rising, gear));
        assert_eq!(rising, at(20.0, -1));
        let mut holding = at(20.0, 0);
        assert!(!decloak_on_hit(&mut holding, gear));
        assert_eq!(holding, at(20.0, 0), "neither full nor fading in");
        let mut early = at(10.0, 1);
        assert!(!decloak_on_hit(&mut early, gear), "not yet cloaked");
        assert_eq!(early, at(10.0, 1));
        let mut kept = at(FADE_FULL, 0);
        assert!(!decloak_on_hit(&mut kept, players(0)));
        assert_eq!(kept, at(FADE_FULL, 0));
    }

    #[test]
    fn a_scanner_is_its_modval_bits_over_every_outfit_owned() {
        let outfits = [
            outfit(310, &[(MOD_SCANNER, SCAN_RADAR)]),
            outfit(311, &[(1, 1), (MOD_SCANNER, SCAN_SCREEN | SCAN_TARGET)]),
            outfit(312, &[(MOD_SCANNER, SCAN_UNTARGETABLE)]),
        ];
        assert_eq!(Scanner::of(&BTreeMap::new(), &outfits), Scanner::default());
        assert_eq!(
            Scanner::of(&one(310), &outfits),
            Scanner {
                on_radar: true,
                ..Scanner::default()
            }
        );
        let both: BTreeMap<OutfitId, u16> = [(OutfitId(310), 1), (OutfitId(311), 2)].into();
        assert_eq!(
            Scanner::of(&both, &outfits),
            Scanner {
                on_screen: true,
                on_radar: true,
                targets: true,
                untargetable: false,
            }
        );
        assert_eq!(
            Scanner::of(&one(312), &outfits),
            Scanner {
                untargetable: true,
                ..Scanner::default()
            }
        );
        let twice = outfit(313, &[(MOD_SCANNER, 0x000f), (MOD_SCANNER, 0x000f)]);
        assert_eq!(
            Scanner::of(&one(313), &[twice]),
            Scanner {
                on_screen: true,
                on_radar: true,
                targets: true,
                untargetable: true,
            },
            "a bit set twice is set"
        );
    }

    #[test]
    fn a_ship_types_scanner_is_its_default_items_scanner_bits_and_no_other_mod() {
        let outfits = [
            outfit(320, &[(MOD_SCANNER, SCAN_UNTARGETABLE | SCAN_TARGET)]),
            outfit(321, &[(MOD_CLOAK, 0x000f), (1, 0x000f)]),
        ];
        assert_eq!(
            CloakGear::of(0, &one(320), &outfits).scanner,
            Scanner {
                targets: true,
                untargetable: true,
                ..Scanner::default()
            }
        );
        assert_eq!(
            CloakGear::of(0, &one(321), &outfits).scanner,
            Scanner::default(),
            "a cloak's bits are not a scanner's"
        );
        let none_held: BTreeMap<OutfitId, u16> = [(OutfitId(320), 0)].into();
        assert_eq!(
            CloakGear::of(0, &none_held, &outfits).scanner,
            Scanner::default()
        );
    }

    #[test]
    fn a_ship_type_is_untargetable_by_its_flags2_bit_0x0004() {
        let gear = |flags2| CloakGear {
            flags2,
            ..CloakGear::default()
        };
        assert!(gear(UNTARGETABLE).untargetable());
        assert!(gear(0x5286).untargetable());
        assert!(!gear(0x5282).untargetable());
        assert!(!gear(0).untargetable());
    }

    const CLOAKED: Cloak = Cloak {
        fade: FADE_FULL,
        direction: 0,
    };

    fn player(x: f32, y: f32, scanner: bool) -> Observer {
        Observer {
            at: Vec2::new(x, y),
            scanner,
            cloaked_escort: false,
        }
    }

    #[test]
    fn an_uncloaked_ship_is_seen_at_any_range_and_an_emerging_one_never() {
        let far = player(5000.0, -5000.0, false);
        assert!(visible(Cloak::default(), Vec2::ZERO, false, far));
        assert!(visible(at(16.0, 1), Vec2::ZERO, false, far), "not yet");
        assert!(!visible(Cloak::default(), Vec2::ZERO, true, far));
        assert!(!visible(
            Cloak::default(),
            Vec2::ZERO,
            true,
            player(0.0, 0.0, true)
        ));
    }

    #[test]
    fn a_cloaked_ship_is_unseen_at_any_range_without_a_scanner() {
        for (x, y) in [(0.0, 0.0), (100.0, 100.0), (5000.0, 0.0)] {
            assert!(!visible(CLOAKED, Vec2::ZERO, false, player(x, y, false)));
        }
    }

    #[test]
    fn a_scanner_sees_a_cloaked_ship_within_200_on_both_axes() {
        let ship = Vec2::new(1000.0, -1000.0);
        for (dx, dy) in [(0.0, 0.0), (200.0, -200.0), (-200.0, 200.0), (150.0, 0.0)] {
            let observer = player(1000.0 + dx, -1000.0 + dy, true);
            assert!(visible(CLOAKED, ship, false, observer), "{dx} {dy}");
        }
        for (dx, dy) in [(201.0, 0.0), (0.0, -201.0), (-201.0, 201.0)] {
            let observer = player(1000.0 + dx, -1000.0 + dy, true);
            assert!(!visible(CLOAKED, ship, false, observer), "{dx} {dy}");
        }
    }

    #[test]
    fn an_escort_with_a_cloak_sees_its_cloaked_lead_at_any_range() {
        let escort = Observer {
            at: Vec2::new(4000.0, 0.0),
            scanner: false,
            cloaked_escort: true,
        };
        assert!(visible(CLOAKED, Vec2::ZERO, false, escort));
        assert!(!visible(CLOAKED, Vec2::ZERO, true, escort), "emerging");
    }

    #[test]
    fn a_ship_mid_fade_is_seen_exactly_while_it_is_not_cloaked() {
        let observer = player(0.0, 0.0, false);
        for (fade, direction, seen) in [
            (16.0, 1, true),
            (16.25, 1, false),
            (16.25, 0, false),
            (16.0, 0, true),
            (8.25, -1, false),
            (8.0, -1, true),
        ] {
            let cloak = at(fade, direction);
            assert_eq!(
                visible(cloak, Vec2::ZERO, false, observer),
                seen,
                "{cloak:?}"
            );
        }
    }

    /// A cloak of the Zephyr's `Flags2`, which carries 0x0200, 0x1000 and
    /// 0x4000.
    const ZEPHYR: CloakGear = CloakGear {
        device: true,
        decloaks_when_hit: true,
        on_radar: true,
        flags2: 0x5282,
        shield_usage: 0,
        fuel_usage: 0,
        zeroes_shields: false,
        can_cloak_mod: Some(0x080b),
        scanner: Scanner::NONE,
    };

    #[test]
    fn a_think_uncloaks_a_ship_with_no_cloak_or_a_disabled_one() {
        let retreating = Situation {
            retreating: true,
            ..Situation::default()
        };
        let mut none = CLOAKED;
        think(&mut none, CloakGear::default(), retreating, &mut 0.0);
        assert_eq!(none, at(FADE_FULL, -1));
        let mut disabled = CLOAKED;
        let downed = Situation {
            disabled: true,
            ..retreating
        };
        think(&mut disabled, ZEPHYR, downed, &mut 0.0);
        assert_eq!(disabled, at(FADE_FULL, -1));
        let mut seen = Cloak::default();
        think(
            &mut seen,
            CloakGear::default(),
            Situation::default(),
            &mut 0.0,
        );
        assert_eq!(seen, Cloak::default());
    }

    #[test]
    fn a_ship_whose_flags2_cloak_it_retreating_cloaks_while_it_retreats() {
        let retreating = Situation {
            retreating: true,
            ..Situation::default()
        };
        let mut cloak = Cloak::default();
        think(&mut cloak, ZEPHYR, retreating, &mut 0.0);
        assert_eq!(cloak, at(0.0, 1));
        let mut without = Cloak::default();
        let other = CloakGear {
            flags2: ZEPHYR.flags2 & !CLOAK_RETREATING,
            ..ZEPHYR
        };
        think(&mut without, other, retreating, &mut 0.0);
        assert_eq!(without, Cloak::default(), "no bit, no cloak");
        let mut cloaked = at(20.0, 0);
        think(&mut cloaked, ZEPHYR, Situation::default(), &mut 0.0);
        assert_eq!(cloaked, at(20.0, -1), "no longer retreating");
    }

    #[test]
    fn a_cloaked_ship_pressing_an_attack_keeps_its_cloak_beyond_165_on_either_axis() {
        let pressing = |x, y| Situation {
            attacking: Some(Vec2::new(x, y)),
            ..Situation::default()
        };
        for (x, y) in [(166.0, 0.0), (0.0, -166.0), (-300.0, 300.0)] {
            let mut cloak = CLOAKED;
            think(&mut cloak, ZEPHYR, pressing(x, y), &mut 0.0);
            assert_eq!(cloak, CLOAKED, "{x} {y}");
        }
        for (x, y) in [(165.0, -165.0), (0.0, 0.0)] {
            let mut cloak = CLOAKED;
            think(&mut cloak, ZEPHYR, pressing(x, y), &mut 0.0);
            assert_eq!(cloak, at(FADE_FULL, -1), "{x} {y}: close in to fight");
        }
        let mut seen = Cloak::default();
        think(&mut seen, ZEPHYR, pressing(500.0, 0.0), &mut 0.0);
        assert_eq!(seen, Cloak::default(), "only a cloaked ship keeps it");
        let mut plain = CLOAKED;
        let other = CloakGear {
            flags2: 0,
            ..ZEPHYR
        };
        think(&mut plain, other, pressing(500.0, 0.0), &mut 0.0);
        assert_eq!(plain, at(FADE_FULL, -1), "no bit");
    }

    #[test]
    fn a_ship_set_up_cloaked_with_nothing_to_keep_it_uncloaks_once_cloaked() {
        let mut cloak = Cloak::default();
        cloak.cloak();
        let mut ticks = 0;
        while !cloak.is_cloaked() {
            think(&mut cloak, ZEPHYR, Situation::default(), &mut 0.0);
            cloak.step(ZEPHYR.fast());
            ticks += 1;
        }
        assert_eq!(cloak.direction, 1, "an early uncloak does nothing");
        think(&mut cloak, ZEPHYR, Situation::default(), &mut 0.0);
        assert_eq!(cloak.direction, -1);
        let mut cloaked = 0;
        while cloak.is_cloaked() {
            cloak.step(ZEPHYR.fast());
            cloaked += 1;
        }
        assert_eq!((ticks, cloaked), (22, 12));
    }

    #[test]
    fn a_ship_that_can_no_longer_cloak_uncloaks() {
        let mut cloak = CLOAKED;
        hold(&mut cloak, true);
        assert_eq!(cloak, CLOAKED);
        hold(&mut cloak, false);
        assert_eq!(cloak, at(FADE_FULL, -1));
    }

    #[test]
    fn a_hit_by_a_ship_it_is_not_attacking_cloaks_a_ship_of_flags2_0x4000() {
        let plain = CloakGear {
            decloaks_when_hit: false,
            ..ZEPHYR
        };
        let mut cloak = Cloak::default();
        hit(&mut cloak, plain, true, false, &mut 0.0);
        assert_eq!(cloak, at(0.0, 1));
        let mut fighting = Cloak::default();
        hit(&mut fighting, plain, true, true, &mut 0.0);
        assert_eq!(fighting, Cloak::default(), "it attacks the ship");
        let mut unable = Cloak::default();
        hit(&mut unable, plain, false, false, &mut 0.0);
        assert_eq!(unable, Cloak::default(), "it cannot cloak");
        let mut no_bit = Cloak::default();
        let other = CloakGear { flags2: 0, ..plain };
        hit(&mut no_bit, other, true, false, &mut 0.0);
        assert_eq!(no_bit, Cloak::default());
    }

    #[test]
    fn a_hit_says_whether_cloak_when_hit_held_cloaked_already_or_not() {
        // `_DamageShip` calls `_AIForceBurstReload` on these conditions
        // alone (@0x3bb95-0x3bbd5), before `_DoShipCloak`, which leaves a
        // ship already cloaked as it is.
        let plain = CloakGear {
            decloaks_when_hit: false,
            ..ZEPHYR
        };
        for start in [Cloak::default(), CLOAKED, at(20.0, 1), at(20.0, -1)] {
            let mut cloak = start;
            assert!(hit(&mut cloak, plain, true, false, &mut 0.0), "{start:?}");
            let mut cloak = start;
            assert!(
                !hit(&mut cloak, plain, true, true, &mut 0.0),
                "{start:?}: attacking"
            );
            let mut cloak = start;
            assert!(
                !hit(&mut cloak, plain, false, false, &mut 0.0),
                "{start:?}: unable"
            );
            let mut cloak = start;
            let other = CloakGear { flags2: 0, ..plain };
            assert!(
                !hit(&mut cloak, other, true, false, &mut 0.0),
                "{start:?}: no bit"
            );
        }
        let mut cloak = Cloak::default();
        assert!(
            hit(&mut cloak, ZEPHYR, true, false, &mut 0.0),
            "a decloaking cloak too"
        );
    }

    #[test]
    fn a_hit_uncloaks_a_cloak_that_decloaks_when_hit_fully_faded_or_fading_in() {
        let gear = CloakGear {
            flags2: 0,
            ..ZEPHYR
        };
        let mut full = CLOAKED;
        hit(&mut full, gear, true, true, &mut 0.0);
        assert_eq!(full, at(FADE_FULL, -1));
        let mut rising = at(20.0, 1);
        hit(&mut rising, gear, true, true, &mut 0.0);
        assert_eq!(rising, at(20.0, -1));
        let mut holding = at(20.0, 0);
        hit(&mut holding, gear, true, true, &mut 0.0);
        assert_eq!(holding, at(20.0, 0), "neither full nor fading in");
        let mut kept = CLOAKED;
        let tough = CloakGear {
            decloaks_when_hit: false,
            ..gear
        };
        hit(&mut kept, tough, true, true, &mut 0.0);
        assert_eq!(kept, CLOAKED);
    }

    #[test]
    fn a_zephyr_hit_by_a_ship_it_is_not_attacking_starts_cloaking_all_the_same() {
        // It cloaks, then its decloak finds it fading in but not yet
        // cloaked, which does nothing.
        let mut cloak = Cloak::default();
        hit(&mut cloak, ZEPHYR, true, false, &mut 0.0);
        assert_eq!(cloak, at(0.0, 1));
    }

    #[test]
    fn an_area_is_centred_and_moved() {
        let area = Area::centred(Vec2::new(10.0, -20.0), 100.0, 60.0);
        assert_eq!(
            area,
            Area {
                left: -40.0,
                top: -50.0,
                right: 60.0,
                bottom: 10.0,
            }
        );
        assert_eq!(
            area.offset(Vec2::new(1.0, 2.0)),
            Area {
                left: -39.0,
                top: -48.0,
                right: 61.0,
                bottom: 12.0,
            }
        );
        assert_eq!(
            DEFAULT_PLAY_AREA,
            Area::centred(Vec2::ZERO, 800.0 - 194.0, 600.0)
        );
    }

    const VIEW: Area = Area {
        left: -300.0,
        top: -200.0,
        right: 300.0,
        bottom: 200.0,
    };

    #[test]
    fn a_ship_is_on_screen_with_its_centre_a_quarter_of_its_sprite_inside_each_edge() {
        // A 40 x 80 sprite: a quarter is 10 across and 20 down.
        let shown = |x, y| on_screen(VIEW, Vec2::new(x, y), 40.0, 80.0);
        assert!(shown(0.0, 0.0));
        assert!(shown(289.0, 0.0));
        assert!(
            !shown(290.0, 0.0),
            "the sprite meets the inset view at its edge"
        );
        assert!(shown(-289.0, 0.0));
        assert!(!shown(-290.0, 0.0));
        assert!(shown(0.0, 179.0));
        assert!(!shown(0.0, 180.0));
        assert!(shown(0.0, -179.0));
        assert!(!shown(0.0, -180.0));
        assert!(!shown(1000.0, 0.0));
    }

    #[test]
    fn the_inset_is_three_quarters_of_the_sprite_truncated() {
        // 0.75 x 30 = 22.5, inset 22; the sprite reaches 15 out.
        let shown = |x| on_screen(VIEW, Vec2::new(x, 0.0), 30.0, 30.0);
        assert!(shown(292.0));
        assert!(!shown(293.0));
    }

    #[test]
    fn a_sprite_whose_inset_closes_the_view_is_never_on_screen() {
        // 0.75 x 400 = 300 each side of a 600-wide view leaves nothing.
        assert!(!on_screen(VIEW, Vec2::ZERO, 400.0, 10.0));
        assert!(on_screen(VIEW, Vec2::ZERO, 398.0, 10.0));
        // 0.75 x 267 = 200.25, inset 200 each side of a 400-tall view,
        // leaves nothing.
        assert!(!on_screen(VIEW, Vec2::ZERO, 10.0, 267.0));
        assert!(on_screen(VIEW, Vec2::ZERO, 10.0, 264.0));
    }

    #[test]
    fn a_sprite_larger_than_the_view_is_never_on_screen() {
        assert!(!on_screen(VIEW, Vec2::ZERO, 900.0, 10.0));
        assert!(!on_screen(VIEW, Vec2::ZERO, 10.0, 600.0));
        assert!(
            on_screen(VIEW, Vec2::ZERO, 0.0, 0.0),
            "a ship with no sprite"
        );
    }

    fn slots(items: &[(i16, u16)]) -> Vec<(OutfitId, u16)> {
        items.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn a_ship_types_cloak_upkeep_is_its_first_cloak_in_slot_order() {
        let outfits = [
            outfit(268, &[(MOD_CLOAK, 0x080b)]),
            outfit(269, &[(1, 1), (MOD_CLOAK, 0x0404 | 0x00f0)]),
            outfit(270, &[(MOD_CLOAK, 0x0300), (MOD_CLOAK, 0x0504)]),
        ];
        let gear = |items: &[(i16, u16)]| CloakGear::default().with_upkeep(&slots(items), &outfits);
        assert_eq!(
            (
                gear(&[(268, 1)]).shield_usage,
                gear(&[(268, 1)]).zeroes_shields
            ),
            (8, false)
        );
        assert_eq!(
            (
                gear(&[(269, 1)]).shield_usage,
                gear(&[(269, 1)]).zeroes_shields
            ),
            (4, true),
            "the nibble alone, past a non-cloak mod"
        );
        assert_eq!(
            (
                gear(&[(268, 1), (269, 1)]).shield_usage,
                gear(&[(268, 1), (269, 1)]).zeroes_shields
            ),
            (8, false),
            "the earlier slot decides"
        );
        assert_eq!(
            (
                gear(&[(269, 1), (268, 1)]).shield_usage,
                gear(&[(269, 1), (268, 1)]).zeroes_shields
            ),
            (4, true),
            "slot order, not outfit order"
        );
        assert_eq!(
            gear(&[(269, 0), (268, 1)]).shield_usage,
            8,
            "a count-0 slot is skipped"
        );
        assert_eq!(
            (
                gear(&[(270, 1)]).shield_usage,
                gear(&[(270, 1)]).zeroes_shields
            ),
            (3, false),
            "the outfit's first cloak mod"
        );
        assert_eq!(gear(&[]), CloakGear::default());
        assert_eq!(gear(&[(999, 1)]), CloakGear::default());
    }

    fn zeroing() -> CloakGear {
        CloakGear {
            device: true,
            zeroes_shields: true,
            ..CloakGear::default()
        }
    }

    #[test]
    fn cloaking_with_a_zeroing_cloak_zeroes_the_shield_unless_already_cloaked() {
        let mut cloak = Cloak::default();
        let mut shield = 50.0;
        assert!(
            cloak_ship(&mut cloak, zeroing(), &mut shield),
            "the sound condition"
        );
        assert_eq!((cloak, shield), (at(0.0, 1), 0.0));

        let mut fading_in = at(10.0, 1);
        let mut shield = 50.0;
        assert!(!cloak_ship(&mut fading_in, zeroing(), &mut shield));
        assert_eq!(shield, 0.0, "fading in, not yet cloaked");

        let mut cloaked = at(FADE_FULL, 0);
        let mut shield = 50.0;
        cloak_ship(&mut cloaked, zeroing(), &mut shield);
        assert_eq!(shield, 50.0, "already cloaked");

        let mut cloak = Cloak::default();
        let mut shield = 50.0;
        cloak_ship(
            &mut cloak,
            CloakGear {
                device: true,
                ..CloakGear::default()
            },
            &mut shield,
        );
        assert_eq!((cloak, shield), (at(0.0, 1), 50.0), "no zeroing bit");
    }

    #[test]
    fn a_think_uncloaks_a_ship_whose_shield_cannot_pay_its_cloak() {
        let gear = CloakGear {
            device: true,
            shield_usage: 8,
            flags2: CLOAK_RETREATING,
            ..CloakGear::default()
        };
        let retreating = Situation {
            retreating: true,
            ..Situation::default()
        };
        let after = |gear, mut shield: f32| {
            let mut cloak = at(FADE_FULL, 0);
            think(&mut cloak, gear, retreating, &mut shield);
            cloak.direction
        };
        assert_eq!(after(gear, 0.0), -1, "shield at 0");
        assert_eq!(after(gear, -1.0), -1, "shield below 0");
        assert_eq!(after(gear, 0.5), 0, "shield left: the conditions hold it");
        let free = CloakGear {
            shield_usage: 0,
            ..gear
        };
        assert_eq!(after(free, 0.0), 0, "no usage: the conditions hold it");
    }

    #[test]
    fn a_think_cloaking_a_zeroing_cloak_zeroes_the_shield() {
        let gear = CloakGear {
            flags2: CLOAK_RETREATING,
            ..zeroing()
        };
        let mut cloak = Cloak::default();
        let situation = Situation {
            retreating: true,
            ..Situation::default()
        };
        let mut shield = 40.0;
        think(&mut cloak, gear, situation, &mut shield);
        assert_eq!((cloak, shield), (at(0.0, 1), 0.0));
    }

    #[test]
    fn cloaking_when_hit_with_a_zeroing_cloak_zeroes_the_shield() {
        let gear = CloakGear {
            flags2: CLOAK_WHEN_HIT,
            ..zeroing()
        };
        let mut cloak = Cloak::default();
        let mut shield = 30.0;
        assert!(hit(&mut cloak, gear, true, false, &mut shield));
        assert_eq!((cloak, shield), (at(0.0, 1), 0.0));
        let mut cloak = Cloak::default();
        let mut shield = 30.0;
        assert!(!hit(&mut cloak, gear, true, true, &mut shield));
        assert_eq!(shield, 30.0, "no cloak, no zeroing");
    }

    #[test]
    fn a_cloaked_ship_pays_its_shield_usage_a_tick_while_it_can() {
        let mut shield = 100.0;
        drain_shield(&mut shield, 8, true);
        assert_eq!(shield, 100.0 - 8.0 * CLOAK_UPKEEP_RATE);
        let mut shield = 8.0;
        drain_shield(&mut shield, 8, true);
        assert_eq!(
            shield,
            8.0 - 8.0 * CLOAK_UPKEEP_RATE,
            "shield equal to usage pays"
        );
        let mut shield = 7.9;
        drain_shield(&mut shield, 8, true);
        assert_eq!(shield, 7.9, "below usage: no drain");
        let mut shield = 100.0;
        drain_shield(&mut shield, 8, false);
        assert_eq!(shield, 100.0, "not cloaked");
        drain_shield(&mut shield, 0, true);
        assert_eq!(shield, 100.0, "no usage");
        let mut shield = 0.1;
        drain_shield(&mut shield, 15, true);
        assert_eq!(shield, 0.1);
        assert_eq!(CLOAK_UPKEEP_RATE, 0.03333);
    }

    /// The cloak after one think of a ship with a cloak and
    /// `flags2`, starting at `start`, in `situation`.
    fn thought(flags2: u16, start: Cloak, situation: Situation) -> Cloak {
        let gear = CloakGear {
            device: true,
            flags2,
            ..CloakGear::default()
        };
        let mut cloak = start;
        think(&mut cloak, gear, situation, &mut 0.0);
        cloak
    }

    /// Checks that `bit` cloaks a ship at rest and keeps a cloaked one
    /// cloaked in `holds`, and that the ship uncloaks without the bit or
    /// in a default situation.
    fn cloaks_by(bit: u16, holds: Situation, what: &str) {
        assert_eq!(
            thought(bit, Cloak::default(), holds),
            at(0.0, 1),
            "{what}: cloaks"
        );
        assert_eq!(thought(bit, CLOAKED, holds), CLOAKED, "{what}: keeps it");
        assert_eq!(
            thought(bit, CLOAKED, Situation::default()),
            at(FADE_FULL, -1),
            "{what}: uncloaks once it no longer holds"
        );
        assert_eq!(
            thought(0, CLOAKED, holds),
            at(FADE_FULL, -1),
            "{what}: without the bit"
        );
        // 0x1000 keeps a cloaked miner's cloak on its own.
        assert_eq!(
            thought(!(bit | CLOAK_PRESSING), CLOAKED, holds),
            at(FADE_FULL, -1),
            "{what}: every other bit"
        );
    }

    #[test]
    fn a_ship_whose_flags2_cloak_it_reloading_cloaks_while_a_weapon_reloads() {
        let reloading = Situation {
            reloading: true,
            ..Situation::default()
        };
        cloaks_by(CLOAK_RELOADING, reloading, "0x0100");
        assert_eq!(CLOAK_RELOADING, 0x0100);
    }

    #[test]
    fn a_ship_whose_flags2_cloak_it_jumping_cloaks_through_a_jump() {
        let jumping = Situation {
            jumping: true,
            ..Situation::default()
        };
        cloaks_by(CLOAK_JUMPING, jumping, "0x0400");
        assert_eq!(CLOAK_JUMPING, 0x0400);
    }

    #[test]
    fn a_ship_whose_flags2_cloak_it_travelling_cloaks_while_it_travels() {
        let travelling = Situation {
            travelling: true,
            ..Situation::default()
        };
        cloaks_by(CLOAK_TRAVELLING, travelling, "0x0800");
        assert_eq!(CLOAK_TRAVELLING, 0x0800);
    }

    #[test]
    fn a_ship_whose_flags2_cloak_it_idle_cloaks_while_it_idles() {
        let idle = Situation {
            idle: true,
            ..Situation::default()
        };
        cloaks_by(CLOAK_IDLE, idle, "0x2000");
        assert_eq!(CLOAK_IDLE, 0x2000);
    }

    #[test]
    fn a_miner_chasing_its_rock_cloaks_by_0x0200() {
        let mining = |order| Situation {
            mining: Some(order),
            ..Situation::default()
        };
        cloaks_by(CLOAK_RETREATING, mining(MineOrder::Chase), "chase");
        for order in [MineOrder::Stop, MineOrder::Close] {
            assert_eq!(
                thought(CLOAK_RETREATING, Cloak::default(), mining(order)),
                Cloak::default(),
                "{order:?}"
            );
        }
    }

    #[test]
    fn a_cloaked_miner_keeps_its_cloak_by_0x1000_until_it_closes() {
        let mining = |order| Situation {
            mining: Some(order),
            ..Situation::default()
        };
        for order in [MineOrder::Stop, MineOrder::Chase] {
            let bit = CLOAK_PRESSING;
            assert_eq!(thought(bit, CLOAKED, mining(order)), CLOAKED, "{order:?}");
            assert_eq!(
                thought(bit, Cloak::default(), mining(order)),
                Cloak::default(),
                "{order:?}: only a cloaked miner keeps it"
            );
            assert_eq!(
                thought(0, CLOAKED, mining(order)),
                at(FADE_FULL, -1),
                "{order:?}: no bit"
            );
        }
        assert_eq!(
            thought(CLOAK_PRESSING, CLOAKED, mining(MineOrder::Close)),
            at(FADE_FULL, -1),
            "inside the box it uncloaks"
        );
    }

    #[test]
    fn an_escort_close_to_its_cloaked_parent_cloaks_whatever_its_bits() {
        let close = Situation {
            close_to_cloaked_parent: true,
            ..Situation::default()
        };
        assert_eq!(thought(0, Cloak::default(), close), at(0.0, 1));
        assert_eq!(thought(0, CLOAKED, close), CLOAKED);
        assert_eq!(
            thought(0, CLOAKED, Situation::default()),
            at(FADE_FULL, -1),
            "not close: no cloak"
        );
        let mut none = CLOAKED;
        think(&mut none, CloakGear::default(), close, &mut 0.0);
        assert_eq!(none, at(FADE_FULL, -1), "no cloak");
        let disabled = Situation {
            disabled: true,
            ..close
        };
        assert_eq!(thought(0, CLOAKED, disabled), at(FADE_FULL, -1), "disabled");
        let costly = CloakGear {
            device: true,
            shield_usage: 4,
            ..CloakGear::default()
        };
        let mut drained = CLOAKED;
        think(&mut drained, costly, close, &mut 0.0);
        assert_eq!(drained, at(FADE_FULL, -1), "no shield to pay");
        let mut shield = 30.0;
        let mut cloak = Cloak::default();
        think(&mut cloak, zeroing(), close, &mut shield);
        assert_eq!((cloak, shield), (at(0.0, 1), 0.0), "zeroes the shield");
    }
}
