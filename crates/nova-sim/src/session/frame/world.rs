//! The world's half of [`Session::fill_frame`]: the ships, shots, beams,
//! cargo boxes, asteroids and stellars the flight view draws, each written
//! into the frame's own buffers, and the record builders the step's report
//! shares ([`Session::frame_ship_player`], [`Session::frame_ship_npc`],
//! [`FrameShot::of`]), so a field the frame and a pass both carry is
//! built once.

use super::super::{Chart, Session, Space};
use crate::catalog::StellarId;
use crate::combat::ShipRef;
use crate::flight_frame::{
    FrameAsteroid, FrameBeam, FrameBox, FrameShip, FrameShot, FrameStellar, GateFade,
};
use crate::glow::drawn_base;
use crate::iff::ShipIff;
use crate::player_fate::Drawn as ShipDrawn;
use crate::ship_layer::ShipLayer;
use crate::traffic::npc::Npc;

impl Session {
    /// The player's ship as the frame and a pass carry it: where and how
    /// it is drawn, its paint (the pilot's), its hover box's place and
    /// frames, and its gauges. It never shows on the radar, which draws it
    /// apart.
    pub(in crate::session) fn frame_ship_player(&self) -> FrameShip {
        let player = self.player();
        let hull = self.hull();
        FrameShip {
            ship: ShipRef::Player,
            class: self.ship(),
            position: player.position,
            velocity: player.velocity,
            heading: player.heading,
            condition: self.player_condition(),
            breakup: hull.breakup,
            shield: self.reserves().shield,
            class_shield: self.class_shield(self.ship()),
            frames: hull.frames,
            drawn: self.player_drawn(),
            shown: self.player_fate().row().drawn,
            glow: self.engine_glow(),
            cloak: self.player_cloak_shown(),
            ion_tint: self.player_ion_tint(),
            paint: self.pilot.paint(),
            gate: GateFade::None,
            hover: self.hover_place(ShipRef::Player),
            framed: self.framed_by(ShipRef::Player),
            govt: None,
            on_radar: false,
            iff: ShipIff::Other,
            mass: hull.mass,
        }
    }

    /// NPC `npc` as the frame and a pass carry it: where and how it is
    /// drawn, in full, its gate's fade, its paint ([`Space::npc_paint`]),
    /// its hover box's place and frames, its gauges, its government and
    /// its radar facts.
    pub(in crate::session) fn frame_ship_npc(&self, npc: &Npc) -> FrameShip {
        let ship = ShipRef::Npc(npc.id);
        FrameShip {
            ship,
            class: npc.ship,
            position: npc.state.position,
            velocity: npc.state.velocity,
            heading: npc.state.heading,
            condition: npc.condition,
            breakup: npc.hull.breakup,
            shield: npc.reserves.shield,
            class_shield: self.class_shield(npc.ship),
            frames: npc.hull.frames,
            drawn: npc.display.drawn,
            shown: ShipDrawn::Full,
            glow: drawn_base(npc.display.glow),
            cloak: self.cloak_shown(npc),
            ion_tint: npc.display.ion_tint,
            paint: self.space.npc_paint(npc, &self.statics),
            gate: GateFade::of(&npc.mode),
            hover: self.hover_place(ship),
            framed: self.framed_by(ship),
            govt: npc.govt,
            on_radar: self.on_radar(npc),
            iff: self.ship_iff(npc),
            mass: npc.hull.mass,
        }
    }

    /// Fills `into` with every ship's record, the player's first, then
    /// each NPC's in the system's order.
    pub(in crate::session) fn fill_ships(&self, into: &mut Vec<FrameShip>) {
        into.clear();
        into.push(self.frame_ship_player());
        into.extend(self.npcs().iter().map(|npc| self.frame_ship_npc(npc)));
    }

    /// Fills `into` with the ships drawn, in draw order, each with its
    /// layer ([`Session::draw_layers`]).
    pub(super) fn fill_drawn(&self, into: &mut Vec<(ShipRef, ShipLayer)>) {
        into.clear();
        into.extend(self.draw_layers());
    }

    /// Fills `into` with each stellar the system lists, shown or not
    /// ([`Chart::stellars_listed`]), each with how it is drawn: whatever
    /// stellar of the system a scene holds has its record.
    pub(super) fn fill_stellars(&self, into: &mut Vec<FrameStellar>) {
        into.clear();
        into.extend(
            self.chart
                .stellars_listed()
                .map(|(id, shown)| FrameStellar {
                    id,
                    shown,
                    destroyed: self.galaxy().destroyed(id),
                    drawn: self.drawn_sheet(id),
                    counted: self.counted_sheet(id),
                    frozen: self.frame_frozen(id),
                    counts_none: self.counts_no_frames(id),
                    gate_frame: self.gate_frame(id),
                }),
        );
    }
}

impl Space {
    /// Fills `into` with the shots in flight, in the order they are drawn
    /// (`Space::shots_drawn`).
    pub(super) fn fill_shots(&self, into: &mut Vec<FrameShot>) {
        into.clear();
        into.extend(self.shots_drawn().into_iter().map(FrameShot::of));
    }

    /// Fills `into` with the beams being fired.
    pub(super) fn fill_beams(&self, into: &mut Vec<FrameBeam>) {
        into.clear();
        into.extend(self.beams().iter().map(FrameBeam::of));
    }

    /// Fills `into` with the cargo boxes in space.
    pub(super) fn fill_boxes(&self, into: &mut Vec<FrameBox>) {
        into.clear();
        into.extend(self.cargo_boxes().iter().map(FrameBox::of));
    }

    /// Fills `into` with the asteroids in the field.
    pub(super) fn fill_asteroids(&self, into: &mut Vec<FrameAsteroid>) {
        into.clear();
        into.extend(self.asteroids().map(FrameAsteroid::of));
    }
}

impl Chart {
    /// Each stellar shown, in nav order, then each other one its system
    /// lists, in its `NavDefs` order, with whether it is shown.
    pub(super) fn stellars_listed(&self) -> impl Iterator<Item = (StellarId, bool)> + '_ {
        let shown = self.sites.iter().map(|site| (site.id, true));
        let others = self
            .listed
            .iter()
            .filter(|listed| !self.sites.iter().any(|site| site.id == listed.id))
            .map(|site| (site.id, false));
        shown.chain(others)
    }
}
