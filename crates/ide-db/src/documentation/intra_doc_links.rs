//! Helper tools for intra doc links.

use pulldown_cmark::{BrokenLink, CowStr, Event, LinkType, Options, Parser, Tag};

use hir::{AttrsWithOwner, HasAttrs, Semantics, db::HirDatabase};
use syntax::{AstNode, SyntaxNode, TextRange, TextSize, ast, match_ast};

use crate::{
    EditionedFileId, RootDatabase,
    defs::Definition,
    documentation::{Documentation, HasDocs},
};

const MARKDOWN_OPTIONS: Options =
    Options::ENABLE_FOOTNOTES.union(Options::ENABLE_TABLES).union(Options::ENABLE_TASKLISTS);

const TYPES: (&[&str], &[&str]) =
    (&["type", "struct", "enum", "mod", "trait", "union", "module", "prim", "primitive"], &[]);
const VALUES: (&[&str], &[&str]) =
    (&["value", "function", "fn", "method", "const", "static", "mod", "module"], &["()"]);
const MACROS: (&[&str], &[&str]) = (&["macro", "derive"], &["!"]);

/// Extract the specified namespace from an intra-doc link, if one exists.
///
/// # Examples
///
/// * `struct MyStruct` -> (`MyStruct`, `Namespace::Types`)
/// * `panic!` -> (`panic`, `Namespace::Macros`)
/// * `fn@from_intra_spec` -> (`from_intra_spec`, `Namespace::Values`)
pub fn parse_intra_doc_link(s: &str) -> (&str, Option<hir::Namespace>) {
    let s = s.trim_matches('`');

    [
        (hir::Namespace::Types, TYPES),
        (hir::Namespace::Values, VALUES),
        (hir::Namespace::Macros, MACROS),
    ]
    .into_iter()
    .find_map(|(ns, (prefixes, suffixes))| {
        if let Some(prefix) = prefixes.iter().find(|&&prefix| {
            s.starts_with(prefix)
                && s.chars().nth(prefix.len()).is_some_and(|c| c == '@' || c == ' ')
        }) {
            Some((&s[prefix.len() + 1..], ns))
        } else {
            suffixes.iter().find_map(|&suffix| s.strip_suffix(suffix).zip(Some(ns)))
        }
    })
    .map_or((s, None), |(s, ns)| (s, Some(ns)))
}

pub fn strip_intra_doc_link_disambiguators(s: &str) -> &str {
    [TYPES, VALUES, MACROS]
        .into_iter()
        .find_map(|(prefixes, suffixes)| {
            if let Some(prefix) = prefixes.iter().find(|&&prefix| {
                s.starts_with(prefix)
                    && s.chars().nth(prefix.len()).is_some_and(|c| c == '@' || c == ' ')
            }) {
                Some(&s[prefix.len() + 1..])
            } else {
                suffixes.iter().find_map(|&suffix| s.strip_suffix(suffix))
            }
        })
        .unwrap_or(s)
}

/// Extracts all intra-doc link occurrences from Markdown documentation.
pub fn extract_intra_doc_link_occurrences(
    docs: &Documentation<'_>,
) -> Vec<(TextRange, String, Option<hir::Namespace>)> {
    Parser::new_with_broken_link_callback(
        docs.as_str(),
        MARKDOWN_OPTIONS,
        Some(&mut broken_link_clone_cb),
    )
    .into_offset_iter()
    .filter_map(|(event, range)| match event {
        Event::Start(Tag::Link(_, target, _)) => {
            let (link, ns) = parse_intra_doc_link(&target);
            Some((
                TextRange::new(range.start.try_into().ok()?, range.end.try_into().ok()?),
                link.to_owned(),
                ns,
            ))
        }
        _ => None,
    })
    .collect()
}

fn extract_intra_doc_link_targets(
    docs: &Documentation<'_>,
) -> Vec<(TextRange, String, Option<hir::Namespace>)> {
    let mut broken_link_callback = broken_link_clone_cb;
    let parser = Parser::new_with_broken_link_callback(
        docs.as_str(),
        MARKDOWN_OPTIONS,
        Some(&mut broken_link_callback),
    );
    let mut targets = parser
        .reference_definitions()
        .iter()
        .filter_map(|(_, definition)| {
            let source = &docs.as_str()[definition.span.clone()];
            let dest_start = source.find("]:")? + 2;
            let dest = definition.dest.as_ref();
            let offset = dest_start + source[dest_start..].find(dest)?;
            let start = definition.span.start + offset;
            let range =
                TextRange::new(start.try_into().ok()?, (start + dest.len()).try_into().ok()?);
            let (link, ns) = parse_intra_doc_link(dest);
            Some((range, link.to_owned(), ns))
        })
        .collect::<Vec<_>>();

    targets.extend(parser.into_offset_iter().filter_map(|(event, range)| {
        let Event::Start(Tag::Link(link_type, target, _)) = event else { return None };
        let source = &docs.as_str()[range.clone()];
        let target = target.as_ref();
        let offset = match link_type {
            LinkType::Inline => {
                let dest_start = source.rfind("](")? + 2;
                dest_start + source[dest_start..].find(target)?
            }
            LinkType::ReferenceUnknown => source.rfind(target)?,
            LinkType::CollapsedUnknown | LinkType::ShortcutUnknown => source.find(target)?,
            _ => return None,
        };
        let start = range.start + offset;
        let range = TextRange::new(start.try_into().ok()?, (start + target.len()).try_into().ok()?);
        let (link, ns) = parse_intra_doc_link(target);
        Some((range, link.to_owned(), ns))
    }));
    targets
}

pub fn resolve_doc_path_for_def<'db>(
    db: &dyn HirDatabase,
    def: Definition<'db>,
    link: &str,
    ns: Option<hir::Namespace>,
    is_inner_doc: hir::IsInnerDoc,
) -> Option<Definition<'db>> {
    match def {
        Definition::Module(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Crate(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Function(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Adt(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::EnumVariant(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Const(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Static(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Trait(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::TypeAlias(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Macro(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::Field(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::SelfType(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::ExternCrateDecl(it) => it.resolve_doc_path(db, link, ns, is_inner_doc),
        Definition::BuiltinAttr(_)
        | Definition::BuiltinType(_)
        | Definition::BuiltinLifetime(_)
        | Definition::ToolModule(_)
        | Definition::TupleField(_)
        | Definition::Local(_)
        | Definition::GenericParam(_)
        | Definition::Label(_)
        | Definition::DeriveHelper(_)
        | Definition::InlineAsmRegOrRegClass(_)
        | Definition::InlineAsmOperand(_) => None,
    }
    .map(Definition::from)
}

pub fn doc_attributes<'db>(
    sema: &Semantics<'db, RootDatabase>,
    node: &SyntaxNode,
) -> Option<(AttrsWithOwner, Definition<'db>)> {
    match_ast! {
        match node {
            ast::SourceFile(it)  => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Module(it)      => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Fn(it)          => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Struct(it)      => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(hir::Adt::Struct(def)))),
            ast::Union(it)       => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(hir::Adt::Union(def)))),
            ast::Enum(it)        => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(hir::Adt::Enum(def)))),
            ast::Variant(it)     => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Trait(it)       => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Static(it)      => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Const(it)       => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::TypeAlias(it)   => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Impl(it)        => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::RecordField(it) => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::TupleField(it)  => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::Macro(it)       => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            ast::ExternCrate(it) => sema.to_def(&it).map(|def| (def.attrs(sema.db), Definition::from(def))),
            _ => None
        }
    }
}

pub(crate) fn intra_doc_links<'db>(
    sema: &Semantics<'db, RootDatabase>,
    file_id: EditionedFileId,
    node: &SyntaxNode,
    name: &str,
) -> Vec<(TextRange, Definition<'db>)> {
    // FIXME: Decide how edits to macro-expanded documentation should map back to source before
    // descending into macro expansions here
    let Some((attributes, owner)) = doc_attributes(sema, node) else { return Vec::new() };
    let Some(docs) = attributes.hir_docs(sema.db) else { return Vec::new() };
    if !docs.docs().contains(name) {
        return Vec::new();
    }

    let mut res = Vec::new();

    for (dest_range, link, ns) in
        extract_intra_doc_link_targets(&Documentation::new_borrowed(docs.docs()))
    {
        let path = link.split_once('#').map_or(link.as_str(), |(path, _)| path);
        if path.is_empty() {
            continue;
        }
        let dest = &docs.docs()[dest_range];
        let dest_path = dest.split_once('#').map_or(dest, |(path, _)| path);
        let Some(path_offset) = dest_path.rfind(path) else { continue };
        let Ok(path_offset) = TextSize::try_from(path_offset) else { continue };
        let path_offset = dest_range.start() + path_offset;

        let mut segment_start = 0;
        for segment in path.split("::") {
            let segment_end = segment_start + segment.len();
            if segment.trim_start_matches("r#") != name {
                segment_start = segment_end + 2;
                continue;
            }
            let (Ok(range_start), Ok(range_end)) =
                (TextSize::try_from(segment_start), TextSize::try_from(segment_end))
            else {
                break;
            };
            let range = TextRange::new(path_offset + range_start, path_offset + range_end);
            let Some((mapped, is_inner)) = docs.find_ast_range(range) else {
                segment_start = segment_end + 2;
                continue;
            };
            if mapped.file_id == file_id {
                let prefix = &path[..segment_end];
                let prefix_ns = (segment_end == path.len()).then_some(ns).flatten();
                if let Some(def) =
                    resolve_doc_path_for_def(sema.db, owner, prefix, prefix_ns, is_inner)
                {
                    res.push((mapped.value, def));
                }
            }
            segment_start = segment_end + 2;
        }
    }

    res
}

fn broken_link_clone_cb(link: BrokenLink<'_>) -> Option<(CowStr<'_>, CowStr<'_>)> {
    Some((link.reference.clone(), link.reference))
}

#[cfg(test)]
mod tests {
    use expect_test::{Expect, expect};

    use super::*;

    fn check(link: &str, expected: Expect) {
        let (link, namespace) = parse_intra_doc_link(link);
        let namespace = namespace.map_or_else(String::new, |it| format!(" ({it:?})"));
        expected.assert_eq(&format!("{link}{namespace}"));
    }

    #[test]
    fn parses_disambiguators() {
        check("foo", expect![[r#"foo"#]]);
        check("struct Struct", expect![[r#"Struct (Types)"#]]);
        check("makro!", expect![[r#"makro (Macros)"#]]);
        check("fn@function", expect![[r#"function (Values)"#]]);
    }
}
