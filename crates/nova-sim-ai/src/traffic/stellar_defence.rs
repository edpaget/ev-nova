//! A stellar's defence fleet: the ships its `spöb` `DefenseDude` sends up
//! once it is under attack, a wave at a time while its count lasts, as
//! `_SpawnStellarDefenseShip` (@0x3d2c9-0x3d4b7), `_EnterMoreShips`'
//! respawn (@0x43c45-0x43d10), `_DoPlayGameWork`'s +0x47 pass
//! (@0x443d4-0x4444f) and `_DeleteAllComputerShips`' refund
//! (@0x3c02a-0x3c06f) do in the `EV Nova` executable. The defenders fly
//! by [`DefenceShipAi`](crate::ai::defence_ship::DefenceShipAi).
//!
//! **The counts.** `_LoadObjectData` keeps a stellar's raw `DefCount` at
//! +0x468 (@0x775eb) and decodes the ships it starts with into +0x466
//! ([`initial_left`], @0x776a6-0x776fb); a wave is `DefCount % 10`
//! ([`wave`], @0x43cd3-0x43cfe). [`DefenceShips`] holds the ships left of
//! each stellar once one is spent, given back, regrown or set, reading the
//! decode until then; [`StellarDefences`] holds them beside which
//! stellars are under attack (+0x47).
//!
//! **Where the state lives.** The ships left are saved state, as the
//! original's `_WritePilotData` writes +0x466 (@0x7282d, @0x7285e), so
//! they rest on the pilot (`nova_sim::Pilot::defence_ships`). The session
//! lends them to the [`Traffic`] for each step and refund
//! ([`Traffic::swap_defence_ships`]) and takes them back after, so the
//! respawn inside
//! [`Traffic::enter_more_ships`](super::Traffic::enter_more_ships) reads
//! them on every caller, the gate exit's extra pass among them, with no
//! parameter threaded through, and two passes in one step see each
//! other's spend. [`StellarDefences`] is a field of the traffic: its
//! under-attack marks are never saved, and a traffic Reset hands it on
//! ([`Traffic::take_defences`](super::Traffic::take_defences)), the ships
//! left empty then, as the pilot holds them between steps.
//!
//! **The regrowth** ([`DefenceShips::regrow`]): each day an active
//! dominated stellar below its [`regrowth_cap`] regrows one on
//! `Rand(450)`'s 0 (`_IncrementGameTime` @0xb5fc-0xb64c), which the
//! session's daily step runs.
//!
//! **Under attack (+0x47)** is stored, not derived, to keep the
//! original's frame order: the defence ship's spawn sets it (@0x3d4a8);
//! each frame, after the ship loop and before the stellars' guns,
//! `_DoPlayGameWork` clears it for the player's system's nav stellars and
//! sets it again for the stellar of every defender in the system
//! ([`StellarDefences::recompute`]); and every rebuild point clears it
//! for every stellar (`_HandleStellarSystemVisibility` @0x32d5d,
//! [`StellarDefences::clear`]). So the respawn, earlier in the frame,
//! reads the last frame's pass or a spawn's own mark, and the guns read
//! this frame's. Only the planet comm dialog's Demand Tribute sets it from
//! nothing (rdm:phase/play-feel-4/phase-2-planet-comm-dialog).
//!
//! **The respawn** ([`StellarDefences::due`]): `_EnterMoreShips` walks the
//! system's 16 `NavDefs` slots in order (system +0x2a, every stellar
//! listed, shown or not, [`World::listed`](super::World::listed)); the
//! first stellar under attack with ships left (+0x466 > 0) and fewer
//! defenders in the system than its wave launches one
//! ([`Traffic::spawn_defender`](super::Traffic::spawn_defender)), spends
//! one whether or not a slot was found (@0x43c39), and ends the walk: at
//! most one a pass. A wave of 0 (a `DefCount` of 20, say) never tops a
//! stellar up. It counts every ship in the system that defends the
//! stellar, as the original counts every active slot (@0x43c9f-0x43cd1).
//! The original counts the ships an in-flight move left behind too, as
//! they hold their slots until the next display pass; those are gone from
//! the system and not counted here, a one-frame difference of a
//! developer move only.
//!
//! **The spawn** ([`defender`]): `_SpecificDudeSpawn(dude, +0x14, 2, 0)`,
//! then with 1 (@0x3d31f, @0x3d357): a ship of the `düde`, its type among
//! those whose `AppearOn` holds, else among them all (the last argument
//! ORs into each type's +0xa41 cache, `_SelectShipFieldFromDude` @0x662b,
//! @0x6662, @0x6716), in a slot below [`DEFENCE_RESERVE`]. Then at the
//! stellar's centre, facing `Rand(360)` at its top speed (`_Accel`
//! @0x3d489), a warship (+0x88 = 3) of the stellar's government (+0x98 =
//! stellar +0xa, the live government), led by no one, of no mission,
//! boarded (+0xb9 = 1, so it can't be plundered), turned on the player
//! (`_AIMakeShipAttackPlayer` @0x3d498, whose quote gate it fails, having
//! no person), and its stellar marked under attack.
//!
//! **The refund** ([`DefenceShips::refund`]): each defender dropped
//! with its system's ships gives one back, capped at the raw `DefCount`,
//! so above 1000 the cap is out of reach as in the original. A defender
//! that lands keeps its slot and its stellar (see
//! [`land_flight`](super::land_flight)), so it gives nothing back.
//!
//! Nothing here plays a sound or draws but the spawn's heading and
//! `_SpecificDudeSpawn`'s own draws.

use std::collections::{BTreeMap, BTreeSet};

use super::npc::{AiType, Npc, NpcId};
use super::spawn::{Fallback, NewShip, specific_dude_ship};
use super::table::SpawnTable;
use super::{NpcFormation, Traffic, World};
use crate::ai::Goal;
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::control::Gate;
use crate::trig;

/// The ship-table slots a defence ship's spawn leaves free:
/// `_SpawnStellarDefenseShip` passes 2 to `_SpecificDudeSpawn`
/// (@0x3d309, @0x3d33d), so it takes a slot in 1-61. Read through
/// [`RuleKey::SpawnReserve`](crate::RuleKey::SpawnReserve) as the
/// traffic's own reserve is.
pub const DEFENCE_RESERVE: u32 = 2;

/// The defence ships a stellar of `DefCount` `def_count` starts with
/// (`_LoadObjectData` @0x776a6-0x776fb): up to 1000 the count itself;
/// 1001 to 10000, a tenth of it less 100; above, a tenth less 1000, each
/// truncating toward zero. A negative count launches none.
#[must_use]
pub const fn initial_left(def_count: i16) -> i16 {
    match def_count {
        ..=1000 => def_count,
        1001..=10000 => def_count / 10 - 100,
        _ => def_count / 10 - 1000,
    }
}

/// The range of the daily regrowth's draw, `Rand(450)` (@0xb637): a
/// dominated stellar's fleet regrows a ship on its 0.
pub const REGROWTH_ODDS: u32 = 450;

/// The count a dominated stellar of `DefCount` `def_count` regrows its
/// defence ships up to (`_IncrementGameTime` @0xb5fc-0xb62b): the count
/// itself up to 1000, else `DefCount % 1000`. Earth's 7006 caps at 6,
/// though it decodes to 600 ([`initial_left`]).
#[must_use]
pub const fn regrowth_cap(def_count: i16) -> i16 {
    if def_count <= 1000 {
        def_count
    } else {
        def_count % 1000
    }
}

/// How many defence ships a stellar of `DefCount` `def_count` keeps up
/// at once: `DefCount % 10`, signed, as C's remainder (@0x43cd3-0x43cfe).
#[must_use]
pub const fn wave(def_count: i16) -> i16 {
    def_count % 10
}

/// The wave a Demand Tribute fought sends up at once from a stellar of
/// `DefCount` `def_count` with `left` ships left, and the ships left
/// after (`_DoPlanetCommDialog` @0x97746-0x977f7): up to a `DefCount` of
/// 1000, every ship left, leaving none; above, [`wave`] ships, taken from
/// those left, the wave shrunk by any overdraft so that none are left
/// below 0. A wave of 0 or less sends none.
#[must_use]
pub const fn wave_on_demand(def_count: i16, left: i16) -> (i16, i16) {
    if def_count <= 1000 {
        return (left, 0);
    }
    let sent = wave(def_count);
    let after = left.saturating_sub(sent);
    if after < 0 {
        (sent.saturating_add(after), 0)
    } else {
        (sent, after)
    }
}

/// How many of `npcs` defend `stellar`.
#[must_use]
pub fn defenders_of(stellar: StellarId, npcs: &[Npc]) -> usize {
    npcs.iter()
        .filter(|npc| npc.defends == Some(stellar))
        .count()
}

/// The stellars' defence ships left (+0x466; see the module docs' "Where
/// the state lives"): the pilot holds them, and lends them to the traffic
/// for a step ([`Traffic::swap_defence_ships`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefenceShips {
    /// The ships left of each stellar whose count has been read or
    /// changed (+0x466), with its raw `DefCount` (+0x468), which a refund
    /// is capped at once its system's stellars are no longer at hand.
    left: BTreeMap<StellarId, Count>,
}

/// The stellars' defence ships left and which are under attack (see the
/// module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StellarDefences {
    /// The ships left, lent by the pilot for a step.
    ships: DefenceShips,
    /// The stellars under attack (+0x47).
    under_attack: BTreeSet<StellarId>,
}

/// A stellar's defence ships left and its raw `DefCount`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Count {
    /// Its ships left (+0x466).
    left: i16,
    /// Its `DefCount`, raw (+0x468).
    def_count: i16,
}

impl DefenceShips {
    /// The defence ships `site` has left: its decoded `DefCount`
    /// ([`initial_left`]) until one is spent or given back.
    #[must_use]
    pub fn left(&self, site: &LandingSite) -> i16 {
        self.left_of(site.id, site.def_count)
    }

    /// The defence ships `stellar`, of raw `DefCount` `def_count`, has
    /// left, as [`left`](Self::left) reads them.
    #[must_use]
    pub fn left_of(&self, stellar: StellarId, def_count: i16) -> i16 {
        self.left
            .get(&stellar)
            .map_or_else(|| initial_left(def_count), |count| count.left)
    }

    /// Sets `site`'s defence ships left to `left`: the planet comm
    /// dialog's arithmetic writes it
    /// (rdm:phase/play-feel-4/phase-2-planet-comm-dialog).
    pub fn set_left(&mut self, site: &LandingSite, left: i16) {
        self.set_left_of(site.id, site.def_count, left);
    }

    /// Sets the defence ships left of `stellar`, of raw `DefCount`
    /// `def_count`, to `left`: what a load writes.
    pub fn set_left_of(&mut self, stellar: StellarId, def_count: i16, left: i16) {
        self.left.insert(stellar, Count { left, def_count });
    }

    /// Each stellar's ships left whose count has been read or changed,
    /// with its raw `DefCount`, by ascending ID: what a save writes. A
    /// stellar not here has its decode.
    pub fn saved(&self) -> impl Iterator<Item = (StellarId, i16, i16)> + '_ {
        self.left
            .iter()
            .map(|(&stellar, count)| (stellar, count.left, count.def_count))
    }

    /// Restores `site`'s count to its decoded `DefCount`
    /// ([`initial_left`]), as the planet comm dialog's Release does
    /// (@0x97422-0x9747e).
    pub fn reset(&mut self, site: &LandingSite) {
        self.set_left(site, initial_left(site.def_count));
    }

    /// Keeps `site`'s count, its decode while it has none, so a refund
    /// finds its `DefCount`: each defender's spawn does.
    pub fn know(&mut self, site: &LandingSite) {
        let left = self.left(site);
        self.set_left(site, left);
    }

    /// Spends one of `site`'s defence ships (`decw` +0x466 @0x43c39).
    pub fn spend(&mut self, site: &LandingSite) {
        let left = self.left(site).saturating_sub(1);
        self.set_left(site, left);
    }

    /// Gives one of `stellar`'s defence ships back, capped at its raw
    /// `DefCount` (`_DeleteAllComputerShips` @0x3c046-0x3c06f); nothing
    /// for a stellar whose count was never kept
    /// ([`DefenceShips::know`]), which has sent up no defender.
    pub fn refund(&mut self, stellar: StellarId) {
        if let Some(count) = self.left.get_mut(&stellar) {
            count.left = count.left.saturating_add(1).min(count.def_count);
        }
    }
}

impl DefenceShips {
    /// The daily regrowth of a dominated stellar's fleet
    /// (`_IncrementGameTime` @0xb62e-0xb64c): `stellar`, of raw `DefCount`
    /// `def_count`, below its [`regrowth_cap`] draws `Rand(450)` on
    /// `chance` and regrows one ship on a 0; at or above it, no draw.
    pub fn regrow(
        &mut self,
        stellar: StellarId,
        def_count: i16,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let left = self.left_of(stellar, def_count);
        if regrowth_cap(def_count) > left && chance.below(REGROWTH_ODDS) == 0 {
            self.set_left_of(stellar, def_count, left.saturating_add(1));
        }
    }
}

impl StellarDefences {
    /// The defence ships left, while the pilot lends them.
    #[must_use]
    pub fn ships(&self) -> &DefenceShips {
        &self.ships
    }

    /// The defence ships left, to change while the pilot lends them.
    pub fn ships_mut(&mut self) -> &mut DefenceShips {
        &mut self.ships
    }

    /// Whether `stellar` is under attack (+0x47).
    #[must_use]
    pub fn under_attack(&self, stellar: StellarId) -> bool {
        self.under_attack.contains(&stellar)
    }

    /// Marks `stellar` under attack, as its defender's spawn does
    /// (@0x3d4a8) and the planet comm dialog's Demand Tribute will.
    pub fn alarm(&mut self, stellar: StellarId) {
        self.under_attack.insert(stellar);
    }

    /// Clears every stellar's mark, as each rebuild point does
    /// (`_HandleStellarSystemVisibility` @0x32d5d).
    pub fn clear(&mut self) {
        self.under_attack.clear();
    }

    /// The frame's pass (`_DoPlayGameWork` @0x443d4-0x4444f): the mark of
    /// each of `nav`, the system's listed stellars, cleared, then set for
    /// the stellar of each of `npcs` that defends one.
    pub fn recompute(&mut self, nav: &[LandingSite], npcs: &[Npc]) {
        for site in nav {
            self.under_attack.remove(&site.id);
        }
        self.under_attack
            .extend(npcs.iter().filter_map(|npc| npc.defends));
    }

    /// The stellar of `nav`, the system's listed stellars in `NavDefs`
    /// order, due a defender among `npcs`: the first under attack with
    /// ships left and fewer defenders in the system than its wave
    /// (@0x43c45-0x43cfe).
    #[must_use]
    pub fn due(&self, nav: &[LandingSite], npcs: &[Npc]) -> Option<LandingSite> {
        nav.iter()
            .find(|site| {
                self.under_attack(site.id)
                    && self.ships.left(site) > 0
                    && i64::try_from(defenders_of(site.id, npcs)).unwrap_or(i64::MAX)
                        < i64::from(wave(site.def_count))
            })
            .copied()
    }
}

/// The defence ship `site` sends up, drawn from `table` with each ship
/// type's `AppearOn` asked by `gate`, on `chance` (see the module docs'
/// "The spawn"), before it is given its slot, its speed and its stellar:
/// none with no `DefenseDude`, or none of its `düde` with a record or a
/// ship.
#[must_use]
pub fn defender(
    table: &SpawnTable,
    site: &LandingSite,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Option<NewShip> {
    let dude = site.defense_dude?;
    let ship = specific_dude_ship(table, dude, None, Fallback::Fails, gate, &mut *chance).or_else(
        || specific_dude_ship(table, dude, None, Fallback::AllShips, gate, &mut *chance),
    )?;
    let heading = chance.below(crate::traffic::spawn::DEGREES) as f32;
    // `_SpecificDudeSpawn`'s ship already has no lead, mission or aux mark.
    Some(NewShip {
        govt: site.govt,
        ai_type: AiType::Warship,
        state: crate::flight::ShipState {
            position: site.position,
            velocity: crate::geometry::Vec2::ZERO,
            heading,
        },
        boarded: true,
        ..ship
    })
}

impl Traffic {
    /// The stellars' defence ships left and which are under attack.
    #[must_use]
    pub fn defences(&self) -> &StellarDefences {
        &self.defences
    }

    /// The stellars' defences, to mark one under attack
    /// ([`StellarDefences::alarm`]) or set its count.
    pub fn defences_mut(&mut self) -> &mut StellarDefences {
        &mut self.defences
    }

    /// The defences, taken to hand on to the next traffic across a
    /// traffic Reset ([`Traffic::keep_defences`]), as the original's
    /// stellar records outlast its ships.
    pub fn take_defences(&mut self) -> StellarDefences {
        std::mem::take(&mut self.defences)
    }

    /// Takes `defences` on from the last traffic.
    pub fn keep_defences(&mut self, defences: StellarDefences) {
        self.defences = defences;
    }

    /// Swaps the defence ships the traffic holds with `ships`: the session
    /// lends the pilot's in for a step and takes them back after (see the
    /// module docs' "Where the state lives").
    pub fn swap_defence_ships(&mut self, ships: &mut DefenceShips) {
        std::mem::swap(&mut self.defences.ships, ships);
    }

    /// The frame's +0x47 pass over `nav`, the system's listed stellars
    /// ([`StellarDefences::recompute`], `_DoPlayGameWork` @0x443d4).
    pub fn recompute_defence_alarms(&mut self, nav: &[LandingSite]) {
        self.defences.recompute(nav, &self.npcs);
    }

    /// Gives back one defence ship to its stellar for each of its
    /// defenders among the NPCs not in `kept`, as their system's ships are
    /// dropped (`_DeleteAllComputerShips` @0x3c02a-0x3c06f; see
    /// [`DefenceShips::refund`]). A ship marked as a mission's
    /// auxiliary ship gives its slot one back instead, as the original's
    /// branch for +0x8a comes first (@0x3bfce-0x3c028).
    pub fn refund_defenders(&mut self, kept: &BTreeSet<NpcId>) {
        let refunds: Vec<StellarId> = self
            .npcs
            .iter()
            .filter(|npc| !kept.contains(&npc.id) && npc.aux.is_none())
            .filter_map(|npc| npc.defends)
            .collect();
        for stellar in refunds {
            self.defences.ships.refund(stellar);
        }
    }

    /// Sends up one of `site`'s defence ships in `world`, rolling on
    /// `chance` (see the module docs' "The spawn"): placed in the first
    /// free slot below `world`'s
    /// [`defence_reserve`](World::defence_reserve), moving at its top
    /// speed along its heading, defending `site`, attacking the player
    /// through the quote gate ([`Traffic::turn_with_quote`]), and `site`
    /// marked under attack. Its number; none, with nothing changed but the
    /// draws, when `site` has no `DefenseDude`, its `düde` brings no ship
    /// or no slot is free. The planet comm dialog's Demand Tribute calls it
    /// through its session (`_DoPlanetCommDialog` @0x97814), and the
    /// respawn ([`Traffic::enter_more_ships`]).
    pub fn spawn_defender(
        &mut self,
        site: &LandingSite,
        world: &World,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<NpcId> {
        self.spawn_defender_gated(site, world, world.persons.gate(), chance)
    }

    /// [`Traffic::spawn_defender`] with each ship type's `AppearOn` asked
    /// by `gate`.
    fn spawn_defender_gated(
        &mut self,
        site: &LandingSite,
        world: &World,
        gate: Gate,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<NpcId> {
        self.defences.ships.know(site);
        let ship = defender(&self.table, site, gate, &mut *chance)?;
        // `_SpecificDudeSpawn` rolls no afterburner and places no fleet.
        let came = self.add(
            vec![ship],
            &BTreeSet::new(),
            Some(world.defence_reserve),
            None,
            world.govts,
            NpcFormation::Original,
            &mut *chance,
        );
        if came == 0 {
            return None;
        }
        let npc = self.npcs.last_mut()?;
        let id = npc.id;
        npc.defends = Some(site.id);
        let speed = npc.top_speed(world.led_speed);
        npc.state.velocity = trig::accel(trig::whole(npc.state.heading), speed);
        // `_AIMakeShipAttackPlayer` (@0x89d3e-0x89d4d): goal 4 on the
        // player, its travel stellar -1.
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        npc.travel_set = false;
        self.turn_with_quote(id, world.gate_view());
        self.defences.alarm(site.id);
        Some(id)
    }

    /// `_EnterMoreShips`' defence respawn in `world`, each ship type's
    /// `AppearOn` asked by `gate` (see the module docs' "The respawn"):
    /// the stellar due one sends up a defender, and spends one whether or
    /// not it found a slot (@0x43c39).
    pub(super) fn respawn_defender(
        &mut self,
        world: &World,
        gate: Gate,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let Some(site) = self.defences.due(world.listed, &self.npcs) else {
            return;
        };
        self.spawn_defender_gated(&site, world, gate, chance);
        self.defences.ships.spend(&site);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::GovtId;
    use crate::stats::ShipStats;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::planet;

    /// Stellar `id` with `DefCount` `def_count`.
    fn site(id: i16, def_count: i16) -> LandingSite {
        LandingSite {
            def_count,
            ..planet(id, 0.0, 0.0)
        }
    }

    /// NPC `id` defending `stellar`, if any.
    fn defending(id: u32, stellar: Option<i16>) -> Npc {
        Npc {
            defends: stellar.map(StellarId),
            ..crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]))
        }
    }

    #[test]
    fn a_def_count_decodes_into_the_ships_it_starts_with() {
        for (def_count, left) in [
            (0, 0),
            (5, 5),
            (1000, 1000),
            (1001, 0),
            (1082, 8),
            (10000, 900),
            (10001, 0),
            (12345, 234),
            (-3, -3),
        ] {
            assert_eq!(initial_left(def_count), left, "{def_count}");
        }
    }

    #[test]
    fn a_wave_is_the_def_count_mod_10_signed() {
        for (def_count, wave_size) in [(5, 5), (20, 0), (1082, 2), (-3, -3)] {
            assert_eq!(wave(def_count), wave_size, "{def_count}");
        }
    }

    #[test]
    fn a_demands_wave_is_every_ship_left_up_to_1000_else_the_def_count_mod_10() {
        for (def_count, left, sent) in [
            (5, 3, (3, 0)),
            (1000, 1000, (1000, 0)),
            (5, -2, (-2, 0)),
            (1082, 8, (2, 6)),
            (1082, 1, (1, 0)),
            (1082, -4, (-4, 0)),
            (1080, 8, (0, 8)),
            (-3, -3, (-3, 0)),
        ] {
            assert_eq!(
                wave_on_demand(def_count, left),
                sent,
                "{def_count} with {left} left"
            );
        }
    }

    #[test]
    fn the_regrowth_cap_is_the_def_count_up_to_1000_else_its_last_three_digits() {
        // @0xb5fc-0xb62b.
        for (def_count, cap) in [
            (0, 0),
            (6, 6),
            (1000, 1000),
            (1001, 1),
            (2206, 206),
            (7006, 6),
            (-1, -1),
            (i16::MIN, i16::MIN),
            (i16::MAX, 767),
        ] {
            assert_eq!(regrowth_cap(def_count), cap, "{def_count}");
        }
    }

    #[test]
    fn a_fleet_regrows_one_on_a_0_in_450_below_its_cap() {
        use nova_sim_base::fixture::Ranged;
        let earth = site(128, 7006);
        let mut ships = DefenceShips::default();
        ships.set_left(&earth, 4);
        let mut chance = Ranged::of(&[(450, 0)]);
        ships.regrow(earth.id, earth.def_count, &mut chance);
        assert_eq!(ships.left(&earth), 5, "@0xb64c");
        assert_eq!(chance.asked, [450], "Rand(450) @0xb637");
        let mut chance = Ranged::of(&[(450, 1)]);
        ships.regrow(earth.id, earth.def_count, &mut chance);
        assert_eq!(ships.left(&earth), 5, "a draw of 1 grows none");
        ships.set_left(&earth, 6);
        let mut chance = Ranged::of(&[]);
        ships.regrow(earth.id, earth.def_count, &mut chance);
        assert_eq!(chance.asked, [] as [u32; 0], "at the cap: no draw");
        assert_eq!(ships.left(&earth), 6);
        let decoded = DefenceShips::default();
        let mut ships = decoded.clone();
        ships.regrow(earth.id, earth.def_count, &mut Ranged::of(&[]));
        assert_eq!(ships, decoded, "the decode, 600, is above the cap 6");
        let small = site(129, 3);
        let mut ships = DefenceShips::default();
        ships.set_left(&small, 0);
        ships.regrow(small.id, small.def_count, &mut Ranged::of(&[(450, 0)]));
        assert_eq!(ships.left(&small), 1, "a count kept with its DefCount");
        ships.refund(small.id);
        ships.refund(small.id);
        ships.refund(small.id);
        assert_eq!(ships.left(&small), 3, "the refund's cap kept");
    }

    #[test]
    fn a_save_reads_each_kept_count_with_its_def_count_by_id() {
        let mut ships = DefenceShips::default();
        assert_eq!(ships.saved().count(), 0, "every count its decode");
        ships.set_left(&site(140, 7006), 3);
        ships.set_left(&site(128, 5), 0);
        assert_eq!(
            ships.saved().collect::<Vec<_>>(),
            [(StellarId(128), 0, 5), (StellarId(140), 3, 7006)]
        );
    }

    #[test]
    fn a_release_restores_the_decoded_count() {
        let mut defences = DefenceShips::default();
        let earth = site(128, 1082);
        defences.set_left(&earth, 0);
        defences.reset(&earth);
        assert_eq!(defences.left(&earth), 8);
        defences.refund(earth.id);
        assert_eq!(defences.left(&earth), 9, "the count kept");
    }

    #[test]
    fn the_ships_left_read_the_decode_until_spent_or_given_back() {
        let mut defences = DefenceShips::default();
        let earth = site(128, 1082);
        assert_eq!(defences.left(&earth), 8);
        defences.spend(&earth);
        assert_eq!(defences.left(&earth), 7);
        defences.refund(earth.id);
        defences.refund(earth.id);
        assert_eq!(
            defences.left(&earth),
            9,
            "above 1000 the raw cap is out of reach"
        );
        let small = site(129, 3);
        defences.refund(small.id);
        assert_eq!(
            defences.left(&small),
            3,
            "a count never kept is left as it is"
        );
        defences.know(&small);
        defences.refund(small.id);
        assert_eq!(defences.left(&small), 3, "capped at the raw DefCount");
        defences.spend(&small);
        defences.refund(small.id);
        assert_eq!(defences.left(&small), 3);
    }

    #[test]
    fn the_frame_pass_marks_the_stellars_whose_defenders_are_in_the_system() {
        let mut defences = StellarDefences::default();
        let nav = [site(128, 3), site(129, 3)];
        defences.alarm(StellarId(128));
        defences.alarm(StellarId(129));
        defences.alarm(StellarId(400));
        defences.recompute(&nav, &[defending(1, Some(129)), defending(2, None)]);
        assert!(!defences.under_attack(StellarId(128)), "no defender up");
        assert!(defences.under_attack(StellarId(129)));
        assert!(
            defences.under_attack(StellarId(400)),
            "a stellar the system doesn't list keeps its mark"
        );
        defences.clear();
        assert!(!defences.under_attack(StellarId(129)));
        assert!(!defences.under_attack(StellarId(400)));
    }

    #[test]
    fn the_first_stellar_under_attack_short_of_its_wave_with_ships_left_is_due() {
        let mut defences = StellarDefences::default();
        let nav = [site(128, 3), site(129, 3), site(130, 20)];
        assert_eq!(defences.due(&nav, &[]), None, "none under attack");
        for id in [128, 129, 130] {
            defences.alarm(StellarId(id));
        }
        assert_eq!(defences.due(&nav, &[]).map(|s| s.id), Some(StellarId(128)));
        let up: Vec<Npc> = (1..=3).map(|id| defending(id, Some(128))).collect();
        assert_eq!(
            defences.due(&nav, &up).map(|s| s.id),
            Some(StellarId(129)),
            "128's wave is up"
        );
        for _ in 0..3 {
            defences.ships_mut().spend(&nav[1]);
        }
        assert_eq!(
            defences.due(&nav, &up),
            None,
            "129 has none left, and a wave of 0 never tops 130 up"
        );
    }

    #[test]
    fn a_defender_of_no_defense_dude_is_none() {
        let table = SpawnTable::default();
        let mut chance = nova_sim_base::fixture::Draws::of(&[]);
        assert_eq!(
            defender(
                &table,
                &site(128, 3),
                crate::traffic::persons::PersonWorld::NONE.gate(),
                &mut chance
            ),
            None
        );
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    /// The traffic's test table with düde 140, its defence düde: ship
    /// 200, a wimpy trader of government 150, `ship_appears` its type's
    /// `AppearOn`; and ship 201 beside it when `second` is given, of weight
    /// 1 each.
    fn defence_table(appear_on: &str, second: bool) -> SpawnTable {
        let mut table = crate::traffic::tests::table(0);
        let mut ships = vec![(crate::catalog::ShipId(200), 1)];
        if second {
            let kind = table.ships[&crate::catalog::ShipId(200)].clone();
            table.ships.insert(crate::catalog::ShipId(201), kind);
            ships.push((crate::catalog::ShipId(201), 1));
        }
        table
            .ships
            .get_mut(&crate::catalog::ShipId(200))
            .expect("there")
            .appear_on = crate::control::Test::parse(appear_on);
        table.dude_records.insert(
            crate::catalog::DudeId(140),
            crate::traffic::table::SpawnDude {
                ai_type: 1,
                govt: Some(GovtId(150)),
                ships,
                booty: 0x0041,
                info_types: 0x8000,
            },
        );
        table
    }

    /// Stellar 128 at (300, -200), of government 160, defended by düde
    /// 140, `DefCount` `def_count`.
    fn earth(def_count: i16) -> LandingSite {
        LandingSite {
            govt: Some(GovtId(160)),
            defense_dude: Some(crate::catalog::DudeId(140)),
            def_count,
            ..planet(128, 300.0, -200.0)
        }
    }

    /// A traffic drawn from `table`, with the player at the centre.
    fn traffic_of(table: SpawnTable) -> super::super::Traffic {
        let mut traffic = super::super::Traffic::new();
        traffic.enter(table, &mut nova_sim_base::fixture::Draws::of(&[]));
        traffic
    }

    use nova_sim_base::fixture::Ranged;

    #[test]
    fn a_defender_comes_at_its_stellar_at_top_speed_attacking_the_player() {
        let sites = [earth(3)];
        let mut traffic = traffic_of(defence_table("", false));
        let world = super::super::World::new(&sites);
        let id = traffic
            .spawn_defender(&sites[0], &world, &mut Ranged::of(&[(360, 90)]))
            .expect("a slot");
        let npc = traffic.npcs().iter().find(|npc| npc.id == id).expect("in");
        assert_eq!(npc.defends, Some(StellarId(128)));
        assert_eq!(npc.leader, None);
        assert_eq!(npc.ai_type, AiType::Warship, "not the düde's wimpy trader");
        assert_eq!(npc.govt, Some(GovtId(160)), "the stellar's government");
        assert_eq!(npc.state.position, sites[0].position);
        assert_eq!(npc.state.heading, 90.0);
        let speed = npc.top_speed(1.0);
        assert!(speed > 0.0);
        assert!(
            (npc.state.velocity - crate::trig::accel(90, speed)).length() < 1e-4,
            "{:?}",
            npc.state.velocity
        );
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(npc.target, Some(ShipRef::Player));
        assert!(npc.boarded, "+0xb9 = 1: it can't be plundered");
        assert_eq!(npc.mission, None);
        assert!(
            traffic.take_attack_quotes().is_empty(),
            "no person, no quote"
        );
        assert!(traffic.defences().under_attack(StellarId(128)));
    }

    #[test]
    fn a_defender_takes_a_slot_below_its_reserve_of_2_or_none_comes() {
        let sites = [earth(3)];
        let world = super::super::World::new(&sites);
        let mut traffic = traffic_of(defence_table("", false));
        for berth in 1..=61 {
            traffic.add_npc_at(defending(0, None), berth);
        }
        assert_eq!(
            traffic.spawn_defender(&sites[0], &world, &mut Ranged::of(&[])),
            None,
            "slots 1-61 held"
        );
        assert!(!traffic.defences().under_attack(StellarId(128)));
        assert_eq!(traffic.npcs().len(), 61);
        let mut traffic = traffic_of(defence_table("", false));
        for berth in 1..=60 {
            traffic.add_npc_at(defending(0, None), berth);
        }
        let id = traffic.spawn_defender(&sites[0], &world, &mut Ranged::of(&[]));
        let berth = traffic
            .npcs()
            .iter()
            .find(|npc| Some(npc.id) == id)
            .map(|npc| npc.berth);
        assert_eq!(berth, Some(61));
    }

    #[test]
    fn a_stellar_of_no_defense_dude_sends_no_defender() {
        let sites = [LandingSite {
            defense_dude: None,
            ..earth(3)
        }];
        let world = super::super::World::new(&sites);
        let mut traffic = traffic_of(defence_table("", false));
        assert_eq!(
            traffic.spawn_defender(&sites[0], &world, &mut Ranged::of(&[])),
            None
        );
        assert!(traffic.npcs().is_empty());
        assert!(!traffic.defences().under_attack(StellarId(128)));
    }

    #[test]
    fn a_defender_of_no_type_that_may_appear_is_drawn_among_them_all() {
        let sites = [earth(3)];
        let world = super::super::World::new(&sites);
        // Its one type refused under bit 7: the retry with 1 takes it.
        let mut traffic = traffic_of(defence_table("b7", false));
        let id = traffic.spawn_defender(&sites[0], &world, &mut Ranged::of(&[]));
        assert!(id.is_some(), "the retry finds ship 200");
        // One refused, one appearing: only the appearing one is drawn.
        let mut traffic = traffic_of(defence_table("b7", true));
        let mut chance = Ranged::of(&[(1, 0), (2, 0)]);
        let id = traffic.spawn_defender(&sites[0], &world, &mut chance);
        let npc = traffic
            .npcs()
            .iter()
            .find(|npc| Some(npc.id) == id)
            .expect("in");
        assert_eq!(npc.ship, crate::catalog::ShipId(201));
        assert!(!chance.was_asked(2), "drawn among the one that appears");
    }

    /// One `_EnterMoreShips` pass over `listed`.
    fn pass(traffic: &mut super::super::Traffic, listed: &[LandingSite]) {
        traffic.enter_more_ships(super::super::World::new(listed), &mut Ranged::of(&[]));
    }

    /// How many ships of `traffic` defend `stellar`.
    fn up(traffic: &super::super::Traffic, stellar: i16) -> usize {
        defenders_of(StellarId(stellar), traffic.npcs())
    }

    #[test]
    fn a_stellar_under_attack_sends_up_one_defender_a_pass_to_its_wave_while_it_has_ships() {
        let listed = [earth(3)];
        let mut traffic = traffic_of(defence_table("", false));
        traffic.defences_mut().ships_mut().set_left(&listed[0], 5);
        traffic.defences_mut().alarm(StellarId(128));
        for (pass_no, (count, left)) in [(1, 4), (2, 3), (3, 2), (3, 2)].into_iter().enumerate() {
            pass(&mut traffic, &listed);
            assert_eq!(
                (
                    up(&traffic, 128),
                    traffic.defences().ships().left(&listed[0])
                ),
                (count, left),
                "pass {pass_no}"
            );
        }
        // One destroyed: exactly one more on the next pass.
        let gone = traffic.npcs()[0].id;
        traffic.remove(gone);
        pass(&mut traffic, &listed);
        assert_eq!(
            (
                up(&traffic, 128),
                traffic.defences().ships().left(&listed[0])
            ),
            (3, 1)
        );
        pass(&mut traffic, &listed);
        assert_eq!(up(&traffic, 128), 3);
    }

    #[test]
    fn a_stellar_with_none_left_or_not_under_attack_sends_none() {
        let listed = [earth(3)];
        let mut traffic = traffic_of(defence_table("", false));
        pass(&mut traffic, &listed);
        assert_eq!(up(&traffic, 128), 0, "not under attack");
        traffic.defences_mut().alarm(StellarId(128));
        traffic.defences_mut().ships_mut().set_left(&listed[0], 0);
        pass(&mut traffic, &listed);
        assert_eq!(up(&traffic, 128), 0, "none left");
        assert_eq!(
            traffic.defences().ships().left(&listed[0]),
            0,
            "nothing spent"
        );
    }

    #[test]
    fn of_two_stellars_short_only_the_first_listed_sends_one_a_pass() {
        let listed = [
            LandingSite {
                id: StellarId(129),
                ..earth(2)
            },
            earth(2),
        ];
        let mut traffic = traffic_of(defence_table("", false));
        traffic.defences_mut().alarm(StellarId(128));
        traffic.defences_mut().alarm(StellarId(129));
        pass(&mut traffic, &listed);
        assert_eq!((up(&traffic, 129), up(&traffic, 128)), (1, 0));
        pass(&mut traffic, &listed);
        assert_eq!((up(&traffic, 129), up(&traffic, 128)), (2, 0));
        pass(&mut traffic, &listed);
        assert_eq!((up(&traffic, 129), up(&traffic, 128)), (2, 1));
    }

    #[test]
    fn a_defender_that_finds_no_slot_is_spent_all_the_same() {
        let listed = [earth(3)];
        let mut traffic = traffic_of(defence_table("", false));
        for berth in 1..=61 {
            traffic.add_npc_at(defending(0, None), berth);
        }
        traffic.defences_mut().alarm(StellarId(128));
        pass(&mut traffic, &listed);
        assert_eq!(up(&traffic, 128), 0);
        assert_eq!(
            traffic.defences().ships().left(&listed[0]),
            2,
            "@0x43c39 spends it"
        );
    }

    #[test]
    fn a_def_count_of_20_never_tops_its_stellar_up() {
        let listed = [earth(20)];
        let mut traffic = traffic_of(defence_table("", false));
        traffic.defences_mut().alarm(StellarId(128));
        pass(&mut traffic, &listed);
        assert_eq!(up(&traffic, 128), 0, "a wave of 0");
        assert_eq!(traffic.defences().ships().left(&listed[0]), 20);
    }

    #[test]
    fn the_respawn_walks_the_listed_stellars_not_only_those_shown() {
        let listed = [earth(3)];
        let mut traffic = traffic_of(defence_table("", false));
        traffic.defences_mut().alarm(StellarId(128));
        let world = super::super::World {
            listed: &listed,
            ..super::super::World::new(&[])
        };
        traffic.enter_more_ships(world, &mut Ranged::of(&[]));
        assert_eq!(up(&traffic, 128), 1);
    }

    #[test]
    fn the_defenders_dropped_give_their_ships_back_but_those_kept_or_aux() {
        let listed = [earth(3)];
        let mut traffic = traffic_of(defence_table("", false));
        traffic.defences_mut().alarm(StellarId(128));
        for _ in 0..3 {
            pass(&mut traffic, &listed);
        }
        assert_eq!(traffic.defences().ships().left(&listed[0]), 0);
        let ids: Vec<NpcId> = traffic.npcs().iter().map(|npc| npc.id).collect();
        traffic.npcs_mut()[2].aux = Some(1);
        traffic.refund_defenders(&BTreeSet::from([ids[1]]));
        assert_eq!(
            traffic.defences().ships().left(&listed[0]),
            1,
            "one dropped, one kept, one aux"
        );
        traffic.npcs_mut()[2].aux = None;
        traffic.refund_defenders(&BTreeSet::new());
        assert_eq!(
            traffic.defences().ships().left(&listed[0]),
            3,
            "capped at the raw DefCount"
        );
    }

    #[test]
    fn a_defender_sent_up_unspent_keeps_its_stellars_count_for_the_refund() {
        // `DefCount` 1005 decodes to 0 left, capped at 1005: a refund of a
        // defender the dialog sent up, never spent, gives 1 back.
        let listed = [earth(1005)];
        let world = super::super::World::new(&listed);
        let mut traffic = traffic_of(defence_table("", false));
        traffic.spawn_defender(&listed[0], &world, &mut Ranged::of(&[]));
        assert_eq!(traffic.defences().ships().left(&listed[0]), 0);
        traffic.refund_defenders(&BTreeSet::new());
        assert_eq!(traffic.defences().ships().left(&listed[0]), 1);
    }

    #[test]
    fn the_frame_pass_reads_the_traffics_defenders() {
        let listed = [earth(3)];
        let world = super::super::World::new(&listed);
        let mut traffic = traffic_of(defence_table("", false));
        traffic.spawn_defender(&listed[0], &world, &mut Ranged::of(&[]));
        traffic.defences_mut().clear();
        traffic.recompute_defence_alarms(&listed);
        assert!(traffic.defences().under_attack(StellarId(128)));
        let id = traffic.npcs()[0].id;
        traffic.remove(id);
        traffic.recompute_defence_alarms(&listed);
        assert!(!traffic.defences().under_attack(StellarId(128)));
    }

    #[test]
    fn the_defences_are_handed_on_across_a_reset() {
        let mut traffic = traffic_of(defence_table("", false));
        traffic.defences_mut().ships_mut().set_left(&earth(3), 4);
        let taken = traffic.take_defences();
        assert_eq!(traffic.defences(), &StellarDefences::default());
        let mut next = traffic_of(defence_table("", false));
        next.keep_defences(taken);
        assert_eq!(next.defences().ships().left(&earth(3)), 4);
    }

    #[test]
    fn the_pilots_ships_are_lent_to_the_traffic_and_taken_back() {
        let mut traffic = traffic_of(defence_table("", false));
        let mut ships = DefenceShips::default();
        ships.set_left(&earth(3), 1);
        let lent = ships.clone();
        traffic.swap_defence_ships(&mut ships);
        assert_eq!(traffic.defences().ships(), &lent);
        assert_eq!(ships, DefenceShips::default(), "the traffic's, empty");
        traffic.defences_mut().ships_mut().spend(&earth(3));
        traffic.swap_defence_ships(&mut ships);
        assert_eq!(ships.left(&earth(3)), 0, "the spend comes back");
        assert_eq!(traffic.defences().ships(), &DefenceShips::default());
    }

    #[test]
    fn a_defender_in_the_traffic_with_no_candidate_heads_back_to_land() {
        let sites = [earth(3)];
        let mut traffic = traffic_of(defence_table("", false));
        let world = super::super::World::new(&sites);
        let id = traffic
            .spawn_defender(&sites[0], &world, &mut Ranged::of(&[]))
            .expect("a slot");
        // Its spawn's target, the player, reads as seen while no player is
        // in the system (`_ShipVisibleToShip` tests no presence): dropped.
        let npc = traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == id)
            .expect("in");
        npc.target = None;
        npc.goal = Goal::Idle;
        traffic.tick_in(
            &crate::ai::NovaAi::default(),
            world,
            &[],
            &mut Ranged::of(&[]),
        );
        let npc = traffic.npcs().iter().find(|npc| npc.id == id).expect("in");
        assert_eq!(
            npc.goal,
            Goal::Land(StellarId(128)),
            "no player in the system"
        );
        assert_eq!(npc.target, None);
    }
}
