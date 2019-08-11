//! Syntax Tree library used throughout the rust analyzer.
//!
//! Properties:
//!   - easy and fast incremental re-parsing
//!   - graceful handling of errors
//!   - full-fidelity representation (*any* text can be precisely represented as
//!     a syntax tree)
//!
//! For more information, see the [RFC]. Current implementation is inspired by
//! the [Swift] one.
//!
//! The most interesting modules here are `syntax_node` (which defines concrete
//! syntax tree) and `ast` (which defines abstract syntax tree on top of the
//! CST). The actual parser live in a separate `ra_parser` crate, though the
//! lexer lives in this crate.
//!
//! See `api_walkthrough` test in this file for a quick API tour!
//!
//! [RFC]: <https://github.com/rust-lang/rfcs/pull/2256>
//! [Swift]: <https://github.com/apple/swift/blob/13d593df6f359d0cb2fc81cfaac273297c539455/lib/Syntax/README.md>

mod syntax_node;
mod syntax_error;
mod parsing;
mod validation;
mod ptr;
#[cfg(test)]
mod tests;

pub mod algo;
pub mod ast;
#[doc(hidden)]
pub mod fuzz;

use std::{fmt::Write, marker::PhantomData, sync::Arc};

use ra_text_edit::AtomTextEdit;

use crate::syntax_node::GreenNode;

pub use crate::{
    algo::InsertPosition,
    ast::{AstNode, AstToken},
    parsing::{classify_literal, tokenize, Token},
    ptr::{AstPtr, SyntaxNodePtr},
    syntax_error::{Location, SyntaxError, SyntaxErrorKind},
    syntax_node::{
        Direction, NodeOrToken, SyntaxElement, SyntaxNode, SyntaxToken, SyntaxTreeBuilder,
    },
};
pub use ra_parser::{SyntaxKind, T};
pub use rowan::{SmolStr, SyntaxText, TextRange, TextUnit, TokenAtOffset, WalkEvent};

/// `Parse` is the result of the parsing: a syntax tree and a collection of
/// errors.
///
/// Note that we always produce a syntax tree, even for completely invalid
/// files.
#[derive(Debug, PartialEq, Eq)]
pub struct Parse<T> {
    green: GreenNode,
    errors: Arc<Vec<SyntaxError>>,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Parse<T> {
    fn clone(&self) -> Parse<T> {
        Parse { green: self.green.clone(), errors: self.errors.clone(), _ty: PhantomData }
    }
}

impl<T> Parse<T> {
    fn new(green: GreenNode, errors: Vec<SyntaxError>) -> Parse<T> {
        Parse { green, errors: Arc::new(errors), _ty: PhantomData }
    }

    pub fn syntax_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
}

impl<T: AstNode> Parse<T> {
    pub fn to_syntax(self) -> Parse<SyntaxNode> {
        Parse { green: self.green, errors: self.errors, _ty: PhantomData }
    }

    pub fn tree(&self) -> T {
        T::cast(self.syntax_node()).unwrap()
    }

    pub fn errors(&self) -> &[SyntaxError] {
        &*self.errors
    }

    pub fn ok(self) -> Result<T, Arc<Vec<SyntaxError>>> {
        if self.errors.is_empty() {
            Ok(self.tree())
        } else {
            Err(self.errors)
        }
    }
}

impl Parse<SyntaxNode> {
    pub fn cast<N: AstNode>(self) -> Option<Parse<N>> {
        if N::cast(self.syntax_node()).is_some() {
            Some(Parse { green: self.green, errors: self.errors, _ty: PhantomData })
        } else {
            None
        }
    }
}

impl Parse<SourceFile> {
    pub fn debug_dump(&self) -> String {
        let mut buf = format!("{:#?}", self.tree().syntax());
        for err in self.errors.iter() {
            writeln!(buf, "error {:?}: {}", err.location(), err.kind()).unwrap();
        }
        buf
    }

    pub fn reparse(&self, edit: &AtomTextEdit) -> Parse<SourceFile> {
        self.incremental_reparse(edit).unwrap_or_else(|| self.full_reparse(edit))
    }

    fn incremental_reparse(&self, edit: &AtomTextEdit) -> Option<Parse<SourceFile>> {
        // FIXME: validation errors are not handled here
        parsing::incremental_reparse(self.tree().syntax(), edit, self.errors.to_vec()).map(
            |(green_node, errors, _reparsed_range)| Parse {
                green: green_node,
                errors: Arc::new(errors),
                _ty: PhantomData,
            },
        )
    }

    fn full_reparse(&self, edit: &AtomTextEdit) -> Parse<SourceFile> {
        let text = edit.apply(self.tree().syntax().text().to_string());
        SourceFile::parse(&text)
    }
}

/// `SourceFile` represents a parse tree for a single Rust file.
pub use crate::ast::SourceFile;

impl SourceFile {
    pub fn parse(text: &str) -> Parse<SourceFile> {
        let (green, mut errors) = parsing::parse_text(text);
        let root = SyntaxNode::new_root(green.clone());

        if cfg!(debug_assertions) {
            validation::validate_block_structure(&root);
        }

        errors.extend(validation::validate(&root));

        assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
        Parse { green, errors: Arc::new(errors), _ty: PhantomData }
    }
}

/// This test does not assert anything and instead just shows off the crate's
/// API.
#[test]
fn api_walkthrough() {
    use ast::{ModuleItemOwner, NameOwner};

    let source_code = "
        fn foo() {
            1 + 1
        }
    ";
    // `SourceFile` is the main entry point.
    //
    // The `parse` method returns a `Parse` -- a pair of syntax tree and a list
    // of errors. That is, syntax tree is constructed even in presence of errors.
    let parse = SourceFile::parse(source_code);
    assert!(parse.errors().is_empty());

    // The `tree` method returns an owned syntax node of type `SourceFile`.
    // Owned nodes are cheap: inside, they are `Rc` handles to the underling data.
    let file: SourceFile = parse.tree();

    // `SourceFile` is the root of the syntax tree. We can iterate file's items.
    // Let's fetch the `foo` function.
    let mut func = None;
    for item in file.items() {
        match item.kind() {
            ast::ModuleItemKind::FnDef(f) => func = Some(f),
            _ => unreachable!(),
        }
    }
    let func: ast::FnDef = func.unwrap();

    // Each AST node has a bunch of getters for children. All getters return
    // `Option`s though, to account for incomplete code. Some getters are common
    // for several kinds of node. In this case, a trait like `ast::NameOwner`
    // usually exists. By convention, all ast types should be used with `ast::`
    // qualifier.
    let name: Option<ast::Name> = func.name();
    let name = name.unwrap();
    assert_eq!(name.text(), "foo");

    // Let's get the `1 + 1` expression!
    let block: ast::Block = func.body().unwrap();
    let expr: ast::Expr = block.expr().unwrap();

    // "Enum"-like nodes are represented using the "kind" pattern. It allows us
    // to match exhaustively against all flavors of nodes, while maintaining
    // internal representation flexibility. The drawback is that one can't write
    // nested matches as one pattern.
    let bin_expr: ast::BinExpr = match expr.kind() {
        ast::ExprKind::BinExpr(e) => e,
        _ => unreachable!(),
    };

    // Besides the "typed" AST API, there's an untyped CST one as well.
    // To switch from AST to CST, call `.syntax()` method:
    let expr_syntax: &SyntaxNode = expr.syntax();

    // Note how `expr` and `bin_expr` are in fact the same node underneath:
    assert!(expr_syntax == bin_expr.syntax());

    // To go from CST to AST, `AstNode::cast` function is used:
    let _expr: ast::Expr = match ast::Expr::cast(expr_syntax.clone()) {
        Some(e) => e,
        None => unreachable!(),
    };

    // The two properties each syntax node has is a `SyntaxKind`:
    assert_eq!(expr_syntax.kind(), SyntaxKind::BIN_EXPR);

    // And text range:
    assert_eq!(expr_syntax.text_range(), TextRange::from_to(32.into(), 37.into()));

    // You can get node's text as a `SyntaxText` object, which will traverse the
    // tree collecting token's text:
    let text: SyntaxText = expr_syntax.text();
    assert_eq!(text.to_string(), "1 + 1");

    // There's a bunch of traversal methods on `SyntaxNode`:
    assert_eq!(expr_syntax.parent().as_ref(), Some(block.syntax()));
    assert_eq!(block.syntax().first_child_or_token().map(|it| it.kind()), Some(T!['{']));
    assert_eq!(
        expr_syntax.next_sibling_or_token().map(|it| it.kind()),
        Some(SyntaxKind::WHITESPACE)
    );

    // As well as some iterator helpers:
    let f = expr_syntax.ancestors().find_map(ast::FnDef::cast);
    assert_eq!(f, Some(func));
    assert!(expr_syntax.siblings_with_tokens(Direction::Next).any(|it| it.kind() == T!['}']));
    assert_eq!(
        expr_syntax.descendants_with_tokens().count(),
        8, // 5 tokens `1`, ` `, `+`, ` `, `!`
           // 2 child literal expressions: `1`, `1`
           // 1 the node itself: `1 + 1`
    );

    // There's also a `preorder` method with a more fine-grained iteration control:
    let mut buf = String::new();
    let mut indent = 0;
    for event in expr_syntax.preorder_with_tokens() {
        match event {
            WalkEvent::Enter(node) => {
                let text = match &node {
                    NodeOrToken::Node(it) => it.text().to_string(),
                    NodeOrToken::Token(it) => it.text().to_string(),
                };
                buf += &format!("{:indent$}{:?} {:?}\n", " ", text, node.kind(), indent = indent);
                indent += 2;
            }
            WalkEvent::Leave(_) => indent -= 2,
        }
    }
    assert_eq!(indent, 0);
    assert_eq!(
        buf.trim(),
        r#"
"1 + 1" BIN_EXPR
  "1" LITERAL
    "1" INT_NUMBER
  " " WHITESPACE
  "+" PLUS
  " " WHITESPACE
  "1" LITERAL
    "1" INT_NUMBER
"#
        .trim()
    );

    // To recursively process the tree, there are three approaches:
    // 1. explicitly call getter methods on AST nodes.
    // 2. use descendants and `AstNode::cast`.
    // 3. use descendants and the visitor.
    //
    // Here's how the first one looks like:
    let exprs_cast: Vec<String> = file
        .syntax()
        .descendants()
        .filter_map(ast::Expr::cast)
        .map(|expr| expr.syntax().text().to_string())
        .collect();

    // An alternative is to use a visitor. The visitor does not do traversal
    // automatically (so it's more akin to a generic lambda) and is constructed
    // from closures. This seems more flexible than a single generated visitor
    // trait.
    use algo::visit::{visitor, Visitor};
    let mut exprs_visit = Vec::new();
    for node in file.syntax().descendants() {
        if let Some(result) =
            visitor().visit::<ast::Expr, _>(|expr| expr.syntax().text().to_string()).accept(&node)
        {
            exprs_visit.push(result);
        }
    }
    assert_eq!(exprs_cast, exprs_visit);
}

/// Generates `if let` chain to match provided `SyntaxNode` with `ast::*` nodes and to call
/// corresponding function with node that matched. Collects syntax errors to the
/// `Vec<SyntaxError>` provided.
///
/// Syntax:
///
/// ```plain
/// match_ast! {
///     match <NODE_VAR> {
///         ast::<NODE TYPE1> => <HANDLER FN1>, &mut <ERRORS VEC>,
///         ast::<NODE TYPE2> => <HANDLER FN2>, &mut <ERRORS VEC>,
///         ...
///         ast::<NODE TYPEn> => <HANDLER FNn>, &mut <ERRORS VEC>,
///     }
/// }
/// ```
///
/// Example:
/// ```rust
/// let mut errors = Vec::new();
/// for node in nodes {
///     match_ast! {
///         match node {
///             ast::Literal => validate_literal, &mut errors,
///             ast::Block => validate_block_node, &mut errors,
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! match_ast {
    (match $node:ident {
        $($p:ty => $f:ident, $err:expr),+
        $(,)*  // consumes trailing commas
    }) => {{
            $(
                if <$p>::can_cast($node.kind()) {
                    if let Some(it) = <$p>::cast($node.clone()) {
                        $f(it, $err)
                    }
                } else
            )+

            {
                ()
            }
    }};
}

/// Same as `match_ast!` but calls `.name_ref()` function on matched nodes.
///
/// Example:
/// ```rust
/// let mut errors = Vec::new();
/// for node in nodes {
///     match_ast_nameref! {
///         match node {
///              ast::FieldExpr => validate_numeric_name, &mut errors,
///              ast::NamedField => validate_numeric_name, &mut errors,
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! match_ast_nameref {
    (match $node:ident {
        $($p:ty => $f:ident, $err:expr),+
        $(,)* // consumes trailing commas
    }) => {{
            $(
                if <$p>::can_cast($node.kind()) {
                    if let Some(it) = <$p>::cast($node.clone()) {
                        $f(it.name_ref(), $err)
                    }
                } else
            )+

            {
                ()
            }
    }};
}
