//! The take-off's reset of every stellar's frames, and the exit gate
//! opened, as `_HandlePlayerDockRequest` does (play-feel-4 phase 14).
//!
//! Every landing, hypergate exit and wormhole exit runs through the dock
//! request's tail (@0x66f40-0x675ce), which, for all 2048 `spöb`
//! records, sets the current and next frames (+0x476, +0x478) to 0 and
//! leaves the timer (+0x490) as it was (@0x675ce-0x675fa). Out of a
//! hypergate (the byte at ebp-0x2a3, set @0x6712a from the exit stellar
//! at ebp-0x298, @0x66ee9), the exit gate then takes its counted count -
//! 1 as current and next, and -`AnimDelay` as its timer
//! (@0x675fc-0x67646). A hypergate entered with no exit skips both
//! (@0x66ef2-0x66f08); a wormhole that leads nowhere runs the reset and
//! opens no gate (see the `wormhole_no_exit` module). Then the stellars are rebuilt (@0x67653), each
//! slot showing its current frame: frame 0.
//!
//! The session resets the gates it keeps ([`Space::reset_gate_frames`];
//! see the `gate_sprites` module) and raises
//! [`SessionEvent::StellarFramesReset`], from which the view resets every
//! other stellar's frames, kept or shown. The view settles it in the
//! catch-up that follows the take-off or the exit, before the first frame
//! is drawn, so the original's order (the reset before the rebuild) needs
//! nothing more. The reset plays no sound.

use super::{Session, SessionEvent, Space};
use crate::catalog::StellarId;
use crate::gate_sprite::{GateAnimation, GateSprite};

impl Session {
    /// The take-off's reset (see the module docs): every gate's frames to
    /// 0, the hypergate `exit` (the player's, when it came out of one)
    /// opened at its last frame, and [`SessionEvent::StellarFramesReset`]
    /// told.
    pub(super) fn reset_stellar_frames(&mut self, exit: Option<StellarId>) {
        let exit = exit.and_then(|id| {
            let site = self.chart.gate_site(id)?;
            Some((id, GateAnimation::of(site)))
        });
        self.space.reset_gate_frames(exit);
        self.out.emit(SessionEvent::StellarFramesReset);
    }
}

impl Space {
    /// Every gate sprite kept, shown or not, takes the take-off's reset
    /// ([`GateSprite::take_off_reset`]); one first shown from now on
    /// starts as the reset leaves it ([`GateSprite::after_reset`]). Then
    /// `exit`, the hypergate the player came out of with its animation,
    /// opens at its last frame ([`GateSprite::exit_gate`]).
    pub(super) fn reset_gate_frames(&mut self, exit: Option<(StellarId, GateAnimation)>) {
        for sprite in self.gate_sprites.values_mut() {
            sprite.take_off_reset();
        }
        self.gate_sprite_afresh = GateSprite::after_reset();
        if let Some((id, anim)) = exit {
            self.gate_sprites.insert(id, GateSprite::exit_gate(&anim));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{LandingSite, SiteAnimation, SystemId};
    use crate::chance::NeverFires;
    use crate::gate::{GateRefusal, HYPERGATE};
    use crate::session::tests::{at_gate, at_wormhole, gate_site, gated, wormholes};
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, land_now, planet};

    /// An animation of 6 frames stepping every 3 ticks.
    const SIX_EVERY_3: SiteAnimation = SiteAnimation {
        delay: 3,
        frame0_bias: 0,
        open_frame: 0,
        type_frames: 6,
        dead_frames: 0,
    };

    /// [`gated`], with hypergate 310, 131's exit, listed in 131 with
    /// [`SIX_EVERY_3`].
    fn gated_with_exit() -> FakePilotCatalog {
        let mut catalog = gated();
        catalog.sites[1].1.push(LandingSite {
            flags2: HYPERGATE,
            animation: SIX_EVERY_3,
            ..planet(310, 100.0, 200.0)
        });
        catalog
    }

    /// A sprite of [`SIX_EVERY_3`] held open 7 ticks: frame 2, its timer
    /// at 1.
    fn stepped() -> GateSprite {
        let anim = GateAnimation {
            delay: 3,
            frames: 6,
            ..GateAnimation::default()
        };
        let mut sprite = GateSprite::new();
        for _ in 0..7 {
            sprite.step(&anim, false, true, &mut NeverFires);
        }
        sprite
    }

    /// `sprite` after the take-off's reset.
    fn reset(mut sprite: GateSprite) -> GateSprite {
        sprite.take_off_reset();
        sprite
    }

    /// Gate 900, not shown, kept as [`stepped`] leaves it.
    const KEPT: StellarId = StellarId(900);

    fn keep_stepped(session: &mut Session) {
        session.space.gate_sprites.insert(KEPT, stepped());
    }

    fn resets_told(session: &mut Session) -> usize {
        session
            .take_events()
            .iter()
            .filter(|event| **event == SessionEvent::StellarFramesReset)
            .count()
    }

    #[test]
    fn a_hypergate_exit_opens_the_exit_gate_at_its_last_frame_and_resets_the_others() {
        let catalog = gated_with_exit();
        let mut session = at_gate(&catalog);
        keep_stepped(&mut session);
        session.take_events();
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
        let exit = StellarId(310);
        let site = *session.chart.gate_site(exit).expect("shown");
        // Then the tail's two stellar passes (see the `stellar_passes`
        // module).
        let anim = GateAnimation::of(&site);
        let mut opened = GateSprite::exit_gate(&anim);
        opened.step(&anim, false, false, &mut NeverFires);
        opened.step(&anim, false, false, &mut NeverFires);
        assert_eq!(session.space.gate_sprites.get(&exit), Some(&opened));
        assert_eq!(session.gate_frame(exit), Some(5), "frames - 1");
        assert_eq!(session.space.gate_sprites[&KEPT], reset(stepped()));
        assert_eq!(resets_told(&mut session), 1);
    }

    #[test]
    fn a_wormhole_exit_resets_every_gate_and_opens_none() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        keep_stepped(&mut session);
        session.take_events();
        session
            .enter_wormhole(&catalog, &mut Scripted::rolling(&[0]))
            .expect("enters");
        assert_eq!(session.space.gate_sprites[&KEPT], reset(stepped()));
        assert!(
            session
                .space
                .gate_sprites
                .values()
                .all(|sprite| *sprite == reset(*sprite)),
            "none opened: {:?}",
            session.space.gate_sprites
        );
        assert_eq!(resets_told(&mut session), 1);
    }

    #[test]
    fn a_wormhole_with_no_exit_resets_every_gate_and_opens_none() {
        // Its -1 runs the tail with ebp-0x2a3 clear (@0x67122-0x6714d):
        // the reset (@0x675ce-0x675fa) and no exit gate.
        let mut catalog = wormholes();
        catalog.gates.retain(|site| site.id == StellarId(400));
        let mut session = at_wormhole(&catalog);
        keep_stepped(&mut session);
        session.take_events();
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NoExit)
        );
        assert_eq!(session.space.gate_sprites[&KEPT], reset(stepped()));
        assert!(
            session
                .space
                .gate_sprites
                .values()
                .all(|sprite| *sprite == reset(*sprite)),
            "none opened: {:?}",
            session.space.gate_sprites
        );
        assert_eq!(resets_told(&mut session), 1);
    }

    #[test]
    fn a_take_off_resets_every_gate_and_opens_none() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        keep_stepped(&mut session);
        session.take_events();
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.space.gate_sprites[&KEPT], reset(stepped()));
        assert_eq!(resets_told(&mut session), 1);
    }

    #[test]
    fn a_refused_hypergate_resets_nothing_and_tells_nothing() {
        let no_links = FakePilotCatalog {
            gates: vec![gate_site(300, (0.0, 0.0), HYPERGATE, &[0, -2], 0)],
            ..gated_with_exit()
        };
        for (catalog, to, refusal) in [
            (no_links, Some(SystemId(131)), GateRefusal::NoLinks),
            (gated_with_exit(), None, GateRefusal::Cancelled),
        ] {
            let mut session = at_gate(&catalog);
            keep_stepped(&mut session);
            session.take_events();
            assert_eq!(
                session.enter_hypergate(to, &catalog, &mut NeverFires),
                Err(refusal)
            );
            assert_eq!(session.space.gate_sprites[&KEPT], stepped(), "{refusal:?}");
            assert_eq!(resets_told(&mut session), 0, "{refusal:?}");
        }
    }

    #[test]
    fn a_gate_first_shown_after_a_reset_starts_as_the_reset_leaves_it() {
        let mut space = Space::new();
        space.keep_gate_sprites([StellarId(300)]);
        assert_eq!(space.gate_sprites[&StellarId(300)], GateSprite::new());
        space.reset_gate_frames(None);
        space.keep_gate_sprites([StellarId(300), StellarId(310)]);
        assert_eq!(
            space.gate_sprites[&StellarId(300)],
            GateSprite::after_reset()
        );
        assert_eq!(
            space.gate_sprites[&StellarId(310)],
            GateSprite::after_reset()
        );
    }

    #[test]
    fn the_exit_gate_is_opened_after_the_reset_whether_kept_before_or_not() {
        let anim = GateAnimation {
            delay: 3,
            frames: 6,
            ..GateAnimation::default()
        };
        for kept in [false, true] {
            let mut space = Space::new();
            if kept {
                space.gate_sprites.insert(StellarId(310), stepped());
            }
            space.reset_gate_frames(Some((StellarId(310), anim)));
            assert_eq!(
                space.gate_sprites[&StellarId(310)],
                GateSprite::exit_gate(&anim),
                "{kept}"
            );
        }
    }
}
