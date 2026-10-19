//! Keeping pilots: where they are saved, and the rules for saving and
//! opening them.
//!
//! - [`PilotStore`]: the port pilots are saved through, as text under a
//!   key.
//! - [`pilot_key`]: the key a pilot is saved under, from its name.
//! - [`PilotKeeper`]: the rules. It saves a pilot in the
//!   [`save`](crate::save) schema under its key, lists the pilots saved,
//!   opens one and deletes one, and reports each problem as a message naming where
//!   the pilots are. Each save and open records the pilot as the last
//!   pilot, which [`PilotKeeper::open_last`] opens at launch.

use std::io;

use crate::pilot::Pilot;
use crate::save::{SaveError, decode, encode};

/// Why a pilot could not be opened ([`PilotKeeper::open`]), each with a
/// message naming the pilot and where the pilots are, which `Display`
/// shows. The original's `_OpenPilotFile` (@0x76644) tells only the
/// wrong version apart: `_LoadPilotData`'s -42 shows `STR#` 140 #34
/// (@0x766c8-0x766ef), and every other error (-43, -45) shows nothing
/// (@0x766fb-0x76701).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// A save from a newer version ([`SaveError::Newer`]): the -42.
    WrongVersion(String),
    /// Nothing saved under the key, a save that cannot be read, or text
    /// that is no save ([`SaveError::Unusable`]): the -43 and -45.
    Unreadable(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongVersion(message) | Self::Unreadable(message) => f.write_str(message),
        }
    }
}

/// Where pilots are saved: text in and out under a key, a pilot's
/// [`pilot_key`]. The format is the keeper's.
pub trait PilotStore {
    /// The keys of every pilot saved, in any order.
    ///
    /// # Errors
    ///
    /// When the pilots cannot be listed.
    fn list(&self) -> io::Result<Vec<String>>;

    /// The text saved under `key`, or `None` when nothing is.
    ///
    /// # Errors
    ///
    /// When the saved text is there but cannot be read.
    fn read(&self, key: &str) -> io::Result<Option<String>>;

    /// Saves `text` under `key`, in place of whatever was saved there.
    ///
    /// # Errors
    ///
    /// When it cannot be saved.
    fn write(&self, key: &str, text: &str) -> io::Result<()>;

    /// Removes what is saved under `key`. Nothing saved there is `Ok`, as
    /// `_DeletePilotFile` checks `_DoesPilotExist` first (@0x72e7b).
    ///
    /// # Errors
    ///
    /// When what is saved there cannot be removed.
    fn delete(&self, key: &str) -> io::Result<()>;

    /// The key the last-pilot record names, or `None` when there is no
    /// record. The record is the original's "Last Pilot" alias (`STR#` 130
    /// #4): it names a pilot's key and holds no pilot, is never one of the
    /// keys [`list`](Self::list) gives, and [`delete`](Self::delete) never
    /// touches it.
    ///
    /// # Errors
    ///
    /// When the record is there but cannot be read.
    fn last_pilot(&self) -> io::Result<Option<String>>;

    /// Makes the last-pilot record name `key`, in place of whatever it
    /// named.
    ///
    /// # Errors
    ///
    /// When the record cannot be written.
    fn set_last_pilot(&self, key: &str) -> io::Result<()>;

    /// Where the pilots are saved, for the player to find them: a
    /// directory's path, say.
    fn location(&self) -> String;
}

/// A boxed store is a store.
impl PilotStore for Box<dyn PilotStore> {
    fn list(&self) -> io::Result<Vec<String>> {
        (**self).list()
    }

    fn read(&self, key: &str) -> io::Result<Option<String>> {
        (**self).read(key)
    }

    fn write(&self, key: &str, text: &str) -> io::Result<()> {
        (**self).write(key, text)
    }

    fn delete(&self, key: &str) -> io::Result<()> {
        (**self).delete(key)
    }

    fn last_pilot(&self) -> io::Result<Option<String>> {
        (**self).last_pilot()
    }

    fn set_last_pilot(&self, key: &str) -> io::Result<()> {
        (**self).set_last_pilot(key)
    }

    fn location(&self) -> String {
        (**self).location()
    }
}

/// The characters a file name cannot hold on some system Nova runs on.
const UNSAFE: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// The key pilot `name` is saved under: the name trimmed, with each
/// character a file name cannot hold (`/ \ : * ? " < > |` and control
/// characters) replaced by `_`. `None` for a name that is empty once
/// trimmed, or is `.` or `..`: no pilot is saved under those.
#[must_use]
pub fn pilot_key(name: &str) -> Option<String> {
    let key: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || UNSAFE.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    (!matches!(key.as_str(), "" | "." | "..")).then_some(key)
}

/// Keeps pilots in a store. It reports problems as messages for its caller
/// to show; it never prints.
#[derive(Debug)]
pub struct PilotKeeper<S: PilotStore> {
    store: S,
}

impl<S: PilotStore> PilotKeeper<S> {
    /// Keeps pilots in `store`.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// The store the pilots are kept in.
    #[must_use]
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The keys of every pilot saved, sorted.
    ///
    /// # Errors
    ///
    /// A message naming where the pilots are, when they cannot be listed.
    pub fn list(&self) -> Result<Vec<String>, String> {
        let mut keys = self.store.list().map_err(|error| {
            format!(
                "nova: cannot list the pilots in {} ({error})",
                self.store.location()
            )
        })?;
        keys.sort();
        Ok(keys)
    }

    /// Whether a pilot named `name` would replace a saved one: its key is
    /// saved, ignoring case, as the Mac's file names do. A name with no key
    /// is not saved, and when the pilots cannot be listed none is.
    #[must_use]
    pub fn exists(&self, name: &str) -> bool {
        let Some(key) = pilot_key(name) else {
            return false;
        };
        let key = key.to_lowercase();
        self.store
            .list()
            .is_ok_and(|keys| keys.iter().any(|saved| saved.to_lowercase() == key))
    }

    /// The pilot saved under `key`.
    ///
    /// # Errors
    ///
    /// [`OpenError::WrongVersion`] for a save from a newer version, and
    /// [`OpenError::Unreadable`] when nothing is saved under `key`, it
    /// cannot be read, or it is no pilot; each with a message naming the
    /// pilot and where the pilots are.
    ///
    /// A pilot that opens becomes the last pilot, as `_LoadPilotData`
    /// rewrites the "Last Pilot" alias (@0x76333) once the save has
    /// decoded. A save that fails (-43, -45, -42) records nothing. A failed
    /// record write is ignored, as `_MakeAlias`'s result is.
    pub fn open(&self, key: &str) -> Result<Pilot, OpenError> {
        let location = self.store.location();
        let text = self
            .store
            .read(key)
            .map_err(|error| {
                OpenError::Unreadable(format!(
                    "nova: cannot read the pilot {key} in {location} ({error})"
                ))
            })?
            .ok_or_else(|| {
                OpenError::Unreadable(format!("nova: there is no pilot {key} in {location}"))
            })?;
        let pilot = decode(&text).map_err(|error| {
            let message = format!("nova: the pilot {key} in {location}: {error}");
            match error {
                SaveError::Newer { .. } => OpenError::WrongVersion(message),
                SaveError::Unusable(_) => OpenError::Unreadable(message),
            }
        })?;
        self.record(key);
        Ok(pilot)
    }

    /// The last pilot, for opening at launch as `_OpenLastPilotFile`
    /// (@0x764ad) does: the pilot the last-pilot record names, opened as
    /// [`open`](Self::open) opens it, so it is recorded again.
    ///
    /// `None`, with no message, when there is no record, the record cannot
    /// be read, nothing is saved under its key (the pilot was deleted), or
    /// the save cannot be read or is from a newer version: the original
    /// returns silently on any of these (@0x764da, @0x7658a).
    #[must_use]
    pub fn open_last(&self) -> Option<Pilot> {
        let key = self.store.last_pilot().ok().flatten()?;
        self.open(&key).ok()
    }

    /// Makes `key` the last pilot, ignoring a failure: `_WritePilotData`
    /// returns 0 whatever `_MakeAlias` did (@0x72dd8).
    fn record(&self, key: &str) {
        let _ignored = self.store.set_last_pilot(key);
    }

    /// Saves `pilot` under its key, replacing any pilot saved there.
    ///
    /// # Errors
    ///
    /// A message saying why, when the pilot has no name to save it under,
    /// or naming the pilot and where the pilots are when it cannot be
    /// written.
    ///
    /// A pilot saved becomes the last pilot, as `_WritePilotData` rewrites
    /// the "Last Pilot" alias (@0x72dd3). A failed save records nothing,
    /// and a failed record write is ignored (@0x72dd8).
    pub fn save(&self, pilot: &Pilot) -> Result<(), String> {
        let key = pilot_key(pilot.name())
            .ok_or_else(|| "nova: a pilot without a name is not saved".to_owned())?;
        self.store.write(&key, &encode(pilot)).map_err(|error| {
            format!(
                "nova: cannot save the pilot {} in {} ({error})",
                pilot.name(),
                self.store.location()
            )
        })?;
        self.record(&key);
        Ok(())
    }

    /// Deletes the pilot named `name`: what is saved under its key. A name
    /// without a key deletes nothing, since no pilot is saved under it, and
    /// a pilot never saved is deleted already. The last-pilot record is
    /// left as it is, even naming this pilot: `_DeletePilotFile`
    /// (@0x72e6b-0x72ef6) never touches the "Last Pilot" alias, so the next
    /// launch finds the record, opens nothing and says nothing.
    ///
    /// # Errors
    ///
    /// A message naming the pilot and where the pilots are, when its save
    /// cannot be removed.
    pub fn delete(&self, name: &str) -> Result<(), String> {
        let Some(key) = pilot_key(name) else {
            return Ok(());
        };
        self.store.delete(&key).map_err(|error| {
            format!(
                "nova: cannot delete the pilot {name} in {} ({error})",
                self.store.location()
            )
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, StellarId, SystemId};
    use crate::fixture::MemoryPilots;
    use crate::reserves::Gauge;
    use crate::session::Session;
    use crate::testkit::{catalog, edge_lander, jump, land_now};

    #[test]
    fn a_pilots_key_is_its_name_trimmed() {
        assert_eq!(pilot_key("Ada"), Some("Ada".to_owned()));
        assert_eq!(
            pilot_key("  Ada Lovelace \t"),
            Some("Ada Lovelace".to_owned())
        );
        assert_eq!(pilot_key("Zoë"), Some("Zoë".to_owned()));
        assert_eq!(pilot_key("..Ada"), Some("..Ada".to_owned()));
    }

    #[test]
    fn characters_file_names_cannot_hold_become_underscores() {
        assert_eq!(
            pilot_key(r#"a/b\c:d*e?f"g<h>i|j"#),
            Some("a_b_c_d_e_f_g_h_i_j".to_owned())
        );
        assert_eq!(pilot_key("tab\tin\u{7f}\u{1}"), Some("tab_in__".to_owned()));
    }

    #[test]
    fn an_empty_name_or_a_directory_name_has_no_key() {
        for name in ["", "   ", ".", "..", " .. "] {
            assert_eq!(pilot_key(name), None, "{name:?}");
        }
    }

    fn keeper(store: &MemoryPilots) -> PilotKeeper<MemoryPilots> {
        PilotKeeper::new(store.clone())
    }

    /// A pilot named `name` that has jumped, landed and changed: a day
    /// and a jump's fuel gone, 7 credits and a legal record, docked at
    /// planet 140 at the edge of system 131.
    fn travelled(name: &str) -> Pilot {
        let catalog = edge_lander();
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, name).expect("starts")).expect("flies");
        jump(&mut session, &catalog, 132);
        land_now(&mut session).expect("lands");
        session.transact(|pilot| {
            pilot.set_cash(7);
            pilot.set_legal_record(GovtId(129), -3);
        });
        session.pilot().clone()
    }

    #[test]
    fn a_saved_pilot_lists_and_opens_with_every_field_as_it_was() {
        let store = MemoryPilots::new();
        let pilot = travelled("Ada");
        assert_eq!(
            pilot.reserves().fuel,
            Gauge {
                now: 200.0,
                max: 300.0
            }
        );
        assert_eq!(pilot.stellar(), Some(StellarId(140)));
        assert_eq!(keeper(&store).save(&pilot), Ok(()));
        assert_eq!(store.writes(), 1);
        let reopened = keeper(&store);
        assert_eq!(reopened.list(), Ok(vec!["Ada".to_owned()]));
        let opened = reopened.open("Ada").expect("opens");
        assert_eq!(opened, pilot);
        assert_eq!(opened.reserves(), pilot.reserves());
        assert_eq!(opened.system(), SystemId(131));
        let resumed = Session::fly(&edge_lander(), opened).expect("flies");
        assert_eq!(crate::testkit::resumed_on(&resumed), Some(StellarId(140)));
        assert_eq!(resumed.reserves().fuel.now, 200.0);
    }

    #[test]
    fn saving_again_replaces_the_save() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        let mut pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        keeper.save(&pilot).expect("saves");
        pilot.set_cash(99);
        keeper.save(&pilot).expect("saves");
        assert_eq!(keeper.list(), Ok(vec!["Ada".to_owned()]));
        assert_eq!(keeper.open("Ada").map(|p| p.cash()), Ok(99));
    }

    #[test]
    fn pilots_are_saved_under_their_keys_and_listed_in_order() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        for name in ["zed", " Ada/B ", "Bob"] {
            keeper
                .save(&Pilot::new(&catalog(), name).expect("starts"))
                .expect("saves");
        }
        assert_eq!(
            keeper.list(),
            Ok(vec!["Ada_B".to_owned(), "Bob".to_owned(), "zed".to_owned()])
        );
        assert_eq!(
            keeper.open("Ada_B").map(|p| p.name().to_owned()),
            Ok(" Ada/B ".to_owned())
        );
    }

    #[test]
    fn a_name_exists_when_its_key_is_saved_whatever_its_case() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        assert!(!keeper.exists("Ada"));
        keeper
            .save(&Pilot::new(&catalog(), "Ada").expect("starts"))
            .expect("saves");
        assert!(keeper.exists("Ada"));
        assert!(keeper.exists(" ada "), "file names ignore case on a Mac");
        assert!(!keeper.exists("Bob"));
        assert!(!keeper.exists(""));
    }

    #[test]
    fn an_unnamed_pilot_is_never_saved() {
        let store = MemoryPilots::new();
        let pilot = Pilot::new(&catalog(), " ").expect("starts");
        assert_eq!(
            keeper(&store).save(&pilot),
            Err("nova: a pilot without a name is not saved".to_owned())
        );
        assert_eq!(store.writes(), 0);
    }

    #[test]
    fn a_failed_write_names_the_pilot_and_where() {
        let store = MemoryPilots::new();
        store.fail_writes(true);
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        assert_eq!(
            keeper(&store).save(&pilot),
            Err("nova: cannot save the pilot Ada in memory (the disk is full)".to_owned())
        );
        assert_eq!(store.keys(), Vec::<String>::new());
    }

    #[test]
    fn a_failed_read_or_list_names_where() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        keeper
            .save(&Pilot::new(&catalog(), "Ada").expect("starts"))
            .expect("saves");
        store.fail_reads(true);
        assert_eq!(
            keeper.open("Ada"),
            Err(OpenError::Unreadable(
                "nova: cannot read the pilot Ada in memory (the disk is unreadable)".to_owned()
            ))
        );
        assert_eq!(
            keeper.list(),
            Err("nova: cannot list the pilots in memory (the disk is unreadable)".to_owned())
        );
        assert!(!keeper.exists("Ada"), "unknown counts as not there");
    }

    #[test]
    fn a_missing_or_unusable_save_says_why() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        assert_eq!(
            keeper.open("Ada"),
            Err(OpenError::Unreadable(
                "nova: there is no pilot Ada in memory".to_owned()
            ))
        );
        store.put("Ada", "not a save");
        let message = keeper.open("Ada").expect_err("unusable").to_string();
        assert!(
            message.starts_with("nova: the pilot Ada in memory: This pilot file can't be used: "),
            "{message}"
        );
        let newer = crate::save::CURRENT + 1;
        store.put("Ada", &format!(r#"{{"version": {newer}}}"#));
        assert_eq!(
            keeper.open("Ada"),
            Err(OpenError::WrongVersion(format!(
                "nova: the pilot Ada in memory: {}",
                crate::save::SaveError::Newer { version: newer }
            )))
        );
    }

    #[test]
    fn a_newer_save_is_the_wrong_version_and_anything_else_unreadable() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        let newer = crate::save::CURRENT + 1;
        store.put("Ada", &format!(r#"{{"version": {newer}}}"#));
        assert!(
            matches!(keeper.open("Ada"), Err(OpenError::WrongVersion(_))),
            "-42's note"
        );
        assert!(matches!(keeper.open("Bea"), Err(OpenError::Unreadable(_))));
        store.put("Bea", "not a save");
        assert!(matches!(keeper.open("Bea"), Err(OpenError::Unreadable(_))));
        store.fail_reads(true);
        assert!(matches!(keeper.open("Ada"), Err(OpenError::Unreadable(_))));
    }

    #[test]
    fn an_open_error_shows_its_message() {
        for error in [
            OpenError::WrongVersion("newer".to_owned()),
            OpenError::Unreadable("gone".to_owned()),
        ] {
            let message = match &error {
                OpenError::WrongVersion(message) | OpenError::Unreadable(message) => {
                    message.clone()
                }
            };
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn deleting_a_pilot_removes_it_from_the_list() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        for name in ["Ada", "Bea"] {
            keeper
                .save(&Pilot::new(&catalog(), name).expect("starts"))
                .expect("saves");
        }
        assert_eq!(keeper.delete("Ada"), Ok(()));
        assert_eq!(keeper.list(), Ok(vec!["Bea".to_owned()]));
        assert!(keeper.open("Ada").is_err());
        assert_eq!(store.deleted(), ["Ada"]);
    }

    #[test]
    fn a_pilot_is_deleted_under_its_key() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        keeper
            .save(&Pilot::new(&catalog(), " Ada/B ").expect("starts"))
            .expect("saves");
        assert_eq!(keeper.delete(" Ada/B "), Ok(()));
        assert_eq!(store.keys(), Vec::<String>::new());
        assert_eq!(store.deleted(), ["Ada_B"]);
    }

    #[test]
    fn deleting_a_pilot_never_saved_is_ok() {
        let store = MemoryPilots::new();
        assert_eq!(keeper(&store).delete("Ada"), Ok(()));
        assert_eq!(store.keys(), Vec::<String>::new());
    }

    #[test]
    fn deleting_an_unnamed_pilot_is_ok_and_touches_nothing() {
        let store = MemoryPilots::new();
        store.put("_", "kept");
        assert_eq!(keeper(&store).delete(" "), Ok(()));
        assert_eq!(keeper(&store).delete(".."), Ok(()));
        assert_eq!(store.keys(), ["_"]);
        assert_eq!(store.deleted(), Vec::<String>::new());
    }

    #[test]
    fn a_failed_delete_names_the_pilot_and_where() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        keeper
            .save(&Pilot::new(&catalog(), "Ada").expect("starts"))
            .expect("saves");
        store.fail_deletes(true);
        assert_eq!(
            keeper.delete("Ada"),
            Err("nova: cannot delete the pilot Ada in memory (the disk is locked)".to_owned())
        );
        assert_eq!(store.keys(), ["Ada"]);
    }

    #[test]
    fn a_boxed_store_is_a_store() {
        let store = MemoryPilots::new();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        keeper
            .save(&Pilot::new(&catalog(), "Ada").expect("starts"))
            .expect("saves");
        assert_eq!(keeper.list(), Ok(vec!["Ada".to_owned()]));
        assert_eq!(
            keeper.open("Ada").map(|p| p.name().to_owned()),
            Ok("Ada".to_owned())
        );
        assert_eq!(keeper.store().location(), "memory");
        assert_eq!(store.writes(), 1);
        assert_eq!(keeper.delete("Ada"), Ok(()));
        assert_eq!(store.deleted(), ["Ada"]);
        store.put_last("Bea");
        assert_eq!(
            keeper.store().last_pilot().expect("reads").as_deref(),
            Some("Bea")
        );
        keeper.store().set_last_pilot("Cy").expect("writes");
        assert_eq!(store.last().as_deref(), Some("Cy"));
    }

    #[test]
    fn a_save_records_its_key_as_the_last_pilot() {
        let store = MemoryPilots::new();
        keeper(&store)
            .save(&Pilot::new(&catalog(), " Ada/B ").expect("starts"))
            .expect("saves");
        assert_eq!(store.last().as_deref(), Some("Ada_B"));
    }

    #[test]
    fn a_failed_save_records_nothing() {
        let store = MemoryPilots::new();
        store.put_last("Bea");
        store.fail_writes(true);
        let keeper = keeper(&store);
        assert!(
            keeper
                .save(&Pilot::new(&catalog(), "Ada").expect("starts"))
                .is_err()
        );
        assert!(
            keeper
                .save(&Pilot::new(&catalog(), " ").expect("starts"))
                .is_err()
        );
        assert_eq!(store.last().as_deref(), Some("Bea"));
    }

    #[test]
    fn a_failed_record_write_fails_neither_a_save_nor_an_open() {
        // `_WritePilotData` returns 0 whatever `_MakeAlias` did (@0x72dd8).
        let store = MemoryPilots::new();
        store.fail_last_writes(true);
        let keeper = keeper(&store);
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        assert_eq!(keeper.save(&pilot), Ok(()));
        assert_eq!(keeper.open("Ada"), Ok(pilot));
        assert_eq!(store.last(), None);
    }

    #[test]
    fn an_open_records_its_key_as_the_last_pilot() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        for name in ["Ada", "Bea"] {
            keeper
                .save(&Pilot::new(&catalog(), name).expect("starts"))
                .expect("saves");
        }
        assert_eq!(store.last().as_deref(), Some("Bea"));
        keeper.open("Ada").expect("opens");
        assert_eq!(store.last().as_deref(), Some("Ada"));
    }

    #[test]
    fn an_open_that_fails_records_nothing() {
        // -43, -45 and -42 all fail before `_MakeAlias` (@0x76333).
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        store.put_last("Bea");
        store.put("Bad", "not a save");
        let newer = crate::save::CURRENT + 1;
        store.put("New", &format!(r#"{{"version": {newer}}}"#));
        for key in ["Gone", "Bad", "New"] {
            assert!(keeper.open(key).is_err(), "{key}");
            assert_eq!(store.last().as_deref(), Some("Bea"), "{key}");
        }
        store.put(
            "Ada",
            &encode(&Pilot::new(&catalog(), "Ada").expect("starts")),
        );
        store.fail_reads(true);
        assert!(keeper.open("Ada").is_err());
        assert_eq!(store.last().as_deref(), Some("Bea"));
    }

    #[test]
    fn a_delete_leaves_the_last_pilot_record_naming_the_deleted_pilot() {
        // `_DeletePilotFile` (@0x72e6b-0x72ef6) never touches "Last Pilot".
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        keeper
            .save(&Pilot::new(&catalog(), "Ada").expect("starts"))
            .expect("saves");
        keeper.delete("Ada").expect("deletes");
        assert_eq!(store.last().as_deref(), Some("Ada"));
        assert_eq!(keeper.open_last(), None, "and the next launch opens none");
    }

    #[test]
    fn the_last_pilot_opens_the_pilot_its_record_names() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        let ada = travelled("Ada");
        keeper.save(&ada).expect("saves");
        keeper
            .save(&Pilot::new(&catalog(), "Bea").expect("starts"))
            .expect("saves");
        store.put_last("Ada");
        assert_eq!(keeper.open_last(), Some(ada));
        assert_eq!(store.last().as_deref(), Some("Ada"));
        assert_eq!(store.writes(), 2, "opening writes no pilot");
    }

    #[test]
    fn opening_the_last_pilot_records_it_again() {
        // `_LoadPilotData` rewrites the alias (@0x76333) on the launch load.
        let inner = MemoryPilots::new();
        inner.put(
            "Ada",
            &encode(&Pilot::new(&catalog(), "Ada").expect("starts")),
        );
        let store = RecordOnly {
            inner: inner.clone(),
            record: "Ada",
        };
        assert!(PilotKeeper::new(store).open_last().is_some());
        assert_eq!(inner.last().as_deref(), Some("Ada"));
    }

    /// A store whose record always reads `record`, while writes go to
    /// `inner`'s record, so a test can see the record written.
    struct RecordOnly {
        inner: MemoryPilots,
        record: &'static str,
    }

    impl PilotStore for RecordOnly {
        fn list(&self) -> io::Result<Vec<String>> {
            self.inner.list()
        }
        fn read(&self, key: &str) -> io::Result<Option<String>> {
            self.inner.read(key)
        }
        fn write(&self, key: &str, text: &str) -> io::Result<()> {
            self.inner.write(key, text)
        }
        fn delete(&self, key: &str) -> io::Result<()> {
            self.inner.delete(key)
        }
        fn last_pilot(&self) -> io::Result<Option<String>> {
            Ok(Some(self.record.to_owned()))
        }
        fn set_last_pilot(&self, key: &str) -> io::Result<()> {
            self.inner.set_last_pilot(key)
        }
        fn location(&self) -> String {
            self.inner.location()
        }
    }

    #[test]
    fn the_last_pilot_is_none_whenever_it_cannot_open() {
        let store = MemoryPilots::new();
        let keeper = keeper(&store);
        assert_eq!(keeper.open_last(), None, "no record");
        store.put_last("Gone");
        assert_eq!(keeper.open_last(), None, "nothing saved under the key");
        store.put("Bad", "not a save");
        store.put_last("Bad");
        assert_eq!(keeper.open_last(), None, "an unusable save");
        let newer = crate::save::CURRENT + 1;
        store.put("New", &format!(r#"{{"version": {newer}}}"#));
        store.put_last("New");
        assert_eq!(keeper.open_last(), None, "a newer save");
        store.put(
            "Ada",
            &encode(&Pilot::new(&catalog(), "Ada").expect("starts")),
        );
        store.put_last("Ada");
        store.fail_last_reads(true);
        assert_eq!(keeper.open_last(), None, "an unreadable record");
        store.fail_last_reads(false);
        store.fail_reads(true);
        assert_eq!(keeper.open_last(), None, "an unreadable save");
        store.fail_reads(false);
        assert!(keeper.open_last().is_some());
    }
}
