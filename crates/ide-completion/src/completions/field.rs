//! Completion of field list position.

use ide_db::{FxHashSet, FxIndexMap};
use syntax::{
    AstNode, T, ToSmolStr,
    ast::{self, HasModuleItem, HasName},
    format_smolstr,
};

use crate::{
    CompletionContext, CompletionItem, CompletionItemKind, Completions,
    context::{PathCompletionCtx, Qualified},
};

pub(crate) fn complete_field_list_tuple_variant(
    acc: &mut Completions,
    ctx: &CompletionContext<'_, '_>,
    path_ctx: &PathCompletionCtx<'_>,
) {
    complete_record_fields(acc, ctx);

    if ctx.qualifier_ctx.vis_node.is_some() {
    } else if let PathCompletionCtx {
        has_macro_bang: false,
        qualified: Qualified::No,
        parent: None,
        has_type_args: false,
        ..
    } = path_ctx
    {
        let mut add_keyword = |kw, snippet| acc.add_keyword_snippet(ctx, kw, snippet);
        add_keyword("pub(crate)", "pub(crate) $0");
        add_keyword("pub(super)", "pub(super) $0");
        add_keyword("pub", "pub $0");
    }
}

pub(crate) fn complete_field_list_record_variant(
    acc: &mut Completions,
    ctx: &CompletionContext<'_, '_>,
) {
    complete_record_fields(acc, ctx);

    if ctx.qualifier_ctx.vis_node.is_none() {
        let mut add_keyword = |kw, snippet| acc.add_keyword_snippet(ctx, kw, snippet);
        add_keyword("pub(crate)", "pub(crate) $0");
        add_keyword("pub(super)", "pub(super) $0");
        add_keyword("pub", "pub $0");
    }
}

fn complete_record_fields(acc: &mut Completions, ctx: &CompletionContext<'_, '_>) {
    let node = ctx.original_token.parent_ancestors().nth(1);
    let Some(field) = node.and_then(ast::RecordField::cast) else { return };
    let Some(field_list) = field.syntax().parent().and_then(ast::RecordFieldList::cast) else {
        return;
    };

    if field.name().is_none_or(|it| it.syntax().next_sibling_or_token().is_some()) {
        return;
    }
    let needs_comma = field.syntax().next_sibling_or_token().is_none_or(|it| it.kind() != T![,]);
    let comma = if needs_comma { "," } else { "" };

    let mut fields = FxIndexMap::default();
    let mut extract_fields = |item: ast::Item| {
        let mut process = |node: &syntax::SyntaxNode| {
            node.descendants().filter_map(ast::RecordField::cast).for_each(|field| {
                if let Some(name) = field.name()
                    && field.colon_token().is_some()
                    && field.ty().is_some()
                {
                    let name_in = name.syntax().text_range() - field.syntax().text_range().start();
                    let text = field.syntax().text().slice(name_in.start()..);
                    fields.insert(format_smolstr!("{text}{comma}"), name);
                }
            });
        };
        (|| {
            match item {
                ast::Item::Enum(it) => process(it.variant_list()?.syntax()),
                ast::Item::Struct(it) => process(it.field_list()?.syntax()),
                ast::Item::Union(union) => process(union.record_field_list()?.syntax()),
                _ => return None,
            }
            Some(())
        })();
    };
    for node in ctx.original_token.parent_ancestors() {
        syntax::match_ast! {
            match node {
                ast::SourceFile(it) => it.items().for_each(&mut extract_fields),
                ast::ItemList(it) => it.items().for_each(&mut extract_fields),
                _ => continue,
            }
        }
    }

    let current = field_list
        .fields()
        .filter(|it| it.colon_token().is_some() && it.ty().is_some())
        .filter_map(|field| Some(field.name()?.to_smolstr()))
        .collect::<FxHashSet<_>>();
    fields.retain(|_, name| !current.contains(name.text()));

    for text in fields.into_keys() {
        CompletionItem::new(
            CompletionItemKind::Binding,
            field.syntax().text_range(),
            text,
            ctx.edition,
        )
        .add_to(acc, ctx.db);
    }
}
