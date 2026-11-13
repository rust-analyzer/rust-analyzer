use hir::AsAssocItem;
use ide_db::assists::{AssistId, GroupLabel};
use syntax::{
    AstNode, T,
    ast::{self, ArithOp, BinaryOp, HasArgList},
    syntax_editor::Position,
};

use crate::{
    assist_context::{AssistContext, Assists},
    utils::wrap_paren,
};

// Assist: replace_arith_with_checked
//
// Replaces arithmetic on integers with the `checked_*` equivalent.
//
// ```
// fn main() {
//   let x = 1 $0+ 2;
// }
// ```
// ->
// ```
// fn main() {
//   let x = 1.checked_add(2);
// }
// ```
pub(crate) fn replace_arith_with_checked(
    acc: &mut Assists,
    ctx: &AssistContext<'_, '_>,
) -> Option<()> {
    replace_arith(acc, ctx, ArithKind::Checked)
}

// Assist: replace_arith_with_saturating
//
// Replaces arithmetic on integers with the `saturating_*` equivalent.
//
// ```
// fn main() {
//   let x = 1 $0+ 2;
// }
// ```
// ->
// ```
// fn main() {
//   let x = 1.saturating_add(2);
// }
// ```
pub(crate) fn replace_arith_with_saturating(
    acc: &mut Assists,
    ctx: &AssistContext<'_, '_>,
) -> Option<()> {
    replace_arith(acc, ctx, ArithKind::Saturating)
}

// Assist: replace_arith_with_strict
//
// Replaces arithmetic on integers with the `strict_*` equivalent.
//
// ```
// fn main() {
//   let x = 1 $0+ 2;
// }
// ```
// ->
// ```
// fn main() {
//   let x = 1.strict_add(2);
// }
// ```
pub(crate) fn replace_arith_with_strict(
    acc: &mut Assists,
    ctx: &AssistContext<'_, '_>,
) -> Option<()> {
    replace_arith(acc, ctx, ArithKind::Strict)
}

// Assist: replace_arith_with_wrapping
//
// Replaces arithmetic on integers with the `wrapping_*` equivalent.
//
// ```
// fn main() {
//   let x = 1 $0+ 2;
// }
// ```
// ->
// ```
// fn main() {
//   let x = 1.wrapping_add(2);
// }
// ```
pub(crate) fn replace_arith_with_wrapping(
    acc: &mut Assists,
    ctx: &AssistContext<'_, '_>,
) -> Option<()> {
    replace_arith(acc, ctx, ArithKind::Wrapping)
}

fn replace_arith(acc: &mut Assists, ctx: &AssistContext<'_, '_>, kind: ArithKind) -> Option<()> {
    let ParsedArithExpr { expr, receiver, argument, operation } = parse_arith_expr(ctx)?;
    let reference_depth = if matches!(operation, Operation::Binary { .. }) {
        0
    } else {
        let mut ty = ctx.sema.type_of_expr(&receiver)?.original;
        let mut depth = 0;
        while let Some(inner) = ty.as_reference_inner() {
            depth += 1;
            ty = inner;
        }
        depth
    };

    acc.add_group(
        &GroupLabel("Replace arithmetic...".into()),
        kind.assist_id(),
        kind.label(),
        expr.syntax().text_range(),
        |builder| {
            let editor = builder.make_editor(expr.syntax());
            let make = editor.make();
            let method_name = kind.method_name(operation);

            match operation {
                Operation::Binary { is_assign, .. } => {
                    let method_receiver =
                        wrap_paren(receiver.clone(), make, ast::prec::ExprPrecedence::Postfix);
                    let mut argument = argument.expect("binary operation always has an argument");
                    if let Some(ty) = ctx.sema.type_of_expr(&argument) {
                        let adjusted = ty.adjusted();
                        if adjusted.strip_reference() != adjusted {
                            argument = if let ast::Expr::RefExpr(ref_expr) = &argument
                                && let Some(inner) = ref_expr.expr()
                            {
                                inner
                            } else {
                                make.expr_prefix(T![*], argument).into()
                            };
                        }
                    }

                    let mut arith_expr = make
                        .expr_method_call(
                            method_receiver,
                            make.name_ref(&method_name),
                            make.arg_list([argument]),
                        )
                        .into();
                    if is_assign {
                        arith_expr = make.expr_assignment(receiver, arith_expr).into();
                    }
                    editor.replace(expr.syntax(), arith_expr.syntax());
                }
                Operation::Neg { primitive } => {
                    // Unlike unary `-`, method lookup does not wait for integer fallback. Keep the
                    // inferred primitive explicit so the replacement does not lose that constraint.
                    let ast::Expr::PrefixExpr(prefix) = &expr else { unreachable!() };
                    let operator =
                        prefix.op_token().expect("prefix expression always has an operator");
                    let path = make.path_from_text_with_edition(
                        &format!("{}::{method_name}", primitive.name().as_str()),
                        ctx.edition(),
                    );
                    let mut replacement =
                        vec![path.syntax().clone().into(), make.token(T!['(']).into()];
                    for _ in 0..reference_depth {
                        replacement.push(make.token(T![*]).into());
                    }
                    editor.replace_with_many(operator, replacement);
                    editor.insert(Position::after(receiver.syntax()), make.token(T![')']));
                }
                Operation::Pow => {
                    let ast::Expr::MethodCallExpr(method_call) = &expr else { unreachable!() };
                    let name =
                        method_call.name_ref().expect("method call expression always has a name");
                    editor.replace(name.syntax(), make.name_ref(&method_name).syntax());

                    if reference_depth > 0 {
                        let mut prefix = vec![make.token(T!['(']).into()];
                        for _ in 0..reference_depth {
                            prefix.push(make.token(T![*]).into());
                        }
                        editor.insert_all(Position::before(receiver.syntax()), prefix);
                        editor.insert(Position::after(receiver.syntax()), make.token(T![')']));
                    }
                }
            }
            builder.add_file_edits(ctx.vfs_file_id(), editor);
        },
    )
}

fn is_primitive_int_or_ref(ctx: &AssistContext<'_, '_>, expr: &ast::Expr) -> bool {
    match ctx.sema.type_of_expr(expr) {
        Some(ty) => ty.original.strip_reference().is_int_or_uint(),
        _ => false,
    }
}

fn is_primitive_int_or_refs(ctx: &AssistContext<'_, '_>, expr: &ast::Expr) -> bool {
    match ctx.sema.type_of_expr(expr) {
        Some(ty) => ty.original.strip_references().is_int_or_uint(),
        _ => false,
    }
}

fn replaceable_negation_primitive(
    ctx: &AssistContext<'_, '_>,
    expr: &ast::Expr,
) -> Option<hir::BuiltinType> {
    let ty = ctx.sema.type_of_expr(expr)?;
    if !ty.original.as_reference().is_none_or(|(_, mutability)| mutability.is_shared())
        || !ty.original.strip_reference().as_builtin().is_some_and(|builtin| builtin.is_int())
    {
        return None;
    }
    let primitive = ty.original.strip_reference().as_builtin()?;

    let mut expr = expr.clone();
    while let ast::Expr::ParenExpr(paren) = expr {
        let inner = paren.expr()?;
        expr = inner;
    }

    // A macro expansion can hide the minimum-value literal that is valid only after unary `-`.
    let ast::Expr::Literal(literal) = &expr else {
        return (!matches!(expr, ast::Expr::MacroExpr(_))).then_some(primitive);
    };
    let ast::LiteralKind::IntNumber(number) = literal.kind() else { return Some(primitive) };

    let ty = ty.original.strip_reference();
    let layout = ty.layout(ctx.db()).ok()?;
    let value_bits = layout.size().checked_mul(8)?;
    let sign_bit = value_bits
        .checked_sub(1)
        .and_then(|shift| u32::try_from(shift).ok())
        .and_then(|shift| 1u128.checked_shl(shift))?;

    number.value().is_ok_and(|value| value < sign_bit).then_some(primitive)
}

struct ParsedArithExpr {
    expr: ast::Expr,
    receiver: ast::Expr,
    argument: Option<ast::Expr>,
    operation: Operation,
}

#[derive(Clone, Copy)]
enum Operation {
    Binary { op: ArithOp, is_assign: bool },
    Neg { primitive: hir::BuiltinType },
    Pow,
}

fn parse_arith_expr(ctx: &AssistContext<'_, '_>) -> Option<ParsedArithExpr> {
    if !ctx.has_empty_selection() {
        return None;
    }

    let expr = ctx.find_node_at_offset::<ast::Expr>()?;
    for expr in expr.syntax().ancestors().filter_map(ast::Expr::cast) {
        match expr {
            ast::Expr::BinExpr(expr) => return parse_binary_op(ctx, expr),
            ast::Expr::PrefixExpr(expr) if expr.op_kind() == Some(ast::UnaryOp::Neg) => {
                return parse_negation(ctx, expr);
            }
            ast::Expr::MethodCallExpr(expr)
                if expr
                    .name_ref()
                    .is_some_and(|name| name.text().trim_start_matches("r#") == "pow") =>
            {
                return parse_pow(ctx, expr);
            }
            _ => {}
        }
    }
    None
}

fn parse_binary_op(ctx: &AssistContext<'_, '_>, expr: ast::BinExpr) -> Option<ParsedArithExpr> {
    let (op, is_assign) = match expr.op_kind()? {
        BinaryOp::ArithOp(arith_op) => (arith_op, false),
        BinaryOp::Assignment { op: Some(op) } => (op, true),
        _ => return None,
    };
    if !matches!(op, ArithOp::Add | ArithOp::Sub | ArithOp::Mul | ArithOp::Div) {
        return None;
    }

    let lhs = expr.lhs()?;
    let rhs = expr.rhs()?;
    if !is_primitive_int_or_ref(ctx, &lhs) || !is_primitive_int_or_ref(ctx, &rhs) {
        return None;
    }

    Some(ParsedArithExpr {
        expr: expr.into(),
        receiver: lhs,
        argument: Some(rhs),
        operation: Operation::Binary { op, is_assign },
    })
}

fn parse_negation(ctx: &AssistContext<'_, '_>, expr: ast::PrefixExpr) -> Option<ParsedArithExpr> {
    if expr.op_kind()? != ast::UnaryOp::Neg {
        return None;
    }
    let operand = expr.expr()?;
    let primitive = replaceable_negation_primitive(ctx, &operand)?;
    let primitive_path =
        ast::make::path_from_text_with_edition(primitive.name().as_str(), ctx.edition());
    let scope = ctx.sema.scope(expr.syntax())?;
    // A user-defined type is allowed to shadow a primitive's textual name.
    if !matches!(
        scope.speculative_resolve(&primitive_path),
        Some(hir::PathResolution::Def(hir::ModuleDef::BuiltinType(resolved)))
            if resolved == primitive
    ) {
        return None;
    }

    Some(ParsedArithExpr {
        expr: expr.into(),
        receiver: operand,
        argument: None,
        operation: Operation::Neg { primitive },
    })
}

fn parse_pow(ctx: &AssistContext<'_, '_>, expr: ast::MethodCallExpr) -> Option<ParsedArithExpr> {
    let receiver = expr.receiver()?;
    if !is_primitive_int_or_refs(ctx, &receiver) {
        return None;
    }

    let mut arguments = expr.arg_list()?.args();
    let exponent = arguments.next()?;
    if arguments.next().is_some() {
        return None;
    }

    let function = ctx.sema.resolve_method_call(&expr)?;
    let assoc = function.as_assoc_item(ctx.db())?;
    if assoc.implemented_trait(ctx.db()).is_some()
        || !assoc.implementing_ty(ctx.db())?.is_int_or_uint()
    {
        return None;
    }

    Some(ParsedArithExpr {
        expr: expr.into(),
        receiver,
        argument: Some(exponent),
        operation: Operation::Pow,
    })
}

pub(crate) enum ArithKind {
    Saturating,
    Wrapping,
    Checked,
    Strict,
}

impl ArithKind {
    fn assist_id(&self) -> AssistId {
        let s = match self {
            ArithKind::Saturating => "replace_arith_with_saturating",
            ArithKind::Checked => "replace_arith_with_checked",
            ArithKind::Wrapping => "replace_arith_with_wrapping",
            ArithKind::Strict => "replace_arith_with_strict",
        };

        AssistId::refactor_rewrite(s)
    }

    fn label(&self) -> &'static str {
        match self {
            ArithKind::Saturating => "Replace arithmetic with call to saturating_*",
            ArithKind::Checked => "Replace arithmetic with call to checked_*",
            ArithKind::Wrapping => "Replace arithmetic with call to wrapping_*",
            ArithKind::Strict => "Replace arithmetic with call to strict_*",
        }
    }

    fn method_name(&self, operation: Operation) -> String {
        let prefix = match self {
            ArithKind::Checked => "checked_",
            ArithKind::Wrapping => "wrapping_",
            ArithKind::Saturating => "saturating_",
            ArithKind::Strict => "strict_",
        };

        let suffix = match operation {
            Operation::Binary { op: ArithOp::Add, .. } => "add",
            Operation::Binary { op: ArithOp::Sub, .. } => "sub",
            Operation::Binary { op: ArithOp::Mul, .. } => "mul",
            Operation::Binary { op: ArithOp::Div, .. } => "div",
            Operation::Neg { .. } => "neg",
            Operation::Pow => "pow",
            Operation::Binary { .. } => {
                unreachable!("only +, -, / and * are parsed as arithmetic operations")
            }
        };
        format!("{prefix}{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_assist, check_assist_not_applicable};

    use super::*;

    #[test]
    fn arith_kind_method_name() {
        assert_eq!(
            ArithKind::Saturating
                .method_name(Operation::Binary { op: ArithOp::Add, is_assign: false }),
            "saturating_add"
        );
        assert_eq!(
            ArithKind::Checked
                .method_name(Operation::Binary { op: ArithOp::Sub, is_assign: false }),
            "checked_sub"
        );
        assert_eq!(
            ArithKind::Strict.method_name(Operation::Neg { primitive: hir::BuiltinType::i32() }),
            "strict_neg"
        );
        assert_eq!(ArithKind::Wrapping.method_name(Operation::Pow), "wrapping_pow");
    }

    #[test]
    fn replace_arith_picks_nearest_negation() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
//- minicore: unary_ops, add, builtin_impls
fn main() {
    let x = &1i32;
    let y = $0-x + 1;
}
"#,
            r#"
fn main() {
    let x = &1i32;
    let y = i32::wrapping_neg(*x) + 1;
}
"#,
        );
    }

    #[test]
    fn replace_arith_does_not_skip_unsupported_nearest_binary() {
        check_assist_not_applicable(
            replace_arith_with_checked,
            "fn main() { let x = 1i32; let y = x $0% 2 + 3; }",
        );
    }

    #[test]
    fn replace_arith_picks_nearest_pow() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
//- /main.rs crate:main deps:core
fn main() {
    let value: i32 = 2;
    let x = &&value;
    let exponent: u32 = 3;
    let y = x.po$0w(exponent) + 1;
}
//- /core.rs crate:core
#![rustc_coherence_is_core]
impl i32 {
    pub fn pow(self, exponent: u32) -> i32 { self }
}
"#,
            r#"
fn main() {
    let value: i32 = 2;
    let x = &&value;
    let exponent: u32 = 3;
    let y = (**x).wrapping_pow(exponent) + 1;
}
"#,
        );
    }

    #[test]
    fn replace_negation_of_representable_literal() {
        check_assist(
            replace_arith_with_checked,
            "fn main() { let y = $0-127i8; }",
            "fn main() { let y = i8::checked_neg(127i8); }",
        );
    }

    #[test]
    fn replace_negation_preserves_inferred_type() {
        for (before, after) in [
            (
                "fn main() { let x = 127; let y = $0-x; }",
                "fn main() { let x = 127; let y = i32::checked_neg(x); }",
            ),
            (
                "fn main() { let x = 127; let r = &x; let y = $0-r; }",
                "fn main() { let x = 127; let r = &x; let y = i32::checked_neg(*r); }",
            ),
            (
                "fn main() { let y = $0-{ 127 }; }",
                "fn main() { let y = i32::checked_neg({ 127 }); }",
            ),
            (
                "fn main() { let y = $0-{ struct i32; 127 }; }",
                "fn main() { let y = i32::checked_neg({ struct i32; 127 }); }",
            ),
            (
                "fn main() { let y = $0-(1i32 + 2); }",
                "fn main() { let y = i32::checked_neg((1i32 + 2)); }",
            ),
            (
                "//- minicore: index, slice\nfn main() { let array = [1i32]; let y = $0-array[0]; }",
                "fn main() { let array = [1i32]; let y = i32::checked_neg(array[0]); }",
            ),
            (
                "fn main() { let tuple = (1i32,); let y = $0-tuple.0; }",
                "fn main() { let tuple = (1i32,); let y = i32::checked_neg(tuple.0); }",
            ),
            (
                "fn fixed(_: i32) -> i32 { 1 } fn main() { let y = $0-fixed(1); }",
                "fn fixed(_: i32) -> i32 { 1 } fn main() { let y = i32::checked_neg(fixed(1)); }",
            ),
            (
                "fn main() { let y = $0-(1 as i32); }",
                "fn main() { let y = i32::checked_neg((1 as i32)); }",
            ),
        ] {
            check_assist(replace_arith_with_checked, before, after);
        }

        check_assist(
            replace_arith_with_wrapping,
            "fn take(_: i8) {} fn main() { let x = 127; take($0-x); }",
            "fn take(_: i8) {} fn main() { let x = 127; take(i8::wrapping_neg(x)); }",
        );
    }

    #[test]
    fn replace_negation_preserves_comments() {
        check_assist(
            replace_arith_with_wrapping,
            "fn main() { let x = 1i32; let y = $0-/* keep */x; }",
            "fn main() { let x = 1i32; let y = i32::wrapping_neg(/* keep */x); }",
        );
    }

    #[test]
    fn replace_pow_preserves_comments() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
//- /main.rs crate:main deps:core
fn main() {
    let value: i32 = 2;
    let x = &&value;
    let exponent: u32 = 3;
    let y = x/* receiver */.po$0w(/* exponent */ exponent);
}
//- /core.rs crate:core
#![rustc_coherence_is_core]
impl i32 {
    pub fn pow(self, exponent: u32) -> i32 { self }
}
"#,
            r#"
fn main() {
    let value: i32 = 2;
    let x = &&value;
    let exponent: u32 = 3;
    let y = (**x)/* receiver */.wrapping_pow(/* exponent */ exponent);
}
"#,
        );
    }

    #[test]
    fn replace_negation_not_applicable() {
        for fixture in [
            "fn main() { let x = 1u32; let y = $0-x; }",
            "fn main() { let y = $0-128i8; }",
            "fn main() { let y = $0-(128i8); }",
            "fn main() { let y = $0-((128i8)); }",
            "fn main() { let y = $0-2147483648; }",
            "fn main() { struct i32; let x = 1i32; let y = $0-x; }",
            r#"
//- /main.rs crate:main deps:core
struct i32;
fn main() {
    let y = $0-{
        use core::primitive::i32;
        1i32
    };
}
//- /core.rs crate:core
pub mod primitive {
    pub use i32;
}
"#,
            r#"
macro_rules! min { () => { 128i8 }; }
fn main() { let y = $0-min!(); }
"#,
        ] {
            check_assist_not_applicable(replace_arith_with_checked, fixture);
        }
    }

    #[test]
    fn replace_pow_not_applicable_to_trait_method_on_integer() {
        check_assist_not_applicable(
            replace_arith_with_checked,
            r#"
trait Pow {
    fn pow(self, exponent: u32) -> Self;
}

impl Pow for i32 {
    fn pow(self, exponent: u32) -> Self { self }
}

fn main() {
    let x: i32 = 2;
    let y = x.po$0w(3) + 1;
}
"#,
        );
    }

    #[test]
    fn replace_arith_with_checked_add() {
        check_assist(
            replace_arith_with_checked,
            r#"
//- minicore: add, builtin_impls
fn main() {
    let x = 1 $0+ 2;
}
"#,
            r#"
fn main() {
    let x = 1.checked_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_saturating_add() {
        check_assist(
            replace_arith_with_saturating,
            r#"
//- minicore: add, builtin_impls
fn main() {
    let x = 1 $0+ 2;
}
"#,
            r#"
fn main() {
    let x = 1.saturating_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_strict_add() {
        check_assist(
            replace_arith_with_strict,
            r#"
fn main() {
    let x = 1 $0+ 2;
}
"#,
            r#"
fn main() {
    let x = 1.strict_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_wrapping_add() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
//- minicore: add, builtin_impls
fn main() {
    let x = 1 $0+ 2;
}
"#,
            r#"
fn main() {
    let x = 1.wrapping_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_wrapping_add_add_parenthesis() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
//- minicore: add, builtin_impls
fn main() {
    let x = 1*3 $0+ 2;
}
"#,
            r#"
fn main() {
    let x = (1*3).wrapping_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_wrapping_add_assign() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
//- minicore: add, builtin_impls
fn main() {
    let mut x = 1;
    x $0+= 2;
}
"#,
            r#"
fn main() {
    let mut x = 1;
    x = x.wrapping_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_wrapping_add_ref() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
fn main() {
    let x = &1;
    x $0+ 2;
}
"#,
            r#"
fn main() {
    let x = &1;
    x.wrapping_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_wrapping_add_remove_ref() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
fn main() {
    1 $0+ &2;
}
"#,
            r#"
fn main() {
    1.wrapping_add(2);
}
"#,
        )
    }

    #[test]
    fn replace_arith_with_wrapping_add_deref() {
        check_assist(
            replace_arith_with_wrapping,
            r#"
fn main() {
    let x = &2
    1 $0+ x;
}
"#,
            r#"
fn main() {
    let x = &2
    1.wrapping_add(*x);
}
"#,
        )
    }

    #[test]
    fn replace_arith_not_applicable_with_non_empty_selection() {
        check_assist_not_applicable(
            replace_arith_with_checked,
            r#"
//- minicore: add, builtin_impls
fn main() {
    let x = 1 $0+$0 2;
}
"#,
        )
    }
}
