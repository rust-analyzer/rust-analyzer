//! Syntax highlighting for asm macro strings.
use ide_db::SymbolKind;
use syntax::{
    ast::{self, FormatSpecifier, HasFormatSpecifier},
    AstNode, AstToken, TextRange,
};

use crate::{syntax_highlighting::highlights::Highlights, HlRange, HlTag};

enum AsmPart {
    Comment,
    Directive,
    FormatClose,
    FormatIdentifier,
    FormatOpen,
    Instruction,
    Label,
    NumericLiteral,
    Register,
}

pub(super) fn highlight_asm_string(stack: &mut Highlights, string: &ast::String, range: TextRange) {
    if is_asm_string(string).is_none() {
        return;
    }

    log::info!("Highlighting asm string");
    let _timer = stdx::timeit("highlighting asm string");
    lex_asm_string(string, |piece_range, kind| {
        stack.add(HlRange {
            range: piece_range + range.start(),
            highlight: highlight_asm_part(kind).into(),
            binding_hash: None,
        });
    });
}

fn is_asm_string(string: &ast::String) -> Option<()> {
    let parent = string.syntax().parent()?;

    let name = parent.parent().and_then(ast::MacroCall::cast)?.path()?.segment()?.name_ref()?;
    if !matches!(name.text().as_str(), "asm" | "global_asm") {
        return None;
    }

    let first_literal = parent
        .children_with_tokens()
        .find_map(|it| it.as_token().cloned().and_then(ast::String::cast))?;

    if &first_literal != string {
        return None;
    }

    Some(())
}

fn highlight_asm_part(kind: AsmPart) -> HlTag {
    match kind {
        AsmPart::Comment => HlTag::Comment,
        AsmPart::Directive => HlTag::Attribute,
        AsmPart::FormatIdentifier => HlTag::Symbol(SymbolKind::Local),
        AsmPart::FormatOpen | AsmPart::FormatClose => HlTag::FormatSpecifier,
        AsmPart::Instruction => HlTag::Symbol(SymbolKind::Function),
        AsmPart::Label => HlTag::Symbol(SymbolKind::LifetimeParam),
        AsmPart::NumericLiteral => HlTag::NumericLiteral,
        AsmPart::Register => HlTag::Symbol(SymbolKind::Static),
    }
}

fn lex_asm_string<F>(string: &ast::String, mut callback: F)
where
    F: FnMut(TextRange, AsmPart),
{
    let char_ranges = match string.char_ranges() {
        Some(char_ranges) => char_ranges,
        None => return,
    };

    let mut chars =
        char_ranges.iter().filter_map(|(r, c)| Some((*r, c.as_ref().ok().copied()?))).peekable();

    while let Some(&(range, first_char)) = chars.peek() {
        match first_char {
            '{' => {
                callback(range, AsmPart::FormatOpen);
                chars.next();

                if let Some(&(_, c)) = chars.peek() {
                    if IdentKind::Rust.is_valid_starting_char(c) {
                        callback(
                            read_identifier(&mut chars, IdentKind::Rust),
                            AsmPart::FormatIdentifier,
                        );
                    }
                }
            }
            '}' => {
                callback(range, AsmPart::FormatClose);
                chars.next();
            }
            // either a label or a directive, we'll find out at the end
            '.' => {
                let mut range = read_identifier(&mut chars, IdentKind::Asm);
                skip_insignificant_whitespace(&mut chars);

                let part = match chars.peek() {
                    Some((_, ':')) => {
                        chars.next();
                        AsmPart::Label
                    }
                    _ => {
                        range = range.cover(read_until_eol(&mut chars));
                        AsmPart::Directive
                    }
                };

                callback(range, part);
            }
            '#' => callback(read_until_eol(&mut chars), AsmPart::Comment),
            // either a label or instruction
            c if c.is_alphabetic() => {
                let range = read_identifier(&mut chars, IdentKind::Asm);
                skip_insignificant_whitespace(&mut chars);

                match chars.peek() {
                    Some((_, ':')) => {
                        chars.next();
                        callback(range, AsmPart::Label);
                    }
                    _ => {
                        callback(range, AsmPart::Instruction);
                        skip_insignificant_whitespace(&mut chars);

                        while let Some(&(r, c)) = chars.peek() {
                            match c {
                                '\n' | '\r' => {
                                    if let ('\r', Some((_, '\n'))) = (c, chars.peek()) {
                                        chars.next();
                                    }

                                    break;
                                }
                                ' ' | '\t' | ',' | ':' => {
                                    chars.next();
                                }
                                '{' => {
                                    callback(r, AsmPart::FormatOpen);
                                    chars.next();

                                    if let Some(&(_, c)) = chars.peek() {
                                        if IdentKind::Rust.is_valid_starting_char(c) {
                                            callback(
                                                read_identifier(&mut chars, IdentKind::Rust),
                                                AsmPart::FormatIdentifier,
                                            );
                                        }
                                    }
                                }
                                '}' => {
                                    callback(r, AsmPart::FormatClose);
                                    chars.next();
                                }
                                '#' => {
                                    callback(read_until_eol(&mut chars), AsmPart::Comment);
                                    break;
                                }
                                c if IdentKind::Asm.is_valid_starting_char(c) => {
                                    let range = read_identifier(&mut chars, IdentKind::Asm);

                                    let (s, e) = (
                                        u32::from(range.start()) as usize,
                                        u32::from(range.end()) as usize,
                                    );

                                    match is_register(&string.text()[s..e]) {
                                        true => callback(range, AsmPart::Register),
                                        false => callback(range, AsmPart::Label),
                                    }
                                }
                                c if c.is_ascii_digit() => {
                                    eprintln!("highlighting numeric literal");
                                    callback(
                                        read_numeric_literal(&mut chars),
                                        AsmPart::NumericLiteral,
                                    )
                                }
                                _ => {
                                    chars.next();
                                }
                            }
                        }
                    }
                }
            }
            c if c.is_ascii_digit() => {
                callback(read_numeric_literal(&mut chars), AsmPart::NumericLiteral)
            }
            _ => {
                chars.next();
            }
        }
    }
}

#[derive(Clone, Copy)]
enum IdentKind {
    Asm,
    Rust,
}

impl IdentKind {
    fn is_valid_starting_char(self, c: char) -> bool {
        match self {
            IdentKind::Asm => c == '.' || c == '_' || c == '$' || c.is_alphabetic(),
            IdentKind::Rust => c == '_' || c.is_alphabetic(),
        }
    }

    fn is_valid_following_char(self, c: char) -> bool {
        match self {
            IdentKind::Asm => c == '.' || c == '_' || c == '$' || c.is_alphanumeric(),
            IdentKind::Rust => c == '_' || c.is_alphanumeric(),
        }
    }
}

fn read_identifier<I>(chars: &mut std::iter::Peekable<I>, kind: IdentKind) -> TextRange
where
    I: Iterator<Item = (TextRange, char)>,
{
    let (mut range, c) = chars.next().unwrap();
    assert!(kind.is_valid_starting_char(c));

    while let Some((r, _)) = chars.next_if(|(_, c)| kind.is_valid_following_char(*c)) {
        range = range.cover(r);
    }

    range
}

fn read_until_eol<I>(chars: &mut std::iter::Peekable<I>) -> TextRange
where
    I: Iterator<Item = (TextRange, char)>,
{
    let (mut range, _) = chars.next().unwrap();

    while let Some((r, c)) = chars.next() {
        if c == '\n' || c == '\r' {
            if let ('\r', Some((_, '\n'))) = (c, chars.peek()) {
                chars.next();
            }

            break;
        }

        range = range.cover(r);
    }

    range
}

fn skip_insignificant_whitespace<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = (TextRange, char)>,
{
    while chars.next_if(|(_, c)| *c == '\t' || *c == ' ').is_some() {}
}

fn read_numeric_literal<I>(chars: &mut std::iter::Peekable<I>) -> TextRange
where
    I: Iterator<Item = (TextRange, char)>,
{
    let (mut range, c) = chars.next().unwrap();

    if c == '0' {
        if let Some((_, 'b')) | Some((_, 'x')) | Some((_, 'o')) = chars.peek() {
            chars.next();
        }
    }

    while let Some((r, _)) = chars.next_if(|(_, c)| c.is_ascii_hexdigit()) {
        range = range.cover(r);
    }

    range
}

fn is_register(s: &str) -> bool {
    [
        "RAX",
        "EAX",
        "AX",
        "AH",
        "AL",
        "RBX",
        "EBX",
        "BX",
        "BH",
        "BL",
        "RCX",
        "ECX",
        "CX",
        "CH",
        "CL",
        "RDX",
        "EDX",
        "DX",
        "DH",
        "DL",
        "RSI",
        "ESI",
        "SI",
        "SIL",
        "RDI",
        "EDI",
        "DI",
        "DIL",
        "RSP",
        "ESP",
        "SP",
        "SPL",
        "RBP",
        "EBP",
        "BP",
        "BPL",
        "R8",
        "R8D",
        "R8W",
        "R8B",
        "R9",
        "R9D",
        "R9W",
        "R9B",
        "R10",
        "R10D",
        "R10W",
        "R10B",
        "R11",
        "R11D",
        "R11W",
        "R11B",
        "R12",
        "R12D",
        "R12W",
        "R12B",
        "R13",
        "R13D",
        "R13W",
        "R13B",
        "R14",
        "R14D",
        "R14W",
        "R14B",
        "R15",
        "R15D",
        "R15W",
        "R15B",
        "CR0",
        "CR1",
        "CR2",
        "CR3",
        "CR4",
        "CR5",
        "CR6",
        "CR7",
        "CR8",
        "CR9",
        "CR10",
        "CR11",
        "CR12",
        "CR13",
        "CR14",
        "CR15",
        "X0",
        "X1",
        "X2",
        "X3",
        "X4",
        "X5",
        "X6",
        "X7",
        "X8",
        "X9",
        "X10",
        "X11",
        "X12",
        "X13",
        "X14",
        "X15",
        "X16",
        "X17",
        "X18",
        "X19",
        "X20",
        "X21",
        "X22",
        "X23",
        "X24",
        "X25",
        "X26",
        "X27",
        "X28",
        "X29",
        "X10",
        "X11",
        "X12",
        "X13",
        "X14",
        "X15",
        "X30",
        "X31",
        "zero",
        "ra",
        "gp",
        "tp",
        "t0",
        "t1",
        "t2",
        "t3",
        "t4",
        "t5",
        "t6",
        "a0",
        "a1",
        "a2",
        "a3",
        "a4",
        "a5",
        "a6",
        "s0",
        "s1",
        "s2",
        "s3",
        "s4",
        "s5",
        "s6",
        "s7",
        "s8",
        "s9",
        "s10",
        "s11",
        "F0",
        "F1",
        "F2",
        "F3",
        "F4",
        "F5",
        "F6",
        "F7",
        "F8",
        "F9",
        "F10",
        "F11",
        "F12",
        "F13",
        "F14",
        "F15",
        "F16",
        "F17",
        "F18",
        "F19",
        "F20",
        "F21",
        "F22",
        "F23",
        "F24",
        "F25",
        "F26",
        "F27",
        "F28",
        "F29",
        "F30",
        "F31",
        "V0",
        "V1",
        "V2",
        "V3",
        "V4",
        "V5",
        "V6",
        "V7",
        "V8",
        "V9",
        "V10",
        "V11",
        "V12",
        "V13",
        "V14",
        "V15",
        "V16",
        "V17",
        "V18",
        "V19",
        "V20",
        "V21",
        "V22",
        "V23",
        "V24",
        "V25",
        "V26",
        "V27",
        "V28",
        "V29",
        "V30",
        "V31",
        "USTATUS",
        "UIE",
        "UTVEC",
        "USCRATCH",
        "UEPC",
        "UCAUSE",
        "UTVAL",
        "UIP",
        "FFLAGS",
        "FRM",
        "FCSR",
        "CYCLE",
        "TIME",
        "INSTRET",
        "HPMCOUNTER3",
        "HPMCOUNTER4",
        "HPMCOUNTER5",
        "HPMCOUNTER6",
        "HPMCOUNTER7",
        "HPMCOUNTER8",
        "HPMCOUNTER9",
        "HPMCOUNTER10",
        "HPMCOUNTER11",
        "HPMCOUNTER12",
        "HPMCOUNTER13",
        "HPMCOUNTER14",
        "HPMCOUNTER15",
        "HPMCOUNTER16",
        "HPMCOUNTER17",
        "HPMCOUNTER18",
        "HPMCOUNTER19",
        "HPMCOUNTER20",
        "HPMCOUNTER21",
        "HPMCOUNTER22",
        "HPMCOUNTER23",
        "HPMCOUNTER24",
        "HPMCOUNTER25",
        "HPMCOUNTER26",
        "HPMCOUNTER27",
        "HPMCOUNTER28",
        "HPMCOUNTER29",
        "HPMCOUNTER30",
        "HPMCOUNTER31",
        "SSTATUS",
        "SEDELEG",
        "SIDELEG",
        "SIE",
        "STVEC",
        "SCOUNTEREN",
        "SSCRATCH",
        "SEPC",
        "SCAUSE",
        "STVAL",
        "SIP",
        "SATP",
        "MVENDORID",
        "MARCHID",
        "MIMPID",
        "MHARTID",
        "MSTATUS",
        "MISA",
        "MEDELEG",
        "MIDELEG",
        "MIE",
        "MTVEC",
        "MCOUNTEREN",
        "MSCRATCH",
        "MEPC",
        "MCAUSE",
        "MTVAL",
        "MIP",
        "PMPCFG0",
        "PMPCFG2",
        "PMPADDR0",
        "PMPADDR1",
        "PMPADDR2",
        "PMPADDR3",
        "PMPADDR4",
        "PMPADDR5",
        "PMPADDR6",
        "PMPADDR7",
        "PMPADDR8",
        "PMPADDR9",
        "PMPADDR10",
        "PMPADDR11",
        "PMPADDR12",
        "PMPADDR13",
        "PMPADDR14",
        "PMPADDR15",
        "MCYCLE",
        "MINSTRET",
        "MHPMCOUNTER3",
        "MHPMCOUNTER4",
        "MHPMCOUNTER5",
        "MHPMCOUNTER6",
        "MHPMCOUNTER7",
        "MHPMCOUNTER8",
        "MHPMCOUNTER9",
        "MHPMCOUNTER10",
        "MHPMCOUNTER11",
        "MHPMCOUNTER12",
        "MHPMCOUNTER13",
        "MHPMCOUNTER14",
        "MHPMCOUNTER15",
        "MHPMCOUNTER16",
        "MHPMCOUNTER17",
        "MHPMCOUNTER18",
        "MHPMCOUNTER19",
        "MHPMCOUNTER20",
        "MHPMCOUNTER21",
        "MHPMCOUNTER22",
        "MHPMCOUNTER23",
        "MHPMCOUNTER24",
        "MHPMCOUNTER25",
        "MHPMCOUNTER26",
        "MHPMCOUNTER27",
        "MHPMCOUNTER28",
        "MHPMCOUNTER29",
        "MHPMCOUNTER30",
        "MHPMCOUNTER31",
        "MCOUNTINHIBIT",
        "MHPMEVENT3",
        "MHPMEVENT4",
        "MHPMEVENT5",
        "MHPMEVENT6",
        "MHPMEVENT7",
        "MHPMEVENT8",
        "MHPMEVENT9",
        "MHPMEVENT10",
        "MHPMEVENT11",
        "MHPMEVENT12",
        "MHPMEVENT13",
        "MHPMEVENT14",
        "MHPMEVENT15",
        "MHPMEVENT16",
        "MHPMEVENT17",
        "MHPMEVENT18",
        "MHPMEVENT19",
        "MHPMEVENT20",
        "MHPMEVENT21",
        "MHPMEVENT22",
        "MHPMEVENT23",
        "MHPMEVENT24",
        "MHPMEVENT25",
        "MHPMEVENT26",
        "MHPMEVENT27",
        "MHPMEVENT28",
        "MHPMEVENT29",
        "MHPMEVENT30",
        "MHPMEVENT31",
        "TSELECT",
        "TDATA1",
        "TDATA2",
        "TDATA3",
        "DCSR",
        "DPC",
        "DSCRATCH0",
        "DSCRATCH1",
    ]
    .iter()
    .any(|r| r.eq_ignore_ascii_case(s))
}
