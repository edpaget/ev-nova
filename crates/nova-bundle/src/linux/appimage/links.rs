//! The glibc floor and the allowed libraries, checked on the built
//! binary from what `objdump -p` shows of it, as macOS's
//! [`check_archs`](crate::bundle::check_archs) checks `lipo -archs`.
//!
//! [`Links::parse`] reads the two things that matter from `objdump -p`'s
//! text: the `NEEDED` entries of the dynamic section, and the highest
//! `GLIBC_x.y` among the version references. [`Links::problems`] says
//! what the floor or the allowed list refuses, and [`check_links`] runs
//! `objdump` through the [`Process`] port and fails on any.

use std::fmt;
use std::path::Path;

use super::{ALLOWED_LIBRARIES, GLIBC_FLOOR};
use crate::bundle::{BundleError, run};
use crate::ports::{Invocation, Process};

/// A glibc symbol version, `GLIBC_<major>.<minor>[.<patch>]`, compared
/// part by part.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlibcVersion(pub Vec<u32>);

impl GlibcVersion {
    /// The version named by a `GLIBC_` symbol version, or `None` when its
    /// suffix is not dotted numbers (`GLIBC_PRIVATE`, `GLIBC_ABI_DT_RELR`).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let parts = name.strip_prefix("GLIBC_")?;
        parts
            .split('.')
            .map(|part| part.parse().ok())
            .collect::<Option<Vec<u32>>>()
            .map(Self)
    }
}

impl fmt::Display for GlibcVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(u32::to_string).collect();
        write!(f, "GLIBC_{}", parts.join("."))
    }
}

/// What `objdump -p` shows of a binary's links.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Links {
    /// The `NEEDED` libraries, in order.
    pub needed: Vec<String>,
    /// The highest numbered `GLIBC_` version referenced, if any.
    pub glibc: Option<GlibcVersion>,
    /// The `GLIBC_` versions that name no number, in order.
    pub unnumbered: Vec<String>,
}

/// One thing the floor or the allowed list refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The binary needs a glibc above the floor.
    AboveFloor(GlibcVersion),
    /// It needs a glibc version that is not a number, which the floor
    /// can't vouch for.
    Unnumbered(String),
    /// It links a library outside [`ALLOWED_LIBRARIES`].
    Disallowed(String),
    /// It needs no library at all: not the dynamically linked ELF the
    /// build makes, or `objdump` showed something else.
    NoNeeded,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AboveFloor(version) => write!(
                f,
                "it needs {version}, above the floor {}",
                GlibcVersion(GLIBC_FLOOR.to_vec())
            ),
            Self::Unnumbered(name) => write!(f, "it needs {name}, which the floor can't vouch for"),
            Self::Disallowed(library) => {
                write!(f, "it links {library}, which is not an allowed library")
            }
            Self::NoNeeded => write!(f, "it needs no library: is it a dynamically linked ELF?"),
        }
    }
}

/// The problems, joined for a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problems(pub Vec<Problem>);

impl fmt::Display for Problems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown: Vec<String> = self.0.iter().map(Problem::to_string).collect();
        write!(f, "{}", shown.join("; "))
    }
}

impl Links {
    /// Reads `objdump -p`'s output: each `NEEDED <library>` line of the
    /// dynamic section, and every `GLIBC_` word, however it is spaced.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut links = Self::default();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            if words.next() == Some("NEEDED")
                && let Some(library) = words.next()
            {
                links.needed.push(library.to_owned());
            }
            for word in line.split_whitespace().filter(|w| w.starts_with("GLIBC_")) {
                match GlibcVersion::parse(word) {
                    Some(version) => {
                        if links.glibc.as_ref().is_none_or(|top| version > *top) {
                            links.glibc = Some(version);
                        }
                    }
                    None => {
                        if !links.unnumbered.iter().any(|seen| seen == word) {
                            links.unnumbered.push(word.to_owned());
                        }
                    }
                }
            }
        }
        links
    }

    /// What the floor ([`GLIBC_FLOOR`]) and the allowed list
    /// ([`ALLOWED_LIBRARIES`]) refuse, in order: the glibc version, then
    /// each library outside the list.
    #[must_use]
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if let Some(version) = &self.glibc
            && version.0.as_slice() > GLIBC_FLOOR.as_slice()
        {
            problems.push(Problem::AboveFloor(version.clone()));
        }
        problems.extend(self.unnumbered.iter().cloned().map(Problem::Unnumbered));
        if self.needed.is_empty() {
            problems.push(Problem::NoNeeded);
        }
        problems.extend(
            self.needed
                .iter()
                .filter(|library| !ALLOWED_LIBRARIES.contains(&library.as_str()))
                .cloned()
                .map(Problem::Disallowed),
        );
        problems
    }
}

/// `objdump -p` on `binary`, captured.
#[must_use]
pub fn objdump(binary: &Path) -> Invocation {
    Invocation::new("objdump").arg("-p").arg(binary).captured()
}

/// Runs `objdump -p` on `binary` and fails, naming the command and what
/// it found, when the binary needs a glibc above the floor or links a
/// library outside the allowed list.
pub fn check_links(process: &mut impl Process, binary: &Path) -> Result<(), BundleError> {
    let invocation = objdump(binary);
    let stdout = run(process, &invocation)?;
    let problems = Links::parse(&String::from_utf8_lossy(&stdout)).problems();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(BundleError::NotPortable {
            command: invocation.to_string(),
            problems: Problems(problems),
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::fmt::Write as _;
    use std::path::PathBuf;

    use super::*;
    use crate::ports::RunError;

    /// `objdump -p` of a binary needing `needed` and, from libc.so.6, the
    /// glibc `versions`, laid out as binutils prints it.
    pub(crate) fn objdump_text(needed: &[&str], versions: &[&str]) -> String {
        let mut text = String::from(
            "\nnova:     file format elf64-x86-64\n\nProgram Header:\n    \
             LOAD off    0x0000000000000000 vaddr 0x0000000000000000 align 2**12\n\n\
             Dynamic Section:\n",
        );
        for library in needed {
            let _ = writeln!(text, "  NEEDED               {library}");
        }
        text.push_str("  INIT                 0x00000000000a3000\n\nVersion References:\n");
        text.push_str("  required from libgcc_s.so.1:\n    0x0b792650 0x00 13 GCC_3.0\n");
        if !versions.is_empty() {
            text.push_str("  required from libc.so.6:\n");
            for (i, version) in versions.iter().enumerate() {
                let _ = writeln!(text, "    0x09691a75 0x00 {:02} {version}", i + 2);
            }
        }
        text
    }

    /// What the build links today.
    pub(crate) const NEEDED: [&str; 5] = [
        "libasound.so.2",
        "libgcc_s.so.1",
        "libm.so.6",
        "libc.so.6",
        "ld-linux-x86-64.so.2",
    ];

    pub(crate) fn portable_text() -> String {
        objdump_text(&NEEDED, &["GLIBC_2.2.5", "GLIBC_2.34", "GLIBC_2.3.4"])
    }

    fn version(text: &str) -> GlibcVersion {
        GlibcVersion::parse(text).expect("a version")
    }

    #[test]
    fn reads_the_needed_libraries_and_the_highest_glibc() {
        let links = Links::parse(&portable_text());
        assert_eq!(links.needed, NEEDED);
        assert_eq!(links.glibc, Some(version("GLIBC_2.34")));
        assert!(links.unnumbered.is_empty());
        assert_eq!(links.problems(), []);
    }

    #[test]
    fn versions_compare_part_by_part() {
        assert!(version("GLIBC_2.10") > version("GLIBC_2.9"));
        assert!(version("GLIBC_2.3.4") > version("GLIBC_2.3"));
        assert!(version("GLIBC_2.3.4") < version("GLIBC_2.4"));
        assert_eq!(version("GLIBC_2.35").to_string(), "GLIBC_2.35");
        assert_eq!(version("GLIBC_2.2.5").to_string(), "GLIBC_2.2.5");
        for name in [
            "GLIBC_PRIVATE",
            "GLIBC_ABI_DT_RELR",
            "GLIBC_2.x",
            "GCC_3.0",
            "GLIBC_",
        ] {
            assert_eq!(GlibcVersion::parse(name), None, "{name}");
        }
    }

    #[test]
    fn the_floor_itself_passes() {
        let links = Links::parse(&objdump_text(&NEEDED, &["GLIBC_2.35", "GLIBC_2.2.5"]));
        assert_eq!(links.glibc, Some(version("GLIBC_2.35")));
        assert_eq!(links.problems(), []);
    }

    #[test]
    fn a_glibc_above_the_floor_is_refused() {
        for above in ["GLIBC_2.36", "GLIBC_2.35.1", "GLIBC_3.0"] {
            let links = Links::parse(&objdump_text(&NEEDED, &["GLIBC_2.2.5", above]));
            assert_eq!(
                links.problems(),
                [Problem::AboveFloor(version(above))],
                "{above}"
            );
        }
    }

    #[test]
    fn a_glibc_version_that_is_not_a_number_is_refused_once() {
        let links = Links::parse(&objdump_text(
            &NEEDED,
            &["GLIBC_2.2.5", "GLIBC_ABI_DT_RELR", "GLIBC_ABI_DT_RELR"],
        ));
        assert_eq!(
            links.problems(),
            [Problem::Unnumbered("GLIBC_ABI_DT_RELR".into())]
        );
    }

    #[test]
    fn a_library_outside_the_allowed_list_is_refused() {
        let needed = [
            "libc.so.6",
            "libwayland-client.so.0",
            "libm.so.6",
            "libstdc++.so.6",
        ];
        let links = Links::parse(&objdump_text(&needed, &["GLIBC_2.2.5"]));
        assert_eq!(
            links.problems(),
            [
                Problem::Disallowed("libwayland-client.so.0".into()),
                Problem::Disallowed("libstdc++.so.6".into()),
            ]
        );
    }

    #[test]
    fn every_allowed_library_passes() {
        let links = Links::parse(&objdump_text(&ALLOWED_LIBRARIES, &["GLIBC_2.2.5"]));
        assert_eq!(links.problems(), []);
    }

    #[test]
    fn no_version_references_pass_the_floor() {
        let links = Links::parse(&objdump_text(&NEEDED, &[]));
        assert_eq!(links.glibc, None);
        assert_eq!(links.problems(), []);
    }

    #[test]
    fn no_needed_library_is_refused() {
        assert_eq!(Links::parse("").problems(), [Problem::NoNeeded]);
        let links = Links::parse(&objdump_text(&[], &["GLIBC_2.40"]));
        assert_eq!(
            links.problems(),
            [
                Problem::AboveFloor(version("GLIBC_2.40")),
                Problem::NoNeeded
            ]
        );
    }

    #[test]
    fn odd_spacing_reads_the_same() {
        let text = "\tNEEDED\tlibc.so.6\r\n NEEDED   libm.so.6  \n\
                    \t\t0x0 0x00 02\tGLIBC_2.17\r\n  0x0 0x00 03 GLIBC_2.28  \nNEEDED\n";
        let links = Links::parse(text);
        assert_eq!(links.needed, ["libc.so.6", "libm.so.6"]);
        assert_eq!(links.glibc, Some(version("GLIBC_2.28")));
        assert_eq!(links.problems(), []);
    }

    #[test]
    fn a_needed_word_elsewhere_in_a_line_is_not_an_entry() {
        let links = Links::parse("  required NEEDED libfoo.so.1\n  NEEDED libc.so.6\n");
        assert_eq!(links.needed, ["libc.so.6"]);
    }

    #[test]
    fn the_problems_read_as_a_sentence() {
        let problems = Problems(vec![
            Problem::AboveFloor(version("GLIBC_2.38")),
            Problem::Unnumbered("GLIBC_ABI_DT_RELR".into()),
            Problem::Disallowed("libfoo.so.1".into()),
            Problem::NoNeeded,
        ]);
        assert_eq!(
            problems.to_string(),
            "it needs GLIBC_2.38, above the floor GLIBC_2.35; \
             it needs GLIBC_ABI_DT_RELR, which the floor can't vouch for; \
             it links libfoo.so.1, which is not an allowed library; \
             it needs no library: is it a dynamically linked ELF?"
        );
    }

    /// Answers every run with `stdout`, recording each.
    struct Answers {
        stdout: Result<Vec<u8>, ()>,
        runs: RefCell<Vec<Invocation>>,
    }

    impl Process for Answers {
        fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError> {
            self.runs.borrow_mut().push(invocation.clone());
            self.stdout.clone().map_err(|()| RunError::Failed(Some(1)))
        }
    }

    fn check(stdout: Result<Vec<u8>, ()>) -> (Result<(), BundleError>, Vec<Invocation>) {
        let mut process = Answers {
            stdout,
            runs: RefCell::new(Vec::new()),
        };
        let binary = PathBuf::from("staged").join("nova");
        let result = check_links(&mut process, &binary);
        (result, process.runs.into_inner())
    }

    #[test]
    fn checks_the_binary_with_objdump_captured() {
        let (result, runs) = check(Ok(portable_text().into_bytes()));
        assert!(result.is_ok(), "{result:?}");
        let binary = PathBuf::from("staged").join("nova");
        assert_eq!(
            runs,
            [Invocation::new("objdump").arg("-p").arg(&binary).captured()]
        );
    }

    #[test]
    fn a_binary_the_floor_refuses_names_the_command_and_what_it_found() {
        let text = objdump_text(&["libc.so.6", "libfoo.so.1"], &["GLIBC_2.39"]);
        let (result, _) = check(Ok(text.into_bytes()));
        let binary = PathBuf::from("staged").join("nova");
        let command = objdump(&binary).to_string();
        assert_eq!(
            result.expect_err("refused").to_string(),
            format!(
                "`{command}` shows a binary that won't run on every Linux the AppImage is for: \
                 it needs GLIBC_2.39, above the floor GLIBC_2.35; \
                 it links libfoo.so.1, which is not an allowed library"
            )
        );
    }

    #[test]
    fn a_failing_objdump_names_its_command() {
        let (result, _) = check(Err(()));
        let Err(BundleError::Run { command, .. }) = result else {
            panic!("{result:?}");
        };
        assert!(command.starts_with("objdump -p"), "{command}");
    }
}
