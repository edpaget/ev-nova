//! Whether this pass is a smoke frame: the frames on which a smoking shot
//! leaves a puff ([`Session::smoke_frame`]).
//!
//! Before each `_DoPlayGameWork` (@0x45bc8), `_PlayGame` sets the byte at
//! `*0x3b555c` to 1 when the frame counter `*0x3b5538` is a multiple of 3
//! (`imull $0x5556`, a signed division truncating toward zero,
//! @0x45b7d-0x45ba7), and to 0 otherwise (@0x45bb9-0x45bbe).
//! `_HandleShot`'s smoke trail spawns a puff only while that byte is set
//! (@0x35cc5-0x35ccd). The counter it reads is the one the pass runs on,
//! the session's pass frame, so double time's second pass, which runs on
//! the same counter, is a smoke frame exactly when the first is.

use super::{Session, Space};

impl Session {
    /// Whether the last advance pass ran on a smoke frame: its frame
    /// counter a multiple of 3, by a remainder truncating toward zero, so
    /// the negative counts after a take-off qualify by the same rule (see
    /// the module docs).
    #[must_use]
    pub fn smoke_frame(&self) -> bool {
        self.space.smoke_frame()
    }
}

impl Space {
    /// Whether the last advance pass ran on a smoke frame: its frame
    /// counter a multiple of 3, by a remainder truncating toward zero, so
    /// the negative counts after a take-off qualify by the same rule (see
    /// the module docs).
    #[must_use]
    fn smoke_frame(&self) -> bool {
        self.pass_frame % 3 == 0
    }
}

#[cfg(test)]
mod tests {
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight::Controls;
    use crate::session::Session;
    use crate::testkit::work;
    use crate::testkit::{FakePilotCatalog, catalog};

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog).expect("starts")
    }

    /// The smoke frame after an advance pass run on `frame`.
    fn after_pass_on(session: &mut Session, frame: i16) -> bool {
        session.space.frame = frame;
        work(session, &crate::testkit::catalog(), Controls::default());
        session.smoke_frame()
    }

    #[test]
    fn a_smoke_frame_is_a_pass_on_a_multiple_of_three() {
        let catalog = catalog();
        let mut session = session(&catalog);
        let frames: Vec<(i16, bool)> = [0, 1, 2, 3, 4, 5, 6]
            .into_iter()
            .map(|frame| (frame, after_pass_on(&mut session, frame)))
            .collect();
        assert_eq!(
            frames,
            [
                (0, true),
                (1, false),
                (2, false),
                (3, true),
                (4, false),
                (5, false),
                (6, true),
            ]
        );
    }

    #[test]
    fn the_smoke_frame_follows_the_counter_across_its_wrap_and_below_zero() {
        let catalog = catalog();
        let mut session = session(&catalog);
        session.space.frame = 1023;
        let mut seen = Vec::new();
        for _ in 0..3 {
            work(&mut session, &catalog, Controls::default());
            seen.push(session.smoke_frame());
        }
        assert_eq!(seen, [true, false, true], "1023, 1024, then 0");
        for (frame, smoke) in [(-3, true), (-2, false), (-1, false), (-15, true)] {
            assert_eq!(after_pass_on(&mut session, frame), smoke, "{frame}");
        }
    }

    #[test]
    fn double_times_second_pass_is_a_smoke_frame_when_the_first_is() {
        let catalog = catalog();
        let mut session = session(&catalog);
        for frame in [3, 4] {
            let first = after_pass_on(&mut session, frame);
            let behaviour = crate::ai::NovaAi::default();
            session.tick_again(
                Controls::default(),
                &behaviour,
                Rules::default(),
                &mut NeverFires,
            );
            assert_eq!(session.smoke_frame(), first, "{frame}");
        }
    }
}
