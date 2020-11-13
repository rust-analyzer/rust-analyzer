use std::{collections::hash_map::Entry, iter};

use hir::{AsName, ModuleDef};
use ide_db::{base_db::FileId, defs::Definition, search::Reference, RootDatabase};
use rustc_hash::{FxHashMap, FxHashSet};
use syntax::{
    algo::{self, SyntaxRewriter},
    ast::{
        self,
        edit::{AstNodeEdit, IndentLevel},
        make, ArgListOwner, GenericParamsOwner, NameOwner, VisibilityOwner,
    },
    match_ast, AstNode, SourceFile, SyntaxNode,
};

use crate::{
    assist_context::{AssistContext, Assists},
    utils::{self, ImportScope},
    AssistId, AssistKind,
};

// Assist: factor_out_from_enum_variants
//
// Factors out common fields from enum variants.
//
// ```
// enum A {<|>
//     One(u32, String),
//     Two(u32, bool),
// }
// ```
// ->
// ```
// struct A {
//     enum_field_0: u32,
//     enum_a: EnumA,
// }
//
// enum EnumA {
//     One(String),
//     Two(bool),
// }
// ```
pub(crate) fn factor_out_from_enum_variants(acc: &mut Assists, ctx: &AssistContext) -> Option<()> {
    let enum_ = ctx.find_node_at_offset::<ast::Enum>()?;
    let enum_vis = enum_.visibility();
    let enum_name = enum_.name()?;

    // FIXME: Support generic enums
    if enum_.generic_param_list().is_some() {
        return None;
    }

    let new_enum_name = make::name(&format!("Enum{}", enum_name.text()));
    let db = ctx.db();
    let enum_hir = ctx.sema.to_def(&enum_)?;
    let module = enum_hir.module(db);
    if utils::existing_type_definition(db, &new_enum_name, module) {
        return None;
    }

    let mut variants = enum_.variant_list()?.variants();
    let first_variant = &variants.next()?;
    let common_fields = match first_variant.field_list()? {
        // FIXME: Support record fields.
        ast::FieldList::RecordFieldList(_) => return None,
        ast::FieldList::TupleFieldList(f) => common_tuple_fields(f, variants)?,
    };

    let target = enum_.syntax().text_range();
    acc.add(
        AssistId("factor_out_from_enum_variants", AssistKind::RefactorRewrite),
        "Factor out from enum variants",
        target,
        |builder| {
            let new_fields = make::record_field_list(
                common_fields
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| {
                        let name = make::name(&format!("enum_field_{}", i));
                        make::record_field(enum_vis.clone(), name, make::ty(ty))
                    })
                    .chain(iter::once({
                        let ty = make::ty(new_enum_name.text());
                        let name = make::name(&stdx::to_lower_snake_case(new_enum_name.text()));
                        make::record_field(enum_vis.clone(), name, ty)
                    })),
            );

            let usages = Definition::ModuleDef(hir::ModuleDef::Adt(hir::Adt::Enum(enum_hir)))
                .usages(&ctx.sema)
                .all();

            let mut visited_modules_set = FxHashSet::default();
            visited_modules_set.insert(module);
            let mut renamed_exprs = FxHashMap::default();
            let mut rewriters = FxHashMap::default();

            for reference in usages {
                let rewriter = rewriters
                    .entry(reference.file_range.file_id)
                    .or_insert_with(SyntaxRewriter::default);
                let source_file = ctx.sema.parse(reference.file_range.file_id);
                update_constructor(
                    ctx,
                    rewriter,
                    &reference,
                    &source_file,
                    &enum_,
                    new_enum_name.text(),
                    &new_fields,
                    common_fields.len(),
                    &mut visited_modules_set,
                );
                update_usage(
                    ctx,
                    rewriter,
                    &reference,
                    &source_file,
                    &new_enum_name.text(),
                    &new_fields,
                    common_fields.len(),
                    &mut renamed_exprs,
                );
            }

            let mut rewriter = rewriters.remove(&ctx.frange.file_id).unwrap_or_default();
            for (file_id, rewriter) in rewriters {
                builder.edit_file(file_id);
                builder.rewrite(rewriter);
            }
            builder.edit_file(ctx.frange.file_id);

            // FIXME: Support generic enums
            let strukt = make::struct_(enum_vis, enum_name.clone(), None, new_fields.into());

            let indent_level = IndentLevel::from_node(enum_.syntax());
            rewriter.insert_before(enum_.syntax(), strukt.indent(indent_level).syntax());
            // Just using make::tokens::blank_line() won't work because we're inserting between the
            // enum_ and its indentation.
            let ws = make::tokens::whitespace(&format!("\n\n{}", indent_level));
            rewriter.insert_before(enum_.syntax(), &ws);

            update_tuple_enum(&mut rewriter, &enum_, common_fields.len());
            rewriter.replace(enum_name.syntax(), new_enum_name.syntax());

            builder.rewrite(rewriter);
        },
    );

    Some(())
}

fn update_usage(
    ctx: &AssistContext,
    rewriter: &mut SyntaxRewriter,
    reference: &Reference,
    source_file: &SourceFile,
    new_enum_name: &str,
    struct_fields: &ast::RecordFieldList,
    n_common_fields: usize,
    renamed_exprs: &mut FxHashMap<(FileId, SyntaxNode), ast::Expr>,
) -> Option<()> {
    eprintln!("==================================\n");

    // Update the enum variant in the pattern.
    // FIXME: check for ast::RecordPat
    let pat = algo::find_node_at_offset::<ast::TupleStructPat>(
        source_file.syntax(),
        reference.file_range.range.start(),
    )?;

    let path = pat.path()?;
    // FIXME: Deal with case where there is no qualifier. E.g., enum variant has been imported.
    let (old_enum_path, variant) = (path.qualifier()?, path.segment()?);
    let new_enum_path = match old_enum_path.qualifier() {
        Some(q) => make::path_qualified(q, make::path_segment(make::name_ref(new_enum_name))),
        _ => make::path_unqualified(make::path_segment(make::name_ref(new_enum_name))),
    };
    let new_path = make::path_qualified(new_enum_path, variant);

    let mut fields = pat.fields().skip(n_common_fields).peekable();
    let tuple_struct_pat;
    let new_pat = match fields.peek() {
        Some(_) => {
            tuple_struct_pat = make::tuple_struct_pat(new_path, fields);
            tuple_struct_pat.syntax()
        }
        _ => new_path.syntax(),
    };
    rewriter.replace(pat.syntax(), new_pat);

    // Update usage so that pattern matching is done against the new enum
    let mut if_or_match = None;
    let expr = pat.syntax().ancestors().find_map(|node| {
        match_ast! {
            match node {
                ast::IfExpr(it) => {
                    if_or_match = Some(it.syntax().clone());
                    let cond = it.condition()?;
                    cond.let_token()?;
                    cond.expr()
                },
                ast::MatchExpr(it) => {
                    if_or_match = Some(it.syntax().clone());
                    it.expr()
                },
                _ => None,
            }
        }
    })?;
    let if_or_match = if_or_match?;

    // Updating usage is easy if the expr is something like a PathExpr.
    // But if the usage is a CallExpr/similar, tacking on a field access to the enum will make us
    // lose access to the other fields.
    // The CallExpr has to be saved in a variable beforehand to use its fields later.
    //
    // This closure will perform said update and return the expr that we can use to access the
    // struct.
    let mut rewrite_and_insert_with = || {
        let field = make::name_ref(&stdx::to_lower_snake_case(new_enum_name));
        match expr.clone() {
            it @ ast::Expr::PathExpr(_) => {
                let enum_field = make::expr_field(it.clone(), field);
                rewriter.replace(it.syntax(), enum_field.syntax());
                Some(it)
            }
            ast::Expr::FieldExpr(it) if matches!(it.expr()?, ast::Expr::PathExpr(_)) => {
                let it = ast::Expr::from(it);
                let enum_field = make::expr_field(it.clone(), field);
                rewriter.replace(it.syntax(), enum_field.syntax());
                Some(it)
            }
            it => {
                // FIXME: Maybe use a better name? Should be a variable that probably won't collide
                let name = make::name("TEMP_VARIABLE");
                let module = ctx.sema.scope(pat.syntax()).module()?;
                if existing_variable_definition(ctx.db(), &name, module) {
                    return None;
                }

                let assign = make::let_stmt(make::ident_pat(name.clone()).into(), Some(it.clone()));
                let level = IndentLevel::from_node(it.syntax());
                let ws = make::tokens::whitespace(&format!("\n{}", level));
                rewriter.insert_before(&if_or_match, assign.indent(level).syntax());
                rewriter.insert_before(&if_or_match, &ws);

                let new_expr = make::expr_path(make::path_unqualified(make::path_segment(
                    make::name_ref(name.text()),
                )));
                let enum_field = make::expr_field(new_expr.clone(), field);
                rewriter.replace(it.syntax(), enum_field.syntax());

                Some(new_expr)
            }
        }
    };
    let struct_expr: &ast::Expr = renamed_exprs
        .entry((reference.file_range.file_id, expr.syntax().clone()))
        .or_insert(rewrite_and_insert_with()?);

    // Update the usage of common fields.
    let struct_fields: Vec<_> = struct_fields.fields().collect();
    // FIXME: Deal with `..` in pattern fields.
    let fields =
        pat.fields().take(n_common_fields).enumerate().filter_map(|(n, field)| match field {
            ast::Pat::IdentPat(ident) => Some((n, ident)),
            _ => None,
        });

    for (n, field) in fields {
        let usages = Definition::Local(ctx.sema.to_def(&field)?).usages(&ctx.sema).all();
        for usage in usages {
            let name_ref = algo::find_node_at_offset::<ast::NameRef>(
                source_file.syntax(),
                usage.file_range.range.start(),
            )?;
            let new = make::expr_field(
                struct_expr.clone(),
                make::name_ref(struct_fields[n].name()?.text()),
            );

            rewriter.replace(name_ref.syntax(), new.syntax());
        }
    }

    Some(())
}

fn common_tuple_fields(
    first_variant: ast::TupleFieldList,
    variants: ast::AstChildren<ast::Variant>,
) -> Option<Vec<String>> {
    let mut common: Vec<_> = first_variant
        .fields()
        // FIXME(rust/#68537): Replace the `.scan()`s with .map_while after it's stabilized.
        // This .scan(..) is equivalent to .map_while(|field| field.ty().map..)
        .scan((), |(), field| field.ty().map(|ty| ty.syntax().text().to_string()))
        .collect();

    for variant in variants {
        let types = match variant.field_list()? {
            ast::FieldList::TupleFieldList(f) => {
                f.fields().scan((), |(), field| field.ty().map(|ty| ty.syntax().text().to_string()))
            }
            _ => return None,
        };

        let in_common = common.iter().zip(types).take_while(|(a, b)| *a == b).count();
        common.drain(in_common..);
    }

    if common.is_empty() {
        None
    } else {
        Some(common)
    }
}

fn update_constructor(
    ctx: &AssistContext,
    rewriter: &mut SyntaxRewriter,
    reference: &Reference,
    source_file: &SourceFile,
    enum_: &ast::Enum,
    new_enum_name: &str,
    struct_fields: &ast::RecordFieldList,
    n_common_fields: usize,
    visited_modules_set: &mut FxHashSet<hir::Module>,
) -> Option<()> {
    // FIXME: check for ast::RecordExpr
    let path_expr = algo::find_node_at_offset::<ast::PathExpr>(
        source_file.syntax(),
        reference.file_range.range.start(),
    )?;
    let path = path_expr.path()?;
    let (old_enum_path, variant) = (path.qualifier()?, path.segment()?);

    // Make sure to only update constructors, not associated methods.
    enum_.variant_list()?.variants().find_map(|it| {
        if variant.name_ref()?.text() == it.name()?.text() {
            Some(())
        } else {
            None
        }
    })?;

    let call = path_expr.syntax().parent().and_then(ast::CallExpr::cast)?;
    let args = call.arg_list()?.args().collect::<Vec<_>>();
    let (common, unique) = args.split_at(n_common_fields);

    let new_enum_path = make::expr_path(make::path_qualified(
        make::path_unqualified(make::path_segment(make::name_ref(new_enum_name))),
        variant,
    ));
    let enum_constructor = if unique.is_empty() {
        new_enum_path
    } else {
        make::expr_call(new_enum_path, make::arg_list(unique.iter().cloned()))
    };

    let enum_record_field = make::record_expr_field(
        make::name_ref(&stdx::to_lower_snake_case(new_enum_name)),
        Some(enum_constructor),
    );
    let name_refs = struct_fields.fields().map(|it| {
        it.name()
            .map(|it| make::name_ref(it.text()))
            .expect("We created this RecordField, so we know it has a name.")
    });

    let record_expr_field_list = make::record_expr_field_list(
        name_refs
            .zip(common.iter().cloned())
            .map(|(name_ref, expr)| make::record_expr_field(name_ref, Some(expr)))
            .chain(iter::once(enum_record_field)),
    );
    let constructor = make::record_expr(old_enum_path, record_expr_field_list);

    let level = IndentLevel::from_node(call.syntax());
    rewriter.replace(call.syntax(), constructor.indent(level).syntax());

    // Insert imports
    let module = ctx.sema.scope(path_expr.syntax()).module()?;
    if !visited_modules_set.contains(&module) {
        visited_modules_set.insert(module);
        let import = make::name(new_enum_name).as_name();
        let enum_hir = ctx.sema.to_def(enum_)?;
        insert_import(ctx, rewriter, path_expr.syntax(), &module, enum_hir.into(), import)?;
    }

    Some(())
}

fn insert_import(
    ctx: &AssistContext,
    rewriter: &mut SyntaxRewriter,
    position: &SyntaxNode,
    module: &hir::Module,
    item: hir::ModuleDef,
    import: hir::Name,
) -> Option<()> {
    let db = ctx.db();
    let mod_path = module.find_use_path_prefixed(db, item, hir::PrefixKind::BySelf);
    if let Some(mut mod_path) = mod_path {
        mod_path.segments.pop();
        mod_path.segments.push(import);
        let scope = ImportScope::find_insert_use_container(position, ctx)?;

        *rewriter += utils::insert_use(
            &scope,
            utils::mod_path_to_ast(&mod_path),
            ctx.config.insert_use.merge,
        );
    }

    Some(())
}

fn update_tuple_enum(
    rewriter: &mut SyntaxRewriter,
    enum_: &ast::Enum,
    n_common_fields: usize,
) -> Option<()> {
    for variant in enum_.variant_list()?.variants() {
        let mut fields = match variant.field_list()? {
            ast::FieldList::TupleFieldList(f) => f.fields().skip(n_common_fields).peekable(),
            _ => return None,
        };

        let unique_variant = make::variant(
            variant.name()?,
            // Trying to instead use .map() here makes borrowck very angry :(
            match fields.peek() {
                Some(_) => Some(make::tuple_field_list(fields).into()),
                _ => None,
            },
        );
        rewriter.replace(variant.syntax(), unique_variant.syntax());
    }

    Some(())
}

fn existing_variable_definition(db: &RootDatabase, name: &ast::Name, module: hir::Module) -> bool {
    module
        .scope(db, None)
        .into_iter()
        // only check variable-namespace
        .filter(|(_, def)| match def {
            hir::ScopeDef::ModuleDef(def) => matches!(def,
                ModuleDef::Const(_) | ModuleDef::Static(_)
            ),
            hir::ScopeDef::Local(_) => true,
            _ => false,
        })
        .any(|(def, _)| def == name.as_name())
}

#[cfg(test)]
mod tests {
    use crate::tests::check_assist;

    use super::*;

    #[test]
    fn factor_out_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
pub enum Data {<|>
    A(usize, [String; 2], bool),
    B(usize, [String; 2]),
    C(usize, [String; 2], u32, i16),
}
"#,
            r#"
pub struct Data {
    pub enum_field_0: usize,
    pub enum_field_1: [String; 2],
    pub enum_data: EnumData,
}

pub enum EnumData {
    A(bool),
    B,
    C(u32, i16),
}
"#,
        );
    }

    #[test]
    fn factor_out_update_construction_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
mod also_test_indentation {
    enum A {<|>
        One(String, bool, usize, f64),
        Two(String, bool,),
    }

    fn func() {
        let _one = A::One("hi".into(), true, 42, 12.);
        let _two = A::Two("hi".into(), true);
    }
}
"#,
            r#"
mod also_test_indentation {
    struct A {
        enum_field_0: String,
        enum_field_1: bool,
        enum_a: EnumA,
    }

    enum EnumA {
        One(usize, f64),
        Two,
    }

    fn func() {
        let _one = A {
            enum_field_0: "hi".into(),
            enum_field_1: true,
            enum_a: EnumA::One(42, 12.),
        };
        let _two = A {
            enum_field_0: "hi".into(),
            enum_field_1: true,
            enum_a: EnumA::Two,
        };
    }
}
"#,
        );
    }

    #[test]
    fn factor_out_with_import_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
enum Data {
    A(i32,<|> bool),
    B(i32),
}

fn func() {
    let _ = Data::B(50);
}

mod foo {
    fn func() {
        let _ = crate::Data::B(50);
    }

    mod bar {
        use crate::Data;

        fn func() {
            let _ = Data::B(50);
        }
    }
}
"#,
            r#"
struct Data {
    enum_field_0: i32,
    enum_data: EnumData,
}

enum EnumData {
    A(bool),
    B,
}

fn func() {
    let _ = Data {
        enum_field_0: 50,
        enum_data: EnumData::B,
    };
}

mod foo {
    use crate::EnumData;

    fn func() {
        let _ = crate::Data {
            enum_field_0: 50,
            enum_data: EnumData::B,
        };
    }

    mod bar {
        use crate::{Data, EnumData};

        fn func() {
            let _ = Data {
                enum_field_0: 50,
                enum_data: EnumData::B,
            };
        }
    }
}
"#,
        );
    }

    #[test]
    fn factor_out_update_match_usage_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
enum A {
    One(String, usize),<|>
    Two(String),
}

fn func(it: A) {
    let _output = match it {
        A::One(text, n) => {
            // repeat the text n times
            text.repeat(n)
        }
        A::Two(text) => text,
    };
}
"#,
            r#"
struct A {
    enum_field_0: String,
    enum_a: EnumA,
}

enum EnumA {
    One(usize),
    Two,
}

fn func(it: A) {
    let _output = match it.enum_a {
        EnumA::One(n) => {
            // repeat the text n times
            it.enum_field_0.repeat(n)
        }
        EnumA::Two => it.enum_field_0,
    };
}
"#,
        );
    }

    #[test]
    fn factor_out_update_iflet_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
enum A {
    One(String, usize),
    Two(String),<|>
}

mod foo {
    fn func(it: crate::A) {
        if let crate::A::One(text, n) = it {
            // repeat the text n times
            let _output = text.repeat(n);
        }
    }
}
"#,
            r#"
struct A {
    enum_field_0: String,
    enum_a: EnumA,
}

enum EnumA {
    One(usize),
    Two,
}

mod foo {
    fn func(it: crate::A) {
        if let crate::EnumA::One(n) = it.enum_a {
            // repeat the text n times
            let _output = it.enum_field_0.repeat(n);
        }
    }
}
"#,
        );
    }

    #[test]
    fn factor_out_update_pattern_with_field_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
enum A {
    One(String, usize),<|>
    Two(String),
}

mod foo {
    fn func(it: (bool, crate::A)) {
        if let crate::A::One(text, n) = it.1 {
            // repeat the text n times
            let _output = text.repeat(n);
        }
    }
}
"#,
            r#"
struct A {
    enum_field_0: String,
    enum_a: EnumA,
}

enum EnumA {
    One(usize),
    Two,
}

mod foo {
    fn func(it: (bool, crate::A)) {
        if let crate::EnumA::One(n) = it.1.enum_a {
            // repeat the text n times
            let _output = it.1.enum_field_0.repeat(n);
        }
    }
}
"#,
        );
    }

    #[test]
    fn factor_out_update_expr_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
enum A {
    One(String, usize),
    Two(String),<|>
}

impl Default for A {
    fn default() -> A {
        A::One("hi".into(), 42)
    }
}

fn func() {
    if let A::One(text, n) = A::default() {
        // repeat the text n times
        let _output = text.repeat(n);
    }
}
"#,
            r#"
struct A {
    enum_field_0: String,
    enum_a: EnumA,
}

enum EnumA {
    One(usize),
    Two,
}

impl Default for A {
    fn default() -> A {
        A {
            enum_field_0: "hi".into(),
            enum_a: EnumA::One(42),
        }
    }
}

fn func() {
    let TEMP_VARIABLE = A::default();
    if let EnumA::One(n) = TEMP_VARIABLE.enum_a {
        // repeat the text n times
        let _output = TEMP_VARIABLE.enum_field_0.repeat(n);
    }
}
"#,
        );
    }

    #[test]
    fn factor_out_update_pattern_with_literal_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
enum A {
    One(usize, usize, usize),<|>
    Two(usize, usize),
}

fn func(it: A) {
    match it {
        A::One(0, j, k) => {
            let _ = j * k;
        }
        A::One(2, j, k) if (j + k) % 2 == 0 => {
            let _ = (j + k) / 2;
        }
        A::One(i @ 5..=20, j, k) => {
            let _ = i + j * k;
        }
        A::One(i, j, k @ 30..=50) => {
            let _ = k + i * j;
        }
        _ => {}
    }
}
"#,
            r#"
struct A {
    enum_field_0: usize,
    enum_field_1: usize,
    enum_a: EnumA,
}

enum EnumA {
    One(usize),
    Two,
}

fn func(it: A) {
    match it.enum_a {
        EnumA::One(k) if it.enum_field_0 == 0 => {
            let _ = it.enum_field_1 * k;
        }
        EnumA::One(k) if it.enum_field_0 == 2 && (it.enum_field_1 + k) % 2 == 0 => {
            let _ = (it.enum_field_1 + k) / 2;
        }
        EnumA::One(k) if (5..=20).contains(&it.enum_field_0) => {
            let _ = it.enum_field_0 + it.enum_field_1 * k;
        }
        EnumA::One(k @ 30..=50) => {
            let _ = k + it.enum_field_0 * it.enum_field_1;
        }
        _ => {}
    }
}
"#,
        );
    }

    #[test]
    fn factor_out_update_pattern_with_rest_works() {
        check_assist(
            factor_out_from_enum_variants,
            r#"
enum A {
    One(u8, String, usize, bool, i32, f64, Vec<char>),
    Two(u8, String, usize, bool, i32),<|>
}

fn func(it: A) {
    match it {
        A::One(0, s, u, ..) => {
            let _: (String, usize) = (s, u);
        }
        A::One(1, s, u, .., v) => {
            let _: (String, usize, Vec<char>) = (s, u, v);
        }
        A::One(2, s, .., f, v) => {
            let _: (String, f64, Vec<char>) = (s, f, v);
        }
        A::One(3, .., i, f, v) => {
            let _: (i32, f64, Vec<char>) = (i, f, v);
        }
        A::One(.., f, v) => {
            let _: (f64, Vec<char>) = (f, v);
        }
        A::Two(4, .., i) => {
            let _: i32 = i;
        }
        A::Two(..) => {}
    };
}
"#,
            r#"
struct A {
    enum_field_0: u8,
    enum_field_1: String,
    enum_field_2: usize,
    enum_field_3: bool,
    enum_field_4: i32,
    enum_a: EnumA,
}

enum EnumA {
    One(f64, Vec<char>),
    Two,
}

fn func(it: A) {
    match it.enum_a {
        EnumA::One(..) if it.enum_field_0 == 0 => {
            let _: (String, usize) = (it.enum_field_1, it.enum_field_2);
        }
        EnumA::One(.., v) if it.enum_field_0 == 1 => {
            let _: (String, usize, Vec<char>) = (it.enum_field_1, it.enum_field_2, v);
        }
        EnumA::One(f, v) if it.enum_field_0 == 2 => {
            let _: (String, f64, Vec<char>) = (it.enum_field_1, f, v);
        }
        EnumA::One(f, v) if it.enum_field_0 == 3 => {
            let _: (i32, f64, Vec<char>) = (it.enum_field_4, f, v);
        }
        EnumA::One(f, v) => {
            let _: (f64, Vec<char>) = (f, v);
        }
        EnumA::Two if it.enum_field_0 == 4 => {
            let _: i32 = it.enum_field_4;
        }
        EnumA::Two => {},
    };
}
"#,
        );
    }
}
