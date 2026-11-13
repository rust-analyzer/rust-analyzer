//! See [`Parser`].

use std::{cell::Cell, num::NonZeroU32};

use drop_bomb::DropBomb;

use crate::{
    Edition,
    SyntaxKind::{self, EOF, ERROR, INT_NUMBER, TOMBSTONE},
    T, TokenSet,
    event::Event,
    input::Input,
};

/// Build a forward-parent offset. The offset is always ≥ 1 because the
/// forward-parent event is created *after* the event it forwards to, so
/// `NonZeroU32` is always valid here. Panics only on a parser bug.
#[inline]
fn fwd_parent(offset: u32) -> NonZeroU32 {
    NonZeroU32::new(offset).expect("forward-parent offset must be non-zero")
}

/// Tracks an in-flight split of a `FLOAT_NUMBER` into synthetic
/// `INT_NUMBER` / `DOT` / `INT_NUMBER` tokens. At most one split is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatSplitStage {
    // Discriminants track the number of synthetic tokens remaining.
    None = 0,
    BeforeFirstInt = 3,
    BeforeDot = 2,
    BeforeSecondInt = 1,
}

/// `Parser` struct provides the low-level API for
/// navigating through the stream of tokens and
/// constructing the parse tree. The actual parsing
/// happens in the [`grammar`](super::grammar) module.
///
/// However, the result of this `Parser` is not a real
/// tree, but rather a flat stream of events of the form
/// "start expression, consume number literal,
/// finish expression". See `Event` docs for more.
pub(crate) struct Parser<'t> {
    inp: &'t Input,
    pos: usize,
    events: Vec<Event>,
    /// Side table of error messages. `Event::Error { err }` carries an index
    /// into this vec, keeping `Event` itself a flat 8-byte enum.
    errors: Vec<String>,
    steps: Cell<u32>,
    float_split_stage: FloatSplitStage,
    float_split_ends_in_dot: bool,
}

const PARSER_STEP_LIMIT: usize = if cfg!(debug_assertions) { 150_000 } else { 15_000_000 };

impl<'t> Parser<'t> {
    pub(super) fn new(inp: &'t Input) -> Parser<'t> {
        Parser {
            inp,
            pos: 0,
            events: Vec::with_capacity(2 * inp.len()),
            errors: Vec::new(),
            steps: Cell::new(0),
            float_split_stage: FloatSplitStage::None,
            float_split_ends_in_dot: false,
        }
    }

    pub(crate) fn finish(self) -> (Vec<Event>, Vec<String>) {
        (self.events, self.errors)
    }

    /// Returns the kind of the current token.
    /// If parser has already reached the end of input,
    /// the special `EOF` kind is returned.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Lookahead operation: returns the kind of the next nth
    /// token.
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        assert!(n <= 3);

        let steps = self.steps.get();
        assert!((steps as usize) < PARSER_STEP_LIMIT, "the parser seems stuck");
        self.steps.set(steps + 1);

        self.logical_kind(n)
    }

    /// Kind at lookahead `n`, honoring an in-flight float split.
    /// Does not touch the stuck-detection `steps` counter.
    fn logical_kind(&self, n: usize) -> SyntaxKind {
        match n.checked_sub(self.float_split_suffix_len()) {
            Some(n) => self.inp.kind(self.pos + n),
            None => self.float_split_synthetic_kind(n),
        }
    }

    /// Length of the remaining synthetic token suffix, or 0 if no split is active.
    fn float_split_suffix_len(&self) -> usize {
        self.float_split_stage as usize - usize::from(self.float_split_ends_in_dot)
    }

    #[cold]
    fn float_split_synthetic_kind(&self, n: usize) -> SyntaxKind {
        match self.float_split_stage {
            FloatSplitStage::None => unreachable!(),
            FloatSplitStage::BeforeFirstInt => match n {
                0 => INT_NUMBER,
                1 => T![.],
                2 if !self.float_split_ends_in_dot => INT_NUMBER,
                _ => unreachable!(),
            },
            FloatSplitStage::BeforeDot => match n {
                0 => T![.],
                1 if !self.float_split_ends_in_dot => INT_NUMBER,
                _ => unreachable!(),
            },
            FloatSplitStage::BeforeSecondInt => {
                assert_eq!(n, 0);
                INT_NUMBER
            }
        }
    }

    /// Checks if the current token is `kind`.
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.nth_at(0, kind)
    }

    pub(crate) fn nth_at(&self, n: usize, kind: SyntaxKind) -> bool {
        match kind {
            T![-=] => self.at_composite2(n, T![-], T![=]),
            T![->] => self.at_composite2(n, T![-], T![>]),
            T![::] => self.at_composite2(n, T![:], T![:]),
            T![!=] => self.at_composite2(n, T![!], T![=]),
            T![..] => self.at_composite2(n, T![.], T![.]),
            T![*=] => self.at_composite2(n, T![*], T![=]),
            T![/=] => self.at_composite2(n, T![/], T![=]),
            T![&&] => self.at_composite2(n, T![&], T![&]),
            T![&=] => self.at_composite2(n, T![&], T![=]),
            T![%=] => self.at_composite2(n, T![%], T![=]),
            T![^=] => self.at_composite2(n, T![^], T![=]),
            T![+=] => self.at_composite2(n, T![+], T![=]),
            T![<<] => self.at_composite2(n, T![<], T![<]),
            T![<=] => self.at_composite2(n, T![<], T![=]),
            T![==] => self.at_composite2(n, T![=], T![=]),
            T![=>] => self.at_composite2(n, T![=], T![>]),
            T![>=] => self.at_composite2(n, T![>], T![=]),
            T![>>] => self.at_composite2(n, T![>], T![>]),
            T![|=] => self.at_composite2(n, T![|], T![=]),
            T![||] => self.at_composite2(n, T![|], T![|]),

            T![...] => self.at_composite3(n, T![.], T![.], T![.]),
            T![..=] => self.at_composite3(n, T![.], T![.], T![=]),
            T![<<=] => self.at_composite3(n, T![<], T![<], T![=]),
            T![>>=] => self.at_composite3(n, T![>], T![>], T![=]),

            _ => self.logical_kind(n) == kind,
        }
    }

    /// Consume the next token if `kind` matches.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if !self.at(kind) {
            return false;
        }
        let n_raw_tokens = match kind {
            T![-=]
            | T![->]
            | T![::]
            | T![!=]
            | T![..]
            | T![*=]
            | T![/=]
            | T![&&]
            | T![&=]
            | T![%=]
            | T![^=]
            | T![+=]
            | T![<<]
            | T![<=]
            | T![==]
            | T![=>]
            | T![>=]
            | T![>>]
            | T![|=]
            | T![||] => 2,

            T![...] | T![..=] | T![<<=] | T![>>=] => 3,
            _ => 1,
        };
        self.do_bump(kind, n_raw_tokens);
        true
    }

    pub(crate) fn eat_contextual_kw(&mut self, kind: SyntaxKind) -> bool {
        if !self.at_contextual_kw(kind) {
            return false;
        }
        self.bump_remap(kind);
        true
    }

    /// This consumes the next token as an identifier, even if it is a strict keyword.
    pub(crate) fn bump_remap_any_ident(&mut self) {
        assert!(self.current().is_any_identifier());
        self.bump_remap(T![ident]);
    }

    fn at_composite2(&self, n: usize, k1: SyntaxKind, k2: SyntaxKind) -> bool {
        let Some(n) = n.checked_sub(self.float_split_suffix_len()) else {
            return false;
        };
        self.inp.kind(self.pos + n) == k1
            && self.inp.kind(self.pos + n + 1) == k2
            && self.inp.is_joint(self.pos + n)
    }

    fn at_composite3(&self, n: usize, k1: SyntaxKind, k2: SyntaxKind, k3: SyntaxKind) -> bool {
        let Some(n) = n.checked_sub(self.float_split_suffix_len()) else {
            return false;
        };
        self.inp.kind(self.pos + n) == k1
            && self.inp.kind(self.pos + n + 1) == k2
            && self.inp.kind(self.pos + n + 2) == k3
            && self.inp.is_joint(self.pos + n)
            && self.inp.is_joint(self.pos + n + 1)
    }

    /// Checks if the current token is in `kinds`.
    pub(crate) fn at_ts(&self, kinds: TokenSet) -> bool {
        kinds.contains(self.current())
    }

    /// Checks if the current token is contextual keyword `kw`.
    pub(crate) fn at_contextual_kw(&self, kw: SyntaxKind) -> bool {
        self.inp.contextual_kind(self.pos) == kw
    }

    /// Checks if the nth token is contextual keyword `kw`.
    pub(crate) fn nth_at_contextual_kw(&self, n: usize, kw: SyntaxKind) -> bool {
        self.inp.contextual_kind(self.pos + n) == kw
    }

    /// Starts a new node in the syntax tree. All nodes and tokens
    /// consumed between the `start` and the corresponding `Marker::complete`
    /// belong to the same node.
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len() as u32;
        self.push_event(Event::tombstone());
        Marker::new(pos)
    }

    /// Consume the next token. Panics if the parser isn't currently at `kind`.
    pub(crate) fn bump(&mut self, kind: SyntaxKind) {
        assert!(self.eat(kind));
    }

    /// Advances the parser by one token
    pub(crate) fn bump_any(&mut self) {
        let kind = self.nth(0);
        if kind == EOF {
            return;
        }
        self.do_bump(kind, 1);
    }

    /// Begin splitting the current `FLOAT_NUMBER` into synthetic
    /// `INT_NUMBER` / `DOT` / optional `INT_NUMBER` tokens.
    pub(crate) fn split_float(&mut self) {
        assert_eq!(
            self.float_split_stage,
            FloatSplitStage::None,
            "cannot split more than one float at a time"
        );
        assert!(self.at(SyntaxKind::FLOAT_NUMBER));
        let ends_in_dot = !self.inp.is_joint(self.pos);
        self.float_split_ends_in_dot = ends_in_dot;
        self.float_split_stage = FloatSplitStage::BeforeFirstInt;
        self.pos += 1;
        self.push_event(Event::FloatSplitHack { ends_in_dot });
    }

    /// Advances the parser by one token, remapping its kind.
    /// This is useful to create contextual keywords from
    /// identifiers. For example, the lexer creates a `union`
    /// *identifier* token, but the parser remaps it to the
    /// `union` keyword, and keyword is what ends up in the
    /// final tree.
    pub(crate) fn bump_remap(&mut self, kind: SyntaxKind) {
        if self.nth(0) == EOF {
            // FIXME: panic!?
            return;
        }
        self.do_bump(kind, 1);
    }

    /// Emit error with the `message`
    /// FIXME: this should be much more fancy and support
    /// structured errors with spans and notes, like rustc
    /// does.
    pub(crate) fn error<T: Into<String>>(&mut self, message: T) {
        let err = self.errors.len() as u32;
        self.errors.push(message.into());
        self.push_event(Event::Error { err });
    }

    /// Consume the next token if it is `kind` or emit an error
    /// otherwise.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error(format!("expected {kind:?}"));
        false
    }

    /// Create an error node and consume the next token.
    pub(crate) fn err_and_bump(&mut self, message: &str) {
        let m = self.start();
        self.error(message);
        self.bump_any();
        m.complete(self, ERROR);
    }

    /// Create an error node and consume the next token unless it is in the recovery set.
    ///
    /// Returns true if recovery kicked in.
    pub(crate) fn err_recover(&mut self, message: &str, recovery: TokenSet) -> bool {
        if matches!(self.current(), T!['{'] | T!['}']) {
            self.error(message);
            return true;
        }

        if self.at_ts(recovery) {
            self.error(message);
            return true;
        }

        let m = self.start();
        self.error(message);
        self.bump_any();
        m.complete(self, ERROR);
        false
    }

    fn do_bump(&mut self, kind: SyntaxKind, n_raw_tokens: u8) {
        if self.float_split_stage == FloatSplitStage::None {
            self.pos += n_raw_tokens as usize;
            self.steps.set(0);
            self.push_event(Event::Token { kind, n_raw_tokens });
            return;
        }

        assert_eq!(kind, self.logical_kind(0), "bump kind must match synthetic token");
        // Ignore caller `n_raw_tokens`: synthetic pieces are not Input slots.
        self.steps.set(0);
        self.push_event(Event::Token { kind, n_raw_tokens: 0 });

        self.float_split_stage = match self.float_split_stage {
            FloatSplitStage::None => unreachable!(),
            FloatSplitStage::BeforeFirstInt => FloatSplitStage::BeforeDot,
            FloatSplitStage::BeforeDot if !self.float_split_ends_in_dot => {
                FloatSplitStage::BeforeSecondInt
            }
            FloatSplitStage::BeforeDot | FloatSplitStage::BeforeSecondInt => FloatSplitStage::None,
        };

        if self.float_split_stage == FloatSplitStage::None {
            self.float_split_ends_in_dot = false;
        }
    }

    fn push_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub(crate) fn current_edition(&self) -> Edition {
        self.inp.edition(self.pos)
    }
}

/// See [`Parser::start`].
pub(crate) struct Marker {
    pos: u32,
    bomb: DropBomb,
}

impl Marker {
    fn new(pos: u32) -> Marker {
        Marker { pos, bomb: DropBomb::new("Marker must be either completed or abandoned") }
    }

    /// Finishes the syntax tree node and assigns `kind` to it,
    /// and mark the create a `CompletedMarker` for possible future
    /// operation like `.precede()` to deal with forward_parent.
    pub(crate) fn complete(mut self, p: &mut Parser<'_>, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        let idx = self.pos as usize;
        match &mut p.events[idx] {
            Event::Start { kind: slot, .. } => {
                *slot = kind;
            }
            _ => unreachable!(),
        }
        p.push_event(Event::Finish);
        let end_pos = p.events.len() as u32;
        CompletedMarker::new(self.pos, end_pos, kind)
    }

    /// Abandons the syntax tree node. All its children
    /// are attached to its parent instead.
    pub(crate) fn abandon(mut self, p: &mut Parser<'_>) {
        self.bomb.defuse();
        let idx = self.pos as usize;
        if idx == p.events.len() - 1 {
            assert!(matches!(
                p.events.pop(),
                Some(Event::Start { kind: TOMBSTONE, forward_parent: None })
            ));
        }
    }
}

pub(crate) struct CompletedMarker {
    start_pos: u32,
    end_pos: u32,
    kind: SyntaxKind,
}

impl CompletedMarker {
    fn new(start_pos: u32, end_pos: u32, kind: SyntaxKind) -> Self {
        CompletedMarker { start_pos, end_pos, kind }
    }

    /// This method allows to create a new node which starts
    /// *before* the current one. That is, parser could start
    /// node `A`, then complete it, and then after parsing the
    /// whole `A`, decide that it should have started some node
    /// `B` before starting `A`. `precede` allows to do exactly
    /// that. See also docs about
    /// [`Event::Start::forward_parent`](crate::event::Event::Start::forward_parent).
    ///
    /// Given completed events `[START, FINISH]` and its corresponding
    /// `CompletedMarker(pos: 0, _)`.
    /// Append a new `START` events as `[START, FINISH, NEWSTART]`,
    /// then mark `NEWSTART` as `START`'s parent with saving its relative
    /// distance to `NEWSTART` into forward_parent(=2 in this case);
    pub(crate) fn precede(self, p: &mut Parser<'_>) -> Marker {
        let new_pos = p.start();
        let idx = self.start_pos as usize;
        match &mut p.events[idx] {
            Event::Start { forward_parent, .. } => {
                *forward_parent = Some(fwd_parent(new_pos.pos - self.start_pos));
            }
            _ => unreachable!(),
        }
        new_pos
    }

    /// Extends this completed marker *to the left* up to `m`.
    pub(crate) fn extend_to(self, p: &mut Parser<'_>, mut m: Marker) -> CompletedMarker {
        m.bomb.defuse();
        let idx = m.pos as usize;
        match &mut p.events[idx] {
            Event::Start { forward_parent, .. } => {
                *forward_parent = Some(fwd_parent(self.start_pos - m.pos));
            }
            _ => unreachable!(),
        }
        self
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub(crate) fn last_token(&self, p: &Parser<'_>) -> Option<SyntaxKind> {
        let end_pos = self.end_pos as usize;
        debug_assert_eq!(p.events[end_pos - 1], Event::Finish);
        p.events[..end_pos].iter().rev().find_map(|event| match event {
            Event::Token { kind, .. } => Some(*kind),
            _ => None,
        })
    }
}
