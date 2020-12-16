//! Provides a way to attach fixes to the diagnostics.
//! The same module also has all curret custom fixes for the diagnostics implemented.
use hir::{
    db::AstDatabase,
    diagnostics::{
        AddReferenceToInitializer, Diagnostic, IncorrectCase, MissingFields, MissingOkInTailExpr,
        NoSuchField, RemoveThisSemicolon, UnresolvedModule,
    },
    HasSource, HirDisplay, InFile, Mutability, Semantics, VariantDef,
};
use ide_db::base_db::{AnchoredPathBuf, FileId};
use ide_db::{
    source_change::{FileSystemEdit, SourceFileEdit},
    RootDatabase,
};
use syntax::{
    algo,
    ast::{self, edit::IndentLevel, make},
    AstNode,
};
use text_edit::TextEdit;

use crate::{diagnostics::Fix, references::rename::rename_with_semantics, FilePosition};

/// A [Diagnostic] that potentially has a fix available.
///
/// [Diagnostic]: hir::diagnostics::Diagnostic
pub(crate) trait DiagnosticWithFix: Diagnostic {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix>;
}

impl DiagnosticWithFix for UnresolvedModule {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix> {
        let root = sema.db.parse_or_expand(self.file)?;
        let unresolved_module = self.decl.to_node(&root);
        Some(Fix::new(
            "Create module",
            FileSystemEdit::CreateFile {
                dst: AnchoredPathBuf {
                    anchor: self.file.original_file(sema.db),
                    path: self.candidate.clone(),
                },
            }
            .into(),
            unresolved_module.syntax().text_range(),
        ))
    }
}

impl DiagnosticWithFix for NoSuchField {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix> {
        let root = sema.db.parse_or_expand(self.file)?;
        missing_record_expr_field_fix(
            &sema,
            self.file.original_file(sema.db),
            &self.field.to_node(&root),
        )
    }
}

impl DiagnosticWithFix for MissingFields {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix> {
        // Note that although we could add a diagnostics to
        // fill the missing tuple field, e.g :
        // `struct A(usize);`
        // `let a = A { 0: () }`
        // but it is uncommon usage and it should not be encouraged.
        if self.missed_fields.iter().any(|it| it.as_tuple_index().is_some()) {
            return None;
        }

        let root = sema.db.parse_or_expand(self.file)?;
        let field_list_parent = self.field_list_parent.to_node(&root);
        let old_field_list = field_list_parent.record_expr_field_list()?;
        let mut new_field_list = old_field_list.clone();
        for f in self.missed_fields.iter() {
            let field =
                make::record_expr_field(make::name_ref(&f.to_string()), Some(make::expr_unit()));
            new_field_list = new_field_list.append_field(&field);
        }

        let edit = {
            let mut builder = TextEdit::builder();
            algo::diff(&old_field_list.syntax(), &new_field_list.syntax())
                .into_text_edit(&mut builder);
            builder.finish()
        };
        Some(Fix::new(
            "Fill struct fields",
            SourceFileEdit { file_id: self.file.original_file(sema.db), edit }.into(),
            sema.original_range(&field_list_parent.syntax()).range,
        ))
    }
}

impl DiagnosticWithFix for MissingOkInTailExpr {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix> {
        let root = sema.db.parse_or_expand(self.file)?;
        let tail_expr = self.expr.to_node(&root);
        let tail_expr_range = tail_expr.syntax().text_range();
        let edit = TextEdit::replace(tail_expr_range, format!("Ok({})", tail_expr.syntax()));
        let source_change =
            SourceFileEdit { file_id: self.file.original_file(sema.db), edit }.into();
        Some(Fix::new("Wrap with ok", source_change, tail_expr_range))
    }
}

impl DiagnosticWithFix for RemoveThisSemicolon {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix> {
        let root = sema.db.parse_or_expand(self.file)?;

        let semicolon = self
            .expr
            .to_node(&root)
            .syntax()
            .parent()
            .and_then(ast::ExprStmt::cast)
            .and_then(|expr| expr.semicolon_token())?
            .text_range();

        let edit = TextEdit::delete(semicolon);
        let source_change =
            SourceFileEdit { file_id: self.file.original_file(sema.db), edit }.into();

        Some(Fix::new("Remove this semicolon", source_change, semicolon))
    }
}

impl DiagnosticWithFix for AddReferenceToInitializer {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix> {
        let root = sema.db.parse_or_expand(self.file)?;
        let arg_expr = self.arg_expr.to_node(&root);

        let arg_with_ref = match self.mutability {
            Mutability::Shared => format!("&{}", arg_expr.syntax()),
            Mutability::Mut => format!("&mut {}", arg_expr.syntax()),
        };

        let text_range = sema.original_range(arg_expr.syntax()).range;

        let edit = TextEdit::replace(text_range, arg_with_ref);
        let source_change =
            SourceFileEdit { file_id: self.file.original_file(sema.db), edit }.into();

        Some(Fix::new("Add reference here", source_change, text_range))
    }
}

impl DiagnosticWithFix for IncorrectCase {
    fn fix(&self, sema: &Semantics<RootDatabase>) -> Option<Fix> {
        let root = sema.db.parse_or_expand(self.file)?;
        let name_node = self.ident.to_node(&root);

        let name_node = InFile::new(self.file, name_node.syntax());
        let frange = name_node.original_file_range(sema.db);
        let file_position = FilePosition { file_id: frange.file_id, offset: frange.range.start() };

        let rename_changes =
            rename_with_semantics(sema, file_position, &self.suggested_text).ok()?;

        let label = format!("Rename to {}", self.suggested_text);
        Some(Fix::new(&label, rename_changes.info, rename_changes.range))
    }
}

fn missing_record_expr_field_fix(
    sema: &Semantics<RootDatabase>,
    usage_file_id: FileId,
    record_expr_field: &ast::RecordExprField,
) -> Option<Fix> {
    let record_lit = ast::RecordExpr::cast(record_expr_field.syntax().parent()?.parent()?)?;
    let def_id = sema.resolve_variant(record_lit)?;
    let module;
    let def_file_id;
    let record_fields = match VariantDef::from(def_id) {
        VariantDef::Struct(s) => {
            module = s.module(sema.db);
            let source = s.source(sema.db);
            def_file_id = source.file_id;
            let fields = source.value.field_list()?;
            record_field_list(fields)?
        }
        VariantDef::Union(u) => {
            module = u.module(sema.db);
            let source = u.source(sema.db);
            def_file_id = source.file_id;
            source.value.record_field_list()?
        }
        VariantDef::EnumVariant(e) => {
            module = e.module(sema.db);
            let source = e.source(sema.db);
            def_file_id = source.file_id;
            let fields = source.value.field_list()?;
            record_field_list(fields)?
        }
    };
    let def_file_id = def_file_id.original_file(sema.db);

    let new_field_type = sema.type_of_expr(&record_expr_field.expr()?)?;
    if new_field_type.is_unknown() {
        return None;
    }
    let new_field = make::record_field(
        None,
        make::name(record_expr_field.field_name()?.text()),
        make::ty(&new_field_type.display_source_code(sema.db, module.into()).ok()?),
    );

    let last_field = record_fields.fields().last()?;
    let last_field_syntax = last_field.syntax();
    let indent = IndentLevel::from_node(last_field_syntax);

    let mut new_field = new_field.to_string();
    if usage_file_id != def_file_id {
        new_field = format!("pub(crate) {}", new_field);
    }
    new_field = format!("\n{}{}", indent, new_field);

    let needs_comma = !last_field_syntax.to_string().ends_with(',');
    if needs_comma {
        new_field = format!(",{}", new_field);
    }

    let source_change = SourceFileEdit {
        file_id: def_file_id,
        edit: TextEdit::insert(last_field_syntax.text_range().end(), new_field),
    };
    return Some(Fix::new(
        "Create field",
        source_change.into(),
        record_expr_field.syntax().text_range(),
    ));

    fn record_field_list(field_def_list: ast::FieldList) -> Option<ast::RecordFieldList> {
        match field_def_list {
            ast::FieldList::RecordFieldList(it) => Some(it),
            ast::FieldList::TupleFieldList(_) => None,
        }
    }
}
