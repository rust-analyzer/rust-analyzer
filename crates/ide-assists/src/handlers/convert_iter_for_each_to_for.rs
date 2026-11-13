use hir::{Name, sym};
use ide_db::{famous_defs::FamousDefs, syntax_helpers::node_ext};
use syntax::{
    AstNode,
    ast::{self, HasArgList, HasLoopBody, edit::AstNodeEdit, syntax_factory::SyntaxFactory},
    syntax_editor::SyntaxEditor,
};

use crate::{AssistContext, AssistId, Assists, utils::wrap_paren};

// Assist: convert_iter_for_each_to_for
//
// Converts an Iterator::for_each function into a for loop.
//
// ```
// # //- minicore: iterators
// # use core::iter;
// fn main() {
//     let iter = iter::repeat((9, 2));
//     iter.for_each$0(|(x, y)| {
//         println!("x: {}, y: {}", x, y);
//     });
// }
// ```
// ->
// ```
// # use core::iter;
// fn main() {
//     let iter = iter::repeat((9, 2));
//     for (x, y) in iter {
//         println!("x: {}, y: {}", x, y);
//     }
// }
// ```
pub(crate) fn convert_iter_for_each_to_for(
    acc: &mut Assists,
    ctx: &AssistContext<'_, '_>,
) -> Option<()> {
    let method = ctx.find_node_at_offset::<ast::MethodCallExpr>()?;

    let closure = match method.arg_list()?.args().next()? {
        ast::Expr::ClosureExpr(expr) => expr,
        _ => return None,
    };

    // FIXME: supports try_for_each() and ControlFlow handler
    let (method, receiver) = validate_method_call_expr(ctx, method)?;

    let param_list = closure.param_list()?;
    let param = param_list.params().next()?.pat()?;
    let body = closure.body()?;

    let stmt = method.syntax().parent().and_then(ast::ExprStmt::cast);
    let range = stmt.as_ref().map_or(method.syntax(), AstNode::syntax).text_range();

    acc.add(
        AssistId::refactor_rewrite("convert_iter_for_each_to_for"),
        "Replace this `Iterator::for_each` with a for loop",
        range,
        |builder| {
            let target_node = stmt.as_ref().map_or(method.syntax(), AstNode::syntax);
            let editor = builder.make_editor(target_node);
            let make = editor.make();
            let indent =
                stmt.as_ref().map_or_else(|| method.indent_level(), ast::ExprStmt::indent_level);

            let block = match body {
                ast::Expr::BlockExpr(block) => block.reset_indent(),
                _ => make.block_expr(Vec::new(), Some(body.reset_indent().indent(1.into()))),
            }
            .indent(indent);
            let block = replace_return_with_continue(block);

            let expr_for_loop = make.expr_for_loop(param, receiver, block);
            editor.replace(target_node, expr_for_loop.syntax());
            builder.add_file_edits(ctx.vfs_file_id(), editor);
        },
    )
}

// Assist: convert_for_loop_with_for_each
//
// Converts a for loop into a for_each loop on the Iterator.
//
// ```
// fn main() {
//     let x = vec![1, 2, 3];
//     for$0 v in x {
//         let y = v * 2;
//     }
// }
// ```
// ->
// ```
// fn main() {
//     let x = vec![1, 2, 3];
//     x.into_iter().for_each(|v| {
//         let y = v * 2;
//     });
// }
// ```
pub(crate) fn convert_for_loop_with_for_each(
    acc: &mut Assists,
    ctx: &AssistContext<'_, '_>,
) -> Option<()> {
    let for_loop = ctx.find_node_at_offset::<ast::ForExpr>()?;
    let iterable = for_loop.iterable()?;
    let pat = for_loop.pat()?;
    let body = for_loop.loop_body()?;
    if body.syntax().text_range().start() < ctx.offset() {
        cov_mark::hit!(not_available_in_body);
        return None;
    }

    acc.add(
        AssistId::refactor_rewrite("convert_for_loop_with_for_each"),
        "Replace this for loop with `Iterator::for_each`",
        for_loop.syntax().text_range(),
        |builder| {
            let editor = builder.make_editor(for_loop.syntax());
            let make = editor.make();

            let mut receiver = iterable.clone();

            let iter_method = if let Some((expr_behind_ref, method, krate)) =
                is_ref_and_impls_iter_method(&ctx.sema, &iterable)
            {
                receiver = expr_behind_ref;
                // We have either "for x in &col" and col implements a method called iter
                //             or "for x in &mut col" and col implements a method called iter_mut
                method.display(ctx.db(), krate.edition(ctx.db())).to_string()
            } else {
                "into_iter".to_owned()
            };

            receiver = wrap_paren(receiver, make, ast::prec::ExprPrecedence::Postfix);

            if !impls_core_iter(&ctx.sema, &iterable) {
                receiver = make
                    .expr_method_call(receiver, make.name_ref(&iter_method), make.arg_list([]))
                    .into();
            }

            let control = LoopControl::from_loop(&for_loop.reset_indent());
            // FIXME: insert some import for `ControlFlow`
            let new_loop = control
                .generate_iter_for(receiver, pat.reset_indent(), make)
                .indent(for_loop.indent_level());

            if new_loop.is_block_like() {
                editor.replace(for_loop.syntax(), new_loop.syntax());
            } else {
                editor.replace(for_loop.syntax(), make.expr_stmt(new_loop).syntax());
            }

            builder.add_file_edits(ctx.vfs_file_id(), editor);
        },
    )
}

/// If iterable is a reference where the expression behind the reference implements a method
/// returning an Iterator called iter or iter_mut (depending on the type of reference) then return
/// the expression behind the reference and the method name
fn is_ref_and_impls_iter_method(
    sema: &hir::Semantics<'_, ide_db::RootDatabase>,
    iterable: &ast::Expr,
) -> Option<(ast::Expr, hir::Name, hir::Crate)> {
    let ref_expr = match iterable {
        ast::Expr::RefExpr(r) => r,
        _ => return None,
    };
    let wanted_method = Name::new_symbol_root(if ref_expr.mut_token().is_some() {
        sym::iter_mut
    } else {
        sym::iter
    });
    let expr_behind_ref = ref_expr.expr()?;
    let ty = sema.type_of_expr(&expr_behind_ref)?.adjusted();
    let scope = sema.scope(iterable.syntax())?;
    let krate = scope.krate();
    let iter_trait = FamousDefs(sema, krate).core_iter_Iterator()?;

    let has_wanted_method = ty
        .iterate_method_candidates(sema.db, &scope, Some(&wanted_method), |func| {
            if func.ret_type(sema.db).impls_trait(sema.db, iter_trait, &[]) {
                return Some(());
            }
            None
        })
        .is_some();
    if !has_wanted_method {
        return None;
    }

    Some((expr_behind_ref, wanted_method, krate))
}

/// Whether iterable implements core::Iterator
fn impls_core_iter(sema: &hir::Semantics<'_, ide_db::RootDatabase>, iterable: &ast::Expr) -> bool {
    (|| {
        let it_typ = sema.type_of_expr(iterable)?.adjusted();

        let module = sema.scope(iterable.syntax())?.module();

        let krate = module.krate(sema.db);
        let iter_trait = FamousDefs(sema, krate).core_iter_Iterator()?;
        cov_mark::hit!(test_already_impls_iterator);
        Some(it_typ.impls_trait(sema.db, iter_trait, &[]))
    })()
    .unwrap_or(false)
}

fn validate_method_call_expr(
    ctx: &AssistContext<'_, '_>,
    expr: ast::MethodCallExpr,
) -> Option<(ast::Expr, ast::Expr)> {
    let name_ref = expr.name_ref()?;
    if !name_ref.syntax().text_range().contains_range(ctx.selection_trimmed()) {
        cov_mark::hit!(test_for_each_not_applicable_invalid_cursor_pos);
        return None;
    }
    if name_ref.text() != "for_each" {
        return None;
    }

    let sema = &ctx.sema;

    let receiver = expr.receiver()?;
    let expr = ast::Expr::MethodCallExpr(expr);

    let it_type = sema.type_of_expr(&receiver)?.adjusted();
    let module = sema.scope(receiver.syntax())?.module();
    let krate = module.krate(ctx.db());

    let iter_trait = FamousDefs(sema, krate).core_iter_Iterator()?;
    it_type.impls_trait(sema.db, iter_trait, &[]).then_some((expr, receiver))
}

fn replace_return_with_continue(block: ast::BlockExpr) -> ast::BlockExpr {
    let (editor, block) = SyntaxEditor::with_ast_node(&block);
    node_ext::for_each_return_expr(block.stmt_list(), &mut |expr| {
        editor.replace(expr.syntax(), editor.make().expr_continue(None).syntax());
    });
    ast::BlockExpr::cast(editor.finish().new_root().clone()).unwrap()
}

struct LoopControl {
    editor: SyntaxEditor,
    body: Option<ast::StmtList>,
    breaks: Vec<ast::BreakExpr>,
    continues: Vec<ast::ContinueExpr>,
    returns: Vec<ast::ReturnExpr>,
}

impl LoopControl {
    fn from_loop(loop_expr: &dyn HasLoopBody) -> Self {
        let mut breaks = vec![];
        let mut continues = vec![];
        let mut returns = vec![];

        let label = loop_expr.label();
        let (editor, body) = SyntaxEditor::with_ast_node(&loop_expr.loop_body().unwrap());
        let body = body.stmt_list();

        node_ext::for_each_return_expr(body.clone(), &mut |expr| {
            returns.push(expr);
        });
        node_ext::for_each_break_and_continue_expr(label, body.clone(), &mut |expr| match expr {
            ast::Expr::BreakExpr(expr) => breaks.push(expr),
            ast::Expr::ContinueExpr(expr) => continues.push(expr),
            expr => unreachable!("{expr:?}"),
        });

        Self { editor, body, breaks, continues, returns }
    }

    fn generate_iter_for(
        self,
        receiver: ast::Expr,
        pat: ast::Pat,
        make: &SyntaxFactory,
    ) -> ast::Expr {
        let editor = self.editor;
        let make_new = editor.make();

        let call = |name, arg| {
            ast::Expr::from(make_new.expr_call(
                make_new.expr_path(make_new.path_from_text(name)),
                make_new.arg_list([arg]),
            ))
        };
        let insert_continue = || {
            if let Some(stmt_list) = self.body {
                let tail_continue = call("ControlFlow::Continue", make_new.expr_unit());
                stmt_list.add_expr(&editor, tail_continue);
            }
        };
        let build = |editor: SyntaxEditor, method| {
            let body = ast::BlockExpr::cast(editor.finish().new_root().clone()).unwrap();
            let loop_arg = make.expr_closure([make.untyped_param(pat)], body.into());
            make.expr_method_call(receiver, make.name_ref(method), make.arg_list([loop_arg.into()]))
        };
        let tuple_struct_pat =
            |name, arg| ast::Pat::from(make.tuple_struct_pat(make.path_from_text(name), [arg]));

        if self.returns.is_empty() && self.breaks.is_empty() {
            let continue_to = make_new.expr_return(None);

            for expr in self.continues {
                editor.replace(expr.syntax(), continue_to.syntax());
            }
            return build(editor, "for_each").into();
        }

        if self.returns.is_empty() {
            let continue_to =
                make_new.expr_return(Some(call("ControlFlow::Continue", make_new.expr_unit())));
            let break_to =
                make_new.expr_return(Some(call("ControlFlow::Break", make_new.expr_unit())));

            for expr in self.continues {
                editor.replace(expr.syntax(), continue_to.syntax());
            }
            for expr in self.breaks {
                editor.replace(expr.syntax(), break_to.syntax());
            }
            insert_continue();
            return build(editor, "try_for_each").into();
        }

        let continue_to =
            make_new.expr_return(Some(call("ControlFlow::Continue", make_new.expr_unit())));
        let break_to = make_new.expr_return(Some(call(
            "ControlFlow::Break",
            make_new.expr_path(make_new.path_from_text("None")),
        )));
        let return_to =
            |expr| make_new.expr_return(Some(call("ControlFlow::Break", call("Some", expr))));

        for expr in self.continues {
            editor.replace(expr.syntax(), continue_to.syntax());
        }
        for expr in self.breaks {
            editor.replace(expr.syntax(), break_to.syntax());
        }
        for expr in self.returns {
            editor.replace(
                expr.syntax(),
                return_to(expr.expr().unwrap_or_else(|| make_new.expr_unit())).syntax(),
            );
        }

        insert_continue();
        let iter_for = build(editor, "try_for_each");
        let return_pat = tuple_struct_pat(
            "ControlFlow::Break",
            tuple_struct_pat("Some", make.simple_ident_pat(make.name("result")).into()),
        );
        let let_expr = make.expr_let(return_pat, iter_for.into());
        let return_stmt = make.expr_stmt(
            make.expr_return(Some(make.expr_path(make.path_from_text("result")))).into(),
        );
        let if_expr =
            make.expr_if(let_expr.into(), make.block_expr([return_stmt.into()], None), None);

        if_expr.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_assist, check_assist_not_applicable};

    use super::*;

    #[test]
    fn test_for_each_in_method_stmt() {
        check_assist(
            convert_iter_for_each_to_for,
            r#"
//- minicore: iterators
fn main() {
    let it = core::iter::repeat(92);
    it.$0for_each(|(x, y)| {
        println!("x: {}, y: {}", x, y);
    });
}
"#,
            r#"
fn main() {
    let it = core::iter::repeat(92);
    for (x, y) in it {
        println!("x: {}, y: {}", x, y);
    }
}
"#,
        )
    }

    #[test]
    fn test_for_each_in_method() {
        check_assist(
            convert_iter_for_each_to_for,
            r#"
//- minicore: iterators
fn main() {
    let it = core::iter::repeat(92);
    it.$0for_each(|(x, y)| {
        println!("x: {}, y: {}", x, y);
    })
}
"#,
            r#"
fn main() {
    let it = core::iter::repeat(92);
    for (x, y) in it {
        println!("x: {}, y: {}", x, y);
    }
}
"#,
        )
    }

    #[test]
    fn test_for_each_without_braces_stmt() {
        check_assist(
            convert_iter_for_each_to_for,
            r#"
//- minicore: iterators
fn main() {
    {
        let it = core::iter::repeat(92);
        it.$0for_each(|param| match param {
            (x, y) => println!("x: {}, y: {}", x, y),
        });
    }
}
"#,
            r#"
fn main() {
    {
        let it = core::iter::repeat(92);
        for param in it {
            match param {
                (x, y) => println!("x: {}, y: {}", x, y),
            }
        }
    }
}
"#,
        )
    }

    #[test]
    fn test_for_each_return_expr() {
        check_assist(
            convert_iter_for_each_to_for,
            r#"
//- minicore: iterators
fn main() {
    {
        let it = core::iter::repeat(92);
        it.$0for_each(|param| match param {
            (0, 0) => return,
            (1, 0) => (|| return)(),
            (x, y) => println!("x: {}, y: {}", x, y),
        });
    }
}
"#,
            r#"
fn main() {
    {
        let it = core::iter::repeat(92);
        for param in it {
            match param {
                (0, 0) => continue,
                (1, 0) => (|| return)(),
                (x, y) => println!("x: {}, y: {}", x, y),
            }
        }
    }
}
"#,
        );
    }

    #[test]
    fn test_for_each_not_applicable() {
        check_assist_not_applicable(
            convert_iter_for_each_to_for,
            r#"
//- minicore: iterators
fn main() {
    ().$0for_each(|x| println!("{}", x));
}"#,
        )
    }

    #[test]
    fn test_for_each_not_applicable_invalid_cursor_pos() {
        cov_mark::check!(test_for_each_not_applicable_invalid_cursor_pos);
        check_assist_not_applicable(
            convert_iter_for_each_to_for,
            r#"
//- minicore: iterators
fn main() {
    core::iter::repeat(92).for_each(|(x, y)| $0println!("x: {}, y: {}", x, y));
}"#,
        )
    }

    #[test]
    fn each_to_for_not_for() {
        check_assist_not_applicable(
            convert_for_loop_with_for_each,
            r"
let mut x = vec![1, 2, 3];
x.iter_mut().$0for_each(|v| *v *= 2);
        ",
        )
    }

    #[test]
    fn each_to_for_simple_for() {
        check_assist(
            convert_for_loop_with_for_each,
            r"
fn main() {
    let x = vec![1, 2, 3];
    for $0v in x {
        v *= 2;
    }
}",
            r"
fn main() {
    let x = vec![1, 2, 3];
    x.into_iter().for_each(|v| {
        v *= 2;
    });
}",
        )
    }

    #[test]
    fn each_to_for_for_in_range() {
        check_assist(
            convert_for_loop_with_for_each,
            r#"
//- minicore: range, iterators
impl<T> core::iter::Iterator for core::ops::Range<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

fn main() {
    for $0x in 0..92 {
        print!("{}", x);
    }
}"#,
            r#"
impl<T> core::iter::Iterator for core::ops::Range<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

fn main() {
    (0..92).for_each(|x| {
        print!("{}", x);
    });
}"#,
        )
    }

    #[test]
    fn each_to_for_not_available_in_body() {
        cov_mark::check!(not_available_in_body);
        check_assist_not_applicable(
            convert_for_loop_with_for_each,
            r"
fn main() {
    let x = vec![1, 2, 3];
    for v in x {
        $0v *= 2;
    }
}",
        )
    }

    #[test]
    fn each_to_for_for_borrowed() {
        check_assist(
            convert_for_loop_with_for_each,
            r#"
//- minicore: iterators
use core::iter::{Repeat, repeat};

struct S;
impl S {
    fn iter(&self) -> Repeat<i32> { repeat(92) }
    fn iter_mut(&mut self) -> Repeat<i32> { repeat(92) }
}

fn main() {
    let x = S;
    for $0v in &x {
        let a = v * 2;
    }
}
"#,
            r#"
use core::iter::{Repeat, repeat};

struct S;
impl S {
    fn iter(&self) -> Repeat<i32> { repeat(92) }
    fn iter_mut(&mut self) -> Repeat<i32> { repeat(92) }
}

fn main() {
    let x = S;
    x.iter().for_each(|v| {
        let a = v * 2;
    });
}
"#,
        )
    }

    #[test]
    fn each_to_for_with_continue() {
        check_assist(
            convert_for_loop_with_for_each,
            r"
fn main() {
    let x = vec![1, 2, 3];
    for $0v in x {
        if v == 2 {
            continue;
        }
        v *= 2;
    }
}",
            r"
fn main() {
    let x = vec![1, 2, 3];
    x.into_iter().for_each(|v| {
        if v == 2 {
            return;
        }
        v *= 2;
    });
}",
        )
    }

    #[test]
    fn each_to_for_with_break() {
        check_assist(
            convert_for_loop_with_for_each,
            r"
fn main() {
    let x = vec![1, 2, 3];
    for $0v in x {
        if v == 2 {
            continue;
        }
        if v == 3 {
            break;
        }
        v *= 2;
    }
}",
            r"
fn main() {
    let x = vec![1, 2, 3];
    x.into_iter().try_for_each(|v| {
        if v == 2 {
            return ControlFlow::Continue(());
        }
        if v == 3 {
            return ControlFlow::Break(());
        }
        v *= 2;
        ControlFlow::Continue(())
    });
}",
        )
    }

    #[test]
    fn each_to_for_with_break_in_tail_expr() {
        check_assist(
            convert_for_loop_with_for_each,
            r"
fn main() {
    let x = vec![1, 2, 3];
    let cb = |_| ();
    for $0v in x {
        if v == 2 {
            continue;
        }
        cb(if v == 3 {
            break;
        } else {
            v *= 2;
        })
    }
}",
            r"
fn main() {
    let x = vec![1, 2, 3];
    let cb = |_| ();
    x.into_iter().try_for_each(|v| {
        if v == 2 {
            return ControlFlow::Continue(());
        }
        cb(if v == 3 {
            return ControlFlow::Break(());
        } else {
            v *= 2;
        });
        ControlFlow::Continue(())
    });
}",
        );
    }

    #[test]
    fn each_to_for_with_return() {
        check_assist(
            convert_for_loop_with_for_each,
            r"
fn main() {
    let x = vec![1, 2, 3, 4];
    for $0v in x {
        if v == 2 {
            continue;
        }
        if v == 3 {
            break;
        }
        if v == 4 {
            return 6;
        }
        v *= 2;
    }
}",
            r"
fn main() {
    let x = vec![1, 2, 3, 4];
    if let ControlFlow::Break(Some(result)) = x.into_iter().try_for_each(|v| {
        if v == 2 {
            return ControlFlow::Continue(());
        }
        if v == 3 {
            return ControlFlow::Break(None);
        }
        if v == 4 {
            return ControlFlow::Break(Some(6));
        }
        v *= 2;
        ControlFlow::Continue(())
    })
    {
        return result;
    }
}",
        )
    }

    #[test]
    fn each_to_for_for_borrowed_no_iter_method() {
        check_assist(
            convert_for_loop_with_for_each,
            r"
struct NoIterMethod;
fn main() {
    let x = NoIterMethod;
    for $0v in &x {
        let a = v * 2;
    }
}
",
            r"
struct NoIterMethod;
fn main() {
    let x = NoIterMethod;
    (&x).into_iter().for_each(|v| {
        let a = v * 2;
    });
}
",
        )
    }

    #[test]
    fn each_to_for_for_borrowed_mut() {
        check_assist(
            convert_for_loop_with_for_each,
            r#"
//- minicore: iterators
use core::iter::{Repeat, repeat};

struct S;
impl S {
    fn iter(&self) -> Repeat<i32> { repeat(92) }
    fn iter_mut(&mut self) -> Repeat<i32> { repeat(92) }
}

fn main() {
    let x = S;
    for $0v in &mut x {
        let a = v * 2;
    }
}
"#,
            r#"
use core::iter::{Repeat, repeat};

struct S;
impl S {
    fn iter(&self) -> Repeat<i32> { repeat(92) }
    fn iter_mut(&mut self) -> Repeat<i32> { repeat(92) }
}

fn main() {
    let x = S;
    x.iter_mut().for_each(|v| {
        let a = v * 2;
    });
}
"#,
        )
    }

    #[test]
    fn each_to_for_for_borrowed_mut_behind_var() {
        check_assist(
            convert_for_loop_with_for_each,
            r"
fn main() {
    let x = vec![1, 2, 3];
    let y = &mut x;
    for $0v in y {
        *v *= 2;
    }
}",
            r"
fn main() {
    let x = vec![1, 2, 3];
    let y = &mut x;
    y.into_iter().for_each(|v| {
        *v *= 2;
    });
}",
        )
    }

    #[test]
    fn each_to_for_already_impls_iterator() {
        cov_mark::check!(test_already_impls_iterator);
        check_assist(
            convert_for_loop_with_for_each,
            r#"
//- minicore: iterators
fn main() {
    for$0 a in core::iter::repeat(92).take(1) {
        println!("{}", a);
    }
}
"#,
            r#"
fn main() {
    core::iter::repeat(92).take(1).for_each(|a| {
        println!("{}", a);
    });
}
"#,
        );
    }
}
