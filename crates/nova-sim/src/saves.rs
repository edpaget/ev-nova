//! Keeping pilots: where they are saved, and the rules for saving and
//! opening them.
//!
//! - [`PilotStore`]: the port pilots are saved through, as text under a
//!   key.
//! - [`pilot_key`]: the key a pilot is saved under, from its name.
//! - [`PilotKeeper`]: the rules. It saves a pilot in the
//!   [`save`](crate::save) schema under its key, lists the pilots saved
//!   and opens one, and reports each problem as a message naming where
//!   the pilots are.

use std::io;

use crate::pilot::Pilot;
use crate::save::{decode, encode};

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
    /// A message naming the pilot and where the pilots are, when nothing
    /// is saved under `key`, it cannot be read, or it is not a pilot this
    /// version can use.
    pub fn open(&self, key: &str) -> Result<Pilot, String> {
        let location = self.store.location();
        let text = self
            .store
            .read(key)
            .map_err(|error| format!("nova: cannot read the pilot {key} in {location} ({error})"))?
            .ok_or_else(|| format!("nova: there is no pilot {key} in {location}"))?;
        decode(&text).map_err(|error| format!("nova: the pilot {key} in {location}: {error}"))
    }

    /// Saves `pilot` under its key, replacing any pilot saved there.
    ///
    /// # Errors
    ///
    /// A message saying why, when the pilot has no name to save it under,
    /// or naming the pilot and where the pilots are when it cannot be
    /// written.
    pub fn save(&self, pilot: &Pilot) -> Result<(), String> {
        let key = pilot_key(pilot.name())
            .ok_or_else(|| "nova: a pilot without a name is not saved".to_owned())?;
        self.store.write(&key, &encode(pilot)).map_err(|error| {
            format!(
                "nova: cannot save the pilot {} in {} ({error})",
                pilot.name(),
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
        assert_eq!(resumed.landed(), Some(StellarId(140)));
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
            Err("nova: cannot read the pilot Ada in memory (the disk is unreadable)".to_owned())
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
            Err("nova: there is no pilot Ada in memory".to_owned())
        );
        store.put("Ada", "not a save");
        let message = keeper.open("Ada").expect_err("unusable");
        assert!(
            message.starts_with("nova: the pilot Ada in memory: This pilot file can't be used: "),
            "{message}"
        );
        store.put("Ada", r#"{"version": 11}"#);
        assert_eq!(
            keeper.open("Ada"),
            Err(format!(
                "nova: the pilot Ada in memory: {}",
                crate::save::SaveError::Newer { version: 11 }
            ))
        );
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
    }
}
