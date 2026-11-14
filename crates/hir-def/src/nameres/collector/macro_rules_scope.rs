//! Textual scopes of `macro_rules!` macros during def collection.
//!
//! This mirrors rustc's `MacroRulesScope` (`compiler/rustc_resolve/src/macros.rs`).
//!
//! ## Core idea
//!
//! Every point in the source has a "scope". The scope is a pointer to the last "thing" that
//! can affect which macros are in scope. There's 2 such things:
//! - Macro definitions (obviously)
//! - Macro invocations (since they can expand to more macro definitions)
//!
//! Each has a "parent" pointing to the previous scope in the current module or the parent
//! module, forming a backwards linked list.
//!
//! Child modules get their own branch in the tree, except if they have `#[macro_use]`
//! their branch is inlined into the parent module's scope.
//!
//! Example:
//!
//! ```rust,ignore
//! macro_rules! a { () => {} }
//! b!();
//! mod child1 { c!(); }
//! #[macro_use] mod child2 { macro_rules! d { () => {} } }
//! e!();
//! ```
//!
//! results in this tree (assuming no invocations are expanded yet):
//! ```ignore
//! Inv(e) -> Def(d) --> Inv(b) -> Def(a) -> Empty
//!           Inv(c) -/
//! ```
//!
//! Each definition and invocation gets an ID. Pointers go through "cells" to allow for
//! "path compression": when an invocation gets expanded we want to repoint everything
//! that pointed to it to the expansion result. We can do this by updating the cell instead
//! of walking the entire tree. This makes invocations that expand to no macro definitions
//! completely disappear from the tree.
//!
//! ## The algorithm
//!
//! To resolve a `name` in a `scope`, walk the linked list backwards:
//! - loop:
//!     - look at `cells[scope]`
//!         - Empty                    → NotFound
//!         - Def(d)                   → if d.name == name: Found(d.id); else scope = d.parent
//!         - Invocation(i), expanded  → scope = i.output (skip into the expansion)
//!         - Invocation(i), forced    → scope = i.parent (treat as empty. more on this later)
//!         - Invocation(i), unexpanded→ Undetermined  (stop: it might define `name`)
//!
//! During collecting, each `ModCollector` has a cursor (field `macro_rules_scope`) which tracks "where we are now".
//!
//! First, the source is walked. For each item:
//!
//! - `macro_rules! m` -> Append the definition to the cursor: `cursor = define(cursor, m)`.
//! - `mod child {...}` -> Start the child collector with the cursor. Parent cursor is unchanged.
//! - `#[macro_use] mod child {...}` -> Start the child collector with the cursor. Parent cursor is set to the child's final cursor.
//! - `m!()` -> Try to resolve `m`.
//!     - Resolved -> expand now, collect the output starting with the cursor, set cursor = scope at end of expansion output.
//!     - Unresolved -> Append the invocation to the cursor: `cursor = invocation(cursor, m)`. Append it to the queue.
//!
//! Second, attempt resolving unresolved macros again. Loop until reaching a fixed point.
//!
//! - loop:
//!     - For each invocation in the queue:
//!         - Attempt resolving it
//!         - If resolved, change the invocation to "Expanded".
//!
//! At the end, every module walks its final scope backwards, collecting all the macros into
//! the `legacy_macros` map, which is what the rest of rust-analyzer reads.
//!
//! ## Some details
//!
//! If the fixpoint loop gets stuck and there's still unresolved invocations, they're all switched to a "force" mode, which makes resolution
//! assume they expand to no macro definitions. This is jank but it's close to what rustc does, see
//! "Differences from rustc" below.
//!
//! Attribute macros are also enqueued as macro invocations.
//!
//! Derives too, one invocation per derive. The invocations are ordered after the item, matching rustc.
//!
//! ## Differences from rustc
//!
//! rustc doesn't stop at an unexpanded invocation. It notes that the result is indeterminate and
//! keeps walking past it (`visit_scopes` and `resolve_ident_in_scope` in
//! `compiler/rustc_resolve/src/ident.rs`). If the name is found further out, rustc uses it right
//! away, and reports an ambiguity error (E0659) if the invocation later defines a closer one. It
//! only waits if the name isn't found anywhere.
//!
//! We stop and wait instead, because we defer more invocations than rustc. rustc expands
//! invocations in source order as soon as they resolve, so e.g. `foo::def!()` is usually already
//! expanded when the calls after it are resolved. We can't resolve paths while walking the source
//! (imports aren't resolved yet), so we defer all path calls. Walking past them would resolve later
//! calls to outer definitions that rustc sees shadowed.
//!
//! Waiting gives the same results for programs rustc accepts, but it can get stuck where rustc
//! doesn't:
//!
//! ```rust,ignore
//! x::def!();   // needs `mod x`
//! make_x!();   // defines `mod x`, but waits for `x::def!()`
//! ```
//!
//! The force step gets us out of this: `x::def!()` is forced, so `make_x!()` resolves and expands.
//! This is why forcing differs from rustc's force mode, which forces all remaining invocations at
//! once and for good:
//! - Invocations that are only waiting for other invocations are not forced.
//! - Forced invocations stay in the queue, so `x::def!()` is still expanded once `mod x` exists.

use hir_expand::name::Name;
use la_arena::{Arena, Idx, RawIdx};

use crate::MacroRulesId;

/// A scope in the textual scope of `macro_rules!` macros.
///
/// Invocation scopes are updated in place once the invocation is expanded ("path compression" in
/// rustc), so that chains don't need to go through every expanded invocation.
pub(super) type MacroRulesScopeId = Idx<MacroRulesScope>;
pub(super) type MacroInvocationId = Idx<MacroInvocation>;
type MacroRulesDefId = Idx<MacroRulesDef>;

#[derive(Debug, Clone, Copy)]
pub(super) enum MacroRulesScope {
    Empty,
    /// A `macro_rules!` definition.
    Def(MacroRulesDefId),
    /// A macro invocation that can expand to `macro_rules!` definitions.
    Invocation(MacroInvocationId),
}

#[derive(Debug)]
pub(super) struct MacroRulesDef {
    name: Name,
    id: MacroRulesId,
    parent: MacroRulesScopeId,
}

#[derive(Debug, Clone, Copy)]
enum InvocationOutput {
    Unexpanded,
    /// The invocation could not be resolved, and was treated as expanding to nothing (rustc's
    /// "force mode", where unresolved macros expand to a dummy).
    Forced,
    Expanded(MacroRulesScopeId),
}

#[derive(Debug)]
pub(super) struct MacroInvocation {
    /// The scope the invocation is in.
    parent: MacroRulesScopeId,
    /// The scope holding [`MacroRulesScope::Invocation`] for this invocation.
    scope: MacroRulesScopeId,
    output: InvocationOutput,
}

enum Step<'a> {
    Empty,
    Def(&'a MacroRulesDef),
    Unexpanded(&'a MacroInvocation),
}

pub(super) enum MacroRulesResolution {
    Resolved(MacroRulesId),
    /// The name can't be resolved before an invocation on the chain is expanded.
    Indeterminate,
    Unresolved,
}

#[derive(Debug)]
pub(super) struct MacroRulesScopes {
    scopes: Arena<MacroRulesScope>,
    defs: Arena<MacroRulesDef>,
    invocations: Arena<MacroInvocation>,
}

impl Default for MacroRulesScopes {
    fn default() -> Self {
        let mut scopes = Arena::new();
        let empty = scopes.alloc(MacroRulesScope::Empty);
        debug_assert_eq!(empty, Self::EMPTY);
        MacroRulesScopes { scopes, defs: Arena::new(), invocations: Arena::new() }
    }
}

impl MacroRulesScopes {
    /// The empty scope at the start of the crate (or block).
    pub(super) const EMPTY: MacroRulesScopeId = Idx::from_raw(RawIdx::from_u32(0));

    /// Adds a `macro_rules!` definition after `parent`, returning the scope after the definition.
    pub(super) fn define(
        &mut self,
        parent: MacroRulesScopeId,
        name: Name,
        id: MacroRulesId,
    ) -> MacroRulesScopeId {
        let def = self.defs.alloc(MacroRulesDef { name, id, parent });
        self.scopes.alloc(MacroRulesScope::Def(def))
    }

    /// Adds an unexpanded macro invocation after `parent`, returning the invocation and the scope
    /// after the invocation.
    pub(super) fn invocation(
        &mut self,
        parent: MacroRulesScopeId,
    ) -> (MacroInvocationId, MacroRulesScopeId) {
        let invocation = self.invocations.alloc(MacroInvocation {
            parent,
            scope: Self::EMPTY,
            output: InvocationOutput::Unexpanded,
        });
        let scope = self.scopes.alloc(MacroRulesScope::Invocation(invocation));
        self.invocations[invocation].scope = scope;
        (invocation, scope)
    }

    /// The scope the invocation is in, which is where its own path is resolved and where the
    /// chain of its expansion starts.
    pub(super) fn invocation_parent(&self, invocation: MacroInvocationId) -> MacroRulesScopeId {
        self.invocations[invocation].parent
    }

    pub(super) fn is_unexpanded(&self, invocation: MacroInvocationId) -> bool {
        matches!(self.invocations[invocation].output, InvocationOutput::Unexpanded)
    }

    /// Records that the invocation has been expanded and that the textual scope at the end of its
    /// expansion is `output`.
    pub(super) fn set_expanded(
        &mut self,
        invocation: MacroInvocationId,
        output: MacroRulesScopeId,
    ) {
        let invocation = &mut self.invocations[invocation];
        invocation.output = InvocationOutput::Expanded(output);
        // Path compression: everything after the invocation now directly continues with the
        // output of the expansion.
        let scope = invocation.scope;
        self.scopes[scope] = self.scopes[output];
    }

    /// Records that the invocation could not be resolved, so it expands to nothing.
    pub(super) fn set_forced(&mut self, invocation: MacroInvocationId) {
        let invocation = &mut self.invocations[invocation];
        if let InvocationOutput::Unexpanded = invocation.output {
            invocation.output = InvocationOutput::Forced;
        }
    }

    /// Steps over expanded invocations, returning the first definition, unexpanded invocation, or
    /// the end of the chain.
    fn skip_expanded(&self, mut scope: MacroRulesScopeId) -> Step<'_> {
        loop {
            match self.scopes[scope] {
                MacroRulesScope::Empty => return Step::Empty,
                MacroRulesScope::Def(def) => return Step::Def(&self.defs[def]),
                MacroRulesScope::Invocation(invocation) => {
                    let invocation = &self.invocations[invocation];
                    match invocation.output {
                        InvocationOutput::Expanded(output) => scope = output,
                        InvocationOutput::Forced => scope = invocation.parent,
                        InvocationOutput::Unexpanded => return Step::Unexpanded(invocation),
                    }
                }
            }
        }
    }

    /// Resolves `name` in the textual scope `scope`, considering only definitions for which
    /// `filter` returns `true`.
    pub(super) fn resolve(
        &self,
        mut scope: MacroRulesScopeId,
        name: &Name,
        mut filter: impl FnMut(MacroRulesId) -> bool,
    ) -> MacroRulesResolution {
        loop {
            match self.skip_expanded(scope) {
                Step::Empty => return MacroRulesResolution::Unresolved,
                Step::Unexpanded(_) => return MacroRulesResolution::Indeterminate,
                Step::Def(def) => {
                    if def.name == *name && filter(def.id) {
                        return MacroRulesResolution::Resolved(def.id);
                    }
                    scope = def.parent;
                }
            }
        }
    }

    /// All `macro_rules!` definitions visible in `scope`, in definition order. Unexpanded
    /// invocations are treated as expanding to nothing.
    pub(super) fn visible_macros(&self, mut scope: MacroRulesScopeId) -> Vec<(Name, MacroRulesId)> {
        let mut res = Vec::new();
        loop {
            match self.skip_expanded(scope) {
                Step::Empty => break,
                Step::Unexpanded(invocation) => scope = invocation.parent,
                Step::Def(def) => {
                    res.push((def.name.clone(), def.id));
                    scope = def.parent;
                }
            }
        }
        res.reverse();
        res
    }
}
