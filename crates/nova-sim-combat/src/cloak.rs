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
//!   at 0, with no direction.
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
//! uncloaks it while none does (@0x83682, @0x83694). Built here are the
//! conditions the stock Zephyr (ship 375 and 376, the ships of missions
//! 603 and 604, `Flags2` 0x5282) carries:
//!
//! - [`CLOAK_RETREATING`] (`Flags2` 0x0200, @0x834b7-0x834e3): while it
//!   retreats (AI state 3, [`Situation::retreating`]).
//! - [`CLOAK_PRESSING`] (`Flags2` 0x1000, @0x83546-0x835fd): once
//!   cloaked, while it attacks a ship more than [`PRESS_RANGE`] (165)
//!   away on either axis ([`Situation::attacking`]); closer, it uncloaks
//!   to fight.
//! - [`CLOAK_WHEN_HIT`] (`Flags2` 0x4000, `_DamageShip` @0x3bb95-0x3bbdd,
//!   [`hit`]): hit by a ship it is not attacking (`_IsShipFightingMe`
//!   @0x827fa), while it can cloak.
//!
//! Every frame, too, a cloaked ship that can no longer cloak
//! ([`CloakGear::can_cloak`], `_ShipCanCloak` @0x553c: not disabled, a
//! cloak, and fuel) uncloaks (`_HandleShip` @0x3413d-0x3415f, [`hold`]),
//! and a weapon's hit uncloaks a ship whose cloak decloaks when hit while
//! it is fully faded or still fading in (`_DamageShip` @0x3bf1f-0x3bf4f,
//! [`hit`]).
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
//! # Not modelled
//!
//! The other AI-cloak conditions, which no stock mission ship carries:
//! `Flags2` 0x0100 (a weapon reloading past its `Reload`), 0x0400, 0x0800
//! and 0x2000, the parent-cloaked rule (@0x83633-0x8366d), 0x0200's
//! state 0x10 substate 0x13 and 0x1000's state 0x10 case (a miner on an
//! asteroid); `_AIForceBurstReload` as a ship cloaks when hit (the
//! Zephyr's weapons have no burst); the think's 30-59 frame delay after a
//! ship's hyperspace glide (`_HandleShip` @0x34d74); a cloak's fuel
//! upkeep (the uncloak @0x83422-0x8343a and the drain @0x34164-0x341ac),
//! which no NPC pays (`_CloakFuelUsage` @0x3e25 is 0 for one,
//! @0x3e31-0x3e39); `_DoShipCloak`'s `_statusRedraw[1]` (@0xdf35-0xdf4b),
//! player-only, as the HUD is drawn every frame; the player's own cloak,
//! with its usages read from its owned outfits (@0x3e3b, @0x3eb0,
//! @0x4046), so raising the cloak
//! sounds (rdm:phase/cloaking-fidelity/phase-8-cloaking-device) and its
//! own full fade (@0x2d199-0x2d1a1); area cloaks; NPCs' own sight of
//! cloaked ships for their targeting (cloaking-fidelity 4; point defence
//! and homing shots already ask [`seen`]).
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
/// The `Flags2` bit that cloaks a ship while it retreats.
pub const CLOAK_RETREATING: u16 = 0x0200;
/// The `Flags2` bit that keeps a ship cloaked while it presses an attack
/// from beyond [`PRESS_RANGE`].
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
    /// Whether a hit uncloaks it ([`CLOAK_DECLOAKS_WHEN_HIT`]).
    pub decloaks_when_hit: bool,
    /// Whether it shows on the radar while cloaked ([`CLOAK_ON_RADAR`]).
    pub on_radar: bool,
    /// Its `Flags2`, whose AI-cloak bits, [`FAST_FADE`] and
    /// [`UNTARGETABLE`] it reads.
    pub flags2: u16,
    /// The shield its cloak costs, a tick, while cloaked
    /// (`_CloakShieldUsage` @0x4033: `(ModVal >> 8) & 0xf` of the first
    /// cloak, see [`CloakGear::with_upkeep`]), times
    /// [`CLOAK_UPKEEP_RATE`].
    pub shield_usage: u8,
    /// Whether cloaking zeroes its shield (`_CloakZeroesShields` @0x3e9d:
    /// the first cloak's `ModVal` bit [`CLOAK_ZEROES_SHIELDS`]).
    pub zeroes_shields: bool,
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
    #[must_use]
    pub fn with_upkeep(self, slots: &[(OutfitId, u16)], outfits: &[OutfitRecord]) -> Self {
        let first = slots
            .iter()
            .filter(|&&(_, count)| count > 0)
            .filter_map(|&(id, _)| outfits.iter().find(|record| record.id == id))
            .flat_map(|record| record.mods)
            .find(|&(kind, _)| kind == MOD_CLOAK);
        let Some((_, mod_val)) = first else {
            return self;
        };
        Self {
            // Masked to a nibble, so it fits.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            shield_usage: ((mod_val >> 8) & 0xf) as u8,
            zeroes_shields: mod_val & CLOAK_ZEROES_SHIELDS != 0,
            ..self
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

    /// Whether a ship of it can cloak (`_ShipCanCloak`): it has a cloak,
    /// is not `disabled`, and has `fuel` left.
    #[must_use]
    pub fn can_cloak(self, disabled: bool, fuel: f32) -> bool {
        self.device && !disabled && fuel > 0.0
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Situation {
    /// Whether it is disabled.
    pub disabled: bool,
    /// Whether it retreats (AI state 3: here, fleeing).
    pub retreating: bool,
    /// Where the ship it attacks is from it, if it attacks one.
    pub attacking: Option<Vec2>,
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
    let retreating = gear.has(CLOAK_RETREATING) && situation.retreating;
    let pressing = gear.has(CLOAK_PRESSING)
        && cloak.is_cloaked()
        && situation
            .attacking
            .is_some_and(|off| off.x.abs() > PRESS_RANGE || off.y.abs() > PRESS_RANGE);
    if retreating || pressing {
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
    if gear.decloaks_when_hit && (cloak.full() || cloak.direction > 0) {
        cloak.uncloak();
    }
    cloaks
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
                zeroes_shields: false,
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

    #[test]
    fn a_ship_can_cloak_with_a_cloak_fuel_and_not_disabled() {
        let gear = CloakGear {
            device: true,
            ..CloakGear::default()
        };
        assert!(gear.can_cloak(false, 1.0));
        assert!(!gear.can_cloak(true, 1.0), "disabled");
        assert!(!gear.can_cloak(false, 0.0), "no fuel");
        assert!(!CloakGear::default().can_cloak(false, 1.0), "no cloak");
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
        zeroes_shields: false,
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
}
