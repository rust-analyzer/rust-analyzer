//! The proc-macro server token stream implementation.

use core::fmt;
use std::{mem, rc::Rc};

use intern::{Symbol, sym};
use tt::{
    Delimiter, DelimiterKind, Ident, IdentKind, Leaf, LitKind, Literal, Punct, Spacing,
    literal_from_lexer,
};

/// Trait for allowing tests to parse tokenstreams with dynamic span ranges
pub trait SpanLike: Copy {
    fn derive_ranged(&self, range: std::ops::Range<usize>) -> Self;

    fn cover(self, other: Self) -> Self;
}

#[derive(Debug, Clone)]
pub struct Group<S> {
    pub delimiter: Delimiter<S>,
    pub stream: Option<TokenStream<S>>,
}

impl<S> Group<S> {
    pub fn stream_len(&self) -> usize {
        self.stream.as_ref().map_or(0, |it| it.len())
    }
}

#[derive(Clone)]
pub enum TokenTree<S> {
    Leaf(Leaf<S>),
    Group(Group<S>),
}

#[derive(Clone)]
#[expect(clippy::rc_buffer, reason = "we commonly mutate this via `Rc::make_mut()`")]
pub struct TokenStream<S>(Rc<Vec<TokenTree<S>>>);

impl<S> Default for TokenStream<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<S> TokenStream<S> {
    #[inline]
    pub fn new(tts: Vec<TokenTree<S>>) -> TokenStream<S> {
        TokenStream(Rc::new(tts))
    }

    #[inline]
    pub fn new_or_empty(tts: Vec<TokenTree<S>>) -> Option<TokenStream<S>> {
        if tts.is_empty() { None } else { Some(TokenStream(Rc::new(tts))) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, TokenTree<S>> {
        self.0.iter()
    }

    #[inline]
    pub fn as_slice(&self) -> &[TokenTree<S>] {
        &self.0
    }

    #[inline]
    pub fn as_single_group(&self) -> Option<&Group<S>> {
        match &**self.0 {
            [TokenTree::Group(group)] => Some(group),
            _ => None,
        }
    }

    pub fn from_str(s: &str, span: S) -> Result<Self, String>
    where
        S: SpanLike + Copy,
    {
        let mut groups = Vec::new();
        groups.push((DelimiterKind::Invisible, 0..0, vec![]));
        let mut offset = 0;
        let mut tokens = rustc_lexer::tokenize(s, rustc_lexer::FrontmatterAllowed::No).peekable();
        while let Some(token) = tokens.next() {
            let range = offset..offset + token.len as usize;
            offset += token.len as usize;

            let mut spacing = || {
                let is_joint = tokens.peek().is_some_and(|token| {
                    matches!(
                        token.kind,
                        rustc_lexer::TokenKind::RawLifetime
                            | rustc_lexer::TokenKind::GuardedStrPrefix
                            | rustc_lexer::TokenKind::Lifetime { .. }
                            | rustc_lexer::TokenKind::Semi
                            | rustc_lexer::TokenKind::Comma
                            | rustc_lexer::TokenKind::Dot
                            | rustc_lexer::TokenKind::OpenParen
                            | rustc_lexer::TokenKind::CloseParen
                            | rustc_lexer::TokenKind::OpenBrace
                            | rustc_lexer::TokenKind::CloseBrace
                            | rustc_lexer::TokenKind::OpenBracket
                            | rustc_lexer::TokenKind::CloseBracket
                            | rustc_lexer::TokenKind::At
                            | rustc_lexer::TokenKind::Pound
                            | rustc_lexer::TokenKind::Tilde
                            | rustc_lexer::TokenKind::Question
                            | rustc_lexer::TokenKind::Colon
                            | rustc_lexer::TokenKind::Dollar
                            | rustc_lexer::TokenKind::Eq
                            | rustc_lexer::TokenKind::Bang
                            | rustc_lexer::TokenKind::Lt
                            | rustc_lexer::TokenKind::Gt
                            | rustc_lexer::TokenKind::Minus
                            | rustc_lexer::TokenKind::And
                            | rustc_lexer::TokenKind::Or
                            | rustc_lexer::TokenKind::Plus
                            | rustc_lexer::TokenKind::Star
                            | rustc_lexer::TokenKind::Slash
                            | rustc_lexer::TokenKind::Percent
                            | rustc_lexer::TokenKind::Caret
                    )
                });
                if is_joint { Spacing::Joint } else { Spacing::Alone }
            };

            let Some((open_delim, _, tokenstream)) = groups.last_mut() else {
                return Err("Unbalanced delimiters".to_owned());
            };
            match token.kind {
                rustc_lexer::TokenKind::OpenParen => {
                    groups.push((DelimiterKind::Parenthesis, range, vec![]))
                }
                rustc_lexer::TokenKind::CloseParen if *open_delim != DelimiterKind::Parenthesis => {
                    return if *open_delim == DelimiterKind::Invisible {
                        Err("Unexpected ')'".to_owned())
                    } else {
                        Err("Expected ')'".to_owned())
                    };
                }
                rustc_lexer::TokenKind::CloseParen => {
                    let (delimiter, open_range, stream) = groups.pop().unwrap();
                    groups.last_mut().ok_or_else(|| "Unbalanced delimiters".to_owned())?.2.push(
                        TokenTree::Group(Group {
                            delimiter: Delimiter {
                                open: span.derive_ranged(open_range),
                                close: span.derive_ranged(range),
                                kind: delimiter,
                            },
                            stream: TokenStream::new_or_empty(stream),
                        }),
                    );
                }
                rustc_lexer::TokenKind::OpenBrace => {
                    groups.push((DelimiterKind::Brace, range, vec![]))
                }
                rustc_lexer::TokenKind::CloseBrace if *open_delim != DelimiterKind::Brace => {
                    return if *open_delim == DelimiterKind::Invisible {
                        Err("Unexpected '}'".to_owned())
                    } else {
                        Err("Expected '}'".to_owned())
                    };
                }
                rustc_lexer::TokenKind::CloseBrace => {
                    let (delimiter, open_range, stream) = groups.pop().unwrap();
                    groups.last_mut().ok_or_else(|| "Unbalanced delimiters".to_owned())?.2.push(
                        TokenTree::Group(Group {
                            delimiter: Delimiter {
                                open: span.derive_ranged(open_range),
                                close: span.derive_ranged(range),
                                kind: delimiter,
                            },
                            stream: TokenStream::new_or_empty(stream),
                        }),
                    );
                }
                rustc_lexer::TokenKind::OpenBracket => {
                    groups.push((DelimiterKind::Bracket, range, vec![]))
                }
                rustc_lexer::TokenKind::CloseBracket if *open_delim != DelimiterKind::Bracket => {
                    return if *open_delim == DelimiterKind::Invisible {
                        Err("Unexpected ']'".to_owned())
                    } else {
                        Err("Expected ']'".to_owned())
                    };
                }
                rustc_lexer::TokenKind::CloseBracket => {
                    let (delimiter, open_range, stream) = groups.pop().unwrap();
                    groups.last_mut().ok_or_else(|| "Unbalanced delimiters".to_owned())?.2.push(
                        TokenTree::Group(Group {
                            delimiter: Delimiter {
                                open: span.derive_ranged(open_range),
                                close: span.derive_ranged(range),
                                kind: delimiter,
                            },
                            stream: TokenStream::new_or_empty(stream),
                        }),
                    );
                }
                rustc_lexer::TokenKind::LineComment { doc_style: None }
                | rustc_lexer::TokenKind::BlockComment { doc_style: None, terminated: _ } => {
                    continue;
                }
                rustc_lexer::TokenKind::LineComment { doc_style: Some(doc_style) } => {
                    let text = &s[range.start + 3..range.end];
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '#',
                        spacing: Spacing::Alone,
                        span,
                    })));
                    if doc_style == rustc_lexer::DocStyle::Inner {
                        tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                            char: '!',
                            spacing: Spacing::Alone,
                            span,
                        })));
                    }
                    let span = span.derive_ranged(range);
                    tokenstream.push(TokenTree::Group(Group {
                        delimiter: Delimiter {
                            open: span,
                            close: span,
                            kind: DelimiterKind::Bracket,
                        },
                        stream: TokenStream::new_or_empty(vec![
                            TokenTree::Leaf(Leaf::Ident(Ident {
                                sym: sym::doc,
                                kind: IdentKind::Normal,
                                span,
                            })),
                            TokenTree::Leaf(Leaf::Punct(Punct {
                                char: '=',
                                spacing: Spacing::Alone,
                                span,
                            })),
                            TokenTree::Leaf(Leaf::Literal(Literal::new_no_suffix(
                                &text.escape_debug().to_string(),
                                span,
                                LitKind::Str,
                            ))),
                        ]),
                    }));
                }
                rustc_lexer::TokenKind::BlockComment { doc_style: Some(doc_style), terminated } => {
                    let text =
                        &s[range.start + 3..if terminated { range.end - 2 } else { range.end }];
                    let span = span.derive_ranged(range);
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '#',
                        spacing: Spacing::Alone,
                        span,
                    })));
                    if doc_style == rustc_lexer::DocStyle::Inner {
                        tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                            char: '!',
                            spacing: Spacing::Alone,
                            span,
                        })));
                    }
                    tokenstream.push(TokenTree::Group(Group {
                        delimiter: Delimiter {
                            open: span,
                            close: span,
                            kind: DelimiterKind::Bracket,
                        },
                        stream: TokenStream::new_or_empty(vec![
                            TokenTree::Leaf(Leaf::Ident(Ident {
                                sym: sym::doc,
                                kind: IdentKind::Normal,
                                span,
                            })),
                            TokenTree::Leaf(Leaf::Punct(Punct {
                                char: '=',
                                spacing: Spacing::Alone,
                                span,
                            })),
                            TokenTree::Leaf(Leaf::Literal(Literal::new_no_suffix(
                                &text.escape_debug().to_string(),
                                span,
                                LitKind::Str,
                            ))),
                        ]),
                    }));
                }
                rustc_lexer::TokenKind::Whitespace => continue,
                rustc_lexer::TokenKind::Frontmatter { .. } => unreachable!(),
                rustc_lexer::TokenKind::Unknown => {
                    return Err(format!("Unknown token: `{}`", &s[range]));
                }
                rustc_lexer::TokenKind::UnknownPrefix => {
                    return Err(format!("Unknown prefix: `{}`", &s[range]));
                }
                rustc_lexer::TokenKind::UnknownPrefixLifetime => {
                    return Err(format!("Unknown lifetime prefix: `{}`", &s[range]));
                }
                // FIXME: Error on edition >= 2024 ... I dont think the proc-macro server can fetch editions currently
                // and whose edition is this?
                rustc_lexer::TokenKind::GuardedStrPrefix => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: s.as_bytes()[range.start].into(),
                        spacing: Spacing::Joint,
                        span: span.derive_ranged(range.start..range.start + 1),
                    })));
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: s.as_bytes()[range.start + 1].into(),
                        spacing: spacing(),
                        span: span.derive_ranged(range.start + 1..range.end),
                    })))
                }
                rustc_lexer::TokenKind::Ident => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Ident(Ident {
                        sym: Symbol::intern(&s[range.clone()]),
                        kind: IdentKind::Normal,
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::InvalidIdent => {
                    return Err(format!("Invalid identifier: `{}`", &s[range]));
                }
                rustc_lexer::TokenKind::RawIdent => {
                    let range = range.start + 2..range.end;
                    tokenstream.push(TokenTree::Leaf(Leaf::Ident(Ident {
                        sym: Symbol::intern(&s[range.clone()]),
                        kind: IdentKind::Raw,
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::ForcedKeywordIdent => {
                    let range = range.start + 2..range.end;
                    tokenstream.push(TokenTree::Leaf(Leaf::Ident(Ident {
                        sym: Symbol::intern(&s[range.clone()]),
                        kind: IdentKind::ForcedKeyword,
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Literal { kind, suffix_start } => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Literal(literal_from_lexer(
                        &s[range.clone()],
                        span.derive_ranged(range),
                        kind,
                        suffix_start,
                    ))))
                }
                rustc_lexer::TokenKind::RawLifetime => {
                    let range = range.start + 1 + 2..range.end;
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '\'',
                        spacing: Spacing::Joint,
                        span: span.derive_ranged(range.start..range.start + 1),
                    })));
                    tokenstream.push(TokenTree::Leaf(Leaf::Ident(Ident {
                        sym: Symbol::intern(&s[range.clone()]),
                        kind: IdentKind::Raw,
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Lifetime { starts_with_number } => {
                    if starts_with_number {
                        return Err("Lifetime cannot start with a number".to_owned());
                    }
                    let range = range.start + 1..range.end;
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '\'',
                        spacing: Spacing::Joint,
                        span: span.derive_ranged(range.start..range.start + 1),
                    })));
                    tokenstream.push(TokenTree::Leaf(Leaf::Ident(Ident {
                        sym: Symbol::intern(&s[range.clone()]),
                        kind: IdentKind::Normal,
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Semi => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: ';',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Comma => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: ',',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Dot => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '.',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::At => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '@',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Pound => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '#',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Tilde => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '~',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Question => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '?',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Colon => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: ':',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Dollar => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '$',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Eq => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '=',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Bang => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '!',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Lt => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '<',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Gt => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '>',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Minus => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '-',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::And => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '&',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Or => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '|',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Plus => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '+',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Star => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '*',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Slash => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '/',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Caret => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '^',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Percent => {
                    tokenstream.push(TokenTree::Leaf(Leaf::Punct(Punct {
                        char: '%',
                        spacing: spacing(),
                        span: span.derive_ranged(range),
                    })))
                }
                rustc_lexer::TokenKind::Eof => break,
            }
        }
        if let Some((DelimiterKind::Invisible, _, tokentrees)) = groups.pop()
            && groups.is_empty()
        {
            Ok(TokenStream::new(tokentrees))
        } else {
            Err("Mismatched token groups".to_owned())
        }
    }
}

impl<S> fmt::Display for TokenStream<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut emit_whitespace = false;
        for tt in self.0.iter() {
            display_token_tree(tt, &mut emit_whitespace, f)?;
        }
        Ok(())
    }
}

fn display_token_tree<S>(
    tt: &TokenTree<S>,
    emit_whitespace: &mut bool,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    if mem::take(emit_whitespace) {
        write!(f, " ")?;
    }
    match tt {
        TokenTree::Group(Group { delimiter, stream }) => {
            let (open, close) = delimiter.kind.display_open_close();
            write!(f, "{open}")?;
            if let Some(stream) = stream {
                write!(f, "{stream}")?;
            }
            write!(f, "{close}")?;
        }
        TokenTree::Leaf(leaf) => {
            fmt::Display::fmt(leaf, f)?;
            *emit_whitespace = match leaf {
                Leaf::Literal(literal) => !matches!(
                    literal.kind,
                    LitKind::Str
                        | LitKind::StrRaw(_)
                        | LitKind::ByteStr
                        | LitKind::ByteStrRaw(_)
                        | LitKind::CStr
                        | LitKind::CStrRaw(_)
                ),
                Leaf::Punct(punct) => punct.spacing == Spacing::Alone,
                Leaf::Ident(_) => true,
            };
        }
    }
    Ok(())
}

impl<S: fmt::Debug> fmt::Debug for TokenStream<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        debug_token_stream(self, 0, f)
    }
}

fn debug_token_stream<S: fmt::Debug>(
    ts: &TokenStream<S>,
    depth: usize,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    for tt in ts.0.iter() {
        debug_token_tree(tt, depth, f)?;
    }
    Ok(())
}

fn debug_token_tree<S: fmt::Debug>(
    tt: &TokenTree<S>,
    depth: usize,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    write!(f, "{:indent$}", "", indent = depth * 2)?;

    match tt {
        TokenTree::Group(Group { delimiter, stream }) => {
            writeln!(
                f,
                "GROUP {} {:#?} {:#?}",
                delimiter.kind.debug_view(),
                delimiter.open,
                delimiter.close,
            )?;
            if let Some(stream) = stream {
                debug_token_stream(stream, depth + 1, f)?;
            }
            return Ok(());
        }
        TokenTree::Leaf(leaf) => leaf.print_debug(f)?,
    }
    writeln!(f)
}

impl<S: Copy> TokenStream<S> {
    pub fn extend_with_streams(&mut self, streams: std::vec::IntoIter<TokenStream<S>>) {
        let vec_mut = Rc::make_mut(&mut self.0);

        vec_mut.reserve(streams.as_slice().iter().map(|item| item.len()).sum());
        streams.into_iter().for_each(|item| vec_mut.extend(item.iter().cloned()));
    }
}

impl<S> FromIterator<TokenTree<S>> for TokenStream<S> {
    fn from_iter<I: IntoIterator<Item = TokenTree<S>>>(iter: I) -> Self {
        TokenStream::new(Vec::from_iter(iter))
    }
}

impl<S: Copy> Extend<TokenTree<S>> for TokenStream<S> {
    fn extend<T: IntoIterator<Item = TokenTree<S>>>(&mut self, iter: T) {
        let vec_mut = Rc::make_mut(&mut self.0);
        vec_mut.extend(iter);
    }
}

impl SpanLike for () {
    fn derive_ranged(&self, _: std::ops::Range<usize>) -> Self {
        *self
    }

    fn cover(self, _other: Self) -> Self {
        self
    }
}

impl SpanLike for span::Span {
    fn derive_ranged(&self, range: std::ops::Range<usize>) -> Self {
        span::Span {
            range: span::TextRange::new(
                span::TextSize::new(range.start as u32),
                span::TextSize::new(range.end as u32),
            ),
            anchor: self.anchor,
            ctx: self.ctx,
        }
    }

    fn cover(self, other: Self) -> Self {
        self.cover(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_to_string() {
        let token_stream =
            TokenStream::from_str("{} () [] <> ;/., \"gfhdgfuiofghd\" 0f32 r#\"dff\"# 'r#lt", ())
                .unwrap();
        assert_eq!(token_stream.to_string(), "{}()[]<> ;/., \"gfhdgfuiofghd\"0f32 r#\"dff\"#'r#lt");
    }

    #[test]
    fn doc_comment_from_str() {
        let token_stream = TokenStream::from_str("/// foo", ()).unwrap();
        assert_eq!(token_stream.to_string(), r#"# [doc = " foo"]"#);
    }
}
