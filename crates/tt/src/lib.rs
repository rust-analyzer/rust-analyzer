//! `tt` crate defines a `TokenTree` data structure: this is the interface (both
//! input and output) of macros.
//!
//! The `TokenTree` is semantically a tree, but for performance reasons it is stored as a flat structure.

#![cfg_attr(feature = "in-rust-tree", feature(rustc_private))]

#[cfg(feature = "in-rust-tree")]
extern crate rustc_driver as _;

stdx::rustc_crates! {
    extern crate rustc_lexer or ra_ap_rustc_lexer;
}

#[cfg(feature = "in-ra")]
pub mod buffer;
#[cfg(feature = "in-ra")]
pub mod iter;
mod leaf_types;
#[cfg(feature = "in-ra")]
mod storage;

#[cfg(feature = "in-ra")]
pub use self::in_ra::*;
pub use self::leaf_types::*;

#[cfg(feature = "in-ra")]
mod in_ra {
    use std::fmt;

    use intern::Symbol;
    use stdx::impl_from;

    pub use span::Span;
    pub use text_size::{TextRange, TextSize};

    use crate::{leaf_types::*, storage::TokenTreesSlice};

    pub use crate::{
        buffer::Cursor,
        iter::{TtElement, TtIter},
        storage::{TopSubtree, TopSubtreeBuilder},
    };

    pub const MAX_GLUED_PUNCT_LEN: usize = 3;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum TokenTree {
        Leaf(Leaf),
        Subtree(Subtree),
    }
    impl_from!(Leaf, Subtree for TokenTree);
    impl TokenTree {
        pub fn first_span(&self) -> Span {
            match self {
                TokenTree::Leaf(l) => *l.span(),
                TokenTree::Subtree(s) => s.delimiter.open,
            }
        }
    }

    impl Leaf {
        pub fn span(&self) -> &Span {
            match self {
                Leaf::Literal(it) => &it.span,
                Leaf::Punct(it) => &it.span,
                Leaf::Ident(it) => &it.span,
            }
        }

        pub(crate) fn symbol(&self) -> Option<&Symbol> {
            match self {
                Leaf::Literal(Literal { text_and_suffix: symbol, .. })
                | Leaf::Ident(Ident { sym: symbol, .. }) => Some(symbol),
                Leaf::Punct(_) => None,
            }
        }
    }
    impl_from!(Literal, Punct, Ident for Leaf);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Subtree {
        pub delimiter: Delimiter,
        /// Number of following token trees that belong to this subtree, excluding this subtree.
        pub len: u32,
    }

    impl Subtree {
        pub fn usize_len(&self) -> usize {
            self.len as usize
        }
    }

    #[derive(Clone, Copy)]
    pub struct TokenTreesView<'a> {
        pub(crate) slice: TokenTreesSlice<'a>,
        pub(crate) len: usize,
    }

    impl<'a> TokenTreesView<'a> {
        #[inline]
        pub fn empty() -> Self {
            Self { slice: TokenTreesSlice::empty(), len: 0 }
        }

        pub fn iter(&self) -> TtIter<'a> {
            TtIter::new(*self)
        }

        pub fn cursor(&self) -> Cursor<'a> {
            Cursor::new(*self)
        }

        pub fn len(&self) -> usize {
            self.len
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        pub fn try_into_subtree(self) -> Option<SubtreeView<'a>> {
            let is_subtree = self.iter_flat_tokens().next().is_some_and(
            |it| matches!(it, TokenTree::Subtree(subtree) if subtree.usize_len() == self.len - 1),
        );
            if is_subtree { Some(SubtreeView(self)) } else { None }
        }

        pub fn strip_invisible(self) -> TokenTreesView<'a> {
            self.try_into_subtree().map(|subtree| subtree.strip_invisible()).unwrap_or(self)
        }

        pub fn split(
            self,
            mut split_fn: impl FnMut(TtElement<'a>) -> bool,
        ) -> impl Iterator<Item = TokenTreesView<'a>> {
            let mut subtree_iter = self.iter();
            let mut need_to_yield_even_if_empty = true;

            std::iter::from_fn(move || {
                if subtree_iter.is_empty() && !need_to_yield_even_if_empty {
                    return None;
                };

                need_to_yield_even_if_empty = false;
                let savepoint = subtree_iter.savepoint();
                let mut result = subtree_iter.from_savepoint(savepoint);
                while let Some(tt) = subtree_iter.next() {
                    if split_fn(tt) {
                        need_to_yield_even_if_empty = true;
                        break;
                    }
                    result = subtree_iter.from_savepoint(savepoint);
                }
                Some(result)
            })
        }

        pub fn first_span(&self) -> Option<Span> {
            self.iter_flat_tokens().next().map(|it| it.first_span())
        }

        /// Note: this is quite expensive, this needs to decode the whole view,
        /// although it "tricks" by skipping subtrees (since we know their byte length).
        pub fn last_span(&self) -> Option<Span> {
            let mut iter = self.iter();
            loop {
                match iter.last()? {
                    TtElement::Leaf(leaf) => return Some(*leaf.span()),
                    TtElement::Subtree(subtree, tt_iter) => {
                        if subtree.len == 0 {
                            return Some(subtree.delimiter.close);
                        } else {
                            iter = tt_iter;
                        }
                    }
                }
            }
        }

        pub fn iter_flat_tokens(&self) -> impl Iterator<Item = TokenTree> + use<'a> {
            self.slice.iter().take(self.len)
        }
    }

    impl fmt::Debug for TokenTreesView<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut iter = self.iter();
            while let Some(tt) = iter.next() {
                print_debug_token(f, 0, tt)?;
                if !iter.is_empty() {
                    writeln!(f)?;
                }
            }
            Ok(())
        }
    }

    impl fmt::Display for TokenTreesView<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            return token_trees_display(f, self.iter());

            fn subtree_display(
                subtree: &Subtree,
                f: &mut fmt::Formatter<'_>,
                iter: TtIter<'_>,
            ) -> fmt::Result {
                let (l, r) = match subtree.delimiter.kind {
                    DelimiterKind::Parenthesis => ("(", ")"),
                    DelimiterKind::Brace => ("{", "}"),
                    DelimiterKind::Bracket => ("[", "]"),
                    DelimiterKind::Invisible => ("", ""),
                };
                f.write_str(l)?;
                token_trees_display(f, iter)?;
                f.write_str(r)?;
                Ok(())
            }

            fn token_trees_display(f: &mut fmt::Formatter<'_>, iter: TtIter<'_>) -> fmt::Result {
                let mut needs_space = false;
                for child in iter {
                    if needs_space {
                        f.write_str(" ")?;
                    }
                    needs_space = true;

                    match child {
                        TtElement::Leaf(Leaf::Punct(p)) => {
                            needs_space = p.spacing == Spacing::Alone;
                            fmt::Display::fmt(&p, f)?;
                        }
                        TtElement::Leaf(leaf) => fmt::Display::fmt(&leaf, f)?,
                        TtElement::Subtree(subtree, subtree_iter) => {
                            subtree_display(&subtree, f, subtree_iter)?
                        }
                    }
                }
                Ok(())
            }
        }
    }

    #[derive(Clone, Copy)]
    // Invariant: always starts with `Subtree` that covers the entire thing.
    pub struct SubtreeView<'a>(pub(crate) TokenTreesView<'a>);

    impl<'a> SubtreeView<'a> {
        pub fn as_token_trees(self) -> TokenTreesView<'a> {
            self.0
        }

        pub fn iter(&self) -> TtIter<'a> {
            self.token_trees().iter()
        }

        pub fn top_subtree(&self) -> Subtree {
            let Some(TokenTree::Subtree(subtree)) = self.0.iter_flat_tokens().next() else {
                unreachable!("the first token tree is always the top subtree");
            };
            subtree
        }

        pub fn strip_invisible(&self) -> TokenTreesView<'a> {
            if self.top_subtree().delimiter.kind == DelimiterKind::Invisible {
                self.token_trees()
            } else {
                self.0
            }
        }

        pub fn token_trees(&self) -> TokenTreesView<'a> {
            let mut result = self.0;
            result.slice.advance();
            result.len -= 1;
            result
        }
    }

    impl fmt::Debug for SubtreeView<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(&self.0, f)
        }
    }

    impl fmt::Display for SubtreeView<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.0, f)
        }
    }

    impl DelimSpan {
        pub fn from_single(sp: Span) -> Self {
            DelimSpan { open: sp, close: sp }
        }

        pub fn from_pair(open: Span, close: Span) -> Self {
            DelimSpan { open, close }
        }
    }

    impl Delimiter {
        pub const fn invisible_spanned(span: Span) -> Self {
            Delimiter { open: span, close: span, kind: DelimiterKind::Invisible }
        }

        pub const fn invisible_delim_spanned(span: DelimSpan) -> Self {
            Delimiter { open: span.open, close: span.close, kind: DelimiterKind::Invisible }
        }

        pub fn delim_span(&self) -> DelimSpan {
            DelimSpan { open: self.open, close: self.close }
        }
    }

    impl Ident {
        pub fn new(text: &str, span: Span) -> Self {
            // let raw_stripped = IdentIsRaw::split_from_symbol(text.as_ref());
            let (kind, text) = IdentKind::split_from_symbol(text);
            Ident { sym: Symbol::intern(text), span, kind }
        }
    }

    fn print_debug_subtree(
        f: &mut fmt::Formatter<'_>,
        subtree: &Subtree,
        level: usize,
        iter: TtIter<'_>,
    ) -> fmt::Result {
        let Delimiter { kind, open, close } = &subtree.delimiter;
        let delim = kind.debug_view();

        write!(f, "SUBTREE {delim} ",)?;
        write!(f, "{open:#?}")?;
        write!(f, " ")?;
        write!(f, "{close:#?}")?;
        for child in iter {
            writeln!(f)?;
            print_debug_token(f, level + 1, child)?;
        }

        Ok(())
    }

    fn print_debug_token(
        f: &mut fmt::Formatter<'_>,
        level: usize,
        tt: TtElement<'_>,
    ) -> fmt::Result {
        write!(f, "{:indent$}", "", indent = level * 2)?;

        match tt {
            TtElement::Leaf(leaf) => leaf.print_debug(f),
            TtElement::Subtree(subtree, subtree_iter) => {
                print_debug_subtree(f, &subtree, level, subtree_iter)
            }
        }
    }

    impl fmt::Debug for TopSubtree {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(&self.view(), f)
        }
    }

    impl fmt::Display for TopSubtree {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.view(), f)
        }
    }

    impl Subtree {
        /// Count the number of tokens recursively
        pub fn count(&self) -> usize {
            self.usize_len()
        }
    }

    pub fn pretty(tkns: TokenTreesView<'_>) -> String {
        return pretty_impl(tkns.iter());

        fn tokentree_to_text(tkn: TtElement<'_>) -> String {
            match tkn {
                TtElement::Leaf(leaf) => {
                    format!("{}", leaf)
                }
                TtElement::Subtree(Subtree { delimiter, .. }, subtree_content) => {
                    let content = pretty_impl(subtree_content);
                    let (open, close) = delimiter.kind.display_open_close();
                    format!("{open}{content}{close}")
                }
            }
        }

        fn pretty_impl(tkns: TtIter<'_>) -> String {
            let mut last = String::new();
            let mut last_to_joint = true;

            for tkn in tkns {
                last = [last, tokentree_to_text(tkn.clone())].join(if last_to_joint {
                    ""
                } else {
                    " "
                });
                last_to_joint = false;
                if let TtElement::Leaf(Leaf::Punct(Punct { spacing, .. })) = tkn
                    && spacing == Spacing::Joint
                {
                    last_to_joint = true;
                }
            }
            last
        }
    }

    #[derive(Debug)]
    pub enum TransformTtAction<'a> {
        Keep,
        ReplaceWith(TokenTreesView<'a>),
    }

    impl TransformTtAction<'_> {
        #[inline]
        pub fn remove() -> Self {
            Self::ReplaceWith(TokenTreesView::empty())
        }
    }

    /// This function takes a token tree, and calls `callback` with each token tree in it.
    /// Then it does what the callback says: keeps the tt or replaces it with a (possibly empty)
    /// tts view.
    pub fn transform_tt<'b>(
        tt: &mut TopSubtree,
        mut callback: impl FnMut(&TokenTree) -> TransformTtAction<'b>,
    ) {
        let mut tt_vec = tt.as_token_trees().iter_flat_tokens().collect::<Vec<_>>();

        // We need to keep a stack of the currently open subtrees, because we need to update
        // them if we change the number of items in them.
        let mut subtrees_stack = Vec::new();
        let mut i = 0;
        while i < tt_vec.len() {
            'pop_finished_subtrees: while let Some(&subtree_idx) = subtrees_stack.last() {
                let TokenTree::Subtree(subtree) = &tt_vec[subtree_idx] else {
                    unreachable!("non-subtree on subtrees stack");
                };
                if i >= subtree_idx + 1 + subtree.usize_len() {
                    subtrees_stack.pop();
                } else {
                    break 'pop_finished_subtrees;
                }
            }

            let current = &tt_vec[i];
            let action = callback(current);
            match action {
                TransformTtAction::Keep => {
                    // This cannot be shared with the replaced case, because then we may push the same subtree
                    // twice, and will update it twice which will lead to errors.
                    if let TokenTree::Subtree(_) = current {
                        subtrees_stack.push(i);
                    }

                    i += 1;
                }
                TransformTtAction::ReplaceWith(replacement) => {
                    let old_len = 1 + match current {
                        TokenTree::Leaf(_) => 0,
                        TokenTree::Subtree(subtree) => subtree.usize_len(),
                    };
                    let len_diff = replacement.len() as i64 - old_len as i64;
                    tt_vec.splice(i..i + old_len, replacement.iter_flat_tokens());
                    // Skip the newly inserted replacement, we don't want to visit it.
                    i += replacement.len();

                    for &subtree_idx in &subtrees_stack {
                        let TokenTree::Subtree(subtree) = &mut tt_vec[subtree_idx] else {
                            unreachable!("non-subtree on subtrees stack");
                        };
                        subtree.len = (i64::from(subtree.len) + len_diff).try_into().unwrap();
                    }
                }
            }
        }

        *tt = TopSubtree::from_serialized(tt_vec);
    }
}
