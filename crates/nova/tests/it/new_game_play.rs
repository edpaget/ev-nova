//! A new-game folder played headless, shared by `new_game_missions` (the
//! checked-in fixture) and `sample_missions` (the sample's cut): the
//! folder opened as the app opens it (`DiskData::open_new_game`, which
//! checks every knot against the Ink runtime), with
//! `nova::dialogs::loader` wired as `main` wires it and the new game's
//! rulebook, and a session driven through its commands.

use std::path::Path;

use nova::chance::SplitMix;
use nova::startup::{DataSource, DiskData, NewGameOpened};
use nova_data::GameData;
use nova_data::new_game::{self, NewGame};
use nova_sim::flight_frame::{Answer, Converse, FlightCommand};
use nova_sim::gate::GateKind;
use nova_sim::storylines::state::MissionState;
use nova_sim::{
    ApplyCollaborators, HailOptions, LandPress, NovaBoarding, NovaLaw, Pilot, Session, StellarId,
    SystemId,
};

use crate::new_game_travel::{gates, new_game_rules, nova_of, pass as travel_pass, pilot_for};

/// The new-game folder `folder` as the app opens it, its knots checked,
/// with no problems, and its ids.
pub(crate) fn opened(folder: &Path) -> (GameData, NewGame) {
    let NewGameOpened::Loaded(loaded) = DiskData.open_new_game(folder) else {
        panic!("{} opens", folder.display());
    };
    assert_eq!(loaded.problems, Vec::<String>::new());
    (
        loaded.data,
        new_game::open(folder).expect("the folder lists"),
    )
}

/// A mission game: the data, its ids and a session flying a pilot.
pub(crate) struct Play {
    pub data: GameData,
    pub game: NewGame,
    pub session: Session,
    pub chance: SplitMix,
}

impl Play {
    /// `folder`'s start pilot, changed by `edit` and put through a save,
    /// flying in system `system`.
    pub(crate) fn new(folder: &Path, system: &str, edit: impl FnOnce(&mut Pilot)) -> Self {
        let (data, game) = opened(folder);
        let mut pilot = pilot_for(&game);
        pilot.system = SystemId(nova_of(&game.ids.systems, system));
        pilot.stellar = None;
        edit(&mut pilot);
        let pilot = nova_sim::save::decode(&nova_sim::save::encode(&pilot)).expect("loads");
        let session = Session::load(&data, pilot, &new_game_rules())
            .expect("flies")
            .with_dialog_loader(nova::dialogs::loader(1));
        Self {
            data,
            game,
            session,
            chance: SplitMix::new(3),
        }
    }

    pub(crate) fn stellar(&self, id: &str) -> StellarId {
        StellarId(nova_of(&self.game.ids.stellars, id))
    }

    pub(crate) fn system(&self, id: &str) -> SystemId {
        SystemId(nova_of(&self.game.ids.systems, id))
    }

    /// The gate of the jump point `id` (`system/point`).
    pub(crate) fn gate(&self, id: &str) -> StellarId {
        StellarId(nova_of(&self.game.ids.gates, id))
    }

    /// Goes through the jump point `id`'s gate (`system/point`) as the
    /// player does: taken off if landed, put over the gate, L pressed (the
    /// request cleared), a pass of the game's work (the approach), L again
    /// (the entry), the one system the hypergate map offers picked and
    /// entered. The system the ship comes out in.
    pub(crate) fn go_through(&mut self, id: &str) -> SystemId {
        if self.session.landed().is_some() {
            self.take_off();
        }
        let stellar = self.gate(id);
        let gate = gates(&self.game)
            .into_iter()
            .find(|gate| gate.stellar == stellar)
            .expect("a gate");
        assert_eq!(
            self.session.pilot().system,
            gate.system,
            "{id}: in its system"
        );
        nova_sim::fixture::put_player_at(&mut self.session, gate.at);
        assert!(self.session.land().is_ok(), "{id}: cleared");
        travel_pass(
            &mut self.session,
            &self.data,
            &new_game_rules(),
            &mut self.chance,
        );
        assert_eq!(
            self.session.land(),
            Ok(LandPress::AtGate {
                stellar,
                kind: GateKind::Hypergate,
            }),
            "{id}"
        );
        let to = gate.partner.1;
        assert_eq!(
            self.session.open_hypergate(&self.data),
            Ok(vec![to]),
            "{id}"
        );
        let entered = self
            .session
            .enter_hypergate(Some(to), &self.data, &mut self.chance);
        assert_eq!(entered, Ok(to), "{id}");
        to
    }

    /// Lands at the stellar `id`, in the system the pilot is in, and
    /// settles the landing's story.
    pub(crate) fn land(&mut self, id: &str) {
        let stellar = self.stellar(id);
        let system = self.session.pilot().system;
        self.session
            .land_at(&self.data, system, stellar)
            .expect("lands");
        self.settle();
        assert_eq!(self.session.landed(), Some(stellar));
    }

    /// Takes off, then lands at the stellar `id` again: a new visit.
    pub(crate) fn visit_again(&mut self, id: &str) {
        self.take_off();
        self.land(id);
    }

    /// Takes off and settles the take-off.
    pub(crate) fn take_off(&mut self) {
        assert!(
            self.session
                .take_off(&self.data, &mut self.chance)
                .is_some()
        );
        self.settle();
    }

    /// Lets `days` days go by and settles them.
    pub(crate) fn pass(&mut self, days: u32) {
        self.session.let_days_pass(days, &mut self.chance);
        self.settle();
    }

    pub(crate) fn settle(&mut self) {
        let _ = self.session.settle_script(&self.data, &mut self.chance);
    }

    pub(crate) fn converse(&mut self, converse: Converse) -> Answer {
        self.command(FlightCommand::Converse(converse))
    }

    /// Sends the player's `command`, as the view sends it, under Nova's
    /// law and boarding rules: what it answers.
    pub(crate) fn command(&mut self, command: FlightCommand) -> Answer {
        self.session.apply(
            command,
            &mut ApplyCollaborators {
                catalog: Some(&self.data),
                chance: &mut self.chance,
                law: &NovaLaw::default(),
                boarding_rule: &NovaBoarding::default(),
                hail_options: &HailOptions::default(),
            },
        )
    }

    /// Opens the visit's next conversation: its lines and choices.
    pub(crate) fn open(&mut self) -> Vec<String> {
        page(&self.converse(Converse::Open))
    }

    /// Takes the choice at `index`: the next page's lines and choices.
    pub(crate) fn choose(&mut self, index: usize) -> Vec<String> {
        page(&self.converse(Converse::Choose(index)))
    }

    pub(crate) fn close(&mut self) {
        assert_eq!(self.converse(Converse::Close), Answer::Done);
    }

    pub(crate) fn state(&self, id: &str) -> Option<MissionState> {
        self.session.pilot().story.mission(id)
    }

    pub(crate) fn flag(&self, name: &str) -> bool {
        self.session.pilot().flags.is_set(name)
    }

    /// What system `id` has heard, as (news id, age).
    pub(crate) fn heard(&self, id: &str) -> Vec<(String, u32)> {
        self.session
            .heard_news(self.system(id))
            .into_iter()
            .map(|news| (news.id, news.age))
            .collect()
    }
}

/// The page `answer` shows: its lines, then its choices.
pub(crate) fn page(answer: &Answer) -> Vec<String> {
    let Answer::Conversation(page) = answer else {
        panic!("not a page: {answer:?}");
    };
    assert_eq!(page.problem, None);
    page.lines.iter().chain(&page.choices).cloned().collect()
}

pub(crate) fn texts(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|&line| line.to_owned()).collect()
}
