//! The text each command prints.

use std::fmt::Write as _;

use crate::annotate;
use crate::index::{Callees, Caller, Found, Function, Index};

/// `fn`: a header (`_Name 0xSTART..0xEND (N bytes)`), then each
/// instruction line as otool printed it, with what it refers to after
/// `  ; `.
#[must_use]
pub fn function(index: &Index, f: &Function) -> String {
    let mut out = format!(
        "{} {:#x}..{:#x} ({} bytes)\n",
        f.name,
        f.start,
        f.end,
        f.size()
    );
    for insn in index.instructions(f) {
        out.push_str(&insn.raw);
        let notes = annotate::annotations(index, insn);
        if !notes.is_empty() {
            let _ = write!(out, "  ; {}", notes.join(", "));
        }
        out.push('\n');
    }
    out
}

/// `callees`: one line per callee (`_Name  xN`, with `(N tail)` when
/// some are tail calls), then `<indirect>  xN` if any.
#[must_use]
pub fn callees(callees: &Callees) -> String {
    let mut out = String::new();
    for c in &callees.named {
        let _ = write!(out, "{}  x{}", c.name, c.calls);
        if c.tail > 0 {
            let _ = write!(out, " ({} tail)", c.tail);
        }
        out.push('\n');
    }
    if callees.indirect > 0 {
        let _ = writeln!(out, "<indirect>  x{}", callees.indirect);
    }
    out
}

/// `callers`: one line per calling function, with its call sites.
#[must_use]
pub fn callers(callers: &[Caller]) -> String {
    let mut out = String::new();
    for caller in callers {
        let sites: Vec<String> = caller
            .sites
            .iter()
            .map(|s| {
                if s.tail {
                    format!("{:#x} (tail)", s.addr)
                } else {
                    format!("{:#x}", s.addr)
                }
            })
            .collect();
        let _ = writeln!(out, "{}  {}", caller.name, sites.join(", "));
    }
    out
}

/// `find`: `name  0xSTART..0xEND  size`, data symbols marked `[data]`.
#[must_use]
pub fn found(found: &[Found]) -> String {
    let mut out = String::new();
    for f in found {
        let (end, size) = match f.end {
            Some(end) => (format!("{end:#x}"), (end - f.start).to_string()),
            None => ("?".to_owned(), "?".to_owned()),
        };
        let _ = write!(out, "{}  {:#x}..{end}  {size}", f.name, f.start);
        if f.data {
            out.push_str("  [data]");
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture;

    fn index() -> Index {
        Index::build(&fixture::listings())
    }

    #[test]
    fn a_function_has_a_header_and_its_lines() {
        let index = index();
        let out = function(&index, index.function_named("_Gamma").unwrap());
        assert_eq!(
            out,
            "_Gamma 0x3080..0x30a0 (32 bytes)\n\
             00003080\tpushl\t%ebp\n\
             00003081\tcalll\t0x3086  ; _Gamma+0x6\n\
             00003086\tpopl\t%eax\n\
             00003087\tret\n"
        );
    }

    #[test]
    fn annotations_follow_otools_own_comment() {
        let index = index();
        let out = function(&index, index.function_named("_Alpha").unwrap());
        assert!(out.contains(
            "00003014\tmovl\t$0xd8200, (%esp)                ## imm = 0xD8200  ; \"Hello world\"\n"
        ));
        assert!(out.contains("00003035\tmovl\t0x3b5004, %eax  ; _errno\n"));
    }

    #[test]
    fn callees_show_counts_tails_and_indirect_calls() {
        let index = index();
        let alpha = callees(&index.callees(index.function_named("_Alpha").unwrap()));
        assert_eq!(
            alpha,
            "_Beta  x2\n_Gamma  x1\n_SysBeep  x1\n_objc_msgSend  x1\n<indirect>  x1\n"
        );
        let beta = callees(&index.callees(index.function_named("_Beta").unwrap()));
        assert_eq!(
            beta,
            "_Gamma  x1\n_Delta  x1 (1 tail)\n_SysBeep  x1 (1 tail)\n"
        );
    }

    #[test]
    fn callers_show_each_site() {
        let index = index();
        assert_eq!(
            callers(&index.callers("_SysBeep")),
            "_Alpha  0x300d\n_Beta  0x3061 (tail)\n"
        );
        assert_eq!(callers(&index.callers("_Beta")), "_Alpha  0x3003, 0x3041\n");
    }

    #[test]
    fn found_shows_range_and_size() {
        let index = index();
        assert_eq!(found(&index.find("alpha")), "_Alpha  0x3000..0x3050  80\n");
        assert_eq!(
            found(&index.find("_g")),
            "_Gamma  0x3080..0x30a0  32\n\
             _gTable  0xdd070..0x186820  694192  [data]\n\
             _gLast  0x231798..?  ?  [data]\n"
        );
    }
}
