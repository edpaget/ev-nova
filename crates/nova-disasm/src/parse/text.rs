//! `otool -tV`: labels and instruction lines.

/// A function label (`_Name:`) and the address of the instruction after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// The label without its colon.
    pub name: String,
    /// The address of the first instruction after the label.
    pub addr: u32,
}

/// One instruction line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Insn {
    /// Its address.
    pub addr: u32,
    /// The mnemonic (`calll`, `movl`, ...).
    pub mnemonic: String,
    /// The operands, without otool's comment.
    pub operands: String,
    /// otool's own `## ...` comment, without the `## `.
    pub comment: Option<String>,
    /// The line exactly as otool printed it.
    pub raw: String,
}

/// The labels and instructions of a listing, in listing order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    /// The labels; a label with no instruction after it is dropped.
    pub labels: Vec<Label>,
    /// Every instruction line.
    pub insns: Vec<Insn>,
}

/// Parses an `otool -tV` listing.
#[must_use]
pub fn parse(listing: &str) -> Text {
    let mut text = Text::default();
    let mut pending: Vec<String> = Vec::new();
    // The first line is the binary's path and a colon, like a label.
    for line in listing.lines().skip(1) {
        if let Some(insn) = insn(line) {
            for name in pending.drain(..) {
                text.labels.push(Label {
                    name,
                    addr: insn.addr,
                });
            }
            text.insns.push(insn);
        } else if let Some(name) = line.strip_suffix(':')
            && !line.starts_with('(')
            && !name.is_empty()
        {
            pending.push(name.to_owned());
        }
    }
    text
}

/// Parses `<8 hex>\t<mnemonic>[\t<operands>[   ## comment]]`.
fn insn(line: &str) -> Option<Insn> {
    let mut fields = line.splitn(3, '\t');
    let addr = super::hex(fields.next()?)?;
    let mnemonic = fields.next()?.trim().to_owned();
    let rest = fields.next().unwrap_or("");
    let (operands, comment) = match rest.split_once("##") {
        Some((ops, comment)) => (ops, Some(comment.trim().to_owned())),
        None => (rest, None),
    };
    Some(Insn {
        addr,
        mnemonic,
        operands: operands.trim().to_owned(),
        comment,
        raw: line.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture::TEXT;

    #[test]
    fn skips_the_two_header_lines_and_reads_the_labels() {
        let text = parse(TEXT);
        let labels: Vec<_> = text
            .labels
            .iter()
            .map(|l| (l.name.as_str(), l.addr))
            .collect();
        assert_eq!(
            labels,
            [
                ("_Alpha", 0x3000),
                ("_Beta", 0x3050),
                ("_Gamma", 0x3080),
                ("_Delta", 0x30a0)
            ]
        );
    }

    #[test]
    fn splits_an_instruction_into_its_fields() {
        let text = parse(TEXT);
        let insn = text.insns.iter().find(|i| i.addr == 0x300d).unwrap();
        assert_eq!(insn.mnemonic, "calll");
        assert_eq!(insn.operands, "0x3b63a1");
        assert_eq!(insn.comment.as_deref(), Some("symbol stub for: _SysBeep"));
        assert!(insn.raw.starts_with("0000300d\tcalll\t0x3b63a1 "));
    }

    #[test]
    fn an_instruction_without_operands_or_comment() {
        let text = parse(TEXT);
        let insn = text.insns.iter().find(|i| i.addr == 0x304c).unwrap();
        assert_eq!(
            (insn.mnemonic.as_str(), insn.operands.as_str()),
            ("ret", "")
        );
        assert_eq!(insn.comment, None);
    }

    #[test]
    fn a_label_with_nothing_after_it_is_dropped() {
        let text = parse("/x:\n(__TEXT,__text) section\n_Empty:\n");
        assert!(text.labels.is_empty());
    }

    #[test]
    fn two_labels_in_a_row_share_the_next_address() {
        let text = parse("/x:\n(__TEXT,__text) section\n_A:\n_B:\n00001000\tret\n");
        let addrs: Vec<_> = text.labels.iter().map(|l| l.addr).collect();
        assert_eq!(addrs, [0x1000, 0x1000]);
    }

    #[test]
    fn objective_c_method_labels_keep_their_spaces() {
        let text = parse("/x:\n(__TEXT,__text) section\n-[Foo bar:]:\n00001000\tret\n");
        assert_eq!(text.labels[0].name, "-[Foo bar:]");
    }
}
