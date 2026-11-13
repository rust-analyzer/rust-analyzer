//! Completion support for Rust doctest code blocks embedded in doc comments.

use base_db::{
    CrateGraphBuilder, DependencyBuilder, LibraryRoots, LocalRoots, SourceDatabase, SourceRoot,
    all_crates,
};
use hir::{ChangeWithProcMacros, HasAttrs, InFile, Semantics, db::DefDatabase};
use ide_db::{
    FilePosition, FxHashMap, RootDatabase,
    defs::Definition,
    range_mapper::RangeMapper,
    rust_doc::rust_fenced_code_lines,
    text_edit::{TextEdit, TextEditBuilder},
};
use syntax::{
    AstNode, AstToken,
    SyntaxKind::{ASSOC_ITEM_LIST, ITEM_LIST, SOURCE_FILE},
    SyntaxNode, SyntaxToken, TextRange, TextSize, ast, match_ast,
};

use crate::{CompletionConfig, CompletionItem};

const DOCTEST_WRAPPER_NAME: &str = "__ra_doctest_completion";
const DOCTEST_WRAPPER_PREFIX: &str = "\n#[allow(dead_code)]\nfn __ra_doctest_completion() {\n";
const DOCTEST_WRAPPER_SUFFIX: &str = "\n}\n";

pub(crate) fn complete_doctest(
    db: &RootDatabase,
    config: &CompletionConfig<'_>,
    position: FilePosition,
    trigger_character: Option<char>,
) -> Option<Vec<CompletionItem>> {
    let sema = Semantics::new(db);
    let editioned_file_id = sema.attach_first_edition(position.file_id);
    let file = sema.parse(editioned_file_id).syntax().clone();
    let token = file.token_at_offset(position.offset).left_biased()?;

    let analysis = hir::attach_db_allow_change(db, || {
        DoctestCompletionAnalysis::new(&sema, position.file_id, position.offset, token)
    })?;
    let doctest_offset = analysis.map_offset_down(position.offset)?;

    let completions = hir::attach_db_allow_change(&analysis.db, || {
        crate::completions(
            &analysis.db,
            config,
            FilePosition { file_id: position.file_id, offset: doctest_offset },
            trigger_character,
        )
    })?;

    completions
        .into_iter()
        .filter(|item| {
            !item.lookup().starts_with(DOCTEST_WRAPPER_NAME)
                && !item.label.primary.as_str().starts_with(DOCTEST_WRAPPER_NAME)
        })
        .map(|item| analysis.upmap_completion_item(item))
        .collect()
}

struct DoctestCompletionAnalysis {
    db: RootDatabase,
    down_mapper: RangeMapper,
    up_mapper: RangeMapper,
}

impl DoctestCompletionAnalysis {
    fn new(
        sema: &Semantics<'_, RootDatabase>,
        file_id: ide_db::FileId,
        original_offset: TextSize,
        doc_token: SyntaxToken,
    ) -> Option<Self> {
        let owner = doc_comment_owner(sema, &doc_token)?;
        let (attrs, _) = doc_attributes(sema, &owner)?;
        let docs = attrs.hir_docs(sema.db)?;
        let insert_offset = insert_offset(&owner)?;
        let file_id_hir: hir::HirFileId = sema.attach_first_edition(file_id).into();

        let mut up_mapper = RangeMapper::default();
        let mut down_mapper = RangeMapper::default();

        let original_text = sema.db.file_text(file_id).text(sema.db);
        let prefix_len: usize = insert_offset.into();
        add_original_segment(
            &mut up_mapper,
            &mut down_mapper,
            &original_text[..prefix_len],
            TextRange::up_to(insert_offset),
        );
        add_unmapped(&mut up_mapper, &mut down_mapper, DOCTEST_WRAPPER_PREFIX);

        let mut has_doctests = false;
        let mut contains_cursor = false;

        for line in rust_fenced_code_lines(docs.docs()) {
            let Some((InFile { file_id: mapped_file_id, value: mapped_range }, _)) =
                docs.find_ast_range(line.range)
            else {
                continue;
            };
            if mapped_file_id != file_id_hir {
                continue;
            }

            has_doctests = true;
            contains_cursor |= mapped_range.contains_inclusive(original_offset);
            add_mapped(&mut up_mapper, &mut down_mapper, line.text, mapped_range);
            add_unmapped(&mut up_mapper, &mut down_mapper, "\n");
        }

        if !has_doctests || !contains_cursor {
            return None;
        }

        add_unmapped(&mut up_mapper, &mut down_mapper, DOCTEST_WRAPPER_SUFFIX);
        let original_suffix = &original_text[prefix_len..];
        add_original_segment(
            &mut up_mapper,
            &mut down_mapper,
            original_suffix,
            TextRange::new(insert_offset, TextSize::of(original_text.as_ref())),
        );

        let new_text = up_mapper.take_text();
        let _ = down_mapper.take_text();

        let db = build_doctest_db(sema.db, file_id, new_text);

        Some(Self { db, down_mapper, up_mapper })
    }

    fn map_offset_down(&self, offset: TextSize) -> Option<TextSize> {
        self.down_mapper.map_offset_down(offset).or_else(|| {
            let prev = offset.checked_sub(TextSize::new(1))?;
            let mapped_prev = self.down_mapper.map_offset_down(prev)?;
            Some(mapped_prev + TextSize::new(1))
        })
    }

    fn map_range_up(&self, range: TextRange) -> Option<TextRange> {
        self.up_mapper.map_range_up(range).next()
    }

    fn map_offset_up(&self, offset: TextSize) -> Option<TextSize> {
        self.map_range_up(TextRange::empty(offset)).map(|range| range.start())
    }

    fn upmap_completion_item(&self, mut item: CompletionItem) -> Option<CompletionItem> {
        item.source_range = self.map_range_up(item.source_range)?;
        item.text_edit = self.upmap_text_edit(item.text_edit)?;
        if let Some((ref_mode, offset)) = item.ref_match {
            item.ref_match = Some((ref_mode, self.map_offset_up(offset)?));
        }
        Some(item)
    }

    fn upmap_text_edit(&self, edit: TextEdit) -> Option<TextEdit> {
        let mut builder = TextEditBuilder::default();
        for indel in edit {
            builder.replace(self.map_range_up(indel.delete)?, indel.insert);
        }
        Some(builder.finish())
    }
}

fn add_original_segment(
    up_mapper: &mut RangeMapper,
    down_mapper: &mut RangeMapper,
    text: &str,
    source_range: TextRange,
) {
    up_mapper.add(text, source_range);
    down_mapper.add_unmapped(text);
}

fn add_mapped(
    up_mapper: &mut RangeMapper,
    down_mapper: &mut RangeMapper,
    text: &str,
    source_range: TextRange,
) {
    up_mapper.add(text, source_range);
    down_mapper.add(text, source_range);
}

fn add_unmapped(up_mapper: &mut RangeMapper, down_mapper: &mut RangeMapper, text: &str) {
    up_mapper.add_unmapped(text);
    down_mapper.add_unmapped(text);
}

fn insert_offset(owner: &SyntaxNode) -> Option<TextSize> {
    if let Some(source_file) = ast::SourceFile::cast(owner.clone()) {
        return Some(source_file.syntax().text_range().end());
    }
    if let Some(module) = ast::Module::cast(owner.clone())
        && let Some(item_list) = module.item_list()
    {
        return Some(item_list.syntax().text_range().end() - TextSize::of("}"));
    }
    if let Some(impl_) = ast::Impl::cast(owner.clone())
        && let Some(item_list) = impl_.assoc_item_list()
    {
        return Some(item_list.syntax().text_range().end() - TextSize::of("}"));
    }
    if let Some(trait_) = ast::Trait::cast(owner.clone())
        && let Some(item_list) = trait_.assoc_item_list()
    {
        return Some(item_list.syntax().text_range().end() - TextSize::of("}"));
    }

    matches!(owner.parent()?.kind(), ITEM_LIST | ASSOC_ITEM_LIST | SOURCE_FILE)
        .then(|| owner.text_range().end())
}

fn doc_comment_owner(
    sema: &Semantics<'_, RootDatabase>,
    doc_token: &SyntaxToken,
) -> Option<SyntaxNode> {
    let (node, is_inner) = match_ast! {
        match doc_token {
            ast::Comment(comment) => {
                if !comment.is_doc() {
                    return None;
                }
                (doc_token.parent()?, comment.is_inner())
            },
            ast::String(string) => {
                let attr = doc_token
                    .parent_ancestors()
                    .find_map(ast::Attr::cast)
                    .filter(|attr| attr.simple_name().as_deref() == Some("doc"))?;
                if doc_token
                    .parent_ancestors()
                    .find_map(ast::MacroCall::cast)
                    .filter(|mac| {
                        mac.path()
                            .and_then(|path| path.segment()?.name_ref())
                            .is_some_and(|name_ref| name_ref.text() == "include_str")
                    })
                    .is_some()
                {
                    return None;
                }
                let is_inner = attr
                    .excl_token()
                    .is_some_and(|excl_token| excl_token.kind() == syntax::SyntaxKind::BANG);
                let _ = string;
                (attr.syntax().parent()?, is_inner)
            },
            _ => return None,
        }
    };

    if is_inner && node.kind() != SOURCE_FILE {
        let parent = node.parent()?;
        if doc_attributes(sema, &parent).is_some() {
            Some(parent)
        } else {
            let grandparent = parent.parent()?;
            doc_attributes(sema, &grandparent).map(|_| grandparent)
        }
    } else {
        doc_attributes(sema, &node).map(|_| node)
    }
}

fn doc_attributes(
    sema: &Semantics<'_, RootDatabase>,
    node: &SyntaxNode,
) -> Option<(hir::AttrsWithOwner, Definition)> {
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

fn build_doctest_db(
    db: &RootDatabase,
    changed_file_id: ide_db::FileId,
    changed_file_text: String,
) -> RootDatabase {
    let mut doctest_db = RootDatabase::new(None);
    if db.expand_proc_attr_macros() {
        doctest_db.enable_proc_attr_macros();
    }

    let roots = source_roots(db);
    let mut change = ChangeWithProcMacros::default();
    change.set_roots(roots.clone());
    for root in &roots {
        for file_id in root.iter() {
            let text = if file_id == changed_file_id {
                changed_file_text.clone()
            } else {
                db.file_text(file_id).text(db).to_string()
            };
            change.change_file(file_id, Some(text));
        }
    }
    change.set_crate_graph(copy_crate_graph(db));
    doctest_db.apply_change(change);
    doctest_db
}

fn source_roots(db: &RootDatabase) -> Vec<SourceRoot> {
    let mut root_ids = LocalRoots::get(db)
        .roots(db)
        .iter()
        .chain(LibraryRoots::get(db).roots(db).iter())
        .copied()
        .collect::<Vec<_>>();
    root_ids.sort_by_key(|root_id| root_id.0);
    root_ids
        .into_iter()
        .map(|root_id| db.source_root(root_id).source_root(db).as_ref().clone())
        .collect()
}

fn copy_crate_graph(db: &RootDatabase) -> CrateGraphBuilder {
    let mut crate_graph = CrateGraphBuilder::default();
    let mut crate_map = FxHashMap::default();

    for &krate in all_crates(db).iter() {
        let data = krate.data(db);
        let extra = krate.extra_data(db);
        let crate_attrs = data
            .crate_attrs
            .iter()
            .filter_map(|attr| {
                attr.strip_prefix("#![")
                    .and_then(|attr| attr.strip_suffix(']'))
                    .map(ToOwned::to_owned)
            })
            .collect();
        let crate_builder_id = crate_graph.add_crate_root(
            data.root_file_id,
            data.edition,
            extra.display_name.clone(),
            extra.version.clone(),
            krate.cfg_options(db).clone(),
            extra.potential_cfg_options.clone(),
            krate.env(db).clone(),
            data.origin.clone(),
            crate_attrs,
            data.is_proc_macro,
            data.proc_macro_cwd.clone(),
            krate.workspace_data(db).clone(),
        );
        crate_map.insert(krate, crate_builder_id);
    }

    for &krate in all_crates(db).iter() {
        for dep in &krate.data(db).dependencies {
            let _ = crate_graph.add_dep(
                crate_map[&krate],
                DependencyBuilder::with_prelude(
                    dep.name.clone(),
                    crate_map[&dep.crate_id],
                    dep.is_prelude(),
                    dep.is_sysroot(),
                ),
            );
        }
    }

    crate_graph
}
