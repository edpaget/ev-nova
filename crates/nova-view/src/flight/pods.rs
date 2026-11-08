//! Drawing the NPCs' escape pods, as `_HandleEscapePods`
//! (@0x39e48-0x3a063) shows them in the `EV Nova` executable. The
//! simulation keeps them (`nova_sim::escape_pod`: their spawn, drift,
//! life, slots and whether each is shown) and hands them over in slot
//! order ([`FlightFrame::pods`](nova_sim::flight_frame::FlightFrame::pods));
//! the view draws each shown one ([`draw_pods`]), and plays `snd ` 372 for
//! each [`CombatEvent::PodEjected`](nova_sim::CombatEvent::PodEjected)
//! ([`Effects`](super::effects::Effects)).
//!
//! - **The place**: at whole pixels from the player, by its sheet's own
//!   sides ([`Camera::sprite_centre`], @0x39ef0-0x39f7d), on the frame of
//!   shän 895's sheet the simulation gave it.
//! - **The fade**: opaque at a life of 32 or more (32.0 at 0xdd654), at
//!   alpha `(32 - trunc(32 - life))/32` over its last 32 ticks, which at a
//!   whole life is life/32, and not drawn at 0 or less (`ucomiss 32, life`
//!   / `jb`, `ucomiss 0, life` / `jbe` @0x39fef-0x3a010, so a NaN life is
//!   hidden). `_HandleEscapePods` sets the sprite's destination factor
//!   +0xa8 to `trunc(32 - life)` (@0x3a00a-0x3a024), its channel factors
//!   left at 0 (`_SWCreateSprite` @0xa8d56-0xa8d71), so its draw proc
//!   (`_BlitPixieRLETranslucentDrawProc`, 0xbb3fc) gives `(src x k) >> 5
//!   | (dst x (32 - k)) >> 5` per channel, k = `32 - trunc(32 - life)`:
//!   the OR crossfade, [`Blend::OrCrossfade`](crate::draw::Blend::OrCrossfade)
//!   at alpha k/32 ([`tint`]). An opaque pod is drawn plainly, as k = 32
//!   is the same copy.
//! - **Hidden**: a pod the simulation hides (its life run out, or a scoop
//!   took it for box slot 0) is not drawn.
//!
//! The pods are drawn in their sprite layer, `_boxLayer` 0x3b53ec, after
//! the cargo boxes (`_InitSprites` @0x1fc71-0x1fd57, added @0x1fcfb), in
//! slot order: above the disabled ships and below the smoke, the shots and
//! the flying ships
//! ([`SpriteLayer::Boxes`](nova_sim::sprite_layer::SpriteLayer::Boxes)).
//!
//! A pod is murked at its position, seen from the player's ship
//! ([`murk`](super::murk); `_HandleSpriteMurkiness` @0x39f90-0x39fbc,
//! rdm:phase/play-feel-4/phase-17-murk-other-sprites), on top of its fade:
//! the murk writes the sprite's +0xb0/+0xb4 and the fade +0xa8, so the
//! two compose.

use nova_sim::escape_pod::Pod;

use super::catalog::Looks;
use super::murk::SeenMurk;
use crate::system::camera::Camera;
use crate::{Blend, Color, DrawList, Point};

/// The life, in ticks, from which a pod is drawn opaque (32.0 at 0xdd654).
pub const POD_FADE: f32 = 32.0;
/// The steps of a pod's fade, the draw proc's 32 (`>> 5`).
const FADE_STEPS: i32 = 32;

/// How `pod` is drawn by its life: white at its alpha, or none at a life
/// of 0 or less (see the module docs).
#[must_use]
pub fn tint(pod: &Pod) -> Option<Color> {
    if pod.life >= POD_FADE {
        Some(Color::WHITE)
    } else if pod.life > 0.0 {
        // The translucency `trunc(32 - life)`, in 0..=31 here
        // (@0x3a00a-0x3a024); the source's share is 32 less it.
        #[allow(clippy::cast_possible_truncation)]
        let murk = (POD_FADE - pod.life).trunc() as i32;
        let shown = FADE_STEPS - murk;
        Some(Color::rgba(
            255,
            255,
            255,
            ((shown * 255 + FADE_STEPS / 2) / FADE_STEPS) as u8,
        ))
    } else {
        None
    }
}

/// Draws each shown pod of `pods`, in their order, as `looks` says, where
/// `camera` places it, at whole pixels by its sheet's sides
/// ([`Camera::sprite_centre`], `_HandleEscapePods` @0x39ef0-0x39f7d),
/// murked as the player sees it, `seen` ([`SeenMurk::at`]), as
/// `_HandleEscapePods` murks it at its position (@0x39f90-0x39fbc) before
/// its fade. None is drawn without the pods' sheet.
pub fn draw_pods(
    list: &mut DrawList,
    camera: &Camera,
    looks: &Looks,
    seen: SeenMurk,
    pods: &[Pod],
) {
    let Some(sheet) = looks.pod_sheet() else {
        return;
    };
    for pod in pods.iter().filter(|pod| pod.shown) {
        let Some(tint) = tint(pod) else {
            continue;
        };
        let colorize = seen.at(pod.position);
        let frame = sheet.image.frame(pod.frame);
        let at = Point::new(pod.position.x, pod.position.y);
        let at = camera.sprite_centre(at, sheet.size());
        let blend = if pod.life >= POD_FADE {
            Blend::Normal
        } else {
            Blend::OrCrossfade
        };
        list.mixed_sprite(frame, at, tint, blend, Some(colorize));
    }
}

/// A pod of `life` at `at`, for the drawing tests.
#[cfg(test)]
pub(crate) fn pod_at(at: nova_sim::Vec2, life: f32) -> Pod {
    Pod {
        position: at,
        velocity: nova_sim::Vec2::ZERO,
        life,
        frame: 9,
        shown: true,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::num::NonZeroU16;

    use nova_sim::Vec2;

    use super::*;
    use crate::DrawCommand;
    use crate::ImageKey;
    use crate::flight::catalog::EffectSheet;
    use crate::system::camera::Anchor;

    /// The pod's sheet, `rlëD` 1095, of 36 rotations.
    fn looks() -> Looks {
        Looks {
            pod: Some(Ok(EffectSheet {
                image: crate::SheetImage::Rled(1095),
                frames: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 0,
                frame_height: 0,
            })),
            ..Looks::default()
        }
    }

    /// No murk to see.
    const CLEAR: SeenMurk = SeenMurk {
        viewer: Vec2::ZERO,
        murk: 0,
        colour: 0,
    };

    /// The murk at its worst as seen from `viewer`.
    fn seen_from(viewer: Vec2) -> SeenMurk {
        SeenMurk {
            viewer,
            murk: 100,
            colour: 0x1081,
        }
    }

    /// What `pods` draws, through the camera at the origin: each sprite's
    /// image, tint, blend and centre.
    fn drawn(pods: &[Pod], looks: &Looks) -> Vec<(ImageKey, Color, Blend, Point)> {
        let mut list = DrawList::new();
        draw_pods(
            &mut list,
            &Camera::centred_on(Point::default()),
            looks,
            CLEAR,
            pods,
        );
        list.into_iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite {
                    image,
                    tint,
                    blend,
                    center,
                    ..
                } => Some((*image, *tint, *blend, *center)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_pod_is_placed_at_whole_pixels_by_its_sheets_own_sides() {
        // At (103.5, 50), so (103.25, 50.5) from the camera, truncated to
        // (103, 50): (518, 434) less (15, 14) is the top left (503, 420).
        let pod = pod_at(Vec2::new(103.5, 50.0), 100.0);
        let sized = Looks {
            pod: Some(Ok(EffectSheet {
                frame_width: 31,
                frame_height: 29,
                ..*looks().pod_sheet().expect("read")
            })),
            ..Looks::default()
        };
        let camera = Camera::centred_on(Point::new(0.25, -0.5)).with_anchor(Anchor::PlayAreaCentre);
        let mut list = DrawList::new();
        draw_pods(&mut list, &camera, &sized, CLEAR, &[pod]);
        let drawn: Vec<Point> = list
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Sprite { center, .. } => Some(center),
                _ => None,
            })
            .collect();
        assert_eq!(drawn, [Point::new(518.5, 434.5)]);
    }

    #[test]
    fn a_pod_is_drawn_on_its_frame_at_its_tint() {
        let pod = pod_at(Vec2::new(10.0, 0.0), 100.0);
        let shown = drawn(&[pod], &looks());
        assert_eq!(shown.len(), 1);
        assert_eq!(
            (shown[0].0, shown[0].1),
            (ImageKey::sprite(1095, 9), Color::WHITE)
        );
        assert!(
            drawn(&[pod], &Looks::default()).is_empty(),
            "no sheet, no pod"
        );
    }

    #[test]
    fn a_hidden_pod_is_not_drawn() {
        let pod = Pod {
            shown: false,
            ..pod_at(Vec2::new(10.0, 0.0), 100.0)
        };
        assert!(drawn(&[pod], &looks()).is_empty());
    }

    #[test]
    fn the_pods_are_drawn_in_the_frames_slot_order() {
        let camera = Camera::centred_on(Point::default());
        let pods = [
            pod_at(Vec2::new(30.0, 0.0), 100.0),
            pod_at(Vec2::new(20.0, 0.0), 100.0),
        ];
        let xs: Vec<f32> = drawn(&pods, &looks())
            .into_iter()
            .map(|(.., center)| center.x)
            .collect();
        let screen = |x| camera.world_to_screen(Point::new(x, 0.0)).x;
        assert_eq!(xs, [screen(30.0), screen(20.0)]);
    }

    #[test]
    fn a_pod_is_opaque_to_32_fades_over_its_last_32_and_is_hidden_at_0() {
        let alpha = |life| tint(&pod_at(Vec2::ZERO, life)).map(|tint| tint.a);
        assert_eq!(alpha(149.0), Some(255));
        assert_eq!(alpha(32.0), Some(255), "opaque at 32");
        assert_eq!(alpha(31.0), Some(247), "31/32");
        assert_eq!(alpha(16.0), Some(128));
        assert_eq!(alpha(1.0), Some(8), "1/32");
        assert_eq!(alpha(0.0), None, "unseen at 0");
        assert_eq!(alpha(f32::NAN), None, "a NaN life is hidden");
        let fading: Vec<u8> = (1..32)
            .rev()
            .filter_map(|life| alpha(life as f32))
            .collect();
        assert!(
            fading.windows(2).all(|pair| pair[0] > pair[1]),
            "{fading:?}"
        );
        assert_eq!(fading.len(), 31);
    }

    #[test]
    fn a_fractional_life_fades_by_its_truncated_translucency() {
        // trunc(32 - 10.5) = 21, so the pod's share is 11 of 32.
        let tint_of = |life| tint(&pod_at(Vec2::ZERO, life)).expect("shown");
        assert_eq!(tint_of(10.5).a, ((11 * 255 + 16) / 32) as u8);
        assert_eq!(
            tint_of(10.0).a,
            ((10 * 255 + 16) / 32) as u8,
            "a whole life's"
        );
        assert_eq!(tint_of(31.5).a, 255, "trunc(0.5) is 0: the whole source");
    }

    #[test]
    fn a_fading_pod_is_crossfaded_by_or_and_an_opaque_one_drawn_plainly() {
        let at = Vec2::new(10.0, 0.0);
        let blends = |life| -> Vec<(Color, Blend)> {
            drawn(&[pod_at(at, life)], &looks())
                .into_iter()
                .map(|(_, tint, blend, _)| (tint, blend))
                .collect()
        };
        assert_eq!(blends(POD_FADE), [(Color::WHITE, Blend::Normal)], "at 32");
        let fading = tint(&pod_at(at, POD_FADE - 1.0)).expect("shown");
        assert_eq!(
            blends(POD_FADE - 1.0),
            [(fading, Blend::OrCrossfade)],
            "at 31"
        );
        assert_eq!(fading.a, 247, "31/32");
    }

    #[test]
    fn a_pod_far_from_the_viewer_in_murk_is_mixed_opaque_or_fading() {
        let position = Vec2::new(3000.0, 50.0);
        let mixed = |life, viewer: Vec2| -> Vec<_> {
            let mut list = DrawList::new();
            let camera = Camera::centred_on(Point::default());
            draw_pods(
                &mut list,
                &camera,
                &looks(),
                seen_from(viewer),
                &[pod_at(position, life)],
            );
            list.into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Sprite { tint, blend, .. } => Some((*tint, *blend, None)),
                    DrawCommand::Colorized {
                        tint,
                        blend,
                        colorize,
                        ..
                    } => Some((*tint, *blend, Some(*colorize))),
                    _ => None,
                })
                .collect()
        };
        let murk = seen_from(Vec2::ZERO).at(position);
        assert!(murk.level > 0, "murky out here: {}", murk.level);
        assert_eq!(
            mixed(100.0, Vec2::ZERO),
            [(Color::WHITE, Blend::Normal, Some(murk))]
        );
        assert_eq!(
            mixed(100.0, position),
            [(Color::WHITE, Blend::Normal, None)],
            "plain at the viewer"
        );
        let fading = tint(&pod_at(position, 20.0)).expect("shown");
        assert_eq!(
            mixed(20.0, Vec2::ZERO),
            [(fading, Blend::OrCrossfade, Some(murk))],
            "the mix under its fade"
        );
    }
}
