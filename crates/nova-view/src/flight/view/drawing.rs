//! [`FlightView`]'s world as drawn, over the [`FlightSim`] port: the ships,
//! shots, cargo boxes, asteroids and beams where the kept frame puts them
//! ([`FlightView::frame`](FlightView#structfield.frame)), each smoothed
//! between the last two steps, the camera on the player's ship as drawn,
//! the boxes the ships are drawn in for the cursor's hover and a click in
//! space, and the brackets round them. Any flight draws them, a scripted
//! one in the component tests.

use nova_sim::brackets::{NavBrackets, TargetBrackets};
use nova_sim::flight_frame::{DrawnBoxes, FlightCommand, FrameShip};
use nova_sim::sprite_layer::SpriteLayer;
use nova_sim::{Drawn as ShipDrawn, NpcId};
use nova_sim::{ShipRef, Vec2, glow_level};

use crate::draw::crossed_box;
use crate::flight::asteroids::AsteroidShown;
use crate::flight::boxes::BoxShown;
use crate::flight::brackets;
use crate::flight::catalog::ShipSheet;
use crate::flight::cloak_fade::{CloakDraw, cloak_rolls, player_cloak_rolls};
use crate::flight::effects::Effects;
use crate::flight::flash::ShieldFlash;
use crate::flight::gate_colorize::gate_colorize;
use crate::flight::hud::STATUS_BAR_WIDTH;
use crate::flight::ion_tint::hull_colorize;
use crate::flight::murk::{self, sprite_murk};
use crate::flight::ship_layers::{Place, ShipDraw, base_alpha, npc_glow_rolls, turning};
use crate::flight::sim_port::FlightSim;
use crate::flight::weapons::{self, BeamShown, ShotShown};
use crate::system::camera::{Anchor, Camera};
use crate::system::scene::{PLACEHOLDER, PLACEHOLDER_SIZE, SystemScene};
use crate::time::ticks;
use crate::{Color, DrawList, Input, Point};

use super::sprites::{Sprite, sprite_stack};
use super::{
    DESELECT_KEY, FlightView, MESSAGE_GAP, OVERLAY_SIZE, drawn_size, npc_tint, placeholder_size,
    shown_heading, shown_position,
};

impl<C, S: FlightSim> FlightView<C, S> {
    /// The session's system, as laid out, if the session started.
    #[must_use]
    pub fn scene(&self) -> Option<&SystemScene> {
        self.scene.as_ref()
    }

    /// The frame the flight shows of `stellar` and its size, while the
    /// system laid out shows it: what the planet comm dialog draws of it
    /// (`_PlanetCommDialogUpdate` @0x90cc6-0x90e26).
    #[must_use]
    pub fn stellar_frame(
        &self,
        stellar: nova_sim::StellarId,
    ) -> Option<crate::ui::planet_comm::StellarFrame> {
        self.scene.as_ref()?.shown_frame(stellar)
    }

    /// How far the display is between the last two steps, in `[0, 1)`.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Where the ship is drawn: `alpha` of the way from where it was a step
    /// ago to where the frame has it.
    #[must_use]
    pub fn shown_position(&self) -> Point {
        shown_position(self.previous, self.frame.player().place(), self.alpha)
    }

    /// Which way the ship is drawn facing: `alpha` of the way, the short way
    /// round, from its heading a step ago to its heading in the frame.
    #[must_use]
    pub fn shown_heading(&self) -> f32 {
        shown_heading(self.previous, self.frame.player().place(), self.alpha)
    }

    /// Where NPC `npc` is drawn and which way it faces: `alpha` of the way
    /// from its place a step ago, as the player's ship; where the frame
    /// has it, when it was not there a step ago.
    pub(super) fn shown_npc(&self, npc: &FrameShip) -> (Point, f32) {
        let now = npc.place();
        let from = self
            .npc_previous
            .iter()
            .find(|&&(id, _)| npc.ship == ShipRef::Npc(id))
            .map_or(now, |&(_, place)| place);
        (
            shown_position(from, now, self.alpha),
            shown_heading(from, now, self.alpha),
        )
    }

    /// The NPCs, each with where it is drawn and which way it faces.
    pub(super) fn shown_npcs(&self) -> Vec<(&FrameShip, Point, f32)> {
        self.frame
            .npcs()
            .map(|npc| {
                let (at, heading) = self.shown_npc(npc);
                (npc, at, heading)
            })
            .collect()
    }

    /// The camera, on the ship as drawn, in the view's size, drawing it at
    /// the flight's [`anchor`](FlightView::anchor).
    #[must_use]
    pub fn camera(&self) -> Camera {
        Camera::centred_on_in(self.shown_position(), self.view).with_anchor(self.anchor())
    }

    /// Where flight draws the player's ship, as its rules read
    /// [`RuleKey::FlightAnchor`](nova_sim::RuleKey::FlightAnchor)
    /// ([`Presentation`](super::stepping::Presentation)).
    pub(super) fn anchor(&self) -> Anchor {
        Anchor::in_flight(self.presentation.anchor)
    }

    /// The view's size in logical units: Nova's 1024 x 768 until
    /// [`FlightView::set_view_size`] widens it.
    #[must_use]
    pub fn view_size(&self) -> (f32, f32) {
        self.view
    }

    /// The frame of the ship's sheet drawn, or `None` without a sheet: its
    /// set's, as its display picked it, at the heading it is drawn at
    /// (see the [`ship_layers`](crate::flight::ship_layers) module).
    #[must_use]
    pub fn frame(&self) -> Option<u16> {
        let sheet = self.sheet.as_ref().ok()?;
        let player = self.frame.player();
        // A failed flight's record turns on no frame table, set 0: the
        // sheet's own rotation frame.
        let spec = turning(&player.frames, sheet);
        Some(spec.index(player.drawn.set, self.shown_heading()))
    }

    /// The fight's explosions, debris and sounds.
    #[must_use]
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    /// `ship`'s shield-flash timer as the last frame showed it
    /// ([`ShieldFlash::shown`]): 32 on the frame of a hit, then a frame
    /// less each frame; none once it has run out, or with no hit. A class
    /// with a shän shield layer shows it at [`ShieldFlash::step`]'s level.
    #[must_use]
    pub fn shield_flash(&self, ship: ShipRef) -> Option<f32> {
        self.flashes
            .get(&ship)
            .map(ShieldFlash::shown)
            .filter(|&shown| shown > 0.0)
    }

    /// Runs the cursor's `steps` flight steps (see the `cursor` module),
    /// hovering as the pointer is over something targetable now
    /// ([`FlightView::hovering`]), once the flight's steps have run and
    /// the ships are where the frame draws them. A key pressed since the
    /// last step, or a flight key held, fades it; the weapon safety key
    /// ([`DESELECT_KEY`]) only by its press, as `_HandlePlayer` sets
    /// *0x3b5550 for it only on the pass it lets go of a secondary
    /// (@0x6ade1), which its press has faded already; a stellar slot key
    /// only while the slot block polls it, as the block sets *0x3b5550 on
    /// every frame it finds one down (@0x69e0d-0x69e12), and the escort
    /// menu's loop never does ([`FlightView::slot_key_polled`]).
    pub(super) fn step_cursor(&mut self, steps: u32) {
        if steps == 0 {
            return;
        }
        let frames = self.looks.cursor_sheet().map(|sheet| sheet.frames);
        let hovering = self.hovering();
        let held = self.held.iter().any(|&key| {
            self.slot_key_polled(key, false)
                .unwrap_or(key != DESELECT_KEY)
        });
        for _ in 0..steps {
            self.cursor.step(hovering, frames, held);
        }
    }

    /// What `input` tells the cursor: a pointer move or button press shows
    /// it where the pointer is, a button holds it whole until its release,
    /// and the pointer leaving the window hides it; a key pressed fades it
    /// at the next flight step (none runs while the map is open, and the M
    /// that opened it has faded it already).
    pub(super) fn cursor_input(&mut self, input: &Input) {
        match *input {
            Input::PointerMoved(at) => self.cursor.pointer(at),
            Input::PointerButton {
                pressed: true, at, ..
            } => {
                self.cursor.pointer(at);
                self.cursor.button(true);
            }
            Input::PointerButton { pressed: false, .. } => self.cursor.button(false),
            Input::PointerLeft => self.cursor.left(),
            Input::Key { pressed: true, .. } => self.cursor.key(),
            _ => {}
        }
    }

    /// Whether the pointer, while the cursor shows, is over something the
    /// player can target, asked of the flight
    /// ([`FlightCommand::Hover`](nova_sim::flight_frame::FlightCommand::Hover),
    /// `nova_sim::Session::hover`) after the steps, the ships boxed where
    /// the frame they left draws them ([`FlightView::drawn_boxes`]). Any
    /// answer but [`Answer::Hovered`](nova_sim::flight_frame::Answer::Hovered)
    /// with something under it, a failed flight's refusal among them, is
    /// nothing hovered.
    pub(super) fn hovering(&mut self) -> bool {
        let Some(at) = self.cursor.shown_at() else {
            return false;
        };
        let world = self.camera().screen_to_world(at);
        let hover = nova_sim::flight_frame::FlightCommand::Hover {
            at: Vec2::new(world.x, world.y),
            drawn: self.drawn_boxes(),
        };
        matches!(
            self.send(hover),
            nova_sim::flight_frame::Answer::Hovered(Some(_))
        )
    }

    /// The boxes the ships are drawn in, for the hover test and a click in
    /// space. Every NPC is drawn, by its sheet or as its crossed box, so
    /// each has a box, even one its cloak hides from the player: the hover
    /// asks the cloak itself (`nova_sim::Session::hover`). Each box is the
    /// rect its hull is drawn in, placed at whole pixels from the player
    /// by the camera ([`Camera::sprite_rect`], or
    /// [`Camera::sprite_rect_from`] from a slot's last offset), the same
    /// placement the hull is drawn by, at the size of a frame of the sheet
    /// its slot's sprite holds
    /// ([`FrameShip::framed`], `nova_sim::Session::framed_by`): a shown
    /// ship's own type's. A hidden ship (`nova_sim::Session::is_hidden`:
    /// its type's `Cost` is -1) is boxed where it would be drawn, as the
    /// original still positions its hidden sprite (`_HandleShipDisplay`
    /// @0x2cd17) and `_HandleMouse` skips only a slot with no sprite
    /// (@0x38171), by the frames its slot last cloned, as
    /// `_SWCloneFramesFromMaster` (@0x2b824) is skipped for it: often the
    /// previous ship's at that slot ([`nova_sim::RuleKey::HiddenShipHoverBox`]
    /// chooses). A ship its slot has not displayed yet is boxed where its
    /// slot was last laid out, at that offset from where the player is
    /// drawn, in its slot's last frames, or has no box in a slot never laid
    /// out ([`FrameShip::hover`], `nova_sim::Session::hover_place`;
    /// [`nova_sim::RuleKey::UndisplayedShips`] chooses).
    pub(super) fn drawn_boxes(&self) -> DrawnBoxes {
        let npcs = self
            .frame
            .npcs()
            .filter_map(|npc| {
                let ShipRef::Npc(id) = npc.ship else {
                    return None;
                };
                Some((id, self.npc_box(npc)?))
            })
            .collect();
        DrawnBoxes {
            player: Some(self.player_box()),
            npcs,
        }
    }

    /// The box NPC `npc` is drawn in, or none when its slot never laid out
    /// (see [`FlightView::drawn_boxes`]).
    fn npc_box(&self, npc: &FrameShip) -> Option<nova_sim::DrawnBox> {
        let size = drawn_size(self.npc_sheets.get(&npc.framed?));
        let camera = self.camera();
        let rect = match npc.hover {
            nova_sim::HoverPlace::Own => camera.sprite_rect(self.shown_npc(npc).0, size),
            nova_sim::HoverPlace::Stale(offset) => camera.sprite_rect_from(offset, size),
            nova_sim::HoverPlace::Unplaced => return None,
        };
        Some(nova_sim::DrawnBox::at(rect))
    }

    /// The box the player's ship is drawn in: a frame of its own sheet
    /// while its slot holds its own type's frames, otherwise of the type's
    /// whose frames it holds (see [`FlightView::drawn_boxes`]).
    fn player_box(&self) -> nova_sim::DrawnBox {
        let player = self.frame.player();
        let size = if player.framed == Some(player.class) {
            drawn_size(Some(&self.sheet))
        } else {
            drawn_size(player.framed.and_then(|ship| self.npc_sheets.get(&ship)))
        };
        nova_sim::DrawnBox::at(self.camera().sprite_rect(self.shown_position(), size))
    }

    /// A click at screen point `at`, as the original's in space
    /// (`_HandlePlayer` @0x6a000-0x6a764): one in the status panel, the
    /// right [`STATUS_BAR_WIDTH`] (0xc2) of the view at any width, x ≥
    /// width − 194 (@0x6a046-0x6a065), radar included, is
    /// no click in space and does nothing (@0x6a065, @0x6a36d). One the
    /// escort menu took never gets here ([`FlightView::click_escort_menu`],
    /// @0x6a060-0x6a12b). Any other goes to the flight
    /// ([`FlightCommand::ClickInSpace`], `nova_sim::Session::click_in_space`),
    /// the ships boxed where they are drawn ([`FlightView::drawn_boxes`]): a
    /// ship under it is targeted, during a jump too, and otherwise the
    /// stellar under it selected.
    pub(super) fn click_in_space(&mut self, at: Point) {
        if at.x >= self.view.0 - STATUS_BAR_WIDTH {
            return;
        }
        let world = self.camera().screen_to_world(at);
        let drawn = self.drawn_boxes();
        self.send(FlightCommand::ClickInSpace {
            at: Vec2::new(world.x, world.y),
            drawn,
        });
    }

    /// The ships and the shots, bottom first, each in its layer as the
    /// flight stacks them ([`sprites`](super::sprites)): the ships as the
    /// frame draws them ([`FlightFrame::drawn`](nova_sim::flight_frame::FlightFrame::drawn),
    /// `nova_sim::Session::draw_layers`; see [`nova_sim::ship_layer`]),
    /// each layer in the order the flight's rules overlap them, and the
    /// shots among them by their layers ([`nova_sim::sprite_layer`]). A
    /// flight that failed to start draws the player alone, in the ships'
    /// layer.
    pub(super) fn sprites_stacked(&self, camera: &Camera) -> Vec<(SpriteLayer, Sprite)> {
        sprite_stack(self.frame.drawn.iter().copied(), self.shown_shots(camera))
    }

    /// Draws the sprites of `stacked` in `layer`, in their order: the
    /// player, as much of it as its record shows
    /// ([`FrameShip::shown`]), the NPCs among `npcs` (as
    /// [`FlightView::shown_npcs`] places them) and the shots, translucent
    /// as [`RuleKey::TranslucentShotFade`](nova_sim::RuleKey::TranslucentShotFade)
    /// reads.
    pub(super) fn draw_sprite_layer(
        &self,
        list: &mut DrawList,
        camera: &Camera,
        stacked: &[(SpriteLayer, Sprite)],
        npcs: &[(&FrameShip, Point, f32)],
        layer: SpriteLayer,
    ) {
        let player = self.shown_position();
        let sprites = stacked.iter().filter(|(on, _)| *on == layer);
        for (_, sprite) in sprites {
            match sprite {
                Sprite::Ship(ShipRef::Player) => {
                    let place = |size| camera.sprite_centre(player, size);
                    self.draw_ship(list, &place, self.frame.player().shown);
                }
                Sprite::Ship(ship @ ShipRef::Npc(_)) => {
                    if let Some(&(npc, at, heading)) =
                        npcs.iter().find(|(npc, ..)| npc.ship == *ship)
                    {
                        let place = |size| camera.sprite_centre(at, size);
                        self.draw_npc(list, npc, &place, heading);
                    }
                }
                Sprite::Shot(shot) => {
                    weapons::draw_shot(list, shot, &self.looks, self.presentation.shot_fade);
                }
            }
        }
    }

    /// Draws NPC `npc` where `place` puts each of its sprites (see
    /// [`ShipDraw::draw`]) facing `heading` with its own ship's sheet,
    /// or a crossed box when its sheet cannot be read, as its cloak leaves
    /// it ([`CloakDraw`], see [`cloak_fade`](crate::flight::cloak_fade)):
    /// jittered and faint while fading, faint or, but for a lit weapon
    /// glow, not at all at a full fade. One
    /// waiting at the gate it comes out of is hidden, then fades in over
    /// its last ticks there ([`npc_tint`]), washed white by the gate, as
    /// one entering a gate is ([`gate_colorize`]); with neither, its hull is as
    /// translucent as its `BaseTransp` ([`base_alpha`]). Its hull and alt
    /// layer are drawn in its paint ([`FrameShip::paint`], see
    /// [`paint`](crate::flight::paint)), whose channels its cloak's
    /// factors follow too. Its hull and
    /// layers are drawn on the frames its display picked and murked (see
    /// the [`ship_layers`](crate::flight::ship_layers) module): its engine
    /// glow from its glow base, rolled on its own, its running lights at
    /// the level its display drew them, its weapon glow while lit
    /// ([`weapon_glow`](crate::flight::weapon_glow)), its alt layer, and
    /// its shield layer while it flashes.
    pub(super) fn draw_npc(
        &self,
        list: &mut DrawList,
        npc: &FrameShip,
        place: Place<'_>,
        heading: f32,
    ) {
        let ShipRef::Npc(id) = npc.ship else {
            return;
        };
        let tick = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
        let cloak = CloakDraw::of(npc.cloak, npc.paint, tick, &cloak_rolls(id));
        let hull = cloak.is_none_or(|cloak| cloak.hull.is_some());
        // Fully faded, a cloak's or a gate's: its held alt is forgotten.
        let Some(tint) = npc_tint(npc.gate).filter(|_| hull) else {
            self.held_alts.borrow_mut().forget(npc.ship);
            // A cloak's full fade still draws the weapon glow (@0x2d287).
            // Its glow, lights and shield are hidden with it.
            if !hull && let Some(Ok(sheet)) = self.npc_sheets.get(&npc.class) {
                let ship = self.npc_ship(npc, sheet, heading, (Color::WHITE, cloak));
                ShipDraw {
                    hull: false,
                    ..ship
                }
                .draw(list, place, None);
            }
            return;
        };
        match self.npc_sheets.get(&npc.class) {
            Some(Ok(sheet)) => {
                let alpha = if npc.gate.at_gate() {
                    tint.a
                } else {
                    base_alpha(sheet.base_transp)
                };
                let ship = self.npc_ship(npc, sheet, heading, (Color { a: alpha, ..tint }, cloak));
                let live = ship.live_alt(place);
                let alt =
                    self.held_alts
                        .borrow_mut()
                        .shown(npc.ship, ship.drawn.alt, live, ship.hull);
                ship.draw(list, place, alt);
            }
            _ => crossed_box(
                list,
                place(placeholder_size()),
                PLACEHOLDER_SIZE,
                PLACEHOLDER,
            ),
        }
    }

    /// NPC `npc` as drawn with `sheet` facing `heading`, its hull at
    /// `tint` changed by `cloak` (see [`FlightView::draw_npc`]), murked as
    /// the player's ship sees it where the frame has both, in the player's
    /// effective murk.
    fn npc_ship<'a>(
        &self,
        npc: &'a FrameShip,
        sheet: &'a ShipSheet,
        heading: f32,
        (tint, cloak): (Color, Option<CloakDraw>),
    ) -> ShipDraw<'a> {
        let viewer = self.frame.player().position;
        let murk = sprite_murk(npc.position, viewer, self.frame.effective_murk);
        let tick = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
        let glow_rolls = npc_glow_rolls(match npc.ship {
            ShipRef::Npc(id) => id,
            ShipRef::Player => NpcId(0),
        });
        // A gate writes the hull's colorize after the ion tint, so while
        // it fades the gate's white shows in place of the tint and the
        // murk ([`gate_colorize`], `_HandleShipDisplay` @0x2bfa5-0x2c17b).
        let colorize = if npc.gate.at_gate() {
            gate_colorize(npc.gate)
        } else {
            hull_colorize(murk, npc.ion_tint, self.presentation.ion_tint_murk)
        };
        ShipDraw {
            sheet,
            spec: &npc.frames,
            drawn: npc.drawn,
            heading,
            tint,
            paint: npc.paint,
            cloak,
            hull: true,
            murk,
            held: true,
            glow: glow_level(npc.glow, tick, &glow_rolls),
            lights: npc.drawn.lights,
            weapon: self.glows.drawn(npc.ship).filter(|_| self.weapon_effects),
            shield: self.shield_drawn(npc.ship),
            colorize,
        }
    }

    /// The level `ship`'s shield layer was last drawn at, while it
    /// flashes ([`ShieldFlash::drawn`]).
    fn shield_drawn(&self, ship: ShipRef) -> Option<u8> {
        self.flashes.get(&ship).and_then(ShieldFlash::drawn)
    }

    /// Draws the player's ship where `place` puts each of its sprites (see
    /// [`ShipDraw::draw`]), as much of it as `drawn` says
    /// (see [`nova_sim::player_fate`]): its hull, as translucent as its
    /// `BaseTransp` and in its paint ([`FrameShip::paint`], see
    /// [`paint`](crate::flight::paint)), its engine glow, running lights at the level its
    /// display drew them (`nova_sim::combat::ship_lights`), weapon glow
    /// ([`weapon_glow`](crate::flight::weapon_glow)), alt layer and shield
    /// flash, never murked, on the frames its display picked (see the
    /// [`ship_layers`](crate::flight::ship_layers) module), all as the
    /// frame's record of it says; or, when its sheet cannot be read, a
    /// crossed box and the reason. The box stands in for the sprite and a
    /// missing sheet has no layers, so only a ship drawn in full has one.
    pub(super) fn draw_ship(&self, list: &mut DrawList, place: Place<'_>, drawn: ShipDrawn) {
        if drawn == ShipDrawn::Hidden {
            self.held_alts.borrow_mut().forget(ShipRef::Player);
            return;
        }
        match &self.sheet {
            Ok(sheet) => {
                let tick = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
                let player = self.frame.player();
                let ship = ShipDraw {
                    sheet,
                    spec: &player.frames,
                    drawn: player.drawn,
                    heading: self.shown_heading(),
                    tint: Color {
                        a: base_alpha(sheet.base_transp),
                        ..Color::WHITE
                    },
                    paint: player.paint,
                    // Its cloak drawn as an NPC's, its full fade always
                    // faint (see `Session::player_cloak_shown`).
                    cloak: CloakDraw::of(player.cloak, player.paint, tick, &player_cloak_rolls()),
                    hull: drawn == ShipDrawn::Full,
                    murk: 0,
                    held: false,
                    glow: glow_level(player.glow, tick, &self.glow_rolls),
                    lights: player.drawn.lights,
                    weapon: self
                        .glows
                        .drawn(ShipRef::Player)
                        .filter(|_| self.weapon_effects),
                    shield: self.shield_drawn(ShipRef::Player),
                    colorize: hull_colorize(0, player.ion_tint, self.presentation.ion_tint_murk),
                };
                let live = ship.live_alt(place);
                let alt = self.held_alts.borrow_mut().shown(
                    ShipRef::Player,
                    ship.drawn.alt,
                    live,
                    ship.hull,
                );
                ship.draw(list, place, alt);
            }
            Err(_) if drawn != ShipDrawn::Full => {}
            Err(reason) => {
                let at = place(placeholder_size());
                crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER);
                let below = Point::new(
                    at.x - PLACEHOLDER_SIZE / 2.0,
                    at.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP,
                );
                list.text(
                    format!("Sprite unavailable: {reason}"),
                    below,
                    OVERLAY_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    /// The shots in flight in the order they are drawn
    /// ([`FlightFrame::shots`](nova_sim::flight_frame::FlightFrame::shots)),
    /// each where it is drawn: `alpha` of the way from where it was a step
    /// ago, as the ships are; each with the index its spin started on
    /// ([`SpinStarts`](crate::flight::spin::SpinStarts)) and its murk where
    /// the frame holds it, seen from the player's ship as the frame holds
    /// it, in the player's effective murk ([`murk::sprite_murk`]), as
    /// `_HandleShot` murks it once a pass after it moves (@0x35c48).
    pub(super) fn shown_shots(&self, camera: &Camera) -> Vec<ShotShown> {
        let behind = 1.0 - self.alpha;
        let viewer = self.frame.player().position;
        let murk = self.frame.effective_murk;
        self.frame
            .shots
            .iter()
            .map(|shot| ShotShown {
                id: shot.id,
                spin_start: self.shot_looks.spin_starts.start(shot.id),
                spin_timer: shot.spin_timer,
                murk: murk::sprite_murk(shot.position, viewer, murk),
                at: camera.world_to_screen(Point::new(
                    shot.velocity.x.mul_add(-behind, shot.position.x),
                    shot.velocity.y.mul_add(-behind, shot.position.y),
                )),
                heading: shot.heading,
                age: shot.age,
                weapon: shot.weapon,
                layer: SpriteLayer::from(shot.layer),
            })
            .collect()
    }

    /// The cargo boxes in space, each where it is drawn: `alpha` of the
    /// way from where it was a step ago, as the shots are; each with its
    /// murk where the frame holds it, seen from the player's ship in the
    /// player's effective murk ([`murk::sprite_murk`]), as `_HandleBoxes`
    /// murks it (@0x2e15e-0x2e181).
    pub(super) fn shown_boxes(&self, camera: &Camera) -> Vec<BoxShown> {
        let behind = 1.0 - self.alpha;
        let viewer = self.frame.player().position;
        let murk = self.frame.effective_murk;
        self.frame
            .cargo_boxes
            .iter()
            .map(|cargo_box| BoxShown {
                at: camera.world_to_screen(Point::new(
                    cargo_box.velocity.x.mul_add(-behind, cargo_box.position.x),
                    cargo_box.velocity.y.mul_add(-behind, cargo_box.position.y),
                )),
                frame: cargo_box.frame,
                fade: cargo_box.fade,
                murk: murk::sprite_murk(cargo_box.position, viewer, murk),
                graphic: cargo_box.graphic,
            })
            .collect()
    }

    /// The asteroids in the field, each where it is drawn: `alpha` of the
    /// way from where it was a step ago, as the cargo boxes are, at its
    /// whole frame, murked as the cargo boxes are (`_HandleAsteroids`
    /// @0x36422-0x36446).
    pub(super) fn shown_asteroids(&self, camera: &Camera) -> Vec<AsteroidShown> {
        let behind = 1.0 - self.alpha;
        let viewer = self.frame.player().position;
        let murk = self.frame.effective_murk;
        self.frame
            .asteroids
            .iter()
            .map(|asteroid| AsteroidShown {
                at: camera.world_to_screen(Point::new(
                    asteroid.velocity.x.mul_add(-behind, asteroid.position.x),
                    asteroid.velocity.y.mul_add(-behind, asteroid.position.y),
                )),
                // Its whole part; a frame below 0, which the single wrap
                // can leave, shows the first.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                frame: asteroid.frame as u16,
                kind: asteroid.kind,
                murk: murk::sprite_murk(asteroid.position, viewer, murk),
            })
            .collect()
    }

    /// The beams being fired, each moved with its firer as it is drawn.
    pub(super) fn shown_beams(&self, camera: &Camera) -> Vec<BeamShown> {
        self.frame
            .beams
            .iter()
            .map(|beam| {
                let (dx, dy) = self.drawn_off(beam.firer);
                let shown = |at: Vec2| camera.world_to_screen(Point::new(at.x + dx, at.y + dy));
                BeamShown {
                    start: shown(beam.start),
                    end: shown(beam.end),
                    weapon: beam.weapon,
                    fade: beam.fade,
                }
            })
            .collect()
    }

    /// How far `ship` is drawn from where the frame has it.
    fn drawn_off(&self, ship: ShipRef) -> (f32, f32) {
        let (shown, now) = match ship {
            ShipRef::Player => (self.shown_position(), self.frame.player().position),
            ShipRef::Npc(_) => match self.frame.ship(ship) {
                Some(npc) => (self.shown_npc(npc).0, npc.position),
                None => return (0.0, 0.0),
            },
        };
        (shown.x - now.x, shown.y - now.y)
    }

    /// Draws the brackets round the nav target `nav`, then those round the
    /// ship target `target`, the flight's (`nova_sim::Session::nav_brackets`,
    /// `nova_sim::Session::target_brackets`), each centred where its sprite
    /// is drawn (see the `brackets` module): the target's where the frame's
    /// record of its ship is drawn. A stellar whose sheet can't be read is
    /// bracketed as one with no sprite; a ship's as its crossed box.
    pub(super) fn draw_brackets(
        &self,
        list: &mut DrawList,
        (scene, camera): (&SystemScene, &Camera),
        nav: Option<NavBrackets>,
        target: Option<TargetBrackets>,
    ) {
        if let Some(nav) = nav {
            // A selection the scene doesn't hold, such as a stellar
            // another system owns, is bracketed at its spöb position with
            // no sprite (`_HandleNavTargetSprites` @0x2fd9f-0x2fe41).
            let (position, sprite) = scene
                .stellars()
                .iter()
                .find(|s| s.id == nav.stellar)
                .map_or((super::point(nav.position), None), |stellar| {
                    let sprite = stellar.drawn().and_then(|sheet| sheet.as_ref().ok());
                    let sprite = sprite.map(|sheet| (sheet.frame_width, sheet.frame_height));
                    (stellar.position, sprite)
                });
            let at = camera.world_to_screen(position);
            brackets::draw_nav(list, at, sprite, nav.expand, nav.dominated);
        }
        if let Some(target) = target
            && let Some(npc) = self.frame.ship(ShipRef::Npc(target.npc))
        {
            let (at, _) = self.shown_npc(npc);
            let sprite = drawn_size(self.npc_sheets.get(&npc.class));
            let at = camera.world_to_screen(at);
            brackets::draw_ship(list, at, Some(sprite), target.expand, target.standing);
        }
    }
}
