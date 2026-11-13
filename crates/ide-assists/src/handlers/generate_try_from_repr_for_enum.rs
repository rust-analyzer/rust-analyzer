use hir::Semantics;
use ide_db::RootDatabase;
use itertools::Itertools;
use syntax::ast::HasAttrs;
use syntax::ast::{self, AstNode, HasName, edit::AstNodeEdit, syntax_factory::SyntaxFactory};
use syntax::syntax_editor::Position;

use crate::{AssistContext, AssistId, Assists, utils::generate_trait_impl_intransitive_with_item};

// Assist: generate_try_from_repr_for_enum
//
// Add a TryFrom<{repr}> impl for this enum.
//
// ```
// #[repr(u32)]
// enum Foo {
//     A = 1$0,
//     B = 1 << 1,
// }
// ```
// ->
// ```
// #[repr(u32)]
// enum Foo {
//     A = 1,
//     B = 1 << 1,
// }
//
// impl TryFrom<u32> for Foo {
//     type Error = ();
//
//     fn try_from(value: u32) -> Result<Self, Self::Error> {
//         match value {
//             1 => Ok(Self::A),
//             2 => Ok(Self::B),
//             _ => Err(()),
//         }
//     }
// }
// ```
pub(crate) fn generate_try_from_repr_for_enum(
    acc: &mut Assists,
    ctx: &AssistContext<'_, '_>,
) -> Option<()> {
    let lit = ctx.find_node_at_offset::<ast::ConstArg>()?;
    let variant = lit.syntax().parent().and_then(ast::Variant::cast)?;
    let parent_enum = variant.parent_enum();

    if parent_enum.variant_list()?.variants().any(|it| it.field_list().is_some()) {
        return None;
    }

    let repr_ty = parent_enum
        .attrs()
        .find_map(|it| it.as_simple_call().filter(|it| it.0 == "repr"))
        .map_or("i32".to_owned(), |(_, repr)| repr.token_trees_and_tokens_non_delim().join(""));
    let variants = parent_enum.variant_list()?.variants();

    // FIXME: Check exists TryFrom<{repr_ty}> implement?

    let target = ast::Adt::Enum(parent_enum.clone()).syntax().text_range();
    let label = format!("Generate TryFrom<{repr_ty}> for enum");
    acc.add(AssistId::generate("generate_try_from_repr_for_enum"), label, target, |builder| {
        let editor = builder.make_editor(parent_enum.syntax());
        let make = editor.make();

        let indent = parent_enum.indent_level();
        let repr_ty = make.ty(&repr_ty);
        let body = body(make, variants, &ctx.sema, &repr_ty);
        let alias = make.ty_alias([], "Error", None, None, None, Some((make.ty_unit(), None)));
        let body = make.assoc_item_list([alias.into(), body.into()]);
        let trait_impl = generate_trait_impl_intransitive_with_item(
            make,
            &parent_enum.clone().into(),
            make.ty(&format!("TryFrom<{repr_ty}>")),
            body,
        );
        editor.insert_all(
            Position::after(parent_enum.syntax()),
            vec![
                make.whitespace(&format!("\n\n{indent}")).into(),
                trait_impl.indent(indent).syntax().clone().into(),
            ],
        );

        builder.add_file_edits(ctx.vfs_file_id(), editor);
    })
}

fn body(
    make: &SyntaxFactory,
    variants: impl IntoIterator<Item = ast::Variant>,
    sema: &Semantics<'_, RootDatabase>,
    repr_ty: &ast::Type,
) -> ast::Fn {
    let db = sema.db;
    let make_arm = |variant: &ast::Variant| {
        let def = sema.to_def(variant)?;
        let value = def.eval(db).ok()?;
        let pat = make.literal_pat(&value.to_string());
        let variant_name = variant.name()?.to_string();
        let expr = make.expr_path(make.path_from_idents(["Self", &variant_name]).unwrap());
        let expr = make.expr_call(make.expr_path(make.ident_path("Ok")), make.arg_list([expr]));
        Some(make.match_arm(pat.into(), None, expr.into()))
    };
    let err_expr =
        make.expr_call(make.expr_path(make.ident_path("Err")), make.arg_list([make.expr_unit()]));
    let arms = variants.into_iter().map(|variant| {
        make_arm(&variant).unwrap_or_else(|| {
            let reason = format!("Invalid variant {variant}");
            let pat = make.literal_pat(&format!("{reason:?}")).into();
            make.match_arm(pat, None, err_expr.clone().into())
        })
    });
    let arms =
        arms.chain([make.match_arm(make.wildcard_pat().into(), None, err_expr.clone().into())]);
    let param = make.ident_path("value");
    let params = make.param_list(None, [make.param(make.path_pat(param.clone()), repr_ty.clone())]);
    let body = make.block_expr(
        [],
        Some(
            make.expr_match(make.expr_path(param), make.match_arm_list(arms))
                .indent(1.into())
                .into(),
        ),
    );
    make.fn_(
        [],
        None,
        make.name("try_from"),
        None,
        None,
        params,
        body,
        Some(make.ret_type(make.ty("Result<Self, Self::Error>"))),
        false,
        false,
        false,
        false,
    )
    .indent(1.into())
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_assist, check_assist_not_applicable};

    use super::*;

    #[test]
    fn basic() {
        check_assist(
            generate_try_from_repr_for_enum,
            r#"
enum Foo {
    A = 1$0,
    B = 1 << 1,
}"#,
            r#"
enum Foo {
    A = 1,
    B = 1 << 1,
}

impl TryFrom<i32> for Foo {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::A),
            2 => Ok(Self::B),
            _ => Err(()),
        }
    }
}"#,
        );
    }

    #[test]
    fn with_repr() {
        check_assist(
            generate_try_from_repr_for_enum,
            r#"
#[repr(u32)]
enum Foo {
    A = 1$0,
    B = 1 << 1,
}"#,
            r#"
#[repr(u32)]
enum Foo {
    A = 1,
    B = 1 << 1,
}

impl TryFrom<u32> for Foo {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::A),
            2 => Ok(Self::B),
            _ => Err(()),
        }
    }
}"#,
        );
    }

    #[test]
    fn indent() {
        check_assist(
            generate_try_from_repr_for_enum,
            r#"
mod foo {
    enum Foo {
        A = 1$0,
        B = 1 << 1,
    }
}"#,
            r#"
mod foo {
    enum Foo {
        A = 1,
        B = 1 << 1,
    }

    impl TryFrom<i32> for Foo {
        type Error = ();

        fn try_from(value: i32) -> Result<Self, Self::Error> {
            match value {
                1 => Ok(Self::A),
                2 => Ok(Self::B),
                _ => Err(()),
            }
        }
    }
}"#,
        );
    }

    #[test]
    fn not_applicable_with_value() {
        check_assist_not_applicable(
            generate_try_from_repr_for_enum,
            r#"
enum Foo {
    A = 1$0,
    B = 1 << 1,
    C(i32),
}"#,
        );
    }
}
