//! The masks a deadly stellar touches ships by, as `_HandleDeadlyStellars`
//! tests each ship with `_SWRLECollision(stellar sprite, ship sprite)`
//! (@0x39d97; see `nova_sim_base::sprite_mask`): each sprite's sheet, the
//! frame it shows and where its rect is placed, all in screen pixels from
//! the player, so the test does not depend on the window's size.
//!
//! **The sheets** are the session's shared cache (see the `sprite_sheets`
//! module), which asks for the stellars' only while the system lists a
//! deadly stellar, so stock never asks for them.
//!
//! **A stellar's mask** ([`Session::deadly_stellar_mask`]). Its slot is
//! laid out as a click reads it ([`Space::slot_layout`]): the sheet drawn
//! (its `Type`, its `DeadType` while destroyed, or graphic type 0's), at
//! the frame drawn's size, its top left at the stellar's place less half
//! the slot's size (`_HandleStellarSprites` @0x2eeab-0x2ef1a), 32 square
//! for a frame frozen. It shows a hypergate's frame
//! ([`Session::gate_frame`]), and frame 0 of any other: the animation
//! frame of a stellar is the view's, not the sim's, which
//! rdm:phase/play-feel-8-fidelity/phase-1-stellar-animation-frame-in-the-sim
//! files. An unowned stale slot is graphic type 0's frame 0 at its ghost's
//! top left ([`Space::ghost_mask`]), and an owned one its owner's.
//!
//! **A ship's mask** ([`Session::ship_masks`]) is its sprite as the
//! `sprite_sheets` module lays it out, at its place after this pass's
//! move, the frame set its last display drew (a divergence
//! rdm:phase/play-feel-8/phase-18-ship-frames-stepped-at-each-display
//! files).
//!
//! Where either mask can't be read, the touch falls back to discs (see
//! `nova_sim::combat::stellar_hazards`). None of this plays a sound.

use super::{Session, Space, Statics};
use crate::catalog::{LandingSite, MaskSheet};
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::sprite_mask::PlacedMask;
use crate::story::DrawnSheet;

impl Space {
    /// The mask of `site`, one of the stellars shown, showing `frame`, the
    /// player at `player` (see the module docs); none when its slot shows
    /// no frame or its sheet isn't read.
    fn stellar_mask(
        &self,
        statics: &Statics,
        setup: &super::Setup,
        site: &LandingSite,
        player: Vec2,
        frame: u16,
    ) -> Option<PlacedMask> {
        let layout = self.slot_layout(statics, setup, site, player)?;
        let graphic = match layout.sheet {
            DrawnSheet::Type => site.graphic,
            DrawnSheet::Dead => site.dead_graphic?,
            DrawnSheet::TypeZero => 0,
            DrawnSheet::NoFrame => return None,
        };
        self.sprite_masks.placed(
            MaskSheet::Stellar(graphic),
            frame,
            layout.top_left,
            layout.size,
        )
    }

    /// The mask of an unowned slot placed at `top_left` from the player:
    /// graphic type 0's frame 0 there, none when its sheet or its frame
    /// size can't be read.
    pub(super) fn ghost_mask(&self, statics: &Statics, top_left: (i32, i32)) -> Option<PlacedMask> {
        self.sprite_masks
            .placed(MaskSheet::Stellar(0), 0, top_left, statics.fallback_frame?)
    }
}

impl Session {
    /// The mask of `site`, one of the stellars shown, as it is drawn now
    /// (see the module docs): a hypergate's frame, else frame 0.
    pub(super) fn deadly_stellar_mask(&self, site: &LandingSite) -> Option<PlacedMask> {
        let frame = self.gate_frame(site.id).unwrap_or(0);
        self.space.stellar_mask(
            &self.statics,
            &self.setup,
            site,
            self.ship.player.position,
            frame,
        )
    }

    /// Each ship's mask whose sheet was read, the player's first, then
    /// each NPC's in turn (see the module docs).
    pub(super) fn ship_masks(&self) -> Vec<(ShipRef, PlacedMask)> {
        let player = self.ship.player.position;
        let arsenal = &self.statics.arsenal;
        let masks = &self.space.sprite_masks;
        let own = masks
            .ship(
                arsenal.sprite_base(self.pilot.ship),
                &self.ship.fit.hull.frames,
                self.ship.ship_display.drawn.set,
                &self.ship.player,
                player,
            )
            .map(|mask| (ShipRef::Player, mask));
        let npcs = self.npcs().iter().filter_map(|npc| {
            let mask = masks.ship(
                arsenal.sprite_base(npc.ship),
                &npc.hull.frames,
                npc.display.drawn.set,
                &npc.state,
                player,
            )?;
            Some((ShipRef::Npc(npc.id), mask))
        });
        own.into_iter().chain(npcs).collect()
    }
}

#[cfg(test)]
mod tests {
    use nova_sim_base::fixture::rle::sheet;

    use super::*;
    use crate::catalog::ShipId;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::deadly_stellar::DEADLY_STELLAR;
    use crate::flight::Controls;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::sprite_mask::RleSheet;
    use crate::story::sprite_pool::SlotSprite;
    use crate::testkit::{FakePilotCatalog, Passive, planet, work};

    /// A stellar's frames' side.
    const SIDE: u32 = 40;

    /// The rows of a frame `side` square, `drawn` at each pixel it says,
    /// as `'#'`, or as `run` where `run` is set.
    fn square(side: u32, drawn: impl Fn(u32, u32) -> bool, run: bool) -> Vec<String> {
        let mark = if run { 'r' } else { '#' };
        (0..side)
            .map(|y| {
                (0..side)
                    .map(|x| if drawn(x, y) { mark } else { '.' })
                    .collect()
            })
            .collect()
    }

    /// A ring 5 pixels thick about a frame [`SIDE`] square: its hole 30
    /// across about the centre.
    fn ring(x: u32, y: u32) -> bool {
        let hole = 5..SIDE - 5;
        !hole.contains(&x) || !hole.contains(&y)
    }

    /// The sheet of `frames`, each a list of rows.
    fn sheet_of(frames: &[Vec<String>]) -> RleSheet {
        let rows: Vec<Vec<&str>> = frames
            .iter()
            .map(|rows| rows.iter().map(String::as_str).collect())
            .collect();
        let frames: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
        sheet(&frames)
    }

    /// A ring [`SIDE`] square, drawn as pixels.
    fn ringed() -> RleSheet {
        sheet_of(&[square(SIDE, ring, false)])
    }

    /// A frame `side` square, every pixel drawn.
    fn solid(side: u32) -> RleSheet {
        sheet_of(&[square(side, |_, _| true, false)])
    }

    /// The NPC's class, 129, by its sprite base.
    const NPC_SHIP: MaskSheet = MaskSheet::Ship(ShipId(129));

    /// [`armed`]'s catalog holding `sheets`.
    fn holding(sheets: Vec<(MaskSheet, RleSheet)>) -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.rle_sheets = sheets;
        catalog
    }

    /// The ring as graphic type 3 and a 10-pixel square as the NPC's.
    fn ring_and_square() -> FakePilotCatalog {
        holding(vec![
            (MaskSheet::Stellar(3), ringed()),
            (NPC_SHIP, solid(10)),
        ])
    }

    /// A deadly stellar 140 of graphic type 3, [`SIDE`] square, centred on
    /// the NPC 100 above the centre: its disc, 20 across, takes in the
    /// NPC's, and leaves the player's clear.
    fn on_the_npc() -> LandingSite {
        LandingSite {
            flags2: DEADLY_STELLAR,
            graphic: 3,
            frame_size: Some((SIDE, SIDE)),
            ..planet(140, 0.0, -100.0)
        }
    }

    /// `catalog`'s session with the player at rest at the centre and its
    /// NPC at rest 100 above facing `heading`, the system listing and
    /// showing `listed`, under `rules`.
    fn among(
        catalog: &FakePilotCatalog,
        listed: &[LandingSite],
        heading: u32,
        rules: Rulebook,
    ) -> Session {
        let mut session = facing_an_npc(catalog, heading).with_rules(rules);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.chart.listed = listed.to_vec();
        session.chart.sites = listed.to_vec();
        session
    }

    fn npc_at(session: &mut Session, x: f32, y: f32) {
        session.space.traffic.npcs_mut()[0].state.position = Vec2::new(x, y);
    }

    /// Whether the NPC lives through a pass of `catalog`'s.
    fn lives(session: &mut Session, catalog: &FakePilotCatalog) -> bool {
        work(session, catalog, Controls::default());
        session.npcs().len() == 1
    }

    #[test]
    fn a_ship_in_a_rings_hole_lives_though_their_discs_overlap_and_one_on_the_ring_dies() {
        let catalog = ring_and_square();
        let mut inside = among(&catalog, &[on_the_npc()], 180, Rulebook::default());
        assert!(lives(&mut inside, &catalog), "its pixels miss the ring");
        let mut on = among(&catalog, &[on_the_npc()], 180, Rulebook::default());
        npc_at(&mut on, 15.0, -100.0);
        assert!(!lives(&mut on, &catalog), "its pixels on the ring");
        // With no masks, the discs decide: the NPC in the hole dies.
        let discs = armed();
        let mut fallback = among(&discs, &[on_the_npc()], 180, Rulebook::default());
        assert!(!lives(&mut fallback, &discs));
    }

    #[test]
    fn the_player_in_a_rings_hole_lives_by_its_own_mask_and_on_the_ring_dies() {
        use crate::combat::hull::Condition;
        // The ring centred on the player, its disc taking in the player's
        // and leaving the NPC's clear; the player's class 128's sheet a
        // 10-pixel square.
        let on_the_player = LandingSite {
            position: Vec2::ZERO,
            ..on_the_npc()
        };
        let player_dies = |catalog: &FakePilotCatalog, x: f32| {
            let mut session = among(catalog, &[on_the_player], 180, Rulebook::default());
            session.ship.player.position = Vec2::new(x, 0.0);
            work(&mut session, catalog, Controls::default());
            assert_eq!(session.npcs().len(), 1, "the NPC is clear");
            session.ship.condition == Condition::Destroyed
        };
        let masks = holding(vec![
            (MaskSheet::Stellar(3), ringed()),
            (MaskSheet::Ship(ShipId(128)), solid(10)),
        ]);
        assert!(!player_dies(&masks, 0.0), "its pixels miss the ring");
        assert!(player_dies(&masks, 15.0), "its pixels on the ring");
        // With no masks, the discs decide: the player in the hole dies.
        assert!(player_dies(&armed(), 0.0));
    }

    #[test]
    fn a_ships_sheet_is_its_sprite_bases_not_its_own_classs() {
        // Class 129 draws class 128's sprites; only 128's sheet is held.
        let mut catalog = holding(vec![
            (MaskSheet::Stellar(3), ringed()),
            (MaskSheet::Ship(ShipId(128)), solid(10)),
        ]);
        let npc_hull = catalog
            .hulls
            .iter_mut()
            .find(|hull| hull.id == ShipId(129))
            .expect("class 129's hull");
        npc_hull.sprite_base = ShipId(128);
        let mut session = among(&catalog, &[on_the_npc()], 180, Rulebook::default());
        assert!(lives(&mut session, &catalog), "in the hole by 128's mask");
        let asked = catalog.rle_sheets_asked.borrow().clone();
        assert!(asked.contains(&MaskSheet::Ship(ShipId(128))), "{asked:?}");
        assert!(!asked.contains(&NPC_SHIP), "{asked:?}");
    }

    #[test]
    fn a_ship_touches_by_the_frame_its_heading_draws() {
        // Rotations 9 to 27 of 36 (90° to 280°) cover the whole frame; the
        // rest only a dot in the hole.
        let frames: Vec<Vec<String>> = (0..36)
            .map(|rotation| {
                if (9..=27).contains(&rotation) {
                    square(SIDE, |_, _| true, false)
                } else {
                    square(SIDE, |x, y| x == SIDE / 2 && y == SIDE / 2, false)
                }
            })
            .collect();
        let catalog = holding(vec![
            (MaskSheet::Stellar(3), ringed()),
            (NPC_SHIP, sheet_of(&frames)),
        ]);
        let mut up = among(&catalog, &[on_the_npc()], 0, Rulebook::default());
        assert!(lives(&mut up, &catalog), "heading 0's frame misses");
        let mut down = among(&catalog, &[on_the_npc()], 180, Rulebook::default());
        assert!(!lives(&mut down, &catalog), "heading 180's touches");
    }

    #[test]
    fn each_ship_touches_by_the_frame_its_own_display_set_draws() {
        use crate::combat::hull::Condition;
        // Two sets of 36: set 0's frames a dot in the hole, set 1's solid.
        let frames: Vec<Vec<String>> = (0..72)
            .map(|frame| {
                if frame < 36 {
                    square(SIDE, |x, y| x == SIDE / 2 && y == SIDE / 2, false)
                } else {
                    square(SIDE, |_, _| true, false)
                }
            })
            .collect();
        let catalog = holding(vec![
            (MaskSheet::Stellar(3), ringed()),
            (MaskSheet::Ship(ShipId(128)), sheet_of(&frames)),
            (NPC_SHIP, sheet_of(&frames)),
        ]);
        // A second ring centred on the player, clear of the NPC.
        let on_the_player = LandingSite {
            flags2: DEADLY_STELLAR,
            graphic: 3,
            frame_size: Some((SIDE, SIDE)),
            ..planet(141, 0.0, 0.0)
        };
        for (player_set, npc_set) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let mut session = among(
                &catalog,
                &[on_the_npc(), on_the_player],
                180,
                Rulebook::default(),
            );
            session.ship.ship_display.drawn.set = player_set;
            session.space.traffic.npcs_mut()[0].display.drawn.set = npc_set;
            let npc_lives = lives(&mut session, &catalog);
            let player_lives = session.ship.condition != Condition::Destroyed;
            assert_eq!(
                (player_lives, npc_lives),
                (player_set == 0, npc_set == 0),
                "player on set {player_set}, NPC on set {npc_set}"
            );
        }
    }

    #[test]
    fn a_destroyed_deadly_stellar_touches_by_its_dead_types_mask() {
        let dead = LandingSite {
            destroyed: true,
            dead_graphic: Some(5),
            dead_frame_size: Some((SIDE, SIDE)),
            ..on_the_npc()
        };
        for (type_3, dead_5, survives) in [
            (solid(SIDE), ringed(), true),
            (ringed(), solid(SIDE), false),
        ] {
            let catalog = holding(vec![
                (MaskSheet::Stellar(3), type_3),
                (MaskSheet::Stellar(5), dead_5),
                (NPC_SHIP, solid(10)),
            ]);
            let mut session = among(&catalog, &[dead], 180, Rulebook::default());
            assert_eq!(lives(&mut session, &catalog), survives);
        }
    }

    #[test]
    fn a_deadly_hypergate_touches_by_the_frame_the_gate_shows() {
        use crate::catalog::SiteAnimation;
        use crate::gate::HYPERGATE;
        use crate::gate_sprite::{GateAnimation, GateSprite};
        // Two frames, its open frame 1, stepping only every 1000 ticks.
        let gate = LandingSite {
            flags2: DEADLY_STELLAR | HYPERGATE,
            animation: SiteAnimation {
                delay: 1000,
                type_frames: 2,
                ..SiteAnimation::default()
            },
            ..on_the_npc()
        };
        // The NPC in the hole lives by a ring frame and dies by a full one.
        let holed = square(SIDE, ring, false);
        let full = square(SIDE, |_, _| true, false);
        for (frames, open, survives) in [
            ([holed.clone(), full.clone()], false, true),
            ([holed.clone(), full.clone()], true, false),
            ([full.clone(), holed.clone()], false, false),
            ([full.clone(), holed.clone()], true, true),
        ] {
            let catalog = holding(vec![
                (MaskSheet::Stellar(3), sheet_of(&frames)),
                (NPC_SHIP, solid(10)),
            ]);
            let mut session = among(&catalog, &[gate], 180, Rulebook::default());
            // Open, it shows its last frame, 1, through the pass.
            let sprite = if open {
                GateSprite::exit_gate(&GateAnimation::of(&gate))
            } else {
                GateSprite::new()
            };
            session.space.gate_sprites.insert(gate.id, sprite);
            assert_eq!(session.gate_frame(gate.id), Some(u16::from(open)));
            assert_eq!(lives(&mut session, &catalog), survives, "open: {open}");
            assert_eq!(session.gate_frame(gate.id), Some(u16::from(open)));
        }
    }

    #[test]
    fn a_stale_slot_touches_by_its_owners_mask_or_graphic_type_0s_at_its_ghost() {
        // H, deadly, hidden after the first rebuild point.
        let hidden = LandingSite {
            position: Vec2::new(3000.0, 3000.0),
            ..on_the_npc()
        };
        // S, harmless, of graphic type 4, at the NPC, shown in H's slot.
        let owner = LandingSite {
            graphic: 4,
            frame_size: Some((SIDE, SIDE)),
            ..planet(141, 0.0, -100.0)
        };
        for (s_sheet, survives) in [(ringed(), true), (solid(SIDE), false)] {
            let catalog = holding(vec![
                (MaskSheet::Stellar(3), solid(SIDE)),
                (MaskSheet::Stellar(4), s_sheet),
                (NPC_SHIP, solid(10)),
            ]);
            let mut session = among(&catalog, &[hidden, owner], 180, Rulebook::default());
            session.chart.sites = vec![hidden];
            session.rebuild_sprite_pool();
            session.chart.sites = vec![owner];
            session.rebuild_sprite_pool();
            assert_eq!(
                session.stellar_sprite_pool().sprite_of(hidden.id, false),
                SlotSprite::Owner(owner.id)
            );
            assert_eq!(lives(&mut session, &catalog), survives, "owned by S");
        }
        // Unowned: graphic type 0's frame at the slot's last place.
        let at_the_npc = on_the_npc();
        for (zero, survives) in [(ringed(), true), (solid(SIDE), false)] {
            let catalog = holding(vec![
                (MaskSheet::Stellar(0), zero),
                (MaskSheet::Stellar(3), solid(SIDE)),
                (NPC_SHIP, solid(10)),
            ]);
            let mut session = among(&catalog, &[at_the_npc], 180, Rulebook::default());
            session.statics_mut().fallback_frame = Some((SIDE, SIDE));
            session.rebuild_sprite_pool();
            session.chart.sites = Vec::new();
            session.rebuild_sprite_pool();
            let SlotSprite::Ghost { top_left } = session
                .stellar_sprite_pool()
                .sprite_of(at_the_npc.id, false)
            else {
                panic!("a ghost");
            };
            // The NPC at the ghost's centre.
            #[allow(clippy::cast_precision_loss)]
            npc_at(
                &mut session,
                top_left.0 as f32 + 20.0,
                top_left.1 as f32 + 20.0,
            );
            assert_eq!(lives(&mut session, &catalog), survives, "the ghost");
        }
    }

    #[test]
    fn double_times_second_pass_touches_by_the_masks_the_first_kept_each_asked_once() {
        let catalog = ring_and_square();
        let mut session = among(&catalog, &[on_the_npc()], 180, Rulebook::default());
        assert!(lives(&mut session, &catalog));
        let again = |session: &mut Session| {
            session.tick_again(
                Controls::default(),
                &Passive,
                Rules::default(),
                &mut NeverFires,
            );
            session.npcs().len() == 1
        };
        assert!(again(&mut session), "in the hole by the kept masks");
        assert!(lives(&mut session, &catalog));
        let asked = catalog.rle_sheets_asked.borrow().clone();
        let mut once = asked.clone();
        once.sort_unstable();
        once.dedup();
        assert_eq!(asked.len(), once.len(), "each once: {asked:?}");
        for sheet in [
            NPC_SHIP,
            MaskSheet::Ship(ShipId(128)),
            MaskSheet::Stellar(3),
            MaskSheet::Stellar(0),
        ] {
            assert!(asked.contains(&sheet), "{sheet:?} in {asked:?}");
        }
        npc_at(&mut session, 15.0, -100.0);
        assert!(!again(&mut session), "on the ring");
    }

    #[test]
    fn with_no_deadly_stellar_listed_no_stellar_sheet_is_asked() {
        let catalog = ring_and_square();
        let harmless = LandingSite {
            flags2: 0,
            ..on_the_npc()
        };
        let mut session = among(&catalog, &[harmless], 180, Rulebook::default());
        assert!(lives(&mut session, &catalog));
        let asked = catalog.rle_sheets_asked.borrow().clone();
        assert!(
            !asked
                .iter()
                .any(|sheet| matches!(sheet, MaskSheet::Stellar(_))),
            "{asked:?}"
        );
        assert!(asked.contains(&NPC_SHIP), "the contacts' own: {asked:?}");
    }

    #[test]
    fn by_the_other_walk_a_ship_on_a_rings_runs_dies() {
        let catalog = holding(vec![
            (MaskSheet::Stellar(3), sheet_of(&[square(SIDE, ring, true)])),
            (NPC_SHIP, solid(10)),
        ]);
        let drawn = Rulebook::default().with_override(RuleKey::RleCollisionWalk, RuleSource::Bible);
        for (rules, survives) in [(Rulebook::default(), true), (drawn, false)] {
            let mut session = among(&catalog, &[on_the_npc()], 180, rules);
            npc_at(&mut session, 15.0, -100.0);
            assert_eq!(lives(&mut session, &catalog), survives, "{rules:?}");
        }
    }
}
