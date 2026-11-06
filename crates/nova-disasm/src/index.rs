//! The index over the listings: functions and their boundaries, symbol
//! lookup, and the call graph.

use std::collections::BTreeMap;

use crate::annotate::{self, Target};
use crate::parse::cstrings::CStrings;
use crate::parse::text::Insn;
use crate::parse::{cstrings, indirect, pointers, sections, symbols, text};
use crate::ports::Listings;

/// One function: a label and the instructions up to the next label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// The label's name.
    pub name: String,
    /// The address of its first instruction.
    pub start: u32,
    /// One past its end: the next label's address. For the last function
    /// in the section it is its last instruction's address plus one, as
    /// the listing doesn't give that instruction's length.
    pub end: u32,
}

impl Function {
    /// `end - start`.
    #[must_use]
    pub fn size(&self) -> u32 {
        self.end - self.start
    }
}

/// A data symbol from `nm` (anything but code, absolute values and imports).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataSymbol {
    /// Its name.
    pub name: String,
    /// Its address.
    pub start: u32,
    /// One past its end: the next symbol's address, or its section's end
    /// if that comes first. `None` when neither is known.
    pub end: Option<u32>,
}

/// A `find` result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The symbol's name.
    pub name: String,
    /// Its address.
    pub start: u32,
    /// One past its end, if known.
    pub end: Option<u32>,
    /// Whether it is a data symbol rather than a function.
    pub data: bool,
}

/// One callee of a function, in first-call order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Callee {
    /// The function or import called.
    pub name: String,
    /// How many call sites reach it, tail calls included.
    pub calls: usize,
    /// How many of those are tail calls (`jmp`).
    pub tail: usize,
}

/// A function's callees.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Callees {
    /// The named callees.
    pub named: Vec<Callee>,
    /// How many calls go through a register or memory (`calll *%eax`).
    pub indirect: usize,
}

/// One call site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    /// The calling instruction's address.
    pub addr: u32,
    /// Whether it is a tail call (`jmp`).
    pub tail: bool,
}

/// One caller of a symbol, with its call sites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
    /// The calling function's name.
    pub name: String,
    /// Its call sites, by address.
    pub sites: Vec<Site>,
}

/// The parsed listings and what is derived from them.
#[derive(Clone, Debug, Default)]
pub struct Index {
    functions: Vec<Function>,
    insns: Vec<Insn>,
    strings: CStrings,
    slots: BTreeMap<u32, u32>,
    imports: BTreeMap<u32, String>,
    data: Vec<DataSymbol>,
}

/// Parses an address as `0x68390` or `68390`. Text that has no decimal
/// digit (`bad`, `face`) is a name, not an address.
#[must_use]
pub fn parse_address(s: &str) -> Option<u32> {
    let digits = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(d) => d,
        None if s.bytes().any(|b| b.is_ascii_digit()) => s,
        None => return None,
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

impl Index {
    /// Builds the index from the listings.
    #[must_use]
    pub fn build(listings: &Listings) -> Index {
        let text = text::parse(&listings.text);
        let mut functions: Vec<Function> = Vec::with_capacity(text.labels.len());
        let last_end = text.insns.iter().map(|i| i.addr + 1).max().unwrap_or(0);
        for (i, label) in text.labels.iter().enumerate() {
            let end = text.labels.get(i + 1).map_or(last_end, |next| next.addr);
            functions.push(Function {
                name: label.name.clone(),
                start: label.addr,
                end,
            });
        }
        functions.sort_by_key(|f| f.start);
        let mut insns = text.insns;
        insns.sort_by_key(|i| i.addr);

        let all = symbols::parse(&listings.symbols);
        let sections = sections::parse(&listings.sections);
        let data = all
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_data())
            .map(|(i, s)| {
                // The first later address: past the symbol and its aliases.
                let next = all[i..].iter().map(|n| n.addr).find(|&a| a > s.addr);
                DataSymbol {
                    name: s.name.clone(),
                    start: s.addr,
                    end: cap_at_section(next, section_end(&sections, s.addr)),
                }
            })
            .collect();

        Index {
            functions,
            insns,
            strings: cstrings::parse(&listings.cstrings),
            slots: pointers::parse(&listings.pointers),
            imports: indirect::parse(&listings.indirect),
            data,
        }
    }

    /// Every function, by address.
    #[must_use]
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// The function named `name`, or `_name`.
    #[must_use]
    pub fn function_named(&self, name: &str) -> Option<&Function> {
        let exact = |n: &str| self.functions.iter().find(|f| f.name == n);
        exact(name).or_else(|| exact(&format!("_{name}")))
    }

    /// The function containing `addr`.
    #[must_use]
    pub fn function_at(&self, addr: u32) -> Option<&Function> {
        let at = self.functions.partition_point(|f| f.start <= addr);
        // Labels in a row share an address: the last of them holds the code.
        self.functions[..at].last().filter(|f| addr < f.end)
    }

    /// A function by name (exact, or with a leading `_` added), else by an
    /// address it contains.
    #[must_use]
    pub fn lookup(&self, query: &str) -> Option<&Function> {
        self.function_named(query)
            .or_else(|| parse_address(query).and_then(|addr| self.function_at(addr)))
    }

    /// A function's instruction lines.
    #[must_use]
    pub fn instructions(&self, f: &Function) -> &[Insn] {
        let from = self.insns.partition_point(|i| i.addr < f.start);
        let to = self.insns.partition_point(|i| i.addr < f.end);
        &self.insns[from..to.max(from)]
    }

    /// The listed strings.
    #[must_use]
    pub fn strings(&self) -> &CStrings {
        &self.strings
    }

    /// The import a `__pointers` slot names.
    pub fn import_slot(&self, addr: u32) -> Option<&str> {
        self.imports.get(&addr).map(String::as_str)
    }

    /// The value stored in a `__pointers` slot.
    #[must_use]
    pub fn slot_value(&self, addr: u32) -> Option<u32> {
        self.slots.get(&addr).copied()
    }

    /// The data symbol whose range holds `addr`, and the offset into it.
    /// A symbol with nothing after it holds only its own address.
    #[must_use]
    pub fn data_at(&self, addr: u32) -> Option<(&DataSymbol, u32)> {
        let at = self.data.partition_point(|d| d.start <= addr);
        let sym = self.data[..at].last()?;
        let inside = match sym.end {
            Some(end) => addr < end,
            None => addr == sym.start,
        };
        inside.then(|| (sym, addr - sym.start))
    }

    /// Functions and data symbols whose name holds `query` (ignoring
    /// case), or that contain the address `query`, by address.
    pub fn find(&self, query: &str) -> Vec<Found> {
        let mut found: Vec<Found> = if let Some(addr) = parse_address(query) {
            let function = self.function_at(addr).map(Found::function);
            let data = self.data_at(addr).map(|(d, _)| Found::data(d));
            function.into_iter().chain(data).collect()
        } else {
            let needle = query.to_lowercase();
            let hit = |name: &str| name.to_lowercase().contains(&needle);
            let functions = self
                .functions
                .iter()
                .filter(|f| hit(&f.name))
                .map(Found::function);
            let data = self.data.iter().filter(|d| hit(&d.name)).map(Found::data);
            functions.chain(data).collect()
        };
        found.sort_by_key(|f| f.start);
        found
    }

    /// What `f` calls, in first-call order.
    #[must_use]
    pub fn callees(&self, f: &Function) -> Callees {
        let mut callees = Callees::default();
        for (site, edge) in self.edges(f) {
            match edge {
                Edge::Indirect => callees.indirect += 1,
                Edge::Named(name) => {
                    let at = if let Some(at) = callees.named.iter().position(|c| c.name == name) {
                        at
                    } else {
                        callees.named.push(Callee {
                            name,
                            calls: 0,
                            tail: 0,
                        });
                        callees.named.len() - 1
                    };
                    callees.named[at].calls += 1;
                    callees.named[at].tail += usize::from(site.tail);
                }
            }
        }
        callees
    }

    /// The functions that call `name` (a function or an import), with
    /// their call sites, by address. `name` may leave off the leading `_`.
    #[must_use]
    pub fn callers(&self, name: &str) -> Vec<Caller> {
        let exact = self.callers_exact(name);
        if exact.is_empty() && !name.starts_with('_') {
            self.callers_exact(&format!("_{name}"))
        } else {
            exact
        }
    }

    fn callers_exact(&self, name: &str) -> Vec<Caller> {
        self.functions
            .iter()
            .filter_map(|f| {
                let sites: Vec<Site> = self
                    .edges(f)
                    .into_iter()
                    .filter(|(_, edge)| matches!(edge, Edge::Named(n) if n == name))
                    .map(|(site, _)| site)
                    .collect();
                (!sites.is_empty()).then(|| Caller {
                    name: f.name.clone(),
                    sites,
                })
            })
            .collect()
    }

    /// The calls `f` makes: every `call` to a function's start, an import
    /// or through a register, and every `jmp` to another function's start
    /// or an import (a tail call). A branch into the middle of a function
    /// (a loop, a PIC thunk's `calll` to the next line) is not a call.
    fn edges(&self, f: &Function) -> Vec<(Site, Edge)> {
        self.instructions(f)
            .iter()
            .filter_map(|insn| {
                let call = insn.mnemonic.starts_with("call");
                let tail = insn.mnemonic == "jmp";
                if !call && !tail {
                    return None;
                }
                let edge = match annotate::branch(self, insn)? {
                    Target::Function { name, offset: 0 } if call || name != f.name => {
                        Edge::Named(name)
                    }
                    Target::Import(name) => Edge::Named(name),
                    Target::Indirect if call => Edge::Indirect,
                    _ => return None,
                };
                Some((
                    Site {
                        addr: insn.addr,
                        tail,
                    },
                    edge,
                ))
            })
            .collect()
    }
}

/// The end of the section holding `addr`, if a listed one does.
fn section_end(sections: &[sections::Section], addr: u32) -> Option<u32> {
    let at = sections.partition_point(|s| s.addr <= addr);
    sections[..at]
        .last()
        .map(sections::Section::end)
        .filter(|&end| addr < end)
}

/// A data symbol's end: the next symbol, unless its section ends first.
/// The last symbol in a section ends with the section, not at a symbol in
/// a later one.
fn cap_at_section(next: Option<u32>, section_end: Option<u32>) -> Option<u32> {
    match (next, section_end) {
        (Some(next), Some(end)) => Some(next.min(end)),
        (next, end) => next.or(end),
    }
}

/// Where one call goes.
enum Edge {
    Named(String),
    Indirect,
}

impl Found {
    fn function(f: &Function) -> Found {
        Found {
            name: f.name.clone(),
            start: f.start,
            end: Some(f.end),
            data: false,
        }
    }

    fn data(d: &DataSymbol) -> Found {
        Found {
            name: d.name.clone(),
            start: d.start,
            end: d.end,
            data: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture;

    fn index() -> Index {
        Index::build(&fixture::listings())
    }

    fn build(text: &str, symbols: &str) -> Index {
        Index::build(&Listings {
            text: text.to_owned(),
            symbols: symbols.to_owned(),
            ..Listings::default()
        })
    }

    #[test]
    fn functions_lists_every_label_by_address() {
        let index = index();
        let names: Vec<_> = index.functions().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["_Alpha", "_Beta", "_Gamma", "_Delta"]);
    }

    #[test]
    fn a_sign_is_not_an_address() {
        assert_eq!(parse_address("0x+1f"), None);
        assert_eq!(parse_address("+1f"), None);
    }

    #[test]
    fn data_symbols_at_one_address_end_at_the_next_address() {
        let index = build(
            "",
            "00002000 d _first\n00002000 d _alias\n00002008 d _next\n",
        );
        let ends: Vec<_> = index
            .find("0x2000")
            .iter()
            .chain(&index.find("_first"))
            .map(|f| (f.name.clone(), f.end))
            .collect();
        assert_eq!(
            ends,
            [
                ("_alias".to_owned(), Some(0x2008)),
                ("_first".to_owned(), Some(0x2008))
            ]
        );
    }

    #[test]
    fn a_data_symbol_ends_where_a_code_symbol_starts() {
        let index = build("", "00002000 d _table\n00002008 T _code\n");
        let (sym, off) = index.data_at(0x2007).unwrap();
        assert_eq!((sym.name.as_str(), off), ("_table", 7));
        assert!(index.data_at(0x2008).is_none());
    }

    const UNDERSCORES: &str = "/x:\n\
        _main:\n\
        00001000\tcalll\tfoo\n\
        00001005\tcalll\t__x\n\
        0000100a\tret\n\
        foo:\n\
        00001010\tret\n\
        __x:\n\
        00001020\tret\n";

    fn caller_names(index: &Index, name: &str) -> Vec<String> {
        index.callers(name).into_iter().map(|c| c.name).collect()
    }

    #[test]
    fn a_name_without_an_underscore_that_has_callers_keeps_them() {
        let index = build(UNDERSCORES, "");
        assert_eq!(caller_names(&index, "foo"), ["_main"]);
    }

    #[test]
    fn a_name_with_an_underscore_gets_no_second_one() {
        let index = build(UNDERSCORES, "");
        assert!(caller_names(&index, "_x").is_empty());
        assert_eq!(caller_names(&index, "__x"), ["_main"]);
    }

    #[test]
    fn a_jmp_to_its_own_start_or_through_a_register_is_not_a_call() {
        let index = build(
            "/x:\n\
             _loop:\n\
             00001000\tnop\n\
             00001001\tjmp\t0x1000\n\
             00001006\tjmp\t*%eax\n",
            "",
        );
        let callees = index.callees(index.function_named("_loop").unwrap());
        assert!(callees.named.is_empty(), "{:?}", callees.named);
        assert_eq!(callees.indirect, 0);
    }

    #[test]
    fn a_call_to_its_own_start_is_recursion() {
        let index = build(
            "/x:\n\
             _rec:\n\
             00001000\tpushl\t%ebp\n\
             00001001\tcalll\t0x1000\n\
             00001006\tret\n",
            "",
        );
        let callees = index.callees(index.function_named("_rec").unwrap());
        assert_eq!(
            callees.named,
            [Callee {
                name: "_rec".into(),
                calls: 1,
                tail: 0
            }]
        );
        assert_eq!(
            index.callers("_rec"),
            [Caller {
                name: "_rec".into(),
                sites: vec![Site {
                    addr: 0x1001,
                    tail: false
                }]
            }]
        );
    }

    fn range(f: &Function) -> (&str, u32, u32) {
        (f.name.as_str(), f.start, f.end)
    }

    #[test]
    fn a_function_ends_where_the_next_label_starts() {
        let index = index();
        let beta = index.function_named("_Beta").unwrap();
        assert_eq!(range(beta), ("_Beta", 0x3050, 0x3080));
        assert_eq!(beta.size(), 0x30);
        let insns = index.instructions(beta);
        assert_eq!(insns.first().unwrap().addr, 0x3050);
        assert_eq!(insns.last().unwrap().addr, 0x3061);
    }

    #[test]
    fn the_last_function_ends_after_its_last_instruction() {
        let index = index();
        let delta = index.function_named("_Delta").unwrap();
        assert_eq!(range(delta), ("_Delta", 0x30a0, 0x30a2));
        assert_eq!(index.instructions(delta).len(), 2);
    }

    #[test]
    fn a_name_may_leave_off_its_underscore() {
        let index = index();
        assert_eq!(index.function_named("Gamma").unwrap().start, 0x3080);
        assert!(index.function_named("Gam").is_none());
    }

    #[test]
    fn an_address_finds_the_function_containing_it() {
        let index = index();
        assert_eq!(index.function_at(0x3050).unwrap().name, "_Beta");
        assert_eq!(index.function_at(0x307f).unwrap().name, "_Beta");
        assert!(index.function_at(0x2fff).is_none());
        assert!(index.function_at(0x30a2).is_none());
    }

    #[test]
    fn lookup_takes_a_name_or_an_address() {
        let index = index();
        assert_eq!(index.lookup("_Alpha").unwrap().name, "_Alpha");
        assert_eq!(index.lookup("0x3055").unwrap().name, "_Beta");
        assert_eq!(index.lookup("3081").unwrap().name, "_Gamma");
        assert!(index.lookup("0x10").is_none());
        assert!(index.lookup("nope").is_none());
    }

    #[test]
    fn parses_addresses_with_or_without_0x() {
        assert_eq!(parse_address("0x68390"), Some(0x68390));
        assert_eq!(parse_address("68390"), Some(0x68390));
        assert_eq!(parse_address("face"), None);
        assert_eq!(parse_address("0x"), None);
        assert_eq!(parse_address("_A1"), None);
    }

    #[test]
    fn data_symbols_cover_up_to_the_next_symbol() {
        let index = index();
        let (sym, off) = index.data_at(0xdd064).unwrap();
        assert_eq!((sym.name.as_str(), off), ("_kScale", 4));
        assert_eq!(index.data_at(0xdd070).unwrap().0.name, "_gTable");
        assert_eq!(index.data_at(0x23_1798).unwrap().0.name, "_gLast");
        assert!(index.data_at(0x23_1799).is_none());
        assert!(index.data_at(0xdd05f).is_none());
    }

    #[test]
    fn the_last_data_symbol_in_a_section_ends_at_the_section_s_end() {
        // _kScale is the last symbol in __TEXT,__const (0xdd060..0xdd068);
        // __DATA,__cfstring after it has no symbol before _gTable at 0xdd070.
        let index = index();
        let (sym, off) = index.data_at(0xdd067).unwrap();
        assert_eq!((sym.name.as_str(), off), ("_kScale", 7));
        assert!(index.data_at(0xdd068).is_none());
        assert!(index.find("0xdd068").is_empty());
        assert_eq!(index.find("_kScale")[0].end, Some(0xdd068));
    }

    #[test]
    fn within_a_section_a_data_symbol_still_ends_at_the_next_symbol() {
        let index = index();
        assert_eq!(index.find("_gTable")[0].end, Some(0x18_6820));
        // _otherStatic is the last symbol in __DATA,__data.
        assert_eq!(index.find("_otherStatic")[0].end, Some(0x18_6828));
        assert!(index.data_at(0x18_6828).is_none());
    }

    #[test]
    fn a_data_symbol_in_no_listed_section_ends_at_the_next_symbol() {
        // The fixture lists no __bss: _quickKeyCheckResult runs to _gLast.
        let index = index();
        assert_eq!(index.find("_quickKey")[0].end, Some(0x23_1798));
    }

    #[test]
    fn the_last_symbol_of_all_ends_at_its_section_s_end() {
        let index = Index::build(&Listings {
            symbols: "00002000 d _only\n".to_owned(),
            sections: "Section\n  sectname __data\n   segname __DATA\n      addr 0x00002000\n      size 0x00000010\n".to_owned(),
            ..Listings::default()
        });
        assert_eq!(index.find("_only")[0].end, Some(0x2010));
        assert_eq!(index.data_at(0x200f).unwrap().1, 0xf);
    }

    #[test]
    fn a_data_symbol_at_a_section_s_end_is_in_no_section() {
        // _after sits at __data's end, in no listed section: it runs to the
        // next symbol, as it would with no sections listed.
        let index = Index::build(&Listings {
            symbols: "00002000 d _in\n00002010 d _after\n00002020 d _next\n".to_owned(),
            sections: "Section\n  sectname __data\n   segname __DATA\n      addr 0x00002000\n      size 0x00000010\n".to_owned(),
            ..Listings::default()
        });
        let (sym, off) = index.data_at(0x2010).unwrap();
        assert_eq!((sym.name.as_str(), off), ("_after", 0));
        assert_eq!(sym.end, Some(0x2020));
    }

    #[test]
    fn an_absolute_symbol_is_not_data() {
        // `__mh_execute_header` (type A) would otherwise swallow small
        // stack displacements like `0x1004(%esp)`.
        let index = index();
        assert!(index.data_at(0x1004).is_none());
        assert!(index.find("mh_execute").is_empty());
    }

    #[test]
    fn find_matches_a_substring_ignoring_case() {
        let found = index().find("alp");
        assert_eq!(
            found,
            [Found {
                name: "_Alpha".into(),
                start: 0x3000,
                end: Some(0x3050),
                data: false
            }]
        );
    }

    #[test]
    fn find_includes_data_symbols_by_address_order() {
        let found = index().find("_g");
        let names: Vec<_> = found.iter().map(|f| (f.name.as_str(), f.data)).collect();
        assert_eq!(
            names,
            [("_Gamma", false), ("_gTable", true), ("_gLast", true)]
        );
        assert_eq!(found[2].end, None);
        assert_eq!(found[1].end, Some(0x18_6820));
    }

    #[test]
    fn find_takes_an_address() {
        let found = index().find("0x3085");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "_Gamma");
        assert_eq!(index().find("dd061")[0].name, "_kScale");
    }

    #[test]
    fn callees_are_unique_with_counts_and_an_indirect_tally() {
        let index = index();
        let callees = index.callees(index.function_named("_Alpha").unwrap());
        let named: Vec<_> = callees
            .named
            .iter()
            .map(|c| (c.name.as_str(), c.calls, c.tail))
            .collect();
        assert_eq!(
            named,
            [
                ("_Beta", 2, 0),
                ("_Gamma", 1, 0),
                ("_SysBeep", 1, 0),
                ("_objc_msgSend", 1, 0)
            ]
        );
        assert_eq!(callees.indirect, 1);
    }

    #[test]
    fn a_jmp_to_another_function_is_a_tail_call() {
        let index = index();
        let callees = index.callees(index.function_named("_Beta").unwrap());
        let named: Vec<_> = callees
            .named
            .iter()
            .map(|c| (c.name.as_str(), c.calls, c.tail))
            .collect();
        // The `jne` inside _Beta and the PIC `calll` inside _Gamma aren't calls.
        assert_eq!(
            named,
            [("_Gamma", 1, 0), ("_Delta", 1, 1), ("_SysBeep", 1, 1)]
        );
        assert!(
            index
                .callees(index.function_named("_Gamma").unwrap())
                .named
                .is_empty()
        );
    }

    #[test]
    fn callers_list_each_function_with_its_sites() {
        let index = index();
        let callers = index.callers("_Gamma");
        assert_eq!(
            callers,
            [
                Caller {
                    name: "_Alpha".into(),
                    sites: vec![Site {
                        addr: 0x3008,
                        tail: false
                    }]
                },
                Caller {
                    name: "_Beta".into(),
                    sites: vec![Site {
                        addr: 0x3051,
                        tail: false
                    }]
                },
            ]
        );
    }

    #[test]
    fn an_import_has_callers_and_a_tail_call_is_marked() {
        let index = index();
        let callers = index.callers("SysBeep");
        assert_eq!(
            callers,
            [
                Caller {
                    name: "_Alpha".into(),
                    sites: vec![Site {
                        addr: 0x300d,
                        tail: false
                    }]
                },
                Caller {
                    name: "_Beta".into(),
                    sites: vec![Site {
                        addr: 0x3061,
                        tail: true
                    }]
                },
            ]
        );
        assert!(index.callers("_Nobody").is_empty());
    }

    #[test]
    fn calls_resolve_through_the_target_helper() {
        let index = index();
        let insn = index.instructions(index.function_named("_Alpha").unwrap())[3].clone();
        assert_eq!(
            annotate::branch(&index, &insn),
            Some(Target::Function {
                name: "_Gamma".into(),
                offset: 0
            })
        );
    }
}
