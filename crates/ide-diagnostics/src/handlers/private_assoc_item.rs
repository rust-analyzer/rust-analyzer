use hir::{EditionedFileId, FileRange, HasCrate, HasSource, Semantics};
use ide_db::{RootDatabase, assists::Assist, source_change::SourceChange, text_edit::TextEdit};
use syntax::{AstNode, TextRange, ast::HasVisibility};

use crate::{Diagnostic, DiagnosticCode, DiagnosticsContext, fix};

// Diagnostic: private-assoc-item
//
// This diagnostic is triggered if the referenced associated item is not visible from the current
// module.
pub(crate) fn private_assoc_item(
    ctx: &DiagnosticsContext<'_, '_>,
    d: &hir::PrivateAssocItem,
) -> Diagnostic {
    let name = d
        .item
        .name(ctx.sema.db)
        .map(|name| format!("`{}` ", name.display(ctx.sema.db, ctx.edition)))
        .unwrap_or_default();
    Diagnostic::new_with_syntax_node_ptr(
        ctx,
        DiagnosticCode::RustcHardError("E0624"),
        format!(
            "{} {}is private",
            match d.item {
                hir::AssocItem::Function(_) => "function",
                hir::AssocItem::Const(_) => "const",
                hir::AssocItem::TypeAlias(_) => "type alias",
            },
            name,
        ),
        d.expr_or_pat.map(Into::into),
    )
    .stable()
    .with_fixes(private_assoc_item_fixes(
        &ctx.sema,
        d.expr_or_pat.file_id.original_file(ctx.sema.db),
        d.item,
        ctx.sema.original_range(d.expr_or_pat.to_node(ctx.sema.db).syntax()).range,
    ))
}

fn private_assoc_item_fixes(
    sema: &Semantics<'_, RootDatabase>,
    usage_file_id: EditionedFileId,
    item: hir::AssocItem,
    fix_range: TextRange,
) -> Option<Vec<Assist>> {
    let def_crate = item.krate(sema.db);
    let usage_crate = sema.file_to_module_def(usage_file_id.file_id(sema.db))?.krate(sema.db);
    let mut visibility_text =
        if usage_crate == def_crate { "pub(crate) " } else { "pub " }.to_owned();

    fn add_vis_to_assoc_item<S: AstNode + HasVisibility>(
        source: hir::InFile<S>,
        sema: &Semantics<'_, RootDatabase>,
        visibility_text: &mut String,
    ) -> Option<FileRange> {
        let existing_visibility = source.value.visibility();
        match existing_visibility {
            Some(visibility) => {
                *visibility_text = visibility_text.trim_end().to_owned();
                source.with_value(visibility.syntax()).original_file_range_opt(sema.db)?.0.into()
            }
            None => {
                let (range, _) = source
                    .map(|it| {
                        it.syntax()
                            .children_with_tokens()
                            .find(|it| {
                                !matches!(
                                    it.kind(),
                                    syntax::SyntaxKind::WHITESPACE
                                        | syntax::SyntaxKind::COMMENT
                                        | syntax::SyntaxKind::ATTR
                                )
                            })
                            .map(|it| it.text_range())
                    })
                    .transpose()?
                    .original_node_file_range_opt(sema.db)?;
                Some(FileRange {
                    file_id: range.file_id,
                    range: TextRange::empty(range.range.start()),
                })
            }
        }
    }

    let range = match item {
        hir::AssocItem::Function(it) => {
            add_vis_to_assoc_item(it.source(sema.db)?, sema, &mut visibility_text)?
        }
        hir::AssocItem::Const(it) => {
            add_vis_to_assoc_item(it.source(sema.db)?, sema, &mut visibility_text)?
        }
        hir::AssocItem::TypeAlias(it) => {
            add_vis_to_assoc_item(it.source(sema.db)?, sema, &mut visibility_text)?
        }
    };

    let source_change = SourceChange::from_text_edit(
        range.file_id.file_id(sema.db),
        TextEdit::replace(range.range, visibility_text),
    );

    Some(vec![fix(
        "increase_assoc_item_visibility",
        "Increase associated item visibility",
        source_change,
        fix_range,
    )])
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_diagnostics, check_fix};

    #[test]
    fn private_method() {
        check_diagnostics(
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method();
  //^^^^^^^^^^ 💡 error: function `method` is private
}
"#,
        );
    }

    #[test]
    fn private_func() {
        check_diagnostics(
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        fn func() {}
    }
}
fn main() {
    module::Struct::func();
  //^^^^^^^^^^^^^^^^^^^^ 💡 error: function `func` is private
}
"#,
        );
    }

    #[test]
    fn private_const() {
        check_diagnostics(
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        const CONST: u32 = 0;
    }
}
fn main() {
    module::Struct::CONST;
  //^^^^^^^^^^^^^^^^^^^^^ 💡 error: const `CONST` is private
}
"#,
        );
    }

    #[test]
    fn private_method_same_crate_fix() {
        check_fix(
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method$0();
}
"#,
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        pub(crate) fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method();
}
"#,
        );
    }

    #[test]
    fn private_method_other_crate_fix() {
        check_fix(
            r#"
//- /lib.rs crate:another_crate
pub struct Struct;
impl Struct {
    fn method(&self) {}
}
//- /lib.rs crate:this_crate deps:another_crate
use another_crate::Struct;

fn main(s: Struct) {
    s.method$0();
}
"#,
            r#"
pub struct Struct;
impl Struct {
    pub fn method(&self) {}
}
"#,
        );
    }

    #[test]
    fn private_method_fix_with_doc_comment() {
        check_fix(
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        /// This is a doc comment.
        fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method$0();
}
"#,
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        /// This is a doc comment.
        pub(crate) fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method();
}
"#,
        );
    }

    #[test]
    fn private_method_fix_with_attr_and_comment() {
        check_fix(
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        #[rustfmt::skip]
        // This is a line comment.
        fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method$0();
}
"#,
            r#"
mod module {
    pub struct Struct;
    impl Struct {
        #[rustfmt::skip]
        // This is a line comment.
        pub(crate) fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method();
}
"#,
        );
    }

    #[test]
    fn private_but_shadowed_in_deref() {
        check_diagnostics(
            r#"
//- minicore: deref
mod module {
    pub struct Struct { field: Inner }
    pub struct Inner;
    impl core::ops::Deref for Struct {
        type Target = Inner;
        fn deref(&self) -> &Inner { &self.field }
    }
    impl Struct {
        fn method(&self) {}
    }
    impl Inner {
        pub fn method(&self) {}
    }
}
fn main(s: module::Struct) {
    s.method();
}
"#,
        );
    }

    #[test]
    fn can_see_through_top_level_anonymous_const() {
        // regression test for #14046.
        check_diagnostics(
            r#"
struct S;
mod m {
    const _: () = {
        impl crate::S {
            pub(crate) fn method(self) {}
            pub(crate) const A: usize = 42;
        }
    };
    mod inner {
        const _: () = {
            impl crate::S {
                pub(crate) fn method2(self) {}
                pub(crate) const B: usize = 42;
                pub(super) fn private(self) {}
                pub(super) const PRIVATE: usize = 42;
            }
        };
    }
}
fn main() {
    S.method();
    S::A;
    S.method2();
    S::B;
    S.private();
  //^^^^^^^^^^^ 💡 error: function `private` is private
    S::PRIVATE;
  //^^^^^^^^^^ 💡 error: const `PRIVATE` is private
}
"#,
        );
    }
}
