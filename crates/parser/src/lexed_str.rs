//! Lexing `&str` into a sequence of Rust tokens.
//!
//! Note that strictly speaking the parser in this crate is not required to work
//! on tokens which originated from text. Macros, eg, can synthesize tokens out
//! of thin air. So, ideally, lexer should be an orthogonal crate. It is however
//! convenient to include a text-based lexer here!
//!
//! Note that these tokens, unlike the tokens we feed into the parser, do
//! include info about comments and whitespace.

use std::ops;

use rustc_lexer::TokenKind;
use rustc_literal_escaper::{
    EscapeError, Mode, unescape_byte, unescape_byte_str, unescape_c_str, unescape_char,
    unescape_str,
};

use crate::{
    Edition,
    SyntaxKind::{self, *},
    T,
};

pub struct LexedStr<'a> {
    text: &'a str,
    kind: Vec<SyntaxKind>,
    start: Vec<u32>,
    error: Vec<LexError>,
}

struct LexError {
    msg: String,
    token: u32,
}

impl<'a> LexedStr<'a> {
    pub fn new(edition: Edition, text: &'a str) -> LexedStr<'a> {
        let _p = tracing::info_span!("LexedStr::new").entered();
        let mut conv = Converter::new(edition, text);
        let tokenize_at = |offset, fm| rustc_lexer::tokenize(&text[offset..], fm);

        if let Some(shebang_len) = rustc_lexer::strip_shebang(text) {
            conv.push(SHEBANG, shebang_len, Vec::new());
        }

        let mut was_frontmatter = false;
        // Try to parse only first frontmatter, skipping whitespaces
        while let Some(token) =
            tokenize_at(conv.offset, rustc_lexer::FrontmatterAllowed::Yes).next()
        {
            let token_text = &text[conv.offset..][..token.len as usize];
            match token.kind {
                TokenKind::Whitespace => {
                    conv.extend_token(&token.kind, token_text);
                }
                TokenKind::Frontmatter { has_invalid_preceding_whitespace, invalid_infostring } => {
                    if was_frontmatter {
                        break;
                    }
                    let (errs, is_unclosed) = Self::validate_frontmatter(
                        has_invalid_preceding_whitespace,
                        invalid_infostring,
                        token_text,
                    );

                    // Peek the next token after the frontmatter to check if this is eof
                    let is_next_eof = tokenize_at(
                        conv.offset + token.len as usize,
                        rustc_lexer::FrontmatterAllowed::Yes,
                    )
                    .next()
                    .is_none();

                    // Push only a normally closed frontmatter or unclosed frontmatter containing tokens after it,
                    // to prevent eating the whole file due to a missing dash.
                    if !is_unclosed || !is_next_eof {
                        conv.push(FRONTMATTER, token.len as usize, errs);
                        was_frontmatter = true;
                    }
                    if is_next_eof {
                        break;
                    }
                }
                _ => break,
            }
        }

        // Re-create the tokenizer from scratch every token because `GuardedStrPrefix` is one token in the lexer
        // but we want to split it to two in edition <2024.
        while let Some(token) = tokenize_at(conv.offset, rustc_lexer::FrontmatterAllowed::No).next()
        {
            let token_text = &text[conv.offset..][..token.len as usize];

            conv.extend_token(&token.kind, token_text);
        }

        conv.finalize_with_eof()
    }

    pub fn single_token(edition: Edition, text: &'a str) -> Option<(SyntaxKind, Option<String>)> {
        if text.is_empty() {
            return None;
        }

        let token = rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::No).next()?;
        if token.len as usize != text.len() {
            return None;
        }

        let mut conv = Converter::new(edition, text);
        conv.extend_token(&token.kind, text);
        match &*conv.res.kind {
            [kind] => Some((*kind, conv.res.error.pop().map(|it| it.msg))),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        self.text
    }

    pub fn len(&self) -> usize {
        self.kind.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn kind(&self, i: usize) -> SyntaxKind {
        assert!(i < self.len());
        self.kind[i]
    }

    pub fn text(&self, i: usize) -> &str {
        self.range_text(i..i + 1)
    }

    pub fn range_text(&self, r: ops::Range<usize>) -> &str {
        assert!(r.start < r.end && r.end <= self.len());
        let lo = self.start[r.start] as usize;
        let hi = self.start[r.end] as usize;
        &self.text[lo..hi]
    }

    // Naming is hard.
    pub fn text_range(&self, i: usize) -> ops::Range<usize> {
        assert!(i < self.len());
        let lo = self.start[i] as usize;
        let hi = self.start[i + 1] as usize;
        lo..hi
    }
    pub fn text_start(&self, i: usize) -> usize {
        assert!(i <= self.len());
        self.start[i] as usize
    }
    pub fn text_len(&self, i: usize) -> usize {
        assert!(i < self.len());
        let r = self.text_range(i);
        r.end - r.start
    }

    pub fn error(&self, i: usize) -> Option<&str> {
        assert!(i < self.len());
        let err = self.error.binary_search_by_key(&(i as u32), |i| i.token).ok()?;
        Some(self.error[err].msg.as_str())
    }

    pub fn errors(&self) -> impl Iterator<Item = (usize, &str)> + '_ {
        self.error.iter().map(|it| (it.token as usize, it.msg.as_str()))
    }

    fn push(&mut self, kind: SyntaxKind, offset: usize) {
        self.kind.push(kind);
        self.start.push(offset as u32);
    }

    // Implements similar checks from the rustc_lexer validate_frontmatter fn
    fn validate_frontmatter(
        has_invalid_preceding_whitespace: bool,
        invalid_infostring: bool,
        text: &str,
    ) -> (Vec<String>, bool) {
        const FENCE_CHAR: char = '-';
        const FENCE_SEQ: &str = "---";
        let mut is_unclosed = false;
        let mut errs = Vec::new();

        // rustc_lexer::tokenize may add '\r' suffix for windows sources (with newlines as CRLF).
        // We strip it here:
        let text = text.strip_suffix('\r').unwrap_or(text);

        if has_invalid_preceding_whitespace {
            errs.push("invalid preceding whitespace for frontmatter opening".to_owned());
        }

        let Some(real_start) = text.find(FENCE_SEQ) else {
            errs.push("invalid frontmatter opening".to_owned());
            return (errs, is_unclosed); // for sanity, as frontmatter starts with '---' in this call
        };
        let real_s = &text[real_start..];
        let within = real_s.trim_start_matches(FENCE_CHAR); // opening may be longer than '---'
        let len_opening = real_s.len() - within.len();

        let line_end = real_s.find('\n').unwrap_or(real_s.len());

        // rustc_lexer::tokenize may set invalid_infostring for valid strings ending with '\r'
        // because it works with normalised inputs ("\r\n" -> "\n").
        // So we here do an additional check for that:
        if invalid_infostring && !real_s[..line_end].ends_with('\r') {
            errs.push("invalid infostring for frontmatter".to_owned());
        }
        let last_line_start = real_s.rfind('\n').map_or(line_end, |i| i + 1);
        let content = &real_s[line_end..last_line_start];
        let content_normalized = content.replace("\r\n", "\n"); // normalise content to handle hanging '\r' correctly
        if content_normalized.contains('\r') {
            errs.push("bare CR not allowed in frontmatter".to_owned());
        }
        let last_line = &real_s[last_line_start..];
        let last_line_trimmed = last_line.trim_start_matches(rustc_lexer::is_horizontal_whitespace);
        if last_line_trimmed.len() != last_line.len() {
            errs.push("invalid preceding whitespace for frontmatter close".to_owned());
        }

        let rest = last_line_trimmed.trim_start_matches(FENCE_CHAR);
        let len_close = last_line_trimmed.len() - rest.len();
        if !last_line_trimmed.starts_with(FENCE_SEQ) {
            errs.push("unclosed frontmatter".to_owned());
            is_unclosed = true;
        } else if len_close != len_opening {
            errs.push("frontmatter close does not match the opening".to_owned());
            is_unclosed = true;
        } else if !rest.trim_matches(rustc_lexer::is_horizontal_whitespace).is_empty() {
            errs.push("extra characters after frontmatter close are not allowed".to_owned());
        }

        // Only up to 255 `-`s are allowed in code fences
        if u8::try_from(len_opening).is_err() {
            errs.push(format!("too many `-` symbols: frontmatter openings may be delimited by up to 255 `-` symbols, but found {len_opening}"));
        }

        (errs, is_unclosed)
    }
}

struct Converter<'a> {
    res: LexedStr<'a>,
    offset: usize,
    edition: Edition,
}

impl<'a> Converter<'a> {
    fn new(edition: Edition, text: &'a str) -> Self {
        Self {
            res: LexedStr {
                text,
                kind: Vec::with_capacity(text.len() / 3),
                start: Vec::with_capacity(text.len() / 3),
                error: Vec::new(),
            },
            offset: 0,
            edition,
        }
    }

    /// Check for likely unterminated string by analyzing STRING token content
    fn has_likely_unterminated_string(&self) -> bool {
        let Some(last_idx) = self.res.kind.len().checked_sub(1) else { return false };

        for i in (0..=last_idx).rev().take(5) {
            if self.res.kind[i] == STRING {
                let start = self.res.start[i] as usize;
                let end = self.res.start.get(i + 1).map(|&s| s as usize).unwrap_or(self.offset);
                let content = &self.res.text[start..end];

                if content.contains('(') && (content.contains("//") || content.contains(";\n")) {
                    return true;
                }
            }
        }
        false
    }

    fn finalize_with_eof(mut self) -> LexedStr<'a> {
        self.res.push(EOF, self.offset);
        self.res
    }

    fn push(&mut self, kind: SyntaxKind, len: usize, errors: Vec<String>) {
        self.res.push(kind, self.offset);
        self.offset += len;

        for msg in errors {
            if !msg.is_empty() {
                self.res.error.push(LexError { msg, token: self.res.len() as u32 });
            }
        }
    }

    fn comment_kind(doc_style: Option<rustc_lexer::DocStyle>) -> SyntaxKind {
        match doc_style {
            Some(rustc_lexer::DocStyle::Outer) => OUTER_DOC_COMMENT,
            Some(rustc_lexer::DocStyle::Inner) => INNER_DOC_COMMENT,
            None => COMMENT,
        }
    }

    fn extend_token(&mut self, kind: &rustc_lexer::TokenKind, mut token_text: &str) {
        // A note on an intended tradeoff:
        // We drop some useful information here (see patterns with double dots `..`)
        // Storing that info in `SyntaxKind` is not possible due to its layout requirements of
        // being `u16` that come from `rowan::SyntaxKind`.
        let mut errors: Vec<String> = vec![];

        let syntax_kind = {
            match kind {
                rustc_lexer::TokenKind::LineComment { doc_style } => Self::comment_kind(*doc_style),
                rustc_lexer::TokenKind::BlockComment { doc_style, terminated } => {
                    if !terminated {
                        errors.push(
                            "Missing trailing `*/` symbols to terminate the block comment".into(),
                        );
                    }
                    Self::comment_kind(*doc_style)
                }

                rustc_lexer::TokenKind::Frontmatter { .. } => {
                    return; // frontmatter is processed outside this function
                }

                rustc_lexer::TokenKind::Whitespace => WHITESPACE,

                rustc_lexer::TokenKind::Ident if token_text == "_" => UNDERSCORE,
                rustc_lexer::TokenKind::Ident => {
                    SyntaxKind::from_keyword(token_text, self.edition).unwrap_or(IDENT)
                }
                rustc_lexer::TokenKind::InvalidIdent => {
                    errors.push("Ident contains invalid characters".into());
                    IDENT
                }

                rustc_lexer::TokenKind::RawIdent => IDENT,

                rustc_lexer::TokenKind::ForcedKeywordIdent
                    if self.edition < Edition::Edition2021 =>
                {
                    // Split the token.
                    token_text = &token_text[.."k".len()];
                    IDENT
                }
                rustc_lexer::TokenKind::ForcedKeywordIdent if token_text == "k#_" => UNDERSCORE,
                rustc_lexer::TokenKind::ForcedKeywordIdent => {
                    SyntaxKind::from_any_keyword(&token_text["k#".len()..]).unwrap_or_else(|| {
                        // FIXME: This is not correct, see https://rust-lang.zulipchat.com/#narrow/channel/185405-t-compiler.2Frust-analyzer/topic/Mismatch.20versus.20rustc.20parsing.20in.20foundation/with/632168944.
                        errors.push("invalid forced keyword".into());
                        ERROR
                    })
                }

                rustc_lexer::TokenKind::GuardedStrPrefix if self.edition.at_least_2024() => {
                    // FIXME: rustc does something better for recovery.
                    errors.push("Invalid string literal (reserved syntax)".into());
                    ERROR
                }
                rustc_lexer::TokenKind::GuardedStrPrefix => {
                    // The token is `#"` or `##`, split it into two.
                    token_text = &token_text[1..];
                    POUND
                }

                rustc_lexer::TokenKind::Literal { kind, .. } => {
                    self.extend_literal(token_text.len(), kind);
                    return;
                }

                rustc_lexer::TokenKind::Lifetime { starts_with_number } => {
                    if *starts_with_number {
                        errors.push("Lifetime name contains invalid characters".into());
                    }
                    LIFETIME_IDENT
                }
                rustc_lexer::TokenKind::UnknownPrefixLifetime => {
                    errors.push("Unknown lifetime prefix".into());
                    LIFETIME_IDENT
                }
                rustc_lexer::TokenKind::RawLifetime => LIFETIME_IDENT,

                rustc_lexer::TokenKind::Semi => T![;],
                rustc_lexer::TokenKind::Comma => T![,],
                rustc_lexer::TokenKind::Dot => T![.],
                rustc_lexer::TokenKind::OpenParen => T!['('],
                rustc_lexer::TokenKind::CloseParen => T![')'],
                rustc_lexer::TokenKind::OpenBrace => T!['{'],
                rustc_lexer::TokenKind::CloseBrace => T!['}'],
                rustc_lexer::TokenKind::OpenBracket => T!['['],
                rustc_lexer::TokenKind::CloseBracket => T![']'],
                rustc_lexer::TokenKind::At => T![@],
                rustc_lexer::TokenKind::Pound => T![#],
                rustc_lexer::TokenKind::Tilde => T![~],
                rustc_lexer::TokenKind::Question => T![?],
                rustc_lexer::TokenKind::Colon => T![:],
                rustc_lexer::TokenKind::Dollar => T![$],
                rustc_lexer::TokenKind::Eq => T![=],
                rustc_lexer::TokenKind::Bang => T![!],
                rustc_lexer::TokenKind::Lt => T![<],
                rustc_lexer::TokenKind::Gt => T![>],
                rustc_lexer::TokenKind::Minus => T![-],
                rustc_lexer::TokenKind::And => T![&],
                rustc_lexer::TokenKind::Or => T![|],
                rustc_lexer::TokenKind::Plus => T![+],
                rustc_lexer::TokenKind::Star => T![*],
                rustc_lexer::TokenKind::Slash => T![/],
                rustc_lexer::TokenKind::Caret => T![^],
                rustc_lexer::TokenKind::Percent => T![%],
                rustc_lexer::TokenKind::Unknown => ERROR,
                rustc_lexer::TokenKind::UnknownPrefix if token_text == "builtin" => IDENT,
                rustc_lexer::TokenKind::UnknownPrefix => {
                    let has_unterminated = self.has_likely_unterminated_string();

                    let error_msg = if has_unterminated {
                        format!(
                            "unknown literal prefix `{token_text}` (note: check for unterminated string literal)"
                        )
                    } else {
                        "unknown literal prefix".to_owned()
                    };
                    errors.push(error_msg);
                    IDENT
                }
                rustc_lexer::TokenKind::Eof => EOF,
            }
        };

        self.push(syntax_kind, token_text.len(), errors);
    }

    fn extend_literal(&mut self, len: usize, kind: &rustc_lexer::LiteralKind) {
        let invalid_raw_msg = String::from("Invalid raw string literal");

        let mut errors = vec![];
        let mut no_end_quote = |c: char, kind: &str| {
            errors.push(format!("Missing trailing `{c}` symbol to terminate the {kind} literal"));
        };

        let syntax_kind = match *kind {
            rustc_lexer::LiteralKind::Int { empty_int, base: _ } => {
                if empty_int {
                    errors.push("Missing digits after the integer base prefix".into());
                }
                INT_NUMBER
            }
            rustc_lexer::LiteralKind::Float { empty_exponent, base: _ } => {
                if empty_exponent {
                    errors.push("Missing digits after the exponent symbol".into());
                }
                FLOAT_NUMBER
            }
            rustc_lexer::LiteralKind::Char { terminated } => {
                if !terminated {
                    no_end_quote('\'', "character");
                } else {
                    let text = &self.res.text[self.offset + 1..][..len - 1];
                    let text = &text[..text.rfind('\'').unwrap()];
                    if let Err(e) = unescape_char(text) {
                        errors.push(err_to_msg(e, Mode::Char));
                    }
                }
                CHAR
            }
            rustc_lexer::LiteralKind::Byte { terminated } => {
                if !terminated {
                    no_end_quote('\'', "byte");
                } else {
                    let text = &self.res.text[self.offset + 2..][..len - 2];
                    let text = &text[..text.rfind('\'').unwrap()];
                    if let Err(e) = unescape_byte(text) {
                        errors.push(err_to_msg(e, Mode::Byte));
                    }
                }
                BYTE
            }
            rustc_lexer::LiteralKind::Str { terminated } => {
                if !terminated {
                    no_end_quote('"', "string");
                } else {
                    let text = &self.res.text[self.offset + 1..][..len - 1];
                    let text = &text[..text.rfind('"').unwrap()];
                    unescape_str(text, |_, res| {
                        if let Err(e) = res {
                            errors.push(err_to_msg(e, Mode::Str));
                        }
                    });
                }
                STRING
            }
            rustc_lexer::LiteralKind::ByteStr { terminated } => {
                if !terminated {
                    no_end_quote('"', "byte string");
                } else {
                    let text = &self.res.text[self.offset + 2..][..len - 2];
                    let text = &text[..text.rfind('"').unwrap()];
                    unescape_byte_str(text, |_, res| {
                        if let Err(e) = res {
                            errors.push(err_to_msg(e, Mode::ByteStr));
                        }
                    });
                }
                BYTE_STRING
            }
            rustc_lexer::LiteralKind::CStr { terminated } => {
                if !terminated {
                    no_end_quote('"', "C string")
                } else {
                    let text = &self.res.text[self.offset + 2..][..len - 2];
                    let text = &text[..text.rfind('"').unwrap()];
                    unescape_c_str(text, |_, res| {
                        if let Err(e) = res {
                            errors.push(err_to_msg(e, Mode::CStr));
                        }
                    });
                }
                C_STRING
            }
            rustc_lexer::LiteralKind::RawStr { n_hashes } => {
                if n_hashes.is_none() {
                    errors.push(invalid_raw_msg);
                }
                STRING
            }
            rustc_lexer::LiteralKind::RawByteStr { n_hashes } => {
                if n_hashes.is_none() {
                    errors.push(invalid_raw_msg);
                }
                BYTE_STRING
            }
            rustc_lexer::LiteralKind::RawCStr { n_hashes } => {
                if n_hashes.is_none() {
                    errors.push(invalid_raw_msg);
                }
                C_STRING
            }
        };

        self.push(syntax_kind, len, errors);
    }
}

fn err_to_msg(error: EscapeError, mode: Mode) -> String {
    match error {
        EscapeError::ZeroChars => "empty character literal",
        EscapeError::MoreThanOneChar => "character literal may only contain one codepoint",
        EscapeError::LoneSlash => "",
        EscapeError::InvalidEscape if mode == Mode::Byte || mode == Mode::ByteStr => {
            "unknown byte escape"
        }
        EscapeError::InvalidEscape => "unknown character escape",
        EscapeError::BareCarriageReturn => "",
        EscapeError::BareCarriageReturnInRawString => "",
        EscapeError::EscapeOnlyChar if mode == Mode::Byte => "byte constant must be escaped",
        EscapeError::EscapeOnlyChar => "character constant must be escaped",
        EscapeError::TooShortHexEscape => "numeric character escape is too short",
        EscapeError::InvalidCharInHexEscape => "invalid character in numeric character escape",
        EscapeError::OutOfRangeHexEscape => "out of range hex escape",
        EscapeError::NoBraceInUnicodeEscape => "incorrect unicode escape sequence",
        EscapeError::InvalidCharInUnicodeEscape => "invalid character in unicode escape",
        EscapeError::EmptyUnicodeEscape => "empty unicode escape",
        EscapeError::UnclosedUnicodeEscape => "unterminated unicode escape",
        EscapeError::LeadingUnderscoreUnicodeEscape => "invalid start of unicode escape",
        EscapeError::OverlongUnicodeEscape => "overlong unicode escape",
        EscapeError::LoneSurrogateUnicodeEscape => "invalid unicode character escape",
        EscapeError::OutOfRangeUnicodeEscape => "invalid unicode character escape",
        EscapeError::UnicodeEscapeInByte => "unicode escape in byte string",
        EscapeError::NonAsciiCharInByte if mode == Mode::Byte => {
            "non-ASCII character in byte literal"
        }
        EscapeError::NonAsciiCharInByte if mode == Mode::ByteStr => {
            "non-ASCII character in byte string literal"
        }
        EscapeError::NonAsciiCharInByte => "non-ASCII character in raw byte string literal",
        EscapeError::NulInCStr => "null character in C string literal",
        EscapeError::UnskippedWhitespaceWarning => "",
        EscapeError::MultipleSkippedLinesWarning => "",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::LexedStr;
    use proptest::collection::vec;
    use proptest::prelude::{Just, any, proptest};
    use proptest::prop_oneof;
    use proptest::strategy::Strategy;

    #[test]
    fn validate_frontmatter_test() {
        let (errs, _) = LexedStr::validate_frontmatter(false, false, "");
        assert_eq!(errs, vec!["invalid frontmatter opening".to_owned()]);

        let (errs, is_unclosed) = LexedStr::validate_frontmatter(false, false, "---\na = 1\n---");
        assert!(errs.is_empty());
        assert!(!is_unclosed);

        // windows crlf
        let (errs, is_unclosed) =
            LexedStr::validate_frontmatter(false, false, "---\r\na = 1\r\n---\r");
        assert!(errs.is_empty());
        assert!(!is_unclosed);

        let (errs, is_unclosed) = LexedStr::validate_frontmatter(false, false, "---");
        assert_eq!(errs, vec!["unclosed frontmatter".to_owned()]);
        assert!(is_unclosed);

        let (errs, is_unclosed) = LexedStr::validate_frontmatter(false, false, "---\na = 1\n ---");
        assert_eq!(errs, vec!["invalid preceding whitespace for frontmatter close".to_owned()]);
        assert!(!is_unclosed);

        let (errs, is_unclosed) = LexedStr::validate_frontmatter(true, false, "    \n  ---");
        assert_eq!(
            errs,
            vec![
                "invalid preceding whitespace for frontmatter opening".to_owned(),
                "unclosed frontmatter".to_owned()
            ]
        );
        assert!(is_unclosed);

        let (errs, is_unclosed) = LexedStr::validate_frontmatter(false, false, "\n---");
        assert_eq!(errs, vec!["unclosed frontmatter".to_owned()]);
        assert!(is_unclosed);

        let (errs, _) = LexedStr::validate_frontmatter(false, false, "---\nb = 1\ra = 1\n---");
        assert_eq!(errs, vec!["bare CR not allowed in frontmatter".to_owned()]);

        // windows crlf
        let (errs, _) =
            LexedStr::validate_frontmatter(false, false, "---\r\nb = 1\ra = 1\r\n---\r");
        assert_eq!(errs, vec!["bare CR not allowed in frontmatter".to_owned()]);

        let (errs, is_unclosed) = LexedStr::validate_frontmatter(false, false, "---\nb = 1\n----");
        assert_eq!(errs, vec!["frontmatter close does not match the opening".to_owned()]);
        assert!(is_unclosed);

        let (errs, is_unclosed) = LexedStr::validate_frontmatter(false, false, "---\nb = 1\n---x");
        assert_eq!(
            errs,
            vec!["extra characters after frontmatter close are not allowed".to_owned()]
        );
        assert!(!is_unclosed);
    }

    fn frontmatter_gen_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just(""),
            Just("fn main(){}"),
            Just("[dependencies]"),
            Just("a=1"),
            Just("x"),
            Just("1"),
            Just("&"),
            Just("<"),
            Just(">"),
            Just("{"),
            Just("}"),
            Just("\u{0301}"),
            Just(" "),
            Just("\t"),
            Just("\n"),
            Just("\r\n"),
            Just("-"),
            Just("--"),
            Just("---"),
            Just("----"),
            Just("-----"),
        ]
    }

    proptest! {
        // Frontmatter validator panic test.
        #[test]
        fn validate_frontmatter_boundaries(text in vec(frontmatter_gen_strategy(), 0..100)
            .prop_map(|s| s.into_iter().collect::<String>()),
            bool1 in any::<bool>(), bool2 in any::<bool>()) {
            let (_, _) = LexedStr::validate_frontmatter(bool1, bool2, text.as_str());
        }
    }
}
