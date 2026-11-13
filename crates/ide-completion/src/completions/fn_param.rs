//! See [`complete_fn_param`].

use hir::HirDisplay;
use ide_db::{FxHashMap, text_edit::TextEdit};
use itertools::Either;
use syntax::{
    AstNode, Direction, SmolStr, SyntaxKind, TextRange, TextSize, ToSmolStr, algo,
    ast::{self, HasModuleItem},
    format_smolstr, match_ast,
};

use crate::{
    CompletionContext, CompletionItem, CompletionItemKind, Completions,
    context::{ParamContext, ParamKind, PatternContext},
};

// FIXME: Make this a submodule of [`pattern`]
/// Complete repeated parameters, both name and type. For example, if all
/// functions in a file have a `spam: &mut Spam` parameter, a completion with
/// `spam: &mut Spam` insert text/label will be suggested.
///
/// Also complete parameters for closure or local functions from the surrounding defined locals.
pub(crate) fn complete_fn_param(
    acc: &mut Completions,
    ctx: &CompletionContext<'_, '_>,
    pattern_ctx: &PatternContext,
) -> Option<()> {
    let (ParamContext { param_list, kind, param, .. }, impl_or_trait) = match pattern_ctx {
        PatternContext { param_ctx: Some(kind), impl_or_trait, .. } => (kind, impl_or_trait),
        _ => return None,
    };

    let qualifier = param_qualifier(param);
    let comma_wrapper = comma_wrapper(ctx);
    let mut add_new_item_to_acc = |label: &str| {
        let label = label.strip_prefix(qualifier.as_str()).unwrap_or(label);
        let insert = if label.starts_with('#') {
            // FIXME: `#[attr] it: i32` -> `#[attr] mut it: i32`
            label.to_smolstr()
        } else {
            format_smolstr!("{qualifier}{label}")
        };
        let mk_item = |edit, range: TextRange| {
            let mut item =
                CompletionItem::new(CompletionItemKind::Binding, range, label, ctx.edition);
            item.text_edit(edit);
            item
        };
        let item = match &comma_wrapper {
            Some((fmt, range)) => mk_item(fmt(&insert), *range),
            None => {
                mk_item(TextEdit::replace(ctx.source_range(), insert.into()), ctx.source_range())
            }
        };
        // Completion lookup is omitted intentionally here.
        // See the full discussion: https://github.com/rust-lang/rust-analyzer/issues/12073
        item.add_to(acc, ctx.db)
    };

    match kind {
        ParamKind::Function(function) => {
            fill_fn_params(ctx, function, param_list, param, impl_or_trait, add_new_item_to_acc);
        }
        ParamKind::Closure(closure) => {
            if is_simple_param(param) {
                let stmt_list = closure.syntax().ancestors().find_map(ast::StmtList::cast)?;
                params_from_stmt_list_scope(ctx, stmt_list, |name, ty| {
                    add_new_item_to_acc(&format_smolstr!(
                        "{}: {ty}",
                        name.display(ctx.db, ctx.edition)
                    ));
                });
            }
        }
    }

    Some(())
}

fn fill_fn_params(
    ctx: &CompletionContext<'_, '_>,
    function: &ast::Fn,
    param_list: &ast::ParamList,
    current_param: &ast::Param,
    impl_or_trait: &Option<Either<ast::Impl, ast::Trait>>,
    mut add_new_item_to_acc: impl FnMut(&str),
) {
    let mut file_params = FxHashMap::default();

    let mut extract_params = |f: ast::Fn| {
        f.param_list().into_iter().flat_map(|it| it.params()).for_each(|param| {
            if let Some(pat) = param.pat() {
                let whole_param = param.to_smolstr();
                let binding = pat.to_smolstr();
                file_params.entry(whole_param).or_insert(binding);
            }
        });
    };

    for node in ctx.token.parent_ancestors() {
        if !is_simple_param(current_param) {
            break;
        }
        match_ast! {
            match node {
                ast::SourceFile(it) => it.items().filter_map(|item| match item {
                    ast::Item::Fn(it) => Some(it),
                    _ => None,
                }).for_each(&mut extract_params),
                ast::ItemList(it) => it.items().filter_map(|item| match item {
                    ast::Item::Fn(it) => Some(it),
                    _ => None,
                }).for_each(&mut extract_params),
                ast::AssocItemList(it) => it.assoc_items().filter_map(|item| match item {
                    ast::AssocItem::Fn(it) => Some(it),
                    _ => None,
                }).for_each(&mut extract_params),
                _ => continue,
            }
        };
    }

    if let Some(stmt_list) = function.syntax().parent().and_then(ast::StmtList::cast)
        && is_simple_param(current_param)
    {
        params_from_stmt_list_scope(ctx, stmt_list, |name, ty| {
            file_params
                .entry(format_smolstr!("{}: {ty}", name.display(ctx.db, ctx.edition)))
                .or_insert(name.display(ctx.db, ctx.edition).to_smolstr());
        });
    }
    remove_duplicated(&mut file_params, param_list.params());
    let self_completion_items = ["self", "&self", "mut self", "&mut self"];
    if should_add_self_completions(ctx.token.text_range().start(), param_list, impl_or_trait) {
        self_completion_items.into_iter().for_each(&mut add_new_item_to_acc);
    }

    file_params.keys().for_each(|whole_param| add_new_item_to_acc(whole_param));
}

fn params_from_stmt_list_scope(
    ctx: &CompletionContext<'_, '_>,
    stmt_list: ast::StmtList,
    mut cb: impl FnMut(hir::Name, String),
) {
    let syntax_node = match stmt_list.syntax().last_child() {
        Some(it) => it,
        None => return,
    };
    if let Some(scope) =
        ctx.sema.scope_at_offset(stmt_list.syntax(), syntax_node.text_range().end())
    {
        let module = scope.module().into();
        scope.process_all_names(&mut |name, def| {
            if let hir::ScopeDef::Local(local) = def
                && let Ok(ty) = local.ty(ctx.db).display_source_code(ctx.db, module, true)
            {
                cb(name, ty);
            }
        });
    }
}

fn remove_duplicated(
    file_params: &mut FxHashMap<SmolStr, SmolStr>,
    fn_params: ast::AstChildren<ast::Param>,
) {
    fn_params.for_each(|param| {
        let whole_param = param.to_smolstr();
        file_params.remove(&whole_param);

        match param.pat() {
            // remove suggestions for patterns that already exist
            // if the type is missing we are checking the current param to be completed
            // in which case this would find itself removing the suggestions due to itself
            Some(pattern) if param.ty().is_some() => {
                let binding = pattern.to_smolstr();
                file_params.retain(|_, v| v != &binding);
            }
            _ => (),
        }
    })
}

fn should_add_self_completions(
    cursor: TextSize,
    param_list: &ast::ParamList,
    impl_or_trait: &Option<Either<ast::Impl, ast::Trait>>,
) -> bool {
    if impl_or_trait.is_none() || param_list.self_param().is_some() {
        return false;
    }
    match param_list.params().next() {
        Some(first) => first.pat().is_some_and(|pat| pat.syntax().text_range().contains(cursor)),
        None => true,
    }
}

fn comma_wrapper(
    ctx: &CompletionContext<'_, '_>,
) -> Option<(impl Fn(&str) -> TextEdit, TextRange)> {
    let param =
        ctx.original_token.parent_ancestors().find(|node| node.kind() == SyntaxKind::PARAM)?;

    let next_token_kind = {
        let t = param.last_token()?.next_token()?;
        let t = algo::skip_whitespace_token(t, Direction::Next)?;
        t.kind()
    };
    let prev_param = param.prev_sibling().and_then(ast::Param::cast);

    let needs_comma_before = prev_param
        .as_ref()
        .and_then(|it| {
            algo::skip_trivia_token(it.syntax().last_token()?.next_token()?, Direction::Next)
        })
        .is_some_and(|it| it.kind() != SyntaxKind::COMMA);
    let needs_comma_after = match next_token_kind {
        SyntaxKind::COMMA => false,
        SyntaxKind::R_PAREN | SyntaxKind::PIPE => param
            .next_sibling_or_token()
            .and_then(|it| it.into_token())
            .is_some_and(|it| it.text().contains("\n")),
        _ => true,
    };

    let insert_comma_before = prev_param.filter(|_| needs_comma_before).map(|prev_param| {
        let needs_space_before =
            prev_param.syntax().next_sibling_or_token().is_none_or(|it| !it.kind().is_trivia());
        (prev_param.syntax().text_range().end(), if needs_space_before { ", " } else { "," })
    });

    let trailing = if needs_comma_after { "," } else { "" };
    let range = param.text_range();

    Some((
        move |label: &_| {
            let insert_text = format!("{label}{trailing}");
            let mut edit = TextEdit::builder();
            if let Some((offset, comma)) = insert_comma_before {
                edit.insert(offset, comma.to_owned());
            }
            edit.replace(range, insert_text);
            edit.finish()
        },
        range,
    ))
}

fn is_simple_param(param: &ast::Param) -> bool {
    param
        .pat()
        .is_none_or(|pat| matches!(pat, ast::Pat::IdentPat(ident_pat) if ident_pat.pat().is_none()))
}

fn param_qualifier(param: &ast::Param) -> SmolStr {
    let mut b = syntax::SmolStrBuilder::new();
    if let Some(ast::Pat::IdentPat(pat)) = param.pat() {
        if pat.ref_token().is_some() {
            b.push_str("ref ");
        }
        if pat.mut_token().is_some() {
            b.push_str("mut ");
        }
    }
    b.finish()
}
