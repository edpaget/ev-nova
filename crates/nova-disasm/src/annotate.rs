//! What an instruction's operands refer to: a call or jump target, a
//! string, an imported or static global, a data symbol.

use crate::index::Index;
use crate::parse::cstrings::StringRef;
use crate::parse::text::Insn;

/// Where a call or jump goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Into a function, `offset` bytes from its start (0 for a call).
    Function {
        /// The function's name.
        name: String,
        /// Bytes from its start.
        offset: u32,
    },
    /// An imported function or Objective-C message, through a stub.
    Import(String),
    /// Through a register or memory (`*%eax`).
    Indirect,
    /// An address in no function.
    Unknown(u32),
}

/// The target of a call or jump instruction, or `None` for any other.
#[must_use]
pub fn branch(index: &Index, insn: &Insn) -> Option<Target> {
    if !insn.mnemonic.starts_with("call") && !insn.mnemonic.starts_with('j') {
        return None;
    }
    if insn.operands.starts_with('*') {
        return Some(Target::Indirect);
    }
    if let Some(comment) = &insn.comment
        && let Some(name) = comment
            .strip_prefix("symbol stub for: ")
            .or_else(|| comment.strip_prefix("Objc message: "))
    {
        return Some(Target::Import(name.trim().to_owned()));
    }
    let operand = insn.operands.as_str();
    if let Some(hex) = operand.strip_prefix("0x") {
        let addr = u32::from_str_radix(hex, 16).ok()?;
        return Some(match index.function_at(addr) {
            Some(f) => Target::Function {
                name: f.name.clone(),
                offset: addr - f.start,
            },
            None => Target::Unknown(addr),
        });
    }
    (!operand.is_empty()).then(|| Target::Function {
        name: operand.to_owned(),
        offset: 0,
    })
}

/// What the instruction's operands name, beyond what otool's own comment
/// already says: a jump into a function, an indirect call, a string, a
/// `__pointers` global or a data symbol.
#[must_use]
pub fn annotations(index: &Index, insn: &Insn) -> Vec<String> {
    if let Some(target) = branch(index, insn) {
        let bare = insn.operands.starts_with("0x");
        return match target {
            Target::Function { name, offset: 0 } if bare => vec![name],
            Target::Function { name, offset } if offset > 0 => vec![format!("{name}+{offset:#x}")],
            Target::Indirect => vec!["<indirect>".to_owned()],
            _ => Vec::new(),
        };
    }
    numbers(&insn.operands)
        .into_iter()
        .filter_map(|(value, immediate)| {
            if immediate {
                string(index, value)
            } else {
                global(index, value)
            }
        })
        .collect()
}

/// Every `0x..` number in the operands, and whether it is an immediate
/// (`$0x..`). Negative numbers (`-0x20(%ebp)`) are stack offsets and
/// left out.
fn numbers(operands: &str) -> Vec<(u32, bool)> {
    let mut out = Vec::new();
    let mut rest = operands;
    while let Some(at) = rest.find("0x") {
        let prefix = rest[..at].chars().next_back();
        let digits: String = rest[at + 2..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect();
        if prefix != Some('-')
            && let Ok(value) = u32::from_str_radix(&digits, 16)
        {
            out.push((value, prefix == Some('$')));
        }
        rest = &rest[at + 2 + digits.len()..];
    }
    out
}

/// An immediate in the string section: the string, or its address.
fn string(index: &Index, addr: u32) -> Option<String> {
    Some(match index.strings().string_at(addr)? {
        StringRef::Listed { string, offset } => {
            let shown = match string.pascal() {
                Some(text) => format!("p\"{text}\""),
                None => format!("\"{}\"", string.text),
            };
            if offset == 0 {
                shown
            } else {
                format!("{shown}+{offset}")
            }
        }
        StringRef::Unlisted => format!("str@{addr:#x}"),
    })
}

/// A memory address: an import's or a static's pointer slot, or a data
/// symbol.
fn global(index: &Index, addr: u32) -> Option<String> {
    if let Some(name) = index.import_slot(addr) {
        return Some(name.to_owned());
    }
    if let Some(value) = index.slot_value(addr) {
        return Some(match data(index, value) {
            Some(name) => format!("-> {name}"),
            None => format!("-> {value:#x}"),
        });
    }
    data(index, addr)
}

/// `_sym` or `_sym+0x8`.
fn data(index: &Index, addr: u32) -> Option<String> {
    let (sym, offset) = index.data_at(addr)?;
    Some(if offset == 0 {
        sym.name.clone()
    } else {
        format!("{}+{offset:#x}", sym.name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture;

    fn index() -> Index {
        Index::build(&fixture::listings())
    }

    fn insn_at(index: &Index, addr: u32) -> Insn {
        let f = index.function_at(addr).unwrap();
        index
            .instructions(f)
            .iter()
            .find(|i| i.addr == addr)
            .unwrap()
            .clone()
    }

    fn target(addr: u32) -> Option<Target> {
        let index = index();
        branch(&index, &insn_at(&index, addr))
    }

    fn notes(addr: u32) -> Vec<String> {
        let index = index();
        annotations(&index, &insn_at(&index, addr))
    }

    #[test]
    fn a_call_by_name_keeps_the_name() {
        assert_eq!(
            target(0x3003),
            Some(Target::Function {
                name: "_Beta".into(),
                offset: 0
            })
        );
        assert!(notes(0x3003).is_empty());
    }

    #[test]
    fn a_call_to_a_bare_address_resolves_to_the_function_there() {
        assert_eq!(
            target(0x3008),
            Some(Target::Function {
                name: "_Gamma".into(),
                offset: 0
            })
        );
        assert_eq!(notes(0x3008), ["_Gamma"]);
    }

    #[test]
    fn a_stub_call_resolves_to_the_import() {
        assert_eq!(target(0x300d), Some(Target::Import("_SysBeep".into())));
        assert_eq!(target(0x3046), Some(Target::Import("_objc_msgSend".into())));
    }

    #[test]
    fn a_call_through_a_register_is_indirect() {
        assert_eq!(target(0x3012), Some(Target::Indirect));
        assert_eq!(notes(0x3012), ["<indirect>"]);
    }

    #[test]
    fn a_jump_inside_a_function_shows_its_offset() {
        assert_eq!(
            target(0x3056),
            Some(Target::Function {
                name: "_Beta".into(),
                offset: 0x10
            })
        );
        assert_eq!(notes(0x3056), ["_Beta+0x10"]);
        assert_eq!(notes(0x3081), ["_Gamma+0x6"]);
    }

    #[test]
    fn other_instructions_have_no_branch_target() {
        assert_eq!(target(0x3000), None);
        assert!(notes(0x3000).is_empty());
    }

    #[test]
    fn a_string_reference_shows_the_string() {
        assert_eq!(notes(0x3014), ["\"Hello world\""]);
    }

    #[test]
    fn a_reference_into_a_string_shows_the_offset() {
        assert_eq!(notes(0x301b), ["\"Hello world\"+3"]);
    }

    #[test]
    fn a_pascal_string_is_marked() {
        assert_eq!(notes(0x3022), ["p\"Pilot\""]);
    }

    #[test]
    fn a_reference_to_an_unlisted_string_is_recorded_as_its_address() {
        assert_eq!(notes(0x3029), ["str@0xd8216"]);
    }

    #[test]
    fn a_local_pointer_resolves_through_nm() {
        assert_eq!(notes(0x3030), ["-> _quickKeyCheckResult"]);
    }

    #[test]
    fn an_import_pointer_names_the_import() {
        assert_eq!(notes(0x3035), ["_errno"]);
    }

    #[test]
    fn a_direct_data_reference_names_the_symbol() {
        assert_eq!(notes(0x303a), ["_kScale+0x4"]);
    }

    #[test]
    fn a_local_pointer_to_no_symbol_shows_its_value() {
        let mut listings = fixture::listings();
        listings.symbols.clear();
        let index = Index::build(&listings);
        assert_eq!(
            annotations(&index, &insn_at(&index, 0x3030)),
            ["-> 0x231790"]
        );
    }

    #[test]
    fn numbers_reads_every_number_after_a_long_operand() {
        assert_eq!(numbers("%eax,%ebx,%ecx,0x1,$0x2"), [(1, false), (2, true)]);
    }

    #[test]
    fn numbers_reads_a_number_right_after_a_long_one() {
        assert_eq!(numbers("0x12345,0x6"), [(0x12345, false), (6, false)]);
    }

    #[test]
    fn small_displacements_and_negative_offsets_name_nothing() {
        let insn = Insn {
            addr: 0x3000,
            mnemonic: "movl".into(),
            operands: "-0xdd064(%ebp), %eax".into(),
            comment: None,
            raw: String::new(),
        };
        assert!(annotations(&index(), &insn).is_empty());
    }

    #[test]
    fn an_immediate_outside_the_strings_names_nothing() {
        let insn = Insn {
            addr: 0x3000,
            mnemonic: "movl".into(),
            operands: "$0xdd064, %eax".into(),
            comment: None,
            raw: String::new(),
        };
        assert!(annotations(&index(), &insn).is_empty());
    }
}
