use hir::HirDisplay;
use ide_db::{assists::AssistId, defs::Definition};
use stdx::to_upper_snake_case;
use syntax::{
    AstNode, T,
    ast::{self, HasName},
    syntax_editor::Position,
};

use crate::{
    assist_context::{AssistContext, Assists},
    utils,
};

// Assist: promote_local_to_const
//
// Promotes a local variable to a const item changing its name to a `SCREAMING_SNAKE_CASE` variant
// if the local uses no non-const expressions.
//
// ```
// fn main() {
//     let foo$0 = true;
//
//     if foo {
//         println!("It's true");
//     } else {
//         println!("It's false");
//     }
// }
// ```
// ->
// ```
// fn main() {
//     const $0FOO: bool = true;
//
//     if FOO {
//         println!("It's true");
//     } else {
//         println!("It's false");
//     }
// }
// ```
pub(crate) fn promote_local_to_const(acc: &mut Assists, ctx: &AssistContext<'_, '_>) -> Option<()> {
    let pat = ctx.find_node_at_offset_with_descend::<ast::IdentPat>()?;
    let name = pat.name()?;
    if !pat.is_simple_ident() {
        cov_mark::hit!(promote_local_non_simple_ident);
        return None;
    }
    let let_stmt = pat.syntax().parent().and_then(ast::LetStmt::cast)?;

    let module = ctx.sema.scope(pat.syntax())?.module();
    let local = ctx.sema.to_def(&pat)?;
    let ty = ctx.sema.type_of_pat(&pat.into())?.original;
    let ty = ty.display_source_code(ctx.db(), module.into(), false).ok()?;

    let initializer = let_stmt.initializer()?;
    if !utils::is_body_const(&ctx.sema, &initializer) {
        cov_mark::hit!(promote_local_non_const);
        return None;
    }

    let const_name = to_upper_snake_case(&name.to_string());
    if const_name.is_empty() || const_name.chars().all(|c| c == '_') {
        return None;
    }

    let let_stmt_range = utils::original_range_in(ctx.file_id(), &ctx.sema, let_stmt.syntax())?;
    let node_in_source = let_stmt.syntax().text_range() == let_stmt_range;

    let usages = Definition::Local(local).usages(&ctx.sema).all();
    if let Some(usages) = usages.references.get(&ctx.file_id()) {
        let shorthand_in_macro = usages.iter().any(|usage| {
            usage.name.as_name_ref().is_some_and(|name_ref| {
                ast::RecordExprField::for_field_name(name_ref).is_some_and(|field| {
                    field.syntax().parent().and_then(|list| list.parent()).is_some_and(|expr| {
                        utils::original_range_in(ctx.file_id(), &ctx.sema, &expr).is_none()
                    })
                })
            })
        });
        if shorthand_in_macro {
            cov_mark::hit!(promote_local_shorthand_in_macro);
            return None;
        }
    }

    acc.add(
        AssistId::refactor("promote_local_to_const"),
        "Promote local to constant",
        let_stmt_range,
        |edit| {
            let source = ctx.source_file().syntax();
            let editor = edit.make_editor(source);
            let make = editor.make();

            if let Some(usages) = usages.references.get(&ctx.file_id()) {
                for usage in usages {
                    let Some(usage_name) = usage.name.as_name_ref() else {
                        continue;
                    };
                    let place = utils::cover_edit_range(source, usage.range);
                    if ast::RecordExprField::for_field_name(usage_name).is_some() {
                        editor.insert_all(
                            Position::after(place.end()),
                            vec![
                                make.token(T![:]).into(),
                                make.whitespace(" ").into(),
                                make.name_ref(&const_name).syntax().clone().into(),
                            ],
                        );
                    } else {
                        editor.replace_all(
                            place,
                            vec![make.name_ref(&const_name).syntax().clone().into()],
                        );
                    }
                }
            }

            let item =
                make.item_const(None, None, make.name(&const_name), make.ty(&ty), initializer);

            if node_in_source && let Some((cap, name)) = ctx.config.snippet_cap.zip(item.name()) {
                let tabstop = edit.make_tabstop_before(cap);
                editor.add_annotation(name.syntax().clone(), tabstop);
            }

            let place = utils::cover_edit_range(source, let_stmt_range);
            editor.replace_all(place, vec![item.syntax().clone().into()]);
            edit.add_file_edits(ctx.vfs_file_id(), editor);
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_assist, check_assist_not_applicable};

    use super::*;

    #[test]
    fn simple() {
        check_assist(
            promote_local_to_const,
            r"
fn foo() {
    let x$0 = 0;
    let y = x;
}
",
            r"
fn foo() {
    const $0X: i32 = 0;
    let y = X;
}
",
        );
    }

    #[test]
    fn multiple_uses() {
        check_assist(
            promote_local_to_const,
            r"
fn foo() {
    let x$0 = 0;
    let y = x;
    let z = (x, x, x, x);
}
",
            r"
fn foo() {
    const $0X: i32 = 0;
    let y = X;
    let z = (X, X, X, X);
}
",
        );
    }

    #[test]
    fn usage_in_field_shorthand() {
        check_assist(
            promote_local_to_const,
            r"
struct Foo {
    bar: usize,
}

fn main() {
    let $0bar = 0;
    let foo = Foo { bar };
}
",
            r"
struct Foo {
    bar: usize,
}

fn main() {
    const $0BAR: usize = 0;
    let foo = Foo { bar: BAR };
}
",
        )
    }

    #[test]
    fn usage_in_macro() {
        check_assist(
            promote_local_to_const,
            r"
macro_rules! identity {
    ($body:expr) => {
        $body
    }
}

fn baz() -> usize {
    let $0foo = 2;
    identity![foo]
}
",
            r"
macro_rules! identity {
    ($body:expr) => {
        $body
    }
}

fn baz() -> usize {
    const $0FOO: usize = 2;
    identity![FOO]
}
",
        )
    }

    #[test]
    fn usage_shorthand_in_macro() {
        check_assist(
            promote_local_to_const,
            r"
struct Foo {
    foo: usize,
}

macro_rules! identity {
    ($body:expr) => {
        $body
    };
}

fn baz() -> Foo {
    let $0foo = 2;
    identity![Foo { foo }]
}
",
            r"
struct Foo {
    foo: usize,
}

macro_rules! identity {
    ($body:expr) => {
        $body
    };
}

fn baz() -> Foo {
    const $0FOO: usize = 2;
    identity![Foo { foo: FOO }]
}
",
        )
    }

    #[test]
    fn not_applicable_non_const_meth_call() {
        cov_mark::check!(promote_local_non_const);
        check_assist_not_applicable(
            promote_local_to_const,
            r"
struct Foo;
impl Foo {
    fn foo(self) {}
}
fn foo() {
    let x$0 = Foo.foo();
}
",
        );
    }

    #[test]
    fn not_applicable_non_const_call() {
        check_assist_not_applicable(
            promote_local_to_const,
            r"
fn bar(self) {}
fn foo() {
    let x$0 = bar();
}
",
        );
    }

    #[test]
    fn not_applicable_unknown_ty() {
        check_assist(
            promote_local_to_const,
            r"
fn foo() {
    let x$0 = bar();
}
",
            r"
fn foo() {
    const $0X: _ = bar();
}
",
        );
    }

    #[test]
    fn not_applicable_when_name_converts_to_all_underscores() {
        check_assist_not_applicable(
            promote_local_to_const,
            r"
fn foo() {
    let _$0_ = 0;
    __;
}
",
        );
    }

    #[test]
    fn let_in_macro() {
        check_assist(
            promote_local_to_const,
            r#"
//- proc_macros: identity
#[proc_macros::identity]
fn f() {
    let x$0 = 0;
    let _ = x;
}
"#,
            r"
#[proc_macros::identity]
fn f() {
    const X: i32 = 0;
    let _ = X;
}
",
        );

        check_assist(
            promote_local_to_const,
            r"
macro_rules! id { ($($tt:tt)*) => { $($tt)* }; }

fn f() {
    id! {
        let x$0 = 0;
        let _ = x;
    }
}
",
            r"
macro_rules! id { ($($tt:tt)*) => { $($tt)* }; }

fn f() {
    id! {
        const X: i32 = 0;
        let _ = X;
    }
}
",
        );
    }

    #[test]
    fn not_applicable_shorthand_in_macro() {
        cov_mark::check!(promote_local_shorthand_in_macro);
        check_assist_not_applicable(
            promote_local_to_const,
            r"
struct Foo {
    foo: usize,
}

macro_rules! make_foo {
    ($v:ident) => {
        Foo { $v }
    };
}

fn baz() -> Foo {
    let $0foo = 2;
    make_foo!(foo)
}
",
        );
    }

    #[test]
    fn not_applicable_non_simple_ident() {
        cov_mark::check!(promote_local_non_simple_ident);
        check_assist_not_applicable(
            promote_local_to_const,
            r"
fn foo() {
    let ref x$0 = ();
}
",
        );
        check_assist_not_applicable(
            promote_local_to_const,
            r"
fn foo() {
    let mut x$0 = ();
}
",
        );
    }
}
