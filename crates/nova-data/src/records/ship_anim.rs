//! `shän`: ship animation (sprite layers and weapon exit points).
//!
//! Layout: field names and meanings from the EV Nova Bible ("The shän
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 517), which sums to the 192 bytes of every stock record. The
//! template stores four exit points per weapon class, where the Bible names
//! each coordinate once. Sprite and mask IDs may name `rlëD` or `PICT`
//! resources and use 0 for unused, so they stay raw.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::graphics::SheetLayout;
use crate::wire::flags::Flags16;
use crate::wire::raw::RawArray;

/// A ship's animation layers.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct ShipAnim {
    /// Bible `BaseImageID` (offset 0x00, i16).
    pub base_image_id: i16,
    /// Bible `BaseMaskID` (offset 0x02, i16).
    pub base_mask_id: i16,
    /// Bible `BaseSetCount` (offset 0x04, i16).
    pub base_set_count: i16,
    /// Bible `BaseXSize` (offset 0x06, i16).
    pub base_x_size: i16,
    /// Bible `BaseYSize` (offset 0x08, i16).
    pub base_y_size: i16,
    /// Bible `BaseTransp` (offset 0x0A, i16): 0 (opaque) to 32.
    pub base_transp: i16,
    /// Bible `AltImageID` (offset 0x0C, i16): 0 if unused.
    pub alt_image_id: i16,
    /// Bible `AltMaskID` (offset 0x0E, i16): 0 if unused.
    pub alt_mask_id: i16,
    /// Bible `AltSetCount` (offset 0x10, i16).
    pub alt_set_count: i16,
    /// Bible `AltXSize` (offset 0x12, i16).
    pub alt_x_size: i16,
    /// Bible `AltYSize` (offset 0x14, i16).
    pub alt_y_size: i16,
    /// Bible `GlowImageID` (offset 0x16, i16): engine glow.
    pub glow_image_id: i16,
    /// Bible `GlowMaskID` (offset 0x18, i16).
    pub glow_mask_id: i16,
    /// Bible `GlowXSize` (offset 0x1A, i16).
    pub glow_x_size: i16,
    /// Bible `GlowYSize` (offset 0x1C, i16).
    pub glow_y_size: i16,
    /// Bible `LightImageID` (offset 0x1E, i16): running lights.
    pub light_image_id: i16,
    /// Bible `LightMaskID` (offset 0x20, i16).
    pub light_mask_id: i16,
    /// Bible `LightXSize` (offset 0x22, i16).
    pub light_x_size: i16,
    /// Bible `LightYSize` (offset 0x24, i16).
    pub light_y_size: i16,
    /// Bible `WeapImageID` (offset 0x26, i16): weapon effects.
    pub weap_image_id: i16,
    /// Bible `WeapMaskID` (offset 0x28, i16).
    pub weap_mask_id: i16,
    /// Bible `WeapXSize` (offset 0x2A, i16).
    pub weap_x_size: i16,
    /// Bible `WeapYSize` (offset 0x2C, i16).
    pub weap_y_size: i16,
    /// Bible `Flags` (offset 0x2E, u16).
    pub flags: Flags16,
    /// Bible `AnimDelay` (offset 0x30, i16): 30ths of a second.
    pub anim_delay: i16,
    /// Bible `WeapDecay` (offset 0x32, i16).
    pub weap_decay: i16,
    /// Bible `FramesPer` (offset 0x34, i16): frames per rotation.
    pub frames_per: i16,
    /// Bible `BlinkMode` (offset 0x36, i16): how the running lights
    /// (`LightImageID`) blink. The Bible names the modes "0 or -1 Ignored",
    /// "1 Square-wave blinking", "2 Triangle-wave pulsing" and "3 Random
    /// pulsing"; what `BlinkValA`–`D` mean in each is on those fields.
    ///
    /// The original keeps a lights intensity from 0 to 32 per ship (32 is
    /// full brightness) and updates it once every 1/30 s tick: every timer
    /// and ramp in `_HandleShipDisplay` (0x2b514), at 0x2c2ea–0x2c565,
    /// scales by `_gSpeedMult`, which `_HandleTimeAdjustment` sets to the
    /// frame's elapsed milliseconds x 0.03 (at 0x333e1), so 1.0 is one
    /// tick. The lights are drawn only while the intensity is above 1, at
    /// level `trunc(intensity)` out of 32 (at 0x2c59a–0x2c6a9). Level 32 is
    /// `_BlitPixieRLEAddOver` (0xc24bf), a bitwise OR into the screen;
    /// lower levels use `_BlitPixieRLETranslucent` (0xc1568), whose
    /// per-pixel `_BlitPixieTranslucentCopy` (0xc1110) ORs in the lights
    /// scaled by level/32 and leaves the screen undimmed. The full record
    /// is on `nova_view::draw::lights_tint`. The intensity starts at 0
    /// (`_InitObjects`, at 0x1c423).
    ///
    /// Any mode but 1, 2 and 3 (stock: -1) holds the intensity at 32
    /// (0x2c565): the lights show steadily, which is what the Bible's
    /// "Ignored" amounts to.
    pub blink_mode: i16,
    /// Bible `BlinkValA` (offset 0x38, i16).
    ///
    /// - Mode 1: the **off-time** between the blinks of a group, in ticks.
    ///   The Bible calls A "the light on-time", but `_HandleShipDisplay`
    ///   loads A into the timer as it turns the light off and counts a
    ///   blink (0x2c380–0x2c395); see `blink_val_b`.
    /// - Mode 2: the minimum intensity, 1 to 32 (Bible). The falling ramp
    ///   stops at A and turns to rise (0x2c466–0x2c480).
    /// - Mode 3: the minimum intensity, 1 to 32 (Bible): each change picks
    ///   A + `Rand(B - A + 1)` (0x2c4dd–0x2c4fb).
    pub blink_val_a: i16,
    /// Bible `BlinkValB` (offset 0x3A, i16).
    ///
    /// - Mode 1: the **on-time** of each blink, in ticks. The Bible calls B
    ///   "the delay between blinks", but `_HandleShipDisplay` loads B into
    ///   the timer as it turns the light on, at intensity 32
    ///   (0x2c39f–0x2c3b4). This follows the executable, not the Bible's
    ///   labels. Each tick the timer is first clamped to at most
    ///   max(B, D) (0x2c31a–0x2c340), then counts down one a tick, and the
    ///   light switches on the tick after it goes below 0, so a phase
    ///   loaded with `v` lasts max(min(v, max(B, D)), -1) + 2 ticks. The
    ///   Shuttle (A=4 B=1 C=2 D=20) is lit for 3 ticks, dark for 6, lit
    ///   for 3, then dark for 28: a double flash every 40 ticks.
    /// - Mode 2: the intensity added each rising tick, in hundredths
    ///   (Bible: "x100"; 0.01 x B x `_gSpeedMult` at 0x2c44d–0x2c4b6).
    /// - Mode 3: the maximum intensity, 1 to 32 (Bible), clamped to at
    ///   most 31 when the ship is loaded (`_LoadExtendedShipSprites`,
    ///   0x233eb–0x233ff).
    pub blink_val_b: i16,
    /// Bible `BlinkValC` (offset 0x3C, i16).
    ///
    /// - Mode 1: the number of blinks in a group (Bible); 0 or less never
    ///   lights (0x2c360).
    /// - Mode 2: the maximum intensity, 1 to 32 (Bible), clamped to at
    ///   most 31 when the ship is loaded (`_LoadExtendedShipSprites`,
    ///   0x233c7–0x233db). The rising ramp stops at C and turns to fall
    ///   (0x2c425–0x2c43f). Each bound is checked before the step, so the
    ///   intensity can overshoot C, or undershoot A, by less than a step
    ///   for one tick.
    /// - Mode 3: the delay between intensity changes, in ticks (the Bible
    ///   gives no unit): each value holds for max(C, 0) + 1 ticks
    ///   (0x2c4c0–0x2c563).
    pub blink_val_c: i16,
    /// Bible `BlinkValD` (offset 0x3E, i16).
    ///
    /// - Mode 1: the delay between groups, in ticks (the Bible gives no
    ///   unit): after the C-th blink's off-time the light stays dark for a
    ///   phase loaded with D (0x2c3cd–0x2c3f4).
    /// - Mode 2: the intensity taken away each falling tick, in hundredths
    ///   (Bible: "x100"; -0.01 x D x `_gSpeedMult` at 0x2c48e–0x2c4b6).
    /// - Mode 3: ignored (Bible).
    pub blink_val_d: i16,
    /// Bible `ShieldImageID` (offset 0x40, i16): shield bubble.
    pub shield_image_id: i16,
    /// Bible `ShieldMaskID` (offset 0x42, i16).
    pub shield_mask_id: i16,
    /// Bible `ShieldXSize` (offset 0x44, i16).
    pub shield_x_size: i16,
    /// Bible `ShieldYSize` (offset 0x46, i16).
    pub shield_y_size: i16,
    /// Bible `GunPosX` (offset 0x48, 4 x i16).
    pub gun_pos_x: [i16; 4],
    /// Bible `GunPosY` (offset 0x50, 4 x i16).
    pub gun_pos_y: [i16; 4],
    /// Bible `TurretPosX` (offset 0x58, 4 x i16).
    pub turret_pos_x: [i16; 4],
    /// Bible `TurretPosY` (offset 0x60, 4 x i16).
    pub turret_pos_y: [i16; 4],
    /// Bible `GuidedPosX` (offset 0x68, 4 x i16).
    pub guided_pos_x: [i16; 4],
    /// Bible `GuidedPosY` (offset 0x70, 4 x i16).
    pub guided_pos_y: [i16; 4],
    /// Bible `BeamPosX` (offset 0x78, 4 x i16).
    pub beam_pos_x: [i16; 4],
    /// Bible `BeamPosY` (offset 0x80, 4 x i16).
    pub beam_pos_y: [i16; 4],
    /// Bible `UpCompressX` (offset 0x88, i16): percent; 0 means 100.
    pub up_compress_x: i16,
    /// Bible `UpCompressY` (offset 0x8A, i16).
    pub up_compress_y: i16,
    /// Bible `DnCompressX` (offset 0x8C, i16).
    pub dn_compress_x: i16,
    /// Bible `DnCompressY` (offset 0x8E, i16).
    pub dn_compress_y: i16,
    /// Bible `GunPosZ` (offset 0x90, 4 x i16).
    pub gun_pos_z: [i16; 4],
    /// Bible `TurretPosZ` (offset 0x98, 4 x i16).
    pub turret_pos_z: [i16; 4],
    /// Bible `GuidedPosZ` (offset 0xA0, 4 x i16).
    pub guided_pos_z: [i16; 4],
    /// Bible `BeamPosZ` (offset 0xA8, 4 x i16).
    pub beam_pos_z: [i16; 4],
    /// Offset 0xB0, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0xb0: RawArray<16>,
}

impl ShipAnim {
    /// The sprite sheet layout for this ship's layers: `frames_per`
    /// columns, so each row is one sprite set, or `None` unless it is
    /// positive. The Bible gives `FramesPer` as "The number of frames for
    /// one rotation of this ship" and `BaseSetCount` as "The number of
    /// sprite sets for the basic sprite images"; every layer shares
    /// `frames_per`. This is an export arrangement: the game itself reads
    /// frames by index.
    #[must_use]
    pub fn sheet_layout(&self) -> Option<SheetLayout> {
        u16::try_from(self.frames_per)
            .ok()
            .and_then(SheetLayout::new)
    }
}

impl Record for ShipAnim {
    const TYPE: ResType = ResType::new([b's', b'h', 0x8A, b'n']);
    const SIZE: Option<usize> = Some(192);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn sprite_layer_fields_sit_at_their_bible_offsets() {
        let mut b = buf::<ShipAnim>();
        for (i, offset) in (0x00..0x2E).step_by(2).enumerate() {
            b = b.i16(offset, 100 + i as i16);
        }
        let shan: ShipAnim = b.decode();
        let base = [
            shan.base_image_id,
            shan.base_mask_id,
            shan.base_set_count,
            shan.base_x_size,
            shan.base_y_size,
            shan.base_transp,
        ];
        assert_eq!(base, [100, 101, 102, 103, 104, 105]);
        let alt = [
            shan.alt_image_id,
            shan.alt_mask_id,
            shan.alt_set_count,
            shan.alt_x_size,
            shan.alt_y_size,
        ];
        assert_eq!(alt, [106, 107, 108, 109, 110]);
        let glow = [
            shan.glow_image_id,
            shan.glow_mask_id,
            shan.glow_x_size,
            shan.glow_y_size,
        ];
        assert_eq!(glow, [111, 112, 113, 114]);
        let light = [
            shan.light_image_id,
            shan.light_mask_id,
            shan.light_x_size,
            shan.light_y_size,
        ];
        assert_eq!(light, [115, 116, 117, 118]);
        let weap = [
            shan.weap_image_id,
            shan.weap_mask_id,
            shan.weap_x_size,
            shan.weap_y_size,
        ];
        assert_eq!(weap, [119, 120, 121, 122]);
    }

    #[test]
    fn animation_and_exit_point_fields_sit_at_their_bible_offsets() {
        let shan: ShipAnim = buf::<ShipAnim>()
            .u16(0x2E, 0x0103)
            .i16(0x30, 5)
            .i16(0x32, 50)
            .i16(0x34, 36)
            .i16(0x36, 1)
            .i16(0x38, 11)
            .i16(0x3A, 12)
            .i16(0x3C, 13)
            .i16(0x3E, 14)
            .i16(0x40, 1500)
            .i16(0x42, 1501)
            .i16(0x44, 60)
            .i16(0x46, 58)
            .i16(0x48, -7)
            .i16(0x50, 21)
            .i16(0x58, 2)
            .i16(0x60, -3)
            .i16(0x68, 4)
            .i16(0x70, -5)
            .i16(0x78, 6)
            .i16(0x80, -8)
            .i16(0x88, 90)
            .i16(0x8A, 80)
            .i16(0x8C, 110)
            .i16(0x8E, 120)
            .i16(0x90, 1)
            .i16(0x98, -1)
            .i16(0xA0, 2)
            .i16(0xA8, -2)
            .i16(0xAE, 9)
            .bytes(0xB0, &[0x66; 16])
            .decode();
        assert_eq!(shan.flags, Flags16(0x0103));
        let anim = [
            shan.anim_delay,
            shan.weap_decay,
            shan.frames_per,
            shan.blink_mode,
        ];
        assert_eq!(anim, [5, 50, 36, 1]);
        let blink = [
            shan.blink_val_a,
            shan.blink_val_b,
            shan.blink_val_c,
            shan.blink_val_d,
        ];
        assert_eq!(blink, [11, 12, 13, 14]);
        let shield = [
            shan.shield_image_id,
            shan.shield_mask_id,
            shan.shield_x_size,
            shan.shield_y_size,
        ];
        assert_eq!(shield, [1500, 1501, 60, 58]);
        assert_eq!((shan.gun_pos_x[0], shan.gun_pos_y[0]), (-7, 21));
        assert_eq!((shan.turret_pos_x[0], shan.turret_pos_y[0]), (2, -3));
        assert_eq!((shan.guided_pos_x[0], shan.guided_pos_y[0]), (4, -5));
        assert_eq!((shan.beam_pos_x[0], shan.beam_pos_y[0]), (6, -8));
        let compress = [
            shan.up_compress_x,
            shan.up_compress_y,
            shan.dn_compress_x,
            shan.dn_compress_y,
        ];
        assert_eq!(compress, [90, 80, 110, 120]);
        let z = [
            shan.gun_pos_z[0],
            shan.turret_pos_z[0],
            shan.guided_pos_z[0],
            shan.beam_pos_z[0],
        ];
        assert_eq!(z, [1, -1, 2, -2]);
        assert_eq!(shan.beam_pos_z[3], 9);
        assert_eq!(shan.unknown_0xb0, RawArray([0x66; 16]));
    }

    #[test]
    fn the_sheet_layout_has_one_rotation_per_row() {
        let shan = |frames_per| ShipAnim {
            frames_per,
            ..buf::<ShipAnim>().decode()
        };
        assert_eq!(shan(36).sheet_layout(), SheetLayout::new(36));
        assert_eq!(shan(1).sheet_layout(), SheetLayout::new(1));
        assert_eq!(shan(0).sheet_layout(), None);
        assert_eq!(shan(-36).sheet_layout(), None);
    }
}
