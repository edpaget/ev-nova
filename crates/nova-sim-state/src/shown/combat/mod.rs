//! The fight's vocabulary, from `nova_sim_combat::combat`, which
//! re-exports each item at its old path: the ids of ships and shots, the
//! [`CombatEvent`]s a fight raises for the view, and the hull, sprite and
//! report types of its submodules.

pub mod hull;
pub mod hull_turn;
pub mod ion_tint;
pub mod projectile;
pub mod report;
pub mod sprite_load;

use nova_sim_base::geometry::Vec2;
use nova_sim_catalog::catalog::{Explosion, ShipId, StellarId, WeaponId};

/// An NPC's number in its system, unique while the player stays there:
/// the order it appeared in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NpcId(pub u32);

/// A ship in a fight: the player's, or an NPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipRef {
    /// The player's ship.
    Player,
    /// This NPC.
    Npc(NpcId),
}

/// Something that happened in a fight, for the view to show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CombatEvent {
    /// A ship fired a weapon, or a shot of its released sub-munitions.
    Fired {
        /// The ship.
        ship: ShipRef,
        /// The weapon.
        weapon: WeaponId,
        /// Where it was fired from: the ship, or the shot releasing them.
        at: Vec2,
        /// How its weapon's `Sound` plays for it, if at all: its priority,
        /// and whether a looped sound waits while it plays anywhere. None
        /// for a call's later copies, which the original plays once after
        /// them all.
        sound: Option<FireSound>,
    },
    /// Point defence destroyed a missile.
    ShotDown {
        /// Where.
        at: Vec2,
    },
    /// A shot ended its life without detonating, its weapon of an
    /// `ExplodType` above none (`nova_sim_combat::combat::weapon::WeaponSpec::fizzles`):
    /// it fizzles out where it is, as `_SpawnExplod(x, y, 0, 0, 0)`
    /// (`_HandleShot` @0x36149-0x36175), explosion type 0, unheard.
    Fizzled {
        /// Where.
        at: Vec2,
    },
    /// A shot or beam exploded where it hit, or a shot detonated.
    Exploded {
        /// Where.
        at: Vec2,
        /// Its explosion.
        explosion: Explosion,
        /// Its size (`_CreateExplosion`'s fourth argument, @0x3f698): how
        /// far an [`Explosion::extra`] one's extras scatter, and how many
        /// there are. A weapon's hit, beam tick or detonation passes its
        /// `BlastRadius` (`nova_sim_combat::combat::weapon::WeaponSpec::blast_radius`,
        /// wëap +0x18); every other explosion passes 0
        /// (`Combat::explode`: an asteroid or a stellar destroyed, a
        /// sabotage bomb). The original's is signed, while the
        /// loaded `BlastRadius` is floored at none: a negative size
        /// scatters none either way.
        size: f32,
    },
    /// An explosion of size 0 whose sound the session has raised already,
    /// as a `SimSound::Explosion`: a stellar a `Y` destroyed (`_EvalSetExp`
    /// @0x15ab4; see the `nova_sim_combat::combat::script_explosions` module). Drawn as an
    /// [`Exploded`](Self::Exploded) is, its sound not played again.
    ExplodedHeard {
        /// Where.
        at: Vec2,
        /// Its explosion.
        explosion: Explosion,
    },
    /// A ship was disabled.
    Disabled {
        /// The ship.
        ship: ShipRef,
    },
    /// A ship's armour is gone, and it is breaking up.
    BreakingUp {
        /// The ship.
        ship: ShipRef,
        /// Where it is.
        at: Vec2,
        /// Its `Explode1`, if any.
        explosion: Option<Explosion>,
    },
    /// Particles were thrown off: a shot's hit on a ship, an asteroid or a
    /// stellar, a beam's on a ship, or an asteroid destroyed (see
    /// `nova_sim_combat::combat::asteroid_hit::hit_burst`).
    Particles(ParticleBurst),
    /// A hit landed on a ship and reached `_DamageShip`'s shield-flash
    /// write (@0x3bef9-0x3bf0c): a shot's direct hit, each ship its blast
    /// reaches, a beam tick, a death's shockwave, or a missile lock, which
    /// does no damage but flashes its target's shield all the same
    /// (@0x35855). Every path past `_DamageShip`'s entry guards reaches the
    /// write, which follows the reaction block's skip target: a hit
    /// answered or not, the hit that takes a ship's last armour, and a hit
    /// on a disabled or breaking-up ship; the escape pod and every class
    /// after it take none (`nova_sim_combat::combat::hull::immune`, @0x3a87c-0x3a882). The sabotage
    /// bomb passes shields (@0x699b2) and raises none. The mark that
    /// follows for a hit on the player, `_statusRedraw[1] = 1`
    /// (@0x3bf0c-0x3bf1b), is left out: the cockpit reads the reserves
    /// every frame in flight, where every hit lands.
    Hit {
        /// The ship hit.
        ship: ShipRef,
        /// The weapon whose shot, blast or beam it was; none for a
        /// shockwave.
        weapon: Option<WeaponId>,
        /// Where it landed, loosely: where its shot met the ship, its
        /// blast's centre, its beam's end, or the wreck whose shockwave
        /// it was.
        at: Vec2,
        /// Whether its weapon passes shields (`Flags` 0x0020), which
        /// leaves the flash as it was.
        passes_shields: bool,
        /// Whether the original shows its flash a frame later, the ship's
        /// `_HandleShipDisplay` having run that frame before the hit: any
        /// hit on the player, whose display comes first (@0x43e29); a
        /// beam's, as `_HandleBeams` runs last (@0x4453d); and a
        /// shockwave's on a ship before its wreck in the fight's order,
        /// the original's ship table's.
        late: bool,
    },
    /// A ship's weapon glow lit (`Flags2` 0x0200, ship +0xc8e8 = 32.0):
    /// the view shows its `shän` `WeapImageID` sprite at full and lets it
    /// fade by its `WeapDecay`. The player's call that fired a glow weapon
    /// lights it (`_FirePlayerWeapon` @0x62c78-0x62c98); a fighter bay's
    /// call only once a fighter it launched is put in the system
    /// (@0x62c61-0x62c6e), which the session decides and lights through
    /// `nova_sim_combat::combat::Combat::light_glow`; and an NPC's pick that spawned one
    /// (`_FireAIShipWeapon` @0x88dc0-0x88de4), once a call; and each
    /// live beam of one, on every advance-pass tick, for its firer
    /// (`_HandleBeams` @0x30b1f-0x30b37), but a point-defence beam's at a
    /// missile (@0x309ab-0x309bb), an escort's formation pick among them.
    /// Point defence's firing, an NPC's bay launch and a sub-munition
    /// release never light it.
    WeaponGlow {
        /// The ship.
        ship: ShipRef,
        /// Whether the original shows it a frame later, as with
        /// [`CombatEvent::Hit::late`]: a beam's, as `_HandleBeams` runs
        /// after every ship's `_HandleShipDisplay` (@0x4453d), so it shows
        /// the next frame, and once after the beam's last tick. A firing
        /// comes before its ship's display (@0x43e19, @0x44483) and shows
        /// that frame.
        late: bool,
    },
    /// A ship was destroyed: the view draws its explosion and debris.
    Destroyed {
        /// The ship.
        ship: ShipRef,
        /// Its ship type.
        ship_type: ShipId,
        /// Where it was.
        at: Vec2,
        /// How it was moving.
        velocity: Vec2,
        /// Its `Explode2`, if any.
        explosion: Option<Explosion>,
        /// The explosion's size (`nova_sim_combat::combat::hull::HullSpec::death_size`): how far its
        /// extra explosions scatter, and how many there are.
        size: f32,
    },
    /// A dying NPC ejected an escape pod, one of its crew's or its
    /// person's (`_SpawnEscapePod` @0x45547-0x456cc, called from
    /// `_HandleShip` @0x351c3 and @0x352c9; see `nova_sim::npc_escape`).
    /// The pod is a sprite, no ship: the view sets it adrift from here and
    /// plays `_escapePodSnd` from here, `snd ` 372
    /// ([`ESCAPE_POD_SOUND`](nova_sim_catalog::sound::ESCAPE_POD_SOUND)),
    /// whether or not it has room for the pod, as `_HandleShip` plays it
    /// after the spawn at priority 5 (`_PlaySoundDistance` @0x351fa,
    /// @0x35300), which the view doesn't play by yet.
    PodEjected {
        /// The ship it left.
        ship: ShipRef,
        /// Where the ship was.
        at: Vec2,
        /// How the ship was moving.
        velocity: Vec2,
    },
    /// A stellar's gun fired a shot (`_HandleStellarWeapons`
    /// @0x2f553-0x2f59a, see `nova_sim_combat::combat::stellar_guns`): its weapon's `Sound` is
    /// heard from the stellar, once a shot, at
    /// `nova_sim_combat::combat::STELLAR_GUN_PRIORITY`, with no looped test. A stellar is no
    /// ship, so [`CombatEvent::Fired`] can't carry it.
    StellarFired {
        /// The stellar.
        stellar: StellarId,
        /// Its weapon.
        weapon: WeaponId,
        /// Where it fired from: the stellar.
        at: Vec2,
        /// How its weapon's `Sound` plays.
        sound: FireSound,
    },
}

/// How a weapon's firing sounds: its `Sound` (`snd ` 200 + `Sound`,
/// which the view reads from the weapon's looks), played at the priority
/// the function that fires it gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireSound {
    /// The priority it plays at (`_PlaySoundDistance`'s third argument):
    /// how much it matters when the audio side's voices are full.
    pub priority: u8,
    /// Whether the function firing it skips a looped weapon's sound
    /// (`Flags` 0x0010) while that `snd ` plays on any voice
    /// (`ST_IsSoundPlaying`), as `_FireAIShipWeapon` (@0x88e00-0x88e1d),
    /// `_FirePlayerWeapon` (@0x62d98-0x62dbf), `_HandleShipPointDefense`
    /// (@0x39981-0x3999e) and `_SpawnShotSubmunitions` (@0x3f25b-0x3f26c)
    /// do. `_AILaunchFighter` tests nothing (@0x81429-0x81466). The view or
    /// the audio side applies the test.
    pub unless_playing: bool,
}

impl FireSound {
    /// A sound at `priority`, skipped while a looped one plays.
    #[must_use]
    pub const fn at(priority: u8) -> Self {
        Self {
            priority,
            unless_playing: true,
        }
    }
}

/// A burst of particles: `_SpawnParticles`' arguments (@0x44b70-0x44d4e),
/// for the view to draw. Each of `count` particles starts at `at`, or
/// within `spread` of it, and flies off at `speed`, varied by
/// `variation` percent either way, in a direction of its own, living a
/// tick count from `life`'s first to its second. None takes on any
/// source's velocity.
///
/// It carries no alpha: the original's argument after the colour
/// (0x1c(%esp)), passed straight on to `_NewParticle` (+0x24, @0xc50de),
/// is only tested against 0x20 by `_DrawParticlesOffscreen` (@0xc5a61,
/// @0xc5b2b), whose paths draw the same either way, every particle opaque
/// and then fading over its last 32 ticks of life. A shot's trail
/// (@0x35ea6), the only caller passing another, is the view's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleBurst {
    /// Where it goes off.
    pub at: Vec2,
    /// Each particle's speed, in pixels a tick.
    pub speed: f32,
    /// How far, in percent, a particle's speed varies either way: none
    /// unless 1-99.
    pub variation: u32,
    /// A particle's least and most life, in ticks.
    pub life: (u32, u32),
    /// Its colour, `00RRGGBB`.
    pub color: u32,
    /// How many particles.
    pub count: u32,
    /// How far from `at` a particle may start, in pixels: none for all
    /// at `at`.
    pub spread: f32,
}
