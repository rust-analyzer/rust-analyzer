//! Types that are shared between rust-analyzer and the proc macro server.

use std::fmt;

use arrayvec::ArrayString;
use intern::Symbol;

#[cfg(feature = "in-ra")]
type DefaultSpan = span::Span;
#[cfg(not(feature = "in-ra"))]
pub enum DefaultSpan {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
// The discriminants are important for `storage.rs` decoding.
pub enum IdentKind {
    Normal = 0,
    Raw = 1,
    ForcedKeyword = 2,
}

impl IdentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            IdentKind::Normal => "",
            IdentKind::Raw => "r#",
            IdentKind::ForcedKeyword => "k#",
        }
    }

    pub fn split_from_symbol(sym: &str) -> (Self, &str) {
        match sym.as_bytes() {
            [b'r', b'#', ..] => (IdentKind::Raw, &sym["r#".len()..]),
            [b'k', b'#', ..] => (IdentKind::ForcedKeyword, &sym["k#".len()..]),
            _ => (IdentKind::Normal, sym),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum LitKind {
    Byte,
    Char,
    Integer, // e.g. `1`, `1u8`, `1f32`
    Float,   // e.g. `1.`, `1.0`, `1e3f32`
    Str,
    StrRaw(u8), // raw string delimited by `n` hash symbols
    ByteStr,
    ByteStrRaw(u8), // raw byte string delimited by `n` hash symbols
    CStr,
    CStrRaw(u8),
    Err(()),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Leaf<Span = DefaultSpan> {
    Literal(Literal<Span>),
    Punct(Punct<Span>),
    Ident(Ident<Span>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DelimSpan<Span = DefaultSpan> {
    pub open: Span,
    pub close: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Delimiter<Span = DefaultSpan> {
    pub open: Span,
    pub close: Span,
    pub kind: DelimiterKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
// The discriminants are important for decoding for `storage.rs`.
pub enum DelimiterKind {
    Parenthesis = 0,
    Brace = 1,
    Bracket = 2,
    Invisible = 3,
}

impl DelimiterKind {
    pub fn display_open_close(self) -> (&'static str, &'static str) {
        match self {
            DelimiterKind::Brace => ("{", "}"),
            DelimiterKind::Bracket => ("[", "]"),
            DelimiterKind::Parenthesis => ("(", ")"),
            DelimiterKind::Invisible => ("", ""),
        }
    }

    pub fn debug_view(self) -> &'static str {
        match self {
            DelimiterKind::Invisible => "$$",
            DelimiterKind::Parenthesis => "()",
            DelimiterKind::Brace => "{}",
            DelimiterKind::Bracket => "[]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Literal<Span = DefaultSpan> {
    /// Escaped, text then suffix concatenated.
    pub text_and_suffix: Symbol,
    pub span: Span,
    pub kind: LitKind,
    pub suffix_len: u8,
}

impl<Span> Literal<Span> {
    #[inline]
    pub fn text_and_suffix(&self) -> (&str, &str) {
        let text_and_suffix = self.text_and_suffix.as_str();
        text_and_suffix.split_at(text_and_suffix.len() - usize::from(self.suffix_len))
    }

    pub fn text_and_suffix_symbols(&self) -> (Symbol, Option<Symbol>) {
        if self.suffix_len == 0 {
            (self.text_and_suffix.clone(), None)
        } else {
            let (text, suffix) = self.text_and_suffix();
            (Symbol::intern(text), Some(Symbol::intern(suffix)))
        }
    }

    #[inline]
    pub fn text(&self) -> &str {
        self.text_and_suffix().0
    }

    #[inline]
    pub fn suffix(&self) -> &str {
        self.text_and_suffix().1
    }

    pub fn new(text: &str, span: Span, kind: LitKind, suffix: &str) -> Self {
        const MAX_INLINE_CAPACITY: usize = 30;
        let text_and_suffix = if suffix.is_empty() {
            Symbol::intern(text)
        } else if (text.len() + suffix.len()) < MAX_INLINE_CAPACITY {
            let mut text_and_suffix = ArrayString::<MAX_INLINE_CAPACITY>::new();
            text_and_suffix.push_str(text);
            text_and_suffix.push_str(suffix);
            Symbol::intern(&text_and_suffix)
        } else {
            let mut text_and_suffix = String::with_capacity(text.len() + suffix.len());
            text_and_suffix.push_str(text);
            text_and_suffix.push_str(suffix);
            Symbol::intern(&text_and_suffix)
        };

        Self { text_and_suffix, span, kind, suffix_len: suffix.len().try_into().unwrap() }
    }

    #[inline]
    pub fn new_no_suffix(text: &str, span: Span, kind: LitKind) -> Self {
        Self { text_and_suffix: Symbol::intern(text), span, kind, suffix_len: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Punct<Span = DefaultSpan> {
    pub char: char,
    pub spacing: Spacing,
    pub span: Span,
}

/// Indicates whether a token can join with the following token to form a
/// compound token. Used for conversions to `proc_macro::Spacing`. Also used to
/// guide pretty-printing, which is where the `JointHidden` value (which isn't
/// part of `proc_macro::Spacing`) comes in useful.
// The discriminants are important for decoding for `storage.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Spacing {
    /// The token cannot join with the following token to form a compound
    /// token.
    ///
    /// In token streams parsed from source code, the compiler will use `Alone`
    /// for any token immediately followed by whitespace, a non-doc comment, or
    /// EOF.
    ///
    /// When constructing token streams within the compiler, use this for each
    /// token that (a) should be pretty-printed with a space after it, or (b)
    /// is the last token in the stream. (In the latter case the choice of
    /// spacing doesn't matter because it is never used for the last token. We
    /// arbitrarily use `Alone`.)
    ///
    /// Converts to `proc_macro::Spacing::Alone`, and
    /// `proc_macro::Spacing::Alone` converts back to this.
    Alone = 0,

    /// The token can join with the following token to form a compound token.
    ///
    /// In token streams parsed from source code, the compiler will use `Joint`
    /// for any token immediately followed by punctuation (as determined by
    /// `Token::is_punct`).
    ///
    /// When constructing token streams within the compiler, use this for each
    /// token that (a) should be pretty-printed without a space after it, and
    /// (b) is followed by a punctuation token.
    ///
    /// Converts to `proc_macro::Spacing::Joint`, and
    /// `proc_macro::Spacing::Joint` converts back to this.
    Joint = 1,

    /// The token can join with the following token to form a compound token,
    /// but this will not be visible at the proc macro level. (This is what the
    /// `Hidden` means; see below.)
    ///
    /// In token streams parsed from source code, the compiler will use
    /// `JointHidden` for any token immediately followed by anything not
    /// covered by the `Alone` and `Joint` cases: an identifier, lifetime,
    /// literal, delimiter, doc comment.
    ///
    /// When constructing token streams, use this for each token that (a)
    /// should be pretty-printed without a space after it, and (b) is followed
    /// by a non-punctuation token.
    ///
    /// Converts to `proc_macro::Spacing::Alone`, but
    /// `proc_macro::Spacing::Alone` converts back to `token::Spacing::Alone`.
    /// Because of that, pretty-printing of `TokenStream`s produced by proc
    /// macros is unavoidably uglier (with more whitespace between tokens) than
    /// pretty-printing of `TokenStream`'s produced by other means (i.e. parsed
    /// source code, internally constructed token streams, and token streams
    /// produced by declarative macros).
    JointHidden = 2,
}

/// Identifier or keyword.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident<Span = DefaultSpan> {
    pub sym: Symbol,
    pub span: Span,
    pub kind: IdentKind,
}

impl<Span> fmt::Display for Leaf<Span> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Leaf::Ident(it) => fmt::Display::fmt(it, f),
            Leaf::Literal(it) => fmt::Display::fmt(it, f),
            Leaf::Punct(it) => fmt::Display::fmt(it, f),
        }
    }
}

impl<Span> fmt::Display for Ident<Span> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind.as_str(), f)?;
        fmt::Display::fmt(&self.sym, f)
    }
}

impl<Span> fmt::Display for Literal<Span> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (text, suffix) = self.text_and_suffix();
        match self.kind {
            LitKind::Byte => write!(f, "b'{}'", text),
            LitKind::Char => write!(f, "'{}'", text),
            LitKind::Integer | LitKind::Float | LitKind::Err(_) => write!(f, "{}", text),
            LitKind::Str => write!(f, "\"{}\"", text),
            LitKind::ByteStr => write!(f, "b\"{}\"", text),
            LitKind::CStr => write!(f, "c\"{}\"", text),
            LitKind::StrRaw(num_of_hashes) => {
                let num_of_hashes = num_of_hashes as usize;
                write!(f, r#"r{0:#<num_of_hashes$}"{text}"{0:#<num_of_hashes$}"#, "", text = text)
            }
            LitKind::ByteStrRaw(num_of_hashes) => {
                let num_of_hashes = num_of_hashes as usize;
                write!(f, r#"br{0:#<num_of_hashes$}"{text}"{0:#<num_of_hashes$}"#, "", text = text)
            }
            LitKind::CStrRaw(num_of_hashes) => {
                let num_of_hashes = num_of_hashes as usize;
                write!(f, r#"cr{0:#<num_of_hashes$}"{text}"{0:#<num_of_hashes$}"#, "", text = text)
            }
        }?;
        write!(f, "{suffix}")?;
        Ok(())
    }
}

impl<Span> fmt::Display for Punct<Span> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.char, f)
    }
}

impl<Span: fmt::Debug> Leaf<Span> {
    pub fn print_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Leaf::Literal(lit) => {
                let (text, suffix) = lit.text_and_suffix();
                write!(f, "LITERAL {:?} {}{} {:#?}", lit.kind, text, suffix, lit.span)?;
            }
            Leaf::Punct(punct) => {
                write!(
                    f,
                    "PUNCT   {} [{}] {:#?}",
                    punct.char,
                    if punct.spacing == Spacing::Alone { "alone" } else { "joint" },
                    punct.span
                )?;
            }
            Leaf::Ident(ident) => {
                write!(f, "IDENT   {}{} {:#?}", ident.kind.as_str(), ident.sym, ident.span)?;
            }
        }

        Ok(())
    }
}

pub fn literal_from_lexer<Span>(
    text: &str,
    span: Span,
    kind: rustc_lexer::LiteralKind,
    suffix_start: u32,
) -> Literal<Span> {
    use rustc_lexer::LiteralKind;

    let (kind, start_offset, end_offset) = match kind {
        LiteralKind::Int { .. } => (LitKind::Integer, 0, 0),
        LiteralKind::Float { .. } => (LitKind::Float, 0, 0),
        LiteralKind::Char { terminated } => (LitKind::Char, 1, terminated as usize),
        LiteralKind::Byte { terminated } => (LitKind::Byte, 2, terminated as usize),
        LiteralKind::Str { terminated } => (LitKind::Str, 1, terminated as usize),
        LiteralKind::ByteStr { terminated } => (LitKind::ByteStr, 2, terminated as usize),
        LiteralKind::CStr { terminated } => (LitKind::CStr, 2, terminated as usize),
        LiteralKind::RawStr { n_hashes } => (
            LitKind::StrRaw(n_hashes.unwrap_or_default()),
            2 + n_hashes.unwrap_or_default() as usize,
            1 + n_hashes.unwrap_or_default() as usize,
        ),
        LiteralKind::RawByteStr { n_hashes } => (
            LitKind::ByteStrRaw(n_hashes.unwrap_or_default()),
            3 + n_hashes.unwrap_or_default() as usize,
            1 + n_hashes.unwrap_or_default() as usize,
        ),
        LiteralKind::RawCStr { n_hashes } => (
            LitKind::CStrRaw(n_hashes.unwrap_or_default()),
            3 + n_hashes.unwrap_or_default() as usize,
            1 + n_hashes.unwrap_or_default() as usize,
        ),
    };

    let (lit, suffix) = text.split_at(suffix_start as usize);
    let lit = &lit[start_offset..lit.len() - end_offset];
    let suffix = match suffix {
        "" | "_" => "",
        // ill-suffixed literals
        _ if !matches!(kind, LitKind::Integer | LitKind::Float | LitKind::Err(_)) => {
            return Literal::new_no_suffix(text, span, LitKind::Err(()));
        }
        suffix => suffix,
    };

    Literal::new(lit, span, kind, suffix)
}

pub fn literal_from_str<Span: Copy>(text: &str, span: Span) -> Result<Literal<Span>, ()> {
    use rustc_lexer::{LiteralKind, Token, TokenKind};

    let mut tokens = rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::No);
    let minus_or_lit = tokens.next().unwrap_or(Token { kind: TokenKind::Eof, len: 0 });

    let lit = if minus_or_lit.kind == TokenKind::Minus {
        let lit = tokens.next().ok_or(())?;
        if !matches!(
            lit.kind,
            TokenKind::Literal { kind: LiteralKind::Int { .. } | LiteralKind::Float { .. }, .. }
        ) {
            return Err(());
        }
        lit
    } else {
        minus_or_lit
    };

    if tokens.next().is_some() {
        return Err(());
    }

    let TokenKind::Literal { kind, suffix_start } = lit.kind else { return Err(()) };
    Ok(literal_from_lexer(text, span, kind, suffix_start))
}

pub fn literal_from_str_or_err<Span: Copy>(text: &str, span: Span) -> Literal<Span> {
    literal_from_str(text, span)
        .unwrap_or_else(|_| Literal::new_no_suffix(text, span, LitKind::Err(())))
}
