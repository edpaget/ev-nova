//! The sounds, over the format's test galaxy (`super::super::tests::galaxy`),
//! whose one weapon is the skiff's `blaster` (`wëap` 128), with clips built
//! by `sound::fixture::wav` and written into the source beside it.

use std::fmt::Write as _;
use std::path::Path;

use nova_galaxy_format::source::MemSource;

use super::super::pins::{FILE, text};
use super::super::tests::galaxy;
use super::super::{NewGame, load};
use super::*;
use crate::records::boom::Boom;
use crate::records::weapon::Weapon;
use crate::sound::fixture::wav;

fn loaded(source: &MemSource) -> NewGame {
    load(source, Path::new("games"))
}

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn problems(game: &NewGame) -> Vec<String> {
    game.problems.iter().map(ToString::to_string).collect()
}

/// A distinct clip for each `tone`: a short 16-bit mono WAV of it.
fn clip(tone: i16) -> Vec<u8> {
    wav(1, 8000, 16, &[tone, -tone, tone / 2, 0])
}

/// `sounds/<name>.md`, its file `media/export/sounds/<name>.wav`.
fn sound_file(name: &str) -> String {
    format!("---\nplaceholder: true\nfile: media/export/sounds/{name}.wav\n---\n")
}

/// `source` with the sound `name` and its clip of `tone`.
fn with_sound(source: MemSource, name: &str, tone: i16) -> MemSource {
    source
        .with(&format!("sounds/{name}.md"), &sound_file(name))
        .with_bytes(&format!("media/export/sounds/{name}.wav"), &clip(tone))
}

/// An outfit `name` of `damage` on a fixed mount, its weapon firing
/// `sound`.
fn gun(name: &str, damage: u32, sound: &str) -> String {
    format!(
        "---\nname: {name}\nmount: fixed\ndamage: {damage}\nrange: 300\nreload: 0.5\n\
         price: 100\nsound: {sound}\n---\n"
    )
}

/// The galaxy with its blaster firing `sound`.
fn blaster_firing(sound: &str) -> MemSource {
    galaxy().with("outfits/blaster.md", &gun("Blaster", 5, sound))
}

/// A `game.md` naming `roles`, as `role: sound` lines.
fn game_md(roles: &[(&str, &str)]) -> String {
    let lines = roles
        .iter()
        .fold(String::new(), |mut lines, (role, sound)| {
            writeln!(lines, "  {role}: {sound}").expect("writes to a string");
            lines
        });
    format!("---\nname: Test\nsounds:\n{lines}---\nAbout.\n")
}

fn files(game: &NewGame) -> &SoundFiles {
    game.data.sound_files().expect("a new game has sound files")
}

fn weapon(game: &NewGame, outfit: &str) -> Weapon {
    let nova = game.ids.weapons[&id(outfit)];
    game.data
        .get::<Weapon>(nova)
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

// A weapon's sound.

#[test]
fn a_weapon_s_sound_takes_the_first_weapon_sound_id_and_fires_it() {
    let game = loaded(&with_sound(blaster_firing("zap"), "zap", 1000));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.ids.sounds, BTreeMap::from([(id("zap"), 200)]));
    assert_eq!(weapon(&game, "blaster").sound, 0, "snd 200 less 200");
    assert_eq!(files(&game).get(SoundId(200)), Some(&clip(1000)[..]));
    assert_eq!(files(&game).ids().collect::<Vec<_>>(), [SoundId(200)]);
}

#[test]
fn the_weapon_sound_constants_are_the_engine_s() {
    assert_eq!(FIRST_WEAPON_SOUND, 200);
    assert_eq!(weapon_sound(200), 0);
    assert_eq!(weapon_sound(299), 99);
    assert_eq!(
        [
            BUTTON_DOWN,
            BUTTON_UP,
            LANDING,
            REFUSED,
            GATE_PASSAGE,
            EXPLOSION
        ]
        .map(|s| s.0),
        [600, 601, 151, 153, 130, 300]
    );
    assert_eq!(BOOMS.map(|(id, _)| id), [128, 129]);
}

#[test]
fn weapon_sounds_number_in_sound_id_order_and_a_pin_keeps_its_id() {
    let source = with_sound(with_sound(blaster_firing("bang"), "bang", 900), "aaa", 800)
        .with("outfits/aaa-gun.md", &gun("Aaa gun", 3, "aaa"));
    let game = loaded(&source);
    assert_eq!(
        game.ids.sounds,
        BTreeMap::from([(id("aaa"), 200), (id("bang"), 201)])
    );
    assert_eq!(weapon(&game, "aaa-gun").sound, 0);
    assert_eq!(weapon(&game, "blaster").sound, 1);
    let pinned = loaded(&source.with(FILE, r#"{"sounds": {"bang": 205}}"#));
    assert_eq!(
        pinned.ids.sounds,
        BTreeMap::from([(id("aaa"), 206), (id("bang"), 205)])
    );
    assert_eq!(weapon(&pinned, "blaster").sound, 5);
    assert_eq!(files(&pinned).get(SoundId(205)), Some(&clip(900)[..]));
    assert_eq!(files(&pinned).get(SoundId(206)), Some(&clip(800)[..]));
}

#[test]
fn two_weapons_firing_one_sound_share_its_id() {
    let source = with_sound(blaster_firing("bang"), "bang", 900)
        .with("outfits/aaa-gun.md", &gun("Aaa gun", 3, "bang"));
    let game = loaded(&source);
    assert_eq!(game.ids.sounds, BTreeMap::from([(id("bang"), 200)]));
    assert_eq!(weapon(&game, "aaa-gun").sound, 0);
    assert_eq!(weapon(&game, "blaster").sound, 0);
}

#[test]
fn a_sound_past_the_last_weapon_sound_id_is_reported_and_its_weapon_fires_silently() {
    let source = with_sound(with_sound(blaster_firing("bang"), "bang", 900), "zzz", 800)
        .with("outfits/zzz-gun.md", &gun("Zzz gun", 3, "zzz"))
        .with(FILE, r#"{"sounds": {"zzz": 299}}"#);
    let game = loaded(&source);
    assert!(!game.ids.sounds.contains_key(&id("bang")));
    assert_eq!(weapon(&game, "blaster").sound, -1);
    assert_eq!(weapon(&game, "zzz-gun").sound, 99);
    assert!(
        problems(&game).contains(
            &"sounds/bang.md: it is past the last weapon sound ID, 299; the weapons naming it \
              fire silently"
                .to_owned()
        ),
        "{:?}",
        problems(&game)
    );
    assert_eq!(files(&game).ids().collect::<Vec<_>>(), [SoundId(299)]);
}

#[test]
fn a_weapon_naming_no_sound_fires_silently_and_a_non_weapon_s_sound_is_ignored() {
    let game = loaded(&galaxy());
    assert_eq!(weapon(&game, "blaster").sound, -1);
    assert!(game.ids.sounds.is_empty());
    assert_eq!(files(&game), &SoundFiles::default(), "silent, not stock");
    let harmless = galaxy().with("outfits/blaster.md", &gun("Blaster", 0, "missing"));
    let game = loaded(&harmless);
    assert_eq!(problems(&game), Vec::<String>::new(), "not a weapon");
    assert!(game.ids.sounds.is_empty());
}

#[test]
fn a_weapon_s_sound_not_in_sounds_is_reported_once_and_fires_silently() {
    let game = loaded(&blaster_firing("missing"));
    assert_eq!(
        problems(&game),
        ["outfits/blaster.md: its sound `missing` is not in sounds/; it fires silently"]
    );
    assert_eq!(weapon(&game, "blaster").sound, -1);
}

#[test]
fn a_sound_that_failed_to_load_is_not_reported_again() {
    let source = blaster_firing("broken").with("sounds/broken.md", "---\nplaceholder: true\n---\n");
    let game = loaded(&source);
    let ours: Vec<_> = problems(&game)
        .into_iter()
        .filter(|p| p.starts_with("outfits/"))
        .collect();
    assert_eq!(ours, Vec::<String>::new());
    assert_eq!(weapon(&game, "blaster").sound, -1);
}

// The roles.

#[test]
fn each_role_fills_its_engine_cue() {
    assert_eq!(cue(SoundRole::ButtonDown), BUTTON_DOWN);
    assert_eq!(cue(SoundRole::ButtonUp), BUTTON_UP);
    assert_eq!(cue(SoundRole::Landing), LANDING);
    assert_eq!(cue(SoundRole::Refused), REFUSED);
    assert_eq!(cue(SoundRole::GatePassage), GATE_PASSAGE);
    assert_eq!(cue(SoundRole::Explosion), EXPLOSION);
}

#[test]
fn each_role_s_clip_sits_at_its_cue_and_nothing_at_an_unnamed_one() {
    let roles = [
        (SoundRole::ButtonDown, "down", 100),
        (SoundRole::ButtonUp, "up", 200),
        (SoundRole::Landing, "chime", 300),
        (SoundRole::Refused, "buzz", 400),
        (SoundRole::GatePassage, "hum", 500),
        (SoundRole::Explosion, "blast", 600),
    ];
    let source = roles.iter().fold(
        galaxy().with(
            "game.md",
            &game_md(&roles.map(|(role, sound, _)| (role.name(), sound))),
        ),
        |source, &(_, sound, tone)| with_sound(source, sound, tone),
    );
    let game = loaded(&source);
    assert_eq!(problems(&game), Vec::<String>::new());
    for (role, _, tone) in roles {
        assert_eq!(
            files(&game).get(cue(role)),
            Some(&clip(tone)[..]),
            "{role:?}"
        );
    }
    assert_eq!(files(&game).ids().count(), 6);
    assert!(
        game.ids.sounds.is_empty(),
        "a role's sound takes no ID of its own"
    );
    let some = galaxy().with("game.md", &game_md(&[("landing", "chime")]));
    let game = loaded(&with_sound(some, "chime", 300));
    assert_eq!(files(&game).ids().collect::<Vec<_>>(), [LANDING]);
    assert_eq!(files(&game).get(REFUSED), None, "left out, silent");
}

#[test]
fn a_clip_a_weapon_and_a_role_share_fills_both() {
    let source = with_sound(blaster_firing("bang"), "bang", 900)
        .with("game.md", &game_md(&[("explosion", "bang")]));
    let game = loaded(&source);
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(files(&game).get(SoundId(200)), Some(&clip(900)[..]));
    assert_eq!(files(&game).get(EXPLOSION), Some(&clip(900)[..]));
}

#[test]
fn a_role_not_in_sounds_is_reported_against_game_md_and_silent() {
    let game = loaded(&galaxy().with("game.md", &game_md(&[("refused", "buzz")])));
    assert_eq!(
        problems(&game),
        ["game.md: its `refused` sound `buzz` is not in sounds/; it is silent"]
    );
    assert_eq!(files(&game).get(REFUSED), None);
}

#[test]
fn a_game_md_that_does_not_parse_fills_no_role() {
    let source = with_sound(galaxy(), "chime", 300).with(
        "game.md",
        "---\nname: Test\nsounds:\n  landing: chime\n  launch: chime\n---\nAbout.\n",
    );
    let game = loaded(&source);
    assert_eq!(problems(&game).len(), 1, "{:?}", problems(&game));
    assert!(
        problems(&game)[0].starts_with("game.md: line 5"),
        "{:?}",
        problems(&game)
    );
    assert!(files(&game).ids().next().is_none());
    assert!(
        game.galaxy.game.is_none(),
        "left out whole, its About text too"
    );
}

// The explosions.

#[test]
fn the_booms_come_only_with_an_explosion_role_that_is_in_sounds() {
    let named =
        with_sound(galaxy(), "blast", 600).with("game.md", &game_md(&[("explosion", "blast")]));
    let game = loaded(&named);
    for (nova, name) in BOOMS {
        let entry = game
            .data
            .get::<Boom>(nova)
            .expect("present")
            .expect("decoded");
        assert_eq!(entry.name, Some(name));
        assert_eq!(
            *entry.record,
            Boom {
                frame_advance: 100,
                sound_index: 0,
                graphic_index: 0,
            }
        );
    }
    let none = loaded(&galaxy());
    assert!(none.data.get::<Boom>(128).is_none());
    let other_role =
        with_sound(galaxy(), "chime", 300).with("game.md", &game_md(&[("landing", "chime")]));
    assert!(loaded(&other_role).data.get::<Boom>(128).is_none());
    let missing = galaxy().with("game.md", &game_md(&[("explosion", "blast")]));
    let game = loaded(&missing);
    assert!(game.data.get::<Boom>(128).is_none());
    assert!(game.data.get::<Boom>(129).is_none());
}

// The clips.

/// The one problem `file`'s bytes give the blaster's sound `bang`, and
/// that it is silent.
fn clip_problem(file: Option<&[u8]>, path: &str) -> Vec<String> {
    let mut source = blaster_firing("bang").with(
        "sounds/bang.md",
        &format!("---\nplaceholder: true\nfile: {path}\n---\n"),
    );
    if let Some(bytes) = file {
        source = source.with_bytes(path, bytes);
    }
    let game = loaded(&source);
    assert_eq!(game.ids.sounds[&id("bang")], 200, "numbered all the same");
    assert_eq!(files(&game).get(SoundId(200)), None, "silent");
    problems(&game)
}

#[test]
fn a_clip_that_is_not_there_is_reported_and_silent() {
    assert_eq!(
        clip_problem(None, "media/export/sounds/bang.wav"),
        ["sounds/bang.md: its file `media/export/sounds/bang.wav` isn't there; it is silent"]
    );
}

#[test]
fn a_clip_outside_the_export_media_is_reported_and_silent() {
    assert_eq!(
        clip_problem(Some(&clip(5)), "media/source/sounds/bang.wav"),
        [
            "sounds/bang.md: its file `media/source/sounds/bang.wav` isn't under media/export/; \
             it is silent"
        ]
    );
}

#[test]
fn a_clip_the_adapter_cannot_decode_is_reported_with_the_decoder_s_error() {
    let float = {
        let mut bytes = clip(5);
        bytes[20] = 3;
        bytes
    };
    let error = decode_wav(&float).expect_err("IEEE float");
    assert_eq!(
        clip_problem(Some(&float), "media/export/sounds/bang.wav"),
        [format!(
            "sounds/bang.md: its file `media/export/sounds/bang.wav` doesn't decode as a WAV \
             file ({error}); it is silent"
        )]
    );
}

#[test]
fn a_clip_of_no_samples_is_reported_and_silent() {
    assert_eq!(
        clip_problem(Some(&wav(1, 8000, 16, &[])), "media/export/sounds/bang.wav"),
        ["sounds/bang.md: its file `media/export/sounds/bang.wav` holds no samples; it is silent"]
    );
}

#[test]
fn a_clip_that_is_an_lfs_pointer_is_silent_and_listed_among_the_pointers() {
    let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 1\n";
    let mut source = blaster_firing("bang").with("sounds/bang.md", &sound_file("bang"));
    source = source.with_bytes("media/export/sounds/bang.wav", pointer);
    let game = loaded(&source);
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.lfs_pointers, ["media/export/sounds/bang.wav"]);
    assert_eq!(files(&game).get(SoundId(200)), None);
}

/// A source that counts its reads of the clips under
/// `media/export/sounds/`, and fails each when `broken`.
struct Clips {
    source: MemSource,
    broken: bool,
    reads: std::cell::Cell<usize>,
}

impl Clips {
    fn new(source: MemSource, broken: bool) -> Self {
        Self {
            source,
            broken,
            reads: std::cell::Cell::new(0),
        }
    }
}

impl Source for Clips {
    fn files(
        &self,
        dir: &str,
        ext: &str,
    ) -> std::io::Result<Vec<nova_galaxy_format::source::SourceFile>> {
        self.source.files(dir, ext)
    }

    fn bytes(&self, path: &str) -> std::io::Result<Option<Vec<u8>>> {
        if path.starts_with("media/export/sounds/") {
            self.reads.set(self.reads.get() + 1);
            if self.broken {
                return Err(std::io::Error::other("the disk is unreadable"));
            }
        }
        self.source.bytes(path)
    }
}

#[test]
fn a_clip_that_cannot_be_read_is_reported_and_silent() {
    let source = Clips::new(with_sound(blaster_firing("bang"), "bang", 5), true);
    let game = load(&source, Path::new("games"));
    assert_eq!(
        problems(&game),
        [
            "sounds/bang.md: its file `media/export/sounds/bang.wav` couldn't be read (the disk \
             is unreadable); it is silent"
        ]
    );
    assert_eq!(files(&game).ids().next(), None);
}

#[test]
fn each_clip_is_read_once_however_many_ids_it_fills() {
    let source = Clips::new(
        with_sound(blaster_firing("bang"), "bang", 5).with(
            "game.md",
            &game_md(&[("landing", "bang"), ("refused", "bang")]),
        ),
        false,
    );
    let game = load(&source, Path::new("games"));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(
        files(&game).ids().collect::<Vec<_>>(),
        [LANDING, REFUSED, SoundId(200)]
    );
    // The LFS check reads each clip once too, before the mapping does.
    assert_eq!(source.reads.get(), 2, "the LFS check's and the mapping's");
}

// The table.

#[test]
fn a_table_of_every_record_pins_the_sounds_and_reports_nothing() {
    let source = with_sound(blaster_firing("bang"), "bang", 900);
    let today = loaded(&source).ids;
    let game = loaded(&source.clone().with(FILE, &text(&today)));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.ids, today);
    let left_out = loaded(&source.with(
        FILE,
        &text(&IdTable {
            sounds: BTreeMap::new(),
            ..today
        }),
    ));
    assert!(
        problems(&left_out)
            .iter()
            .any(|p| p.contains("leaves out 1 record, the first `sounds/bang`")),
        "{:?}",
        problems(&left_out)
    );
}
