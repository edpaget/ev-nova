//! The screen router: every screen the app can show, as one [`Screen`].

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_view::ships::ShipBrowser;
use nova_view::{DrawList, Input, Screen, ScreenAction};

/// The screen the app is showing. Each variant is one of the game's
/// screens; the router forwards input, ticks and drawing to it.
#[derive(Clone, Debug)]
pub enum AppScreen {
    /// The ship browser, reading the game data the renderer draws from.
    ShipBrowser(ShipBrowser<Rc<GameData>>),
}

/// The screen the app opens on: the ship browser over `data`.
#[must_use]
pub fn start_screen(data: Rc<GameData>) -> AppScreen {
    AppScreen::ShipBrowser(ShipBrowser::new(data))
}

impl Screen for AppScreen {
    fn input(&mut self, input: &Input) -> ScreenAction {
        match self {
            Self::ShipBrowser(screen) => screen.input(input),
        }
    }

    fn tick(&mut self, dt: Duration) {
        match self {
            Self::ShipBrowser(screen) => screen.tick(dt),
        }
    }

    fn draw(&self, list: &mut DrawList) {
        match self {
            Self::ShipBrowser(screen) => screen.draw(list),
        }
    }
}

#[cfg(test)]
mod tests {
    fn drawn(screen: &impl super::Screen) -> super::DrawList {
        let mut list = super::DrawList::new();
        screen.draw(&mut list);
        list
    }

    mod ship_browser {
        use std::io;
        use std::path::Path;
        use std::rc::Rc;

        use nova_data::graphics::RLED;
        use nova_data::graphics::fixture::RledBuilder;
        use nova_data::records::ship::Ship;
        use nova_data::records::ship_anim::ShipAnim;
        use nova_data::store::fs::{DirLister, EntryKind, Listing};
        use nova_data::{GameData, Record};
        use nova_rsrc::fixture::ForkBuilder;
        use nova_rsrc::{Fork, ForkReader};
        use nova_view::Key;
        use nova_view::ships::{ShipBrowser, ShipId};

        use super::super::*;
        use super::drawn;

        /// One data file, `/data/Nova Data`, holding a fork.
        struct OneFile(Vec<u8>);

        impl DirLister for OneFile {
            fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
                Ok(vec![Listing {
                    name: "Nova Data".into(),
                    kind: EntryKind::File,
                }])
            }
        }

        impl ForkReader for OneFile {
            fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
                Ok((fork == Fork::Data).then(|| self.0.clone()))
            }
        }

        /// Ships 129 and 128, each with a `shän` naming a 4-frame `rlëD`
        /// (one set of 4 rotations).
        fn data() -> Rc<GameData> {
            let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
            anim[0x00..0x02].copy_from_slice(&1000_i16.to_be_bytes());
            anim[0x04..0x06].copy_from_slice(&1_i16.to_be_bytes());
            anim[0x34..0x36].copy_from_slice(&4_i16.to_be_bytes());
            let sheet = (0..4)
                .fold(RledBuilder::new(1, 1), |sheet, _| {
                    sheet.frame(|f| f.line().pixels(&[0x7C00]))
                })
                .build();
            let ship = vec![0; Ship::SIZE.expect("fixed")];
            let fork = ForkBuilder::new()
                .resource(Ship::TYPE, 129, Some(b"Second"), &ship)
                .resource(Ship::TYPE, 128, Some(b"First"), &ship)
                .resource(ShipAnim::TYPE, 128, None, &anim)
                .resource(ShipAnim::TYPE, 129, None, &anim)
                .resource(RLED, 1000, None, &sheet)
                .build()
                .bytes;
            let file = OneFile(fork);
            let data = GameData::load(&file, &file, Path::new("/data"), None).expect("opens");
            Rc::new(data)
        }

        fn selected(screen: &AppScreen) -> Option<ShipId> {
            let AppScreen::ShipBrowser(browser) = screen;
            browser.selected()
        }

        #[test]
        fn the_app_starts_on_the_ship_browser_at_the_first_ship() {
            let screen = start_screen(data());
            assert_eq!(selected(&screen), Some(ShipId(128)));
        }

        #[test]
        fn input_goes_to_the_ship_browser() {
            let mut screen = start_screen(data());
            let right = Input::Key {
                key: Key::Right,
                pressed: true,
            };
            assert_eq!(screen.input(&right), ScreenAction::None);
            assert_eq!(selected(&screen), Some(ShipId(129)));
        }

        #[test]
        fn ticks_and_drawing_go_to_the_ship_browser() {
            let mut direct = ShipBrowser::new(data());
            let mut screen = AppScreen::ShipBrowser(direct.clone());
            let unticked = drawn(&direct);
            let dt = Duration::from_millis(100);

            direct.tick(dt);
            screen.tick(dt);

            assert_ne!(drawn(&direct), unticked, "the tick turns the ship");
            assert_eq!(drawn(&screen), drawn(&direct));
        }
    }
}
