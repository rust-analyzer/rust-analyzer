use syntax::{
    AstNode,
    ast::{self, HasArgList, edit::AstNodeEdit},
};

use crate::{AssistContext, AssistId, Assists};

// Assist: simplify_match
//
// Simplifies a `match` expression where the scrutinee is a constructor call.
// Unreachable arms for other constructors are removed, and arms matching the
// constructor have the outer wrapper stripped.
//
// ```
// # //- minicore: option
// fn f(x: i32) -> i32 {
//     $0match Some(x + 1) {
//         Some(n) => n,
//         None => 0,
//     }
// }
// ```
// ->
// ```
// fn f(x: i32) -> i32 {
//     match x + 1 {
//         n => n,
//     }
// }
// ```
pub(crate) fn simplify_match(acc: &mut Assists, ctx: &AssistContext<'_, '_>) -> Option<()> {
    let match_expr = ctx.find_node_at_offset_with_descend::<ast::MatchExpr>()?;
    let scrutinee = match_expr.expr()?;
    let arm_list = match_expr.match_arm_list()?;

    let call = match scrutinee {
        ast::Expr::CallExpr(c) => c,
        _ => return None,
    };
    let callee_path = match call.expr()? {
        ast::Expr::PathExpr(p) => p.path()?,
        _ => return None,
    };
    let ctor_variant = match ctx.sema.resolve_path(&callee_path)? {
        hir::PathResolution::Def(hir::ModuleDef::EnumVariant(v)) => v,
        _ => return None,
    };
    let ctor_enum = ctor_variant.parent_enum(ctx.sema.db);
    let args: Vec<ast::Expr> = call.arg_list()?.args().collect();
    if args.is_empty() {
        return None;
    }

    enum ArmClass {
        Keep(ast::MatchArm),
        Remove,
        Simplify(ast::MatchArm, Vec<ast::Pat>),
    }

    let mut any_simplify = false;
    let mut classified: Vec<ArmClass> = Vec::new();

    for arm in arm_list.arms() {
        if arm.guard().is_some() {
            return None;
        }
        let pat = arm.pat()?;
        let cls = match pat {
            ast::Pat::WildcardPat(_) => ArmClass::Keep(arm),
            ast::Pat::IdentPat(ref ip) if ip.at_token().is_none() => {
                // Single-segment variant paths like `None` are parsed as IdentPat, not PathPat.
                match ctx.sema.resolve_bind_pat_to_const(ip) {
                    Some(hir::ModuleDef::EnumVariant(v))
                        if v.parent_enum(ctx.sema.db) == ctor_enum =>
                    {
                        ArmClass::Remove
                    }
                    Some(_) => return None,
                    None => ArmClass::Keep(arm),
                }
            }
            ast::Pat::OrPat(_) => return None,
            ast::Pat::TupleStructPat(ref tsp) => {
                let path = tsp.path()?;
                let variant = match ctx.sema.resolve_path(&path)? {
                    hir::PathResolution::Def(hir::ModuleDef::EnumVariant(v)) => v,
                    _ => return None,
                };
                if variant == ctor_variant {
                    let inner: Vec<ast::Pat> = tsp.fields().collect();
                    any_simplify = true;
                    ArmClass::Simplify(arm, inner)
                } else if variant.parent_enum(ctx.sema.db) == ctor_enum {
                    ArmClass::Remove
                } else {
                    return None;
                }
            }
            ast::Pat::PathPat(ref pp) => {
                let path = pp.path()?;
                let variant = match ctx.sema.resolve_path(&path)? {
                    hir::PathResolution::Def(hir::ModuleDef::EnumVariant(v)) => v,
                    _ => return None,
                };
                if variant == ctor_variant {
                    // Scrutinee has args but pattern has none: structurally impossible.
                    return None;
                } else if variant.parent_enum(ctx.sema.db) == ctor_enum {
                    ArmClass::Remove
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        classified.push(cls);
    }

    // Require at least one arm to peel; remove-only would change the scrutinee without a
    // corresponding structural simplification and risks producing an empty match.
    if !any_simplify {
        return None;
    }

    let indent = match_expr.indent_level();

    acc.add(
        AssistId::refactor_rewrite("simplify_match"),
        "Simplify match arms",
        match_expr.syntax().text_range(),
        |builder| {
            let editor = builder.make_editor(match_expr.syntax());
            let make = editor.make();

            let new_arms: Vec<ast::MatchArm> = classified
                .into_iter()
                .filter_map(|cls| match cls {
                    ArmClass::Keep(arm) => Some(arm),
                    ArmClass::Remove => None,
                    ArmClass::Simplify(arm, inner_pats) => {
                        let new_pat = if inner_pats.len() == 1 {
                            inner_pats.into_iter().next().unwrap()
                        } else {
                            make.tuple_pat(inner_pats).into()
                        };
                        let body = arm.expr()?;
                        Some(make.match_arm(new_pat, None, body))
                    }
                })
                .collect();

            let new_scrutinee: ast::Expr = if args.len() == 1 {
                args.into_iter().next().unwrap()
            } else {
                make.expr_tuple(args).into()
            };

            let new_match =
                make.expr_match(new_scrutinee, make.match_arm_list(new_arms)).indent(indent);
            editor.replace(match_expr.syntax(), new_match.syntax());
            builder.add_file_edits(ctx.vfs_file_id(), editor);
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_assist, check_assist_not_applicable};

    use super::*;

    #[test]
    fn simplify_match_single_field_removes_dead_arm() {
        check_assist(
            simplify_match,
            r#"
//- minicore: option
fn f(x: i32) -> i32 {
    $0match Some(x + 1) {
        Some(n) => n,
        None => 0,
    }
}
"#,
            r#"
fn f(x: i32) -> i32 {
    match x + 1 {
        n => n,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_single_field_only() {
        check_assist(
            simplify_match,
            r#"
//- minicore: option
fn f(x: i32) -> i32 {
    $0match Some(x) {
        Some(n) => n * 2,
    }
}
"#,
            r#"
fn f(x: i32) -> i32 {
    match x {
        n => n * 2,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_multi_field_tuple_scrutinee() {
        check_assist(
            simplify_match,
            r#"
enum Pair { Both(i32, i32), Neither }
fn f(x: i32) -> i32 {
    $0match Pair::Both(x, x + 1) {
        Pair::Both(a, b) => a + b,
        Pair::Neither => 0,
    }
}
"#,
            r#"
enum Pair { Both(i32, i32), Neither }
fn f(x: i32) -> i32 {
    match (x, x + 1) {
        (a, b) => a + b,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_result_variant() {
        check_assist(
            simplify_match,
            r#"
//- minicore: result
fn f(x: i32) -> i32 {
    $0match Ok(x) {
        Ok(n) => n,
        Err(_) => 0,
        _ => -1,
    }
}
"#,
            r#"
fn f(x: i32) -> i32 {
    match x {
        n => n,
        _ => -1,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_wildcard_kept() {
        check_assist(
            simplify_match,
            r#"
//- minicore: option
fn f(x: i32) -> i32 {
    $0match Some(x) {
        Some(n) => n * 2,
        _ => 0,
    }
}
"#,
            r#"
fn f(x: i32) -> i32 {
    match x {
        n => n * 2,
        _ => 0,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_ident_binding_kept() {
        check_assist(
            simplify_match,
            r#"
//- minicore: option
fn f(x: i32) -> i32 {
    $0match Some(x) {
        Some(n) => n,
        other => 0,
    }
}
"#,
            r#"
fn f(x: i32) -> i32 {
    match x {
        n => n,
        other => 0,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_not_applicable_guard() {
        check_assist_not_applicable(
            simplify_match,
            r#"
//- minicore: option
fn f(x: i32) -> i32 {
    $0match Some(x) {
        Some(n) if n > 0 => n,
        _ => 0,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_not_applicable_or_pat() {
        check_assist_not_applicable(
            simplify_match,
            r#"
//- minicore: option
fn f(x: i32) -> i32 {
    $0match Some(x) {
        Some(0) | Some(1) => 1,
        _ => 0,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_not_applicable_no_simplify_arm() {
        // Only dead-arm removal without any constructor-peeling arm would change the scrutinee
        // without a structural match, risking an empty match expression.
        check_assist_not_applicable(
            simplify_match,
            r#"
//- minicore: option
fn f(x: i32) -> i32 {
    $0match Some(x) {
        None => 0,
        _ => 1,
    }
}
"#,
        );
    }

    #[test]
    fn simplify_match_not_applicable_non_enum_call() {
        check_assist_not_applicable(
            simplify_match,
            r#"
fn make_pair(a: i32, b: i32) -> (i32, i32) { (a, b) }
fn f(x: i32) -> (i32, i32) {
    $0match make_pair(x, x + 1) {
        _ => (0, 0),
    }
}
"#,
        );
    }
}
