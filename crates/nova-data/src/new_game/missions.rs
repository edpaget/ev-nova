//! The new game's missions, news and jump points, mapped onto the store's
//! [`Storylines`].
//!
//! A mission keeps its format ids as text (missions, news, flags,
//! factions, `system/point` refs), and takes Nova IDs only where the
//! engine places something or acts on it: the stellar an offer, a
//! completion or a delivery is at, and the `oütf` of each outfit an
//! effect grants or removes. Its `offer` and `complete` dialogs become
//! [`DialogPlace`](crate::store::DialogPlace)s, resolved against the
//! compiled texts ([`Compiled`]). A news item keeps its headline and body,
//! and its `from` system's `sÿst`. Every jump point that has a gate is
//! kept with its gate, owner and stability.
//!
//! A mission that names a stellar, mission, news item, jump point,
//! faction or outfit that isn't there is left out, with one problem against its file
//! for each such name; so is one whose offer or completion dialog has no
//! compiled file. A news item whose `from` isn't a system is left out the
//! same way. A name that failed to load, a record left out for its ID, or
//! a news item left out isn't reported again, but still leaves the
//! mission out. A mission whose `requires` names a mission that was left
//! out (here, or later by [`NewGame::check_dialogs`]) is left out too,
//! with a problem against its own file naming the one it needs, and so on
//! down the chain ([`drop_dependents`]). A mission's `storyline`, `act`
//! and `giver` aren't read yet.
//!
//! [`NewGame::check_dialogs`]: super::NewGame::check_dialogs

use std::collections::BTreeMap;

use nova_galaxy_format::id::{DialogRef, Id, PointRef};
use nova_galaxy_format::load::{Galaxy, Problem};
use nova_galaxy_format::model::{Conditions, Effects, Mission, Objective};

use super::dialogs::Compiled;
use super::ids::IdTable;
use super::map::absent;
use crate::store::{
    MissionCompletion, MissionObjective, MissionOffer, NewsItem, PointAt, PointState, Requires,
    StoryEffects, StoryMission, Storylines,
};
use crate::wire::id::{GovtId, OutfitId, StellarId, SystemId};

/// Maps `galaxy`'s missions, news and jump points, with `ids`' Nova IDs and
/// `compiled`'s stories, reporting what is left out to `problems`.
#[must_use]
pub fn map(
    galaxy: &Galaxy,
    ids: &IdTable,
    compiled: &Compiled,
    problems: &mut Vec<Problem>,
) -> Storylines {
    let points = points(galaxy, ids);
    let mut news = BTreeMap::new();
    for (id, doc) in &galaxy.news {
        let from = &doc.data.from;
        let Some(&system) = ids.systems.get(from) else {
            if absent(galaxy, &galaxy.systems, "systems", from) {
                problems.push(Problem::new(
                    &doc.path,
                    format!("its `from` system `{from}` is not in systems/; it is left out"),
                ));
            }
            continue;
        };
        news.insert(
            id.to_string(),
            NewsItem {
                headline: doc.data.headline.clone(),
                text: doc.body.trim().to_owned(),
                from: SystemId(system),
            },
        );
    }
    let mut missions = BTreeMap::new();
    for (id, doc) in &galaxy.missions {
        let mut names = Names {
            galaxy,
            ids,
            compiled,
            news: &news,
            broken: Vec::new(),
            left_out: false,
        };
        let mission = names.mission(&doc.data);
        problems.extend(
            names
                .broken
                .into_iter()
                .map(|why| Problem::new(&doc.path, why)),
        );
        if !names.left_out {
            missions.insert(id.to_string(), mission);
        }
    }
    drop_dependents(&mut missions, problems);
    let factions = ids
        .factions
        .iter()
        .map(|(id, &nova)| (id.to_string(), GovtId(nova)))
        .collect();
    Storylines {
        missions,
        news,
        points,
        factions,
    }
}

/// Leaves out of `missions` each one whose `requires` (nested groups and
/// all) names a mission that is in the galaxy but not in `missions`,
/// because it was itself left out, until none does. Each name is one
/// problem against the dependent's file.
///
/// The galaxy's own check ([`Names::mission_named`]) has already left out
/// a mission naming one that isn't there at all, so every name missing
/// here was left out with its own problem.
pub(super) fn drop_dependents(
    missions: &mut BTreeMap<String, StoryMission>,
    problems: &mut Vec<Problem>,
) {
    loop {
        let mut dropped = Vec::new();
        for (id, mission) in missions.iter() {
            let mut gone = Vec::new();
            missing_needs("requires", &mission.requires, missions, &mut gone);
            if !gone.is_empty() {
                dropped.push(id.clone());
            }
            problems.extend(gone.into_iter().map(|(field, name)| {
                Problem::new(
                    format!("missions/{id}.md"),
                    format!(
                        "{field} names the mission `{name}`, which was left out; it is left out"
                    ),
                )
            }));
        }
        if dropped.is_empty() {
            return;
        }
        for id in dropped {
            missions.remove(&id);
        }
    }
}

/// Each mission `requires`, at `field`, names in `missions_done` (nested
/// groups and all) that isn't in `missions`, with the field naming it,
/// pushed onto `gone`.
fn missing_needs(
    field: &str,
    requires: &Requires,
    missions: &BTreeMap<String, StoryMission>,
    gone: &mut Vec<(String, String)>,
) {
    for name in &requires.missions_done {
        if !missions.contains_key(name) {
            gone.push((format!("{field}.missions_done"), name.clone()));
        }
    }
    for (group, list) in [
        ("all", &requires.all),
        ("any", &requires.any),
        ("none", &requires.none),
    ] {
        for (i, nested) in list.iter().enumerate() {
            missing_needs(&format!("{field}.{group}[{i}]"), nested, missions, gone);
        }
    }
}

/// Every jump point with a gate, by `system/point`, with its static owner
/// and stability.
fn points(galaxy: &Galaxy, ids: &IdTable) -> BTreeMap<String, PointAt> {
    ids.gates
        .iter()
        .filter_map(|(point, &gate)| {
            let found = galaxy.jump_point(point)?;
            Some((
                point.to_string(),
                PointAt {
                    gate: StellarId(gate),
                    owner: found.owner.as_ref().map(ToString::to_string),
                    stability: found.stability,
                },
            ))
        })
        .collect()
}

/// One mission's names, resolved: each that isn't there is a problem in
/// `broken`, and any that can't be resolved leaves the mission out.
struct Names<'a> {
    galaxy: &'a Galaxy,
    ids: &'a IdTable,
    compiled: &'a Compiled,
    /// The news items mapped, which a mission's releases must name.
    news: &'a BTreeMap<String, NewsItem>,
    broken: Vec<String>,
    left_out: bool,
}

impl Names<'_> {
    /// `mission` mapped. Its names that don't resolve set
    /// [`left_out`](Self::left_out), and what it then holds doesn't
    /// matter.
    fn mission(&mut self, mission: &Mission) -> StoryMission {
        let offer = mission.offer.as_ref().and_then(|offer| {
            let at = self.stellar("offer.at", &offer.at);
            let dialog = self.dialog("offer", &offer.dialog);
            Some(MissionOffer {
                at: at?,
                dialog: dialog?,
            })
        });
        let complete = mission.complete.as_ref().and_then(|complete| {
            let at = complete
                .at
                .as_ref()
                .map(|at| self.stellar("complete.at", at));
            let dialog = self.dialog("complete", &complete.dialog);
            Some(MissionCompletion {
                at: at.map_or(Some(None), |at| at.map(Some))?,
                dialog: dialog?,
            })
        });
        let objective = mission
            .objective
            .as_ref()
            .and_then(|objective| match objective {
                Objective::Deliver { cargo, tons, to } => Some(MissionObjective::Deliver {
                    cargo: cargo.clone(),
                    tons: *tons,
                    to: self.stellar("objective.deliver.to", to)?,
                }),
                Objective::TakePoint { point, held_by } => {
                    self.point("objective.take_point.point", point);
                    self.faction("objective.take_point.held_by", held_by);
                    Some(MissionObjective::TakePoint {
                        point: point.to_string(),
                        held_by: held_by.to_string(),
                    })
                }
            });
        StoryMission {
            title: mission.title.clone(),
            requires: self.requires("requires", &mission.requires),
            offer,
            objective,
            complete,
            on_offer: self.effects("on_offer", &mission.on_offer),
            on_accept: self.effects("on_accept", &mission.on_accept),
            on_decline: self.effects("on_decline", &mission.on_decline),
            on_success: self.effects("on_success", &mission.on_success),
        }
    }

    /// The conditions `conditions`, at `field`, nested groups and all.
    fn requires(&mut self, field: &str, conditions: &Conditions) -> Requires {
        for mission in &conditions.missions_done {
            self.mission_named(&format!("{field}.missions_done"), mission);
        }
        let mut groups = |name: &str, list: &[Conditions]| -> Vec<Requires> {
            list.iter()
                .enumerate()
                .map(|(i, group)| self.requires(&format!("{field}.{name}[{i}]"), group))
                .collect()
        };
        let all = groups("all", &conditions.all);
        let any = groups("any", &conditions.any);
        let none = groups("none", &conditions.none);
        Requires {
            missions_done: texts(&conditions.missions_done),
            flags: texts(&conditions.flags),
            flags_unset: texts(&conditions.flags_unset),
            points: self.point_states(&format!("{field}.points"), &conditions.points),
            not_before_day: conditions.not_before_day,
            all,
            any,
            none,
        }
    }

    /// The effects `effects`, at `field`.
    fn effects(&mut self, field: &str, effects: &Effects) -> StoryEffects {
        for news in &effects.news {
            self.news_named(&format!("{field}.news"), news);
        }
        StoryEffects {
            flags: texts(&effects.flags),
            clear_flags: texts(&effects.clear_flags),
            points: self.point_states(&format!("{field}.points"), &effects.points),
            news: texts(&effects.news),
            grant_outfits: self.outfits(&format!("{field}.grant_outfits"), &effects.grant_outfits),
            remove_outfits: self
                .outfits(&format!("{field}.remove_outfits"), &effects.remove_outfits),
        }
    }

    /// The `oütf`s of the outfits `ids`, named at `field`: one that isn't
    /// there is reported, and one that failed to load or was left out for
    /// its ID leaves the mission out unreported.
    fn outfits(&mut self, field: &str, ids: &[Id]) -> Vec<OutfitId> {
        ids.iter()
            .filter_map(|id| {
                if let Some(&nova) = self.ids.outfits.get(id) {
                    return Some(OutfitId(nova));
                }
                if absent(self.galaxy, &self.galaxy.outfits, "outfits", id) {
                    self.missing(field, id.as_str(), "outfit", "outfits");
                }
                self.left_out = true;
                None
            })
            .collect()
    }

    /// The point states `states`, at `field`, each point and owner
    /// checked.
    fn point_states(
        &mut self,
        field: &str,
        states: &[nova_galaxy_format::model::PointState],
    ) -> Vec<PointState> {
        states
            .iter()
            .map(|state| {
                self.point(&format!("{field}.point"), &state.point);
                if let Some(owner) = &state.owner {
                    self.faction(&format!("{field}.owner"), owner);
                }
                PointState {
                    point: state.point.to_string(),
                    owner: state.owner.as_ref().map(ToString::to_string),
                    stability: state.stability,
                }
            })
            .collect()
    }

    /// The `spöb` of the stellar `id`, named at `field`.
    fn stellar(&mut self, field: &str, id: &Id) -> Option<StellarId> {
        if let Some(&nova) = self.ids.stellars.get(id) {
            return Some(StellarId(nova));
        }
        if absent(self.galaxy, &self.galaxy.stellars, "stellars", id) {
            self.missing(field, id.as_str(), "stellar", "stellars");
        }
        self.left_out = true;
        None
    }

    /// Checks the mission `id`, named at `field`, is there.
    fn mission_named(&mut self, field: &str, id: &Id) {
        if absent(self.galaxy, &self.galaxy.missions, "missions", id) {
            self.missing(field, id.as_str(), "mission", "missions");
        } else if !self.galaxy.missions.contains_key(id) {
            self.left_out = true;
        }
    }

    /// Checks the news item `id`, named at `field`, was mapped: one that
    /// isn't there is reported, and one that failed to load or was left
    /// out (its own problem said why) leaves the mission out unreported.
    fn news_named(&mut self, field: &str, id: &Id) {
        if absent(self.galaxy, &self.galaxy.news, "news", id) {
            self.missing(field, id.as_str(), "news", "news");
        } else if !self.news.contains_key(id.as_str()) {
            self.left_out = true;
        }
    }

    /// Checks the faction `id`, named at `field`, is there.
    fn faction(&mut self, field: &str, id: &Id) {
        if absent(self.galaxy, &self.galaxy.factions, "factions", id) {
            self.missing(field, id.as_str(), "faction", "factions");
        } else if !self.galaxy.factions.contains_key(id) {
            self.left_out = true;
        }
    }

    /// Checks the jump point `point`, named at `field`, has a gate.
    fn point(&mut self, field: &str, point: &PointRef) {
        if self.ids.gates.contains_key(point) {
            return;
        }
        if self.galaxy.jump_point(point).is_none()
            && !self.galaxy.is_unreadable("systems", &point.system())
        {
            self.broken.push(format!(
                "{field} names `{point}`, which is no jump point; it is left out"
            ));
        }
        self.left_out = true;
    }

    /// The place `dialog` enters, for the mission's `which` (`offer` or
    /// `complete`).
    fn dialog(&mut self, which: &str, dialog: &DialogRef) -> Option<crate::store::DialogPlace> {
        match self.compiled.place(dialog) {
            Ok(place) => Some(place),
            Err(why) => {
                self.broken.push(format!("{which} dialog {dialog}: {why}"));
                self.left_out = true;
                None
            }
        }
    }

    /// Reports `id`, a `what` named at `field`, as not in `dir/`.
    fn missing(&mut self, field: &str, id: &str, what: &str, dir: &str) {
        self.broken.push(format!(
            "{field} names the {what} `{id}`, which is not in {dir}/; it is left out"
        ));
        self.left_out = true;
    }
}

/// Each of `ids` as text.
fn texts<T: ToString>(ids: &[T]) -> Vec<String> {
    ids.iter().map(ToString::to_string).collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::new_game::tests::galaxy;
    use crate::new_game::{NewGame, load};
    use crate::store::{DialogPlace, GateStability};

    // `galaxy()`: the format's test galaxy, whose mission `parcel` is
    // offered at `alpha-yard` by `dialog/errands.ink#parcel`, delivers a
    // ton of parcel to `alpha-yard` and releases `parcel-delivered`, from
    // `alpha`, on success; with its story compiled.

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("sample"))
    }

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    fn stellar(game: &NewGame, id: &str) -> StellarId {
        StellarId(game.ids.stellars[&Id::try_from(id.to_owned()).expect("an id")])
    }

    fn place(file: &str, knot: &str) -> DialogPlace {
        DialogPlace {
            file: file.to_owned(),
            knot: knot.to_owned(),
        }
    }

    const HEAD: &str = "---\ntitle: Toll\nstoryline: errands\nact: crossroads\n";

    /// The galaxy with `missions/toll.md`: [`HEAD`], then `yaml`.
    fn with_toll(yaml: &str) -> MemSource {
        galaxy().with("missions/toll.md", &format!("{HEAD}{yaml}---\n"))
    }

    fn toll(game: &NewGame) -> Option<&StoryMission> {
        game.data.storylines().missions.get("toll")
    }

    #[test]
    fn a_mission_maps_its_offer_objective_and_effects() {
        let game = loaded(&galaxy());
        assert_eq!(problems(&game), Vec::<String>::new());
        let yard = stellar(&game, "alpha-yard");
        let parcel = &game.data.storylines().missions["parcel"];
        assert_eq!(
            parcel,
            &StoryMission {
                title: "Parcel".to_owned(),
                offer: Some(MissionOffer {
                    at: yard,
                    dialog: place("dialog/errands.ink", "parcel"),
                }),
                objective: Some(MissionObjective::Deliver {
                    cargo: "Parcel".to_owned(),
                    tons: 1,
                    to: yard,
                }),
                on_success: StoryEffects {
                    flags: vec!["parcel_done".to_owned()],
                    news: vec!["parcel-delivered".to_owned()],
                    ..StoryEffects::default()
                },
                ..StoryMission::default()
            }
        );
    }

    #[test]
    fn a_news_item_keeps_its_headline_body_and_from_system() {
        let game = loaded(&galaxy());
        let news = &game.data.storylines().news;
        assert_eq!(
            news.keys().map(String::as_str).collect::<Vec<_>>(),
            ["parcel-delivered"]
        );
        assert_eq!(
            news["parcel-delivered"],
            NewsItem {
                headline: "Parcel delivered".to_owned(),
                text: "The yard has its parcel.".to_owned(),
                from: SystemId(game.ids.systems[&Id::try_from("alpha".to_owned()).expect("id")]),
            }
        );
    }

    #[test]
    fn every_jump_point_is_kept_with_its_gate_owner_and_stability() {
        let game = loaded(&galaxy());
        let gate_of = |text: &str| {
            let point = PointRef::try_from(text.to_owned()).expect("a point");
            StellarId(game.ids.gates[&point])
        };
        assert_eq!(
            game.data.storylines().points,
            BTreeMap::from([
                (
                    "alpha/to-beta".to_owned(),
                    PointAt {
                        gate: gate_of("alpha/to-beta"),
                        owner: None,
                        stability: GateStability::Stable,
                    }
                ),
                (
                    "beta/to-alpha".to_owned(),
                    PointAt {
                        gate: gate_of("beta/to-alpha"),
                        owner: Some("league".to_owned()),
                        stability: GateStability::Unstable,
                    }
                ),
            ])
        );
    }

    #[test]
    fn requires_keep_every_field_and_nested_groups_their_shape() {
        let game = loaded(&with_toll(
            "requires:\n  missions_done: [parcel]\n  flags: [a]\n  flags_unset: [b]\n\
             \x20 points: [{point: beta/to-alpha, owner: league, stability: dead}]\n\
             \x20 not_before_day: 9\n\
             \x20 all:\n    - any: [{flags: [c]}, {none: [{flags: [d]}]}]\n\
             \x20 none:\n    - points: [{point: alpha/to-beta, stability: unstable}]\n",
        ));
        assert_eq!(problems(&game), Vec::<String>::new());
        let flags = |names: &[&str]| names.iter().map(|&n| n.to_owned()).collect::<Vec<_>>();
        assert_eq!(
            toll(&game).expect("mapped").requires,
            Requires {
                missions_done: flags(&["parcel"]),
                flags: flags(&["a"]),
                flags_unset: flags(&["b"]),
                points: vec![PointState {
                    point: "beta/to-alpha".to_owned(),
                    owner: Some("league".to_owned()),
                    stability: Some(GateStability::Dead),
                }],
                not_before_day: Some(9),
                all: vec![Requires {
                    any: vec![
                        Requires {
                            flags: flags(&["c"]),
                            ..Requires::default()
                        },
                        Requires {
                            none: vec![Requires {
                                flags: flags(&["d"]),
                                ..Requires::default()
                            }],
                            ..Requires::default()
                        },
                    ],
                    ..Requires::default()
                }],
                any: Vec::new(),
                none: vec![Requires {
                    points: vec![PointState {
                        point: "alpha/to-beta".to_owned(),
                        owner: None,
                        stability: Some(GateStability::Unstable),
                    }],
                    ..Requires::default()
                }],
            }
        );
    }

    #[test]
    fn each_effect_block_maps_its_flags_points_and_news() {
        let game = loaded(&with_toll(
            "on_offer: {flags: [a]}\non_accept: {clear_flags: [b]}\n\
             on_decline: {points: [{point: alpha/to-beta, owner: league}]}\n\
             on_success: {news: [parcel-delivered]}\n",
        ));
        assert_eq!(problems(&game), Vec::<String>::new());
        let toll = toll(&game).expect("mapped");
        assert_eq!(toll.on_offer.flags, ["a"]);
        assert_eq!(toll.on_accept.clear_flags, ["b"]);
        assert_eq!(
            toll.on_decline.points,
            [PointState {
                point: "alpha/to-beta".to_owned(),
                owner: Some("league".to_owned()),
                stability: None,
            }]
        );
        assert_eq!(toll.on_success.news, ["parcel-delivered"]);
        assert_eq!(toll.offer, None, "an event");
    }

    #[test]
    fn a_completion_and_a_take_point_objective_map() {
        let game = loaded(&with_toll(
            "complete: {at: alpha-prime, dialog: dialog/errands.ink#done}\n\
             objective: {take_point: {point: beta/to-alpha, held_by: league}}\n",
        ));
        assert_eq!(problems(&game), Vec::<String>::new());
        let mapped = toll(&game).expect("mapped");
        assert_eq!(
            mapped.complete,
            Some(MissionCompletion {
                at: Some(stellar(&game, "alpha-prime")),
                dialog: place("dialog/errands.ink", "done"),
            })
        );
        assert_eq!(
            mapped.objective,
            Some(MissionObjective::TakePoint {
                point: "beta/to-alpha".to_owned(),
                held_by: "league".to_owned(),
            })
        );
        let game = loaded(&with_toll("complete: {dialog: dialog/errands.ink#done}\n"));
        assert_eq!(
            toll(&game).expect("mapped").complete.as_ref().map(|c| c.at),
            Some(None)
        );
    }

    /// The mission `toll` written as `yaml` is left out, with the one
    /// problem `why` against its file.
    fn left_out_with(yaml: &str, why: &str) {
        let game = loaded(&with_toll(yaml));
        assert_eq!(problems(&game), [format!("missions/toll.md: {why}")]);
        assert_eq!(toll(&game), None);
        assert!(game.data.storylines().missions.contains_key("parcel"));
    }

    #[test]
    fn a_missing_stellar_leaves_the_mission_out() {
        left_out_with(
            "offer: {at: nowhere, dialog: dialog/errands.ink#toll}\n",
            "offer.at names the stellar `nowhere`, which is not in stellars/; it is left out",
        );
        left_out_with(
            "complete: {at: nowhere, dialog: dialog/errands.ink#done}\n",
            "complete.at names the stellar `nowhere`, which is not in stellars/; it is left out",
        );
        left_out_with(
            "objective: {deliver: {cargo: Plate, tons: 1, to: nowhere}}\n",
            "objective.deliver.to names the stellar `nowhere`, which is not in stellars/; \
             it is left out",
        );
    }

    #[test]
    fn a_missing_mission_leaves_the_mission_out_however_deep() {
        left_out_with(
            "requires: {any: [{missions_done: [gone]}]}\n",
            "requires.any[0].missions_done names the mission `gone`, which is not in \
             missions/; it is left out",
        );
    }

    /// The test galaxy's `blaster` with the outfit `extra` beside it.
    fn with_outfit(source: MemSource, extra: &str) -> MemSource {
        source.with(
            &format!("outfits/{extra}.md"),
            "---\nname: Pass\nmount: fixed\ndamage: 0\nrange: 0\nreload: 1\nprice: 0\n---\n",
        )
    }

    #[test]
    fn each_effect_block_maps_its_outfit_grants_and_removals_to_their_ids() {
        let source = with_outfit(
            with_toll(
                "on_offer: {grant_outfits: [pass, blaster, pass]}\n\
                 on_accept: {remove_outfits: [blaster]}\n\
                 on_decline: {grant_outfits: [blaster], remove_outfits: [pass]}\n\
                 on_success: {remove_outfits: [pass, pass]}\n",
            ),
            "pass",
        );
        let game = loaded(&source);
        assert_eq!(problems(&game), Vec::<String>::new());
        let (blaster, pass) = (OutfitId(128), OutfitId(129));
        let toll = toll(&game).expect("mapped");
        assert_eq!(toll.on_offer.grant_outfits, [pass, blaster, pass]);
        assert_eq!(toll.on_offer.remove_outfits, []);
        assert_eq!(toll.on_accept.grant_outfits, []);
        assert_eq!(toll.on_accept.remove_outfits, [blaster]);
        assert_eq!(toll.on_decline.grant_outfits, [blaster]);
        assert_eq!(toll.on_decline.remove_outfits, [pass]);
        assert_eq!(toll.on_success.remove_outfits, [pass, pass]);
    }

    #[test]
    fn a_missing_outfit_leaves_the_mission_out() {
        left_out_with(
            "on_accept: {grant_outfits: [gone]}\n",
            "on_accept.grant_outfits names the outfit `gone`, which is not in outfits/; \
             it is left out",
        );
        left_out_with(
            "on_success: {remove_outfits: [gone]}\n",
            "on_success.remove_outfits names the outfit `gone`, which is not in outfits/; \
             it is left out",
        );
    }

    #[test]
    fn an_outfit_that_failed_to_load_leaves_the_mission_out_unreported() {
        let source = with_toll("on_accept: {grant_outfits: [blaster]}\n")
            .with("outfits/blaster.md", "---\nnot: valid\n---\n");
        let game = loaded(&source);
        assert_eq!(toll(&game), None);
        assert!(
            problems(&game).iter().all(|p| !p.starts_with("missions/")),
            "{:?}",
            problems(&game)
        );
    }

    #[test]
    fn an_outfit_left_out_for_its_id_leaves_the_mission_out_unreported_again() {
        let source = (0..512).fold(
            with_toll("on_accept: {remove_outfits: [blaster]}\n"),
            |source, n| with_outfit(source, &format!("a{n:03}")),
        );
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            [
                "outfits/blaster.md: it is past the last oütf ID, 639; it is left out",
                "mods/gun-mount.md: it is past the last oütf ID, 639; it is left out",
            ],
            "the outfit's problem, and the kit's after it, alone"
        );
        assert_eq!(toll(&game), None);
        assert!(game.data.storylines().missions.contains_key("parcel"));
    }

    #[test]
    fn a_missing_news_item_leaves_the_mission_out() {
        left_out_with(
            "on_accept: {news: [gone]}\n",
            "on_accept.news names the news `gone`, which is not in news/; it is left out",
        );
    }

    #[test]
    fn a_missing_point_leaves_the_mission_out() {
        left_out_with(
            "on_offer: {points: [{point: alpha/nowhere, stability: dead}]}\n",
            "on_offer.points.point names `alpha/nowhere`, which is no jump point; it is left out",
        );
        left_out_with(
            "objective: {take_point: {point: beta/nowhere, held_by: league}}\n",
            "objective.take_point.point names `beta/nowhere`, which is no jump point; \
             it is left out",
        );
    }

    #[test]
    fn with_systems_unlisted_a_point_into_it_is_not_reported() {
        use crate::new_game::tests::with_systems_unlisted;
        use crate::new_game::{dialogs, map as map_ids};

        let source = with_toll("on_offer: {points: [{point: alpha/to-beta, stability: dead}]}\n");
        let galaxy = with_systems_unlisted(&source);
        let ids = map_ids(&galaxy).ids;
        let mut problems = Vec::new();
        let compiled = dialogs::compiled(&galaxy, &source, &mut problems);
        let storylines = map(&galaxy, &ids, &compiled, &mut problems);
        let lines: Vec<_> = problems.iter().map(ToString::to_string).collect();
        assert_eq!(lines, Vec::<String>::new());
        assert!(!storylines.missions.contains_key("toll"), "it is left out");
    }

    #[test]
    fn a_missing_faction_leaves_the_mission_out() {
        left_out_with(
            "requires: {points: [{point: alpha/to-beta, owner: nobody}]}\n",
            "requires.points.owner names the faction `nobody`, which is not in factions/; \
             it is left out",
        );
        left_out_with(
            "objective: {take_point: {point: beta/to-alpha, held_by: nobody}}\n",
            "objective.take_point.held_by names the faction `nobody`, which is not in \
             factions/; it is left out",
        );
    }

    #[test]
    fn a_missing_compiled_file_is_a_problem_against_the_mission() {
        left_out_with(
            "offer: {at: alpha-yard, dialog: dialog/gone.ink#toll}\n",
            "offer dialog dialog/gone.ink#toll: no dialog/gone.ink.json; \
             compile dialog/gone.ink with inklecate",
        );
        left_out_with(
            "complete: {dialog: dialog/gone.ink#done}\n",
            "complete dialog dialog/gone.ink#done: no dialog/gone.ink.json; \
             compile dialog/gone.ink with inklecate",
        );
    }

    #[test]
    fn every_missing_name_of_a_mission_is_its_own_problem() {
        let game = loaded(&with_toll(
            "offer: {at: nowhere, dialog: dialog/gone.ink#toll}\n",
        ));
        assert_eq!(problems(&game).len(), 2, "{:?}", problems(&game));
        assert_eq!(toll(&game), None);
    }

    #[test]
    fn a_name_that_failed_to_load_leaves_the_mission_out_unreported() {
        let source = with_toll("offer: {at: alpha-prime, dialog: dialog/errands.ink#toll}\n")
            .with("stellars/alpha-prime.md", "---\nnot: valid\n---\n");
        let game = loaded(&source);
        assert_eq!(toll(&game), None);
        assert!(
            problems(&game).iter().all(|p| !p.starts_with("missions/")),
            "{:?}",
            problems(&game)
        );
    }

    #[test]
    fn check_dialogs_drops_a_mission_whose_offer_or_completion_knot_is_refused() {
        use crate::new_game::dialogs::fixture::FakeCheck;
        let mut game = loaded(&with_toll(
            "complete: {at: alpha-prime, dialog: dialog/errands.ink#done}\n",
        ));
        game.check_dialogs(&FakeCheck::default().story("{errands}", &["parcel"]));
        assert_eq!(
            problems(&game),
            ["missions/toll.md: complete dialog dialog/errands.ink#done: no knot or stitch `done`"]
        );
        assert_eq!(toll(&game), None);
        assert!(game.data.storylines().missions.contains_key("parcel"));

        let mut game = loaded(&galaxy());
        game.check_dialogs(&FakeCheck::default().story("{errands}", &["done"]));
        assert_eq!(
            problems(&game),
            [
                "missions/parcel.md: offer dialog dialog/errands.ink#parcel: no knot or stitch `parcel`"
            ]
        );
        assert!(game.data.storylines().missions.is_empty());
        assert_eq!(game.data.storylines().news.len(), 1, "the news stays");
    }

    /// The galaxy with `toll` written as `toll_yaml`, `after` needing
    /// `toll` done, and `last` needing `after` done in a nested group.
    fn with_chain(toll_yaml: &str) -> MemSource {
        with_toll(toll_yaml)
            .with(
                "missions/after.md",
                &format!("{HEAD}requires: {{missions_done: [toll]}}\n---\n"),
            )
            .with(
                "missions/last.md",
                &format!("{HEAD}requires: {{all: [{{any: [{{missions_done: [after]}}]}}]}}\n---\n"),
            )
    }

    /// What a mission named at `field` that was left out reports.
    fn needs_left_out(file: &str, field: &str, id: &str) -> String {
        format!(
            "missions/{file}.md: {field} names the mission `{id}`, which was left out; it is left out"
        )
    }

    #[test]
    fn a_mission_needing_one_left_out_for_a_missing_name_is_left_out_too() {
        let game = loaded(&with_chain(
            "offer: {at: nowhere, dialog: dialog/errands.ink#toll}\n",
        ));
        assert_eq!(
            problems(&game),
            [
                "missions/toll.md: offer.at names the stellar `nowhere`, which is not in \
                 stellars/; it is left out"
                    .to_owned(),
                needs_left_out("after", "requires.missions_done", "toll"),
                needs_left_out("last", "requires.all[0].any[0].missions_done", "after"),
            ]
        );
        let kept: Vec<_> = game.data.storylines().missions.keys().cloned().collect();
        assert_eq!(kept, ["parcel"]);
    }

    #[test]
    fn check_dialogs_leaves_out_a_mission_needing_one_it_refused() {
        use crate::new_game::dialogs::fixture::FakeCheck;
        let mut game = loaded(&with_chain(
            "complete: {at: alpha-prime, dialog: dialog/errands.ink#done}\n",
        ));
        assert_eq!(problems(&game), Vec::<String>::new());
        game.check_dialogs(&FakeCheck::default().story("{errands}", &["parcel"]));
        assert_eq!(
            problems(&game),
            [
                "missions/toll.md: complete dialog dialog/errands.ink#done: no knot or stitch \
                 `done`"
                    .to_owned(),
                needs_left_out("after", "requires.missions_done", "toll"),
                needs_left_out("last", "requires.all[0].any[0].missions_done", "after"),
            ]
        );
        let kept: Vec<_> = game.data.storylines().missions.keys().cloned().collect();
        assert_eq!(kept, ["parcel"]);
    }

    #[test]
    fn a_point_in_a_system_that_failed_to_load_leaves_the_mission_out_unreported() {
        let source = with_toll("on_offer: {points: [{point: beta/to-alpha, stability: dead}]}\n")
            .with("systems/beta.md", "---\nnot: valid\n---\n");
        let game = loaded(&source);
        assert_eq!(toll(&game), None);
        assert!(
            problems(&game)
                .iter()
                .all(|p| !p.starts_with("missions/toll.md")),
            "{:?}",
            problems(&game)
        );
    }

    #[test]
    fn a_news_item_that_failed_to_load_leaves_the_mission_out_unreported() {
        let source = with_toll("on_accept: {news: [parcel-delivered]}\n")
            .with("news/parcel-delivered.md", "---\nnot: valid\n---\n");
        let game = loaded(&source);
        assert_eq!(toll(&game), None);
        assert_eq!(game.data.storylines().missions.get("parcel"), None);
        assert!(
            problems(&game).iter().all(|p| !p.starts_with("missions/")),
            "{:?}",
            problems(&game)
        );
    }

    #[test]
    fn a_news_item_left_out_leaves_the_mission_naming_it_out_unreported_again() {
        let source = with_toll("on_accept: {news: [rumour]}\n").with(
            "news/rumour.md",
            "---\nheadline: Rumour\nfrom: nowhere\n---\nWho knows.\n",
        );
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            ["news/rumour.md: its `from` system `nowhere` is not in systems/; it is left out"],
            "the news item's problem alone"
        );
        assert_eq!(toll(&game), None);
        assert!(game.data.storylines().missions.contains_key("parcel"));
    }

    #[test]
    fn a_news_item_with_an_unknown_from_is_left_out() {
        let source = galaxy().with(
            "news/rumour.md",
            "---\nheadline: Rumour\nfrom: nowhere\n---\nWho knows.\n",
        );
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            ["news/rumour.md: its `from` system `nowhere` is not in systems/; it is left out"]
        );
        assert!(!game.data.storylines().news.contains_key("rumour"));
    }

    #[test]
    fn with_the_folder_unlisted_a_mission_s_ref_is_read_on_its_own() {
        let source = galaxy().with_bytes("dialog/bad.ink.json", &[0xff, 0xfe]);
        let game = loaded(&source);
        assert_eq!(game.problems.len(), 1, "{:?}", game.problems);
        assert_eq!(game.problems[0].file, "dialog/");
        assert!(game.data.storylines().missions.contains_key("parcel"));
        assert_eq!(
            &*game.data.dialogs().compiled["dialog/errands.ink"],
            "{errands}"
        );
    }

    #[test]
    fn with_the_folder_unlisted_each_named_file_is_read_once() {
        use std::cell::RefCell;

        use nova_galaxy_format::source::{Source, SourceFile};

        /// The galaxy, counting each read of a compiled story.
        struct Counting(MemSource, RefCell<Vec<String>>);
        impl Source for Counting {
            fn files(&self, dir: &str, ext: &str) -> std::io::Result<Vec<SourceFile>> {
                self.0.files(dir, ext)
            }
            fn bytes(&self, path: &str) -> std::io::Result<Option<Vec<u8>>> {
                if path.starts_with("dialog/") {
                    self.1.borrow_mut().push(path.to_owned());
                }
                self.0.bytes(path)
            }
        }
        // Two missions name the same readable file, and two the same
        // unreadable one; a bad file fails the listing.
        let source = with_toll("complete: {dialog: dialog/errands.ink#done}\n")
            .with(
                "missions/other.md",
                &format!("{HEAD}complete: {{dialog: dialog/bad.ink#a}}\n---\n"),
            )
            .with(
                "missions/third.md",
                &format!("{HEAD}complete: {{dialog: dialog/bad.ink#b}}\n---\n"),
            )
            .with_bytes("dialog/bad.ink.json", &[0xff, 0xfe]);
        let counting = Counting(source, RefCell::default());
        let _ = load(&counting, Path::new("sample"));
        let mut read = counting.1.into_inner();
        read.sort();
        assert_eq!(read, ["dialog/bad.ink.json", "dialog/errands.ink.json"]);
    }

    #[test]
    fn a_mission_s_unreadable_compiled_file_is_reported_as_unreadable() {
        let source = galaxy().with_bytes("dialog/errands.ink.json", &[0xff, 0xfe]);
        let game = loaded(&source);
        let lines = problems(&game);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[1].starts_with(
                "missions/parcel.md: offer dialog dialog/errands.ink#parcel: \
                 dialog/errands.ink.json couldn't be read: "
            ),
            "{lines:?}"
        );
        assert!(game.data.storylines().missions.is_empty());
    }
}
