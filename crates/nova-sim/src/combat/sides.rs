//! Who a shot or a beam may hit: the original's `_ShotCanHitShip`
//! (@0x4477e-0x44b6f) for shots and seeking sub-munitions
//! (`_FindNearestValidShotTarget` @0x6a3a-0x6af3 asks it of every active
//! ship, the player's included), and `_HandleBeams`' candidate filter
//! (@0x30b8a-0x30dd8) for beams. Each reads the ships' [`Ties`]: leader,
//! government and `düde`.
//!
//! Two facts about a ship recur, each walking leaders through the fight's
//! ships:
//!
//! - **Allied with the player** (`_AlliedWithPlayer` @0x6985-0x69d0): the
//!   player, a ship it leads, or a ship led by one it leads.
//! - **On the player's side**: its chain of leaders, itself first, reaches
//!   the player (@0x448f8-0x44934).
//!
//! A shot ([`Sides::shot_can_hit`]) never hits a destroyed ship (the
//! callers' slot test, +0xb8), but does hit one breaking up, and never:
//!
//! 1. a ship of class 0x2ff (shïp 895, the escape pod) or after it
//!    (@0x447b3-0x447d9): the shot passes through it unspent;
//! 2. its firer (@0x447df-0x447f1);
//! 3. as a homing shot, any ship but its target, unless `Flags2` 0x0008
//!    (@0x44812-0x44836), nor anything once lost (the engine's stand-in
//!    for the original's jam 998, @0x447f7, until the shot tick takes it);
//! 4. as a shot no ship fired ([`Source::Other`]), any ship but the one it
//!    is aimed at, when it is aimed at one (@0x44a71-0x44a88); nothing
//!    below applies to it.
//!
//! A ship's shot, its firer read as it is now (see below), also never
//! hits a ship:
//!
//! - with the same leader as its firer (6a, @0x44846-0x44873);
//! - of its firer's government (6b, @0x44879-0x4488d), an independent's
//!   and the player's excepted;
//! - that is an escort's fighter (its leader's leader the player), allied
//!   with the player, when its firer is on the player's side (6f,
//!   @0x44936-0x4495f);
//! - of a government of `Flags` 0x0008 ([`PLAYER_SHOTS_PASS`]), or of a
//!   `düde` of `Booty` 0x0100 ([`UNTOUCHABLE_BY_PLAYER`]), or any ship at
//!   all when its firer's government has `Flags` 0x0040
//!   ([`NEVER_ATTACKS_PLAYER`]), when its firer is on the player's side
//!   (6h, 6i, 6g; @0x4496d-0x449f6);
//! - allied with the player, when its firer's `düde` has `Booty` 0x0100,
//!   or, with no `düde`, its leader's does (6j, @0x449fc-0x44a4e);
//! - of a government of `Flags` 0x0040, but for the player, when its firer
//!   is the player or a ship the player leads (9, @0x44ad2-0x44b2f);
//! - that shares its firer's root (`_ShipsShareParent` @0x7926-0x79c1,
//!   10): each climbs its leaders while it is active and has a leader not
//!   itself, so the player's shots pass through the ships it leads and
//!   theirs through it.
//!
//! A beam ([`Sides::beam_can_hit`]) never hits a destroyed ship, but does
//! stop on one breaking up, and tests no government; it never hits its
//! firer, its firer's leader, a ship its firer leads, a led ship allied
//! with the player when its firer is too (@0x30ba0-0x30c05, @0x30d10), the
//! `Booty` 0x0100 pairs as a shot's (@0x30c3c-0x30cf2), or class 0x2ff
//! itself, a later class being hit (@0x30dd2-0x30dd8).
//!
//! The flag rules (6g-6j, 9 and the beam's pairs) count the player as the
//! fight's [`PassThrough`] says: the original's whole player side, or the
//! Bible's player alone, for whom the player's own shots and beams pass
//! through ships of `Flags` 0x0008 or 0x0040 (its beams only 0x0040's,
//! "player's weapons can't hit them") and `Booty` 0x0100 `düde`s, and
//! those `düde`s' own fire passes through the player, with no leader's
//! `düde` read and no 6g.
//!
//! The original reads the firer's slot as it is when the shot meets a
//! ship. Here the firer is read among the fight's ships, and once it has
//! left them, from the copy of its ties the shot or beam took as it
//! fired: a stale slot is the nearest thing to what the original reads.
//! Each leader's walk reads the ships in the fight; a leader that has
//! left it ends the walk (the original reads its stale slot), and a walk
//! stops after one pass over the ships, should leaders form a loop.
//!
//! Left out, each with its owner: a shot past its life (@0x447c2) and the
//! jam (@0x447f7), weapons-fidelity phase 3; the nag ship (@0x44803), as
//! there is none; two defence ships of one stellar (6c, @0x44893) and a
//! non-ship shot's +0x8c clause (@0x44a81), task `planet-domination`; the
//! turret miss (6d, @0x448ad), weapons-fidelity phase 10; a miner's shots
//! and beams (6k, @0x44a54; @0x30f4c), asteroids phase 5; the planet-type
//! match (8, @0x44a8e; the beam's @0x30d4f), stellar-fidelity phase 2;
//! the beam's system test (@0x30dc1), as only the player's system is
//! simulated. None of these functions draws or plays a sound.

use super::ShipRef;
use super::beam::Beam;
use super::hull::ESCAPE_POD_SHIP;
use super::projectile::{Shot, Source, Target};
use super::weapon::Guidance;
use crate::catalog::GovtId;
use crate::govt::{Governments, NEVER_ATTACKS_PLAYER, PLAYER_SHOTS_PASS};
use crate::rulebook::RuleSource;

/// `düde` `Booty`: ships of the `düde` can't be hit by the player, and
/// their shots can't hit the player (the Bible); in the original, by and
/// at the whole player side (@0x449d6-0x44a4e, @0x30c66-0x30cf2).
pub const UNTOUCHABLE_BY_PLAYER: u16 = 0x0100;

/// What the hit rules read of a ship besides where it is and how it is
/// holding up: its leader (the original's +0x9a), its government (+0x98)
/// and its `düde`'s `Booty` (through +0x78).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ties {
    /// The ship it follows: the player for one that escorts the player,
    /// else the lead it escorts or the carrier it left; none for a ship
    /// led by no one, the player included (whose +0x9a is -1,
    /// `_ResetPlayer` @0x1d4e5).
    pub leader: Option<ShipRef>,
    /// Its government, or `None` for an independent ship or the player.
    pub govt: Option<GovtId>,
    /// Its `düde`'s `Booty`, or `None` when it has no `düde`.
    pub booty: Option<u16>,
}

impl Ties {
    /// Whether its `düde` has `Booty` 0x0100.
    fn untouchable(self) -> bool {
        self.booty
            .is_some_and(|booty| booty & UNTOUCHABLE_BY_PLAYER != 0)
    }
}

/// Who counts as the player where a government's or a `düde`'s flags let
/// shots and beams pass through, as the rulebook's
/// [`RuleKey::PlayerPassThrough`](crate::RuleKey::PlayerPassThrough) says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PassThrough {
    /// The original's: the whole player side, as each branch reads it.
    #[default]
    PlayerSide,
    /// The Bible's: the player's own ship alone.
    PlayerOnly,
}

impl From<RuleSource> for PassThrough {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::PlayerSide,
            RuleSource::Bible => Self::PlayerOnly,
        }
    }
}

/// What the hit rules read beyond the shot or beam and the ship: every
/// ship in the fight this tick, the governments' flags, and who counts as
/// the player for the pass-through flags.
#[derive(Clone, Copy, Debug)]
pub struct Sides<'a> {
    /// Every ship in the fight.
    pub targets: &'a [Target],
    /// Every government.
    pub govts: &'a Governments,
    /// Who counts as the player for the pass-through flags.
    pub pass_through: PassThrough,
}

impl Sides<'_> {
    /// Whether `shot` can hit `target` (see the module docs).
    #[must_use]
    pub fn shot_can_hit(&self, shot: &Shot, target: &Target) -> bool {
        if !target.condition.active() || target.ship_type >= ESCAPE_POD_SHIP || shot.lost {
            return false;
        }
        if shot.weapon.guidance == Guidance::Homing
            && shot.target != Some(target.ship)
            && !shot.weapon.proximity_by_others()
        {
            return false;
        }
        let kept = match shot.source {
            Source::Ship(kept) => kept,
            Source::Other => return shot.aimed_at.is_none_or(|aimed| aimed == target.ship),
        };
        let firer = shot.firer;
        let ties = self.ties_of(firer, kept);
        let theirs = target.ties;
        if target.ship == firer
            || (ties.leader.is_some() && ties.leader == theirs.leader)
            || (ties.govt.is_some() && ties.govt == theirs.govt)
        {
            return false;
        }
        let allied = self.allied_with_player(target.ship, theirs.leader);
        let players_side = self.on_players_side(firer, ties.leader);
        let escorts_fighter =
            theirs.leader.and_then(|leader| self.leader_of(leader)) == Some(ShipRef::Player);
        if players_side && allied && escorts_fighter {
            return false;
        }
        let (by_player, at_player) = self.counted(firer, players_side, target.ship, allied);
        if by_player
            && (self.govts.flag(ties.govt, NEVER_ATTACKS_PLAYER)
                || self.govts.flag(theirs.govt, PLAYER_SHOTS_PASS)
                || theirs.untouchable())
        {
            return false;
        }
        if at_player && self.firer_untouchable(ties) {
            return false;
        }
        let direct = match self.pass_through {
            PassThrough::PlayerSide => ties.leader == Some(ShipRef::Player),
            PassThrough::PlayerOnly => false,
        };
        if target.ship != ShipRef::Player
            && self.govts.flag(theirs.govt, NEVER_ATTACKS_PLAYER)
            && (firer == ShipRef::Player || direct)
        {
            return false;
        }
        self.root(target.ship) != self.root(firer)
    }

    /// Whether `beam` can hit `target` (see the module docs).
    #[must_use]
    pub fn beam_can_hit(&self, beam: &Beam, target: &Target) -> bool {
        let firer = beam.firer;
        if !target.condition.active() || target.ship == firer {
            return false;
        }
        let ties = self.ties_of(firer, beam.ties);
        let theirs = target.ties;
        let allied = self.allied_with_player(target.ship, theirs.leader);
        if let Some(leader) = theirs.leader
            && (leader == firer || (allied && self.allied_with_player(firer, ties.leader)))
        {
            return false;
        }
        if ties.leader == Some(target.ship) {
            return false;
        }
        let players_side = self.on_players_side(firer, ties.leader);
        let (by_player, at_player) = self.counted(firer, players_side, target.ship, allied);
        if (by_player && theirs.untouchable()) || (at_player && self.firer_untouchable(ties)) {
            return false;
        }
        if self.pass_through == PassThrough::PlayerOnly
            && firer == ShipRef::Player
            && self.govts.flag(theirs.govt, NEVER_ATTACKS_PLAYER)
        {
            return false;
        }
        target.ship_type != ESCAPE_POD_SHIP
    }

    /// Who the flag rules count as the player's: whether the firer's fire
    /// is, and whether the ship hit is, as [`PassThrough`] says. The
    /// original's are the firer `on_players_side` and the ship `allied`
    /// with the player; the Bible's, each being the player.
    fn counted(
        &self,
        firer: ShipRef,
        on_players_side: bool,
        ship: ShipRef,
        allied: bool,
    ) -> (bool, bool) {
        match self.pass_through {
            PassThrough::PlayerSide => (on_players_side, allied),
            PassThrough::PlayerOnly => (firer == ShipRef::Player, ship == ShipRef::Player),
        }
    }

    /// Whether a firer of `ties` has a `düde` of `Booty` 0x0100: its own,
    /// or, with none, its leader's, by the original's reading
    /// (@0x44a02-0x44a4e, @0x30cad-0x30cf2); its own alone by the Bible's.
    fn firer_untouchable(&self, ties: Ties) -> bool {
        match (ties.booty, self.pass_through) {
            (Some(_), _) => ties.untouchable(),
            (None, PassThrough::PlayerOnly) => false,
            (None, PassThrough::PlayerSide) => ties
                .leader
                .and_then(|leader| self.find(leader))
                .is_some_and(|leader| leader.ties.untouchable()),
        }
    }

    /// The fight's ship `ship`, if it is in the fight.
    fn find(&self, ship: ShipRef) -> Option<&Target> {
        self.targets.iter().find(|target| target.ship == ship)
    }

    /// `ship`'s ties as they are now, or `kept` once it has left the
    /// fight.
    fn ties_of(&self, ship: ShipRef, kept: Ties) -> Ties {
        self.find(ship).map_or(kept, |target| target.ties)
    }

    /// `ship`'s leader now; none once it has left the fight.
    fn leader_of(&self, ship: ShipRef) -> Option<ShipRef> {
        self.find(ship).and_then(|target| target.ties.leader)
    }

    /// Whether `ship`, led by `leader`, is allied with the player
    /// (`_AlliedWithPlayer`).
    fn allied_with_player(&self, ship: ShipRef, leader: Option<ShipRef>) -> bool {
        ship == ShipRef::Player
            || leader == Some(ShipRef::Player)
            || leader.and_then(|leader| self.leader_of(leader)) == Some(ShipRef::Player)
    }

    /// Whether `ship`, led by `leader`, is on the player's side: its chain
    /// of leaders, itself first, reaches the player.
    fn on_players_side(&self, ship: ShipRef, leader: Option<ShipRef>) -> bool {
        if ship == ShipRef::Player {
            return true;
        }
        let mut next = leader;
        for _ in 0..=self.targets.len() {
            match next {
                Some(ShipRef::Player) => return true,
                Some(ship) => next = self.leader_of(ship),
                None => return false,
            }
        }
        false
    }

    /// The ship `ship`'s leaders lead up to (`_ShipsShareParent`): each
    /// step from a ship still active to a leader not itself.
    fn root(&self, ship: ShipRef) -> ShipRef {
        let mut at = ship;
        for _ in 0..=self.targets.len() {
            let leader = self
                .find(at)
                .filter(|target| target.condition.active())
                .and_then(|target| target.ties.leader);
            match leader {
                Some(leader) if leader != at => at = leader,
                _ => break,
            }
        }
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engines_reading_counts_the_player_side_and_the_bibles_the_player_alone() {
        assert_eq!(
            PassThrough::from(RuleSource::Engine),
            PassThrough::PlayerSide
        );
        assert_eq!(
            PassThrough::from(RuleSource::Bible),
            PassThrough::PlayerOnly
        );
        assert_eq!(PassThrough::default(), PassThrough::PlayerSide);
        assert_eq!(UNTOUCHABLE_BY_PLAYER, 0x0100);
    }

    #[test]
    fn a_ship_is_untouchable_by_its_dudes_bit_alone() {
        let of = |booty| Ties {
            booty,
            ..Ties::default()
        };
        assert!(of(Some(0x0141)).untouchable());
        assert!(!of(Some(0x0041)).untouchable());
        assert!(!of(None).untouchable());
    }
}
