//! This module provides `StaticIndex` which is used for powering
//! read-only code browsers and emitting LSIF

use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
};

use arrayvec::ArrayVec;
use dashmap::{
    DashMap,
    mapref::one::{Ref, RefMut},
};
use either::Either;
use hir::{Crate, InFile, Module, Semantics, db::HirDatabase};
use ide_db::{
    FileId, FileRange, FxHashMap, RootDatabase,
    base_db::{SourceDatabase, VfsPath},
    defs::{Definition, IdentClass},
    documentation::Documentation,
    famous_defs::FamousDefs,
    ra_fixture::RaFixtureConfig,
};
use syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, ast::MacroCall,
};

use crate::navigation_target::UpmappingResult;
use crate::{
    Analysis, Fold, HoverConfig, HoverResult, TryToNav,
    hover::{SubstTyLen, hover_for_definition},
    moniker::{MonikerResult, SymbolInformationKind, def_to_kind, def_to_moniker},
    parent_module::crates_for,
};

/// A static representation of fully analyzed source code.
///
/// The intended use-case is powering read-only code browsers and emitting LSIF/SCIP.
#[derive(Debug)]
pub struct StaticIndex {
    pub files: Vec<StaticIndexedFile>,
    pub tokens: TokenStore,
}

#[derive(Debug)]
pub struct ReferenceData {
    pub range: FileRange,
    pub is_definition: bool,
}

#[derive(Debug)]
pub struct TokenStaticData {
    // FIXME: Make this have the lifetime of the database.
    pub documentation: Option<Documentation<'static>>,
    pub hover: Option<HoverResult>,
    /// The position of the token itself.
    ///
    /// For example, in `fn foo() {}` this is the position of `foo`.
    pub definition: Option<FileRange>,
    /// The position of the entire definition that this token belongs to.
    ///
    /// For example, in `fn foo() {}` this is the position from `fn`
    /// to the closing brace.
    ///
    /// This excludes trivia (whitespace/comments) other than doc
    /// comments. This differs from LSP, which includes trivia.
    ///
    /// SCIP:
    ///
    /// > source range of the nearest non-trivial enclosing AST node.
    ///
    /// <https://github.com/scip-code/scip/blob/20459645420419b3c2a10d6a9f57436abeeb273b/scip.proto#L747-L796>
    ///
    /// LSP:
    ///
    /// > range enclosing this symbol not including leading/trailing
    /// > whitespace but everything else like comments.
    ///
    /// <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#locationLink>
    pub definition_body: Option<FileRange>,
    pub references: Vec<ReferenceData>,
    pub moniker: Option<MonikerResult>,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub kind: SymbolInformationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenId(usize);

impl TokenId {
    pub fn raw(self) -> usize {
        self.0
    }
}

#[derive(Default, Debug)]
pub struct TokenStore {
    data: DashMap<TokenId, TokenStaticData>,
    last_index: AtomicUsize,
}

impl TokenStore {
    pub fn insert(&self, data: TokenStaticData) -> TokenId {
        let id = TokenId(self.last_index.fetch_add(1, Ordering::Relaxed));
        self.data.insert(id, data);
        id
    }

    pub fn get_mut(&self, id: TokenId) -> Option<RefMut<'_, TokenId, TokenStaticData>> {
        self.data.get_mut(&id)
    }

    pub fn get(&self, id: TokenId) -> Option<Ref<'_, TokenId, TokenStaticData>> {
        self.data.get(&id)
    }

    pub fn iter(self) -> impl Iterator<Item = (TokenId, TokenStaticData)> {
        self.data.into_iter()
    }
}

#[derive(Debug)]
pub struct StaticIndexedFile {
    pub file_id: FileId,
    pub folds: Vec<Fold>,
    pub tokens: Vec<(TextRange, TokenId)>,
}

fn all_modules(db: &dyn HirDatabase) -> Vec<Module> {
    let mut worklist: Vec<_> =
        Crate::all(db).into_iter().map(|krate| krate.root_module(db)).collect();
    let mut modules = Vec::new();

    while let Some(module) = worklist.pop() {
        modules.push(module);
        worklist.extend(module.children(db));
    }

    modules
}

fn documentation_for_definition(
    sema: &Semantics<'_, RootDatabase>,
    def: Definition<'_>,
    scope_node: &SyntaxNode,
) -> Option<Documentation<'static>> {
    let famous_defs = match &def {
        Definition::BuiltinType(_) => Some(FamousDefs(sema, sema.scope(scope_node)?.krate())),
        _ => None,
    };

    def.docs(sema.db, famous_defs.as_ref(), def.krate(sema.db)?.to_display_target(sema.db))
        .map(Documentation::into_owned)
}

// FIXME: This is a weird function
fn get_definitions<'db>(
    sema: &Semantics<'db, RootDatabase>,
    token: SyntaxToken,
) -> Option<ArrayVec<(Definition<'db>, Option<hir::GenericSubstitution<'db>>), 2>> {
    for token in sema.descend_into_macros_exact(token) {
        let def = IdentClass::classify_token(sema, &token).map(IdentClass::definitions);
        if let Some(defs) = def
            && !defs.is_empty()
        {
            return Some(defs);
        }
    }
    None
}

pub enum VendoredLibrariesConfig<'a> {
    Included { workspace_root: &'a VfsPath },
    Excluded,
}

/// Computes the index for a single file identified by file_id.
/// Note this doesn't return a single StaticIndexedFile but multiple because this also indexes include! expansions.
fn index_file<'a>(
    analysis: &'a Analysis,
    token_store: &TokenStore,
    def_map: &DashMap<Definition<'a>, TokenId>,
    include_hovers: bool,
    include_folds: bool,
    file_id: FileId,
) -> FxHashMap<FileId, StaticIndexedFile> {
    let db = &analysis.db;

    let current_crate = crates_for(db, file_id).pop().map(Into::into);
    let folds =
        if include_folds { analysis.folding_ranges(file_id, true).unwrap() } else { vec![] };
    // hovers
    let sema = hir::Semantics::new(db);
    let root = sema.parse_guess_edition(file_id).syntax().clone();
    let edition = sema.attach_first_edition(file_id).edition(sema.db);
    let Some(krate) = sema.first_crate(file_id) else {
        return Default::default();
    };
    let display_target = krate.to_display_target(sema.db);
    let hover_config = if include_hovers {
        Some(HoverConfig {
            links_in_hover: true,
            memory_layout: None,
            documentation: true,
            keywords: true,
            format: crate::HoverDocFormat::Markdown,
            max_trait_assoc_items_count: None,
            max_fields_count: Some(5),
            max_enum_variants_count: Some(5),
            max_subst_ty_len: SubstTyLen::Unlimited,
            show_drop_glue: true,
            ra_fixture: RaFixtureConfig::default(),
        })
    } else {
        None
    };
    let mut result: FxHashMap<FileId, StaticIndexedFile> = Default::default();
    result.insert(file_id, StaticIndexedFile { file_id, folds, tokens: vec![] });

    let mut add_token =
        |def: Definition<'a>, file_id: FileId, range: TextRange, scope_node: &SyntaxNode| {
            let id = *def_map.entry(def).or_insert_with(|| {
                let nav = def.try_to_nav(&sema).map(UpmappingResult::call_site);
                token_store.insert(TokenStaticData {
                    documentation: documentation_for_definition(&sema, def, scope_node),
                    hover: hover_config.as_ref().map(|hover_config| {
                        hover_for_definition(
                            &sema,
                            file_id,
                            def,
                            None,
                            scope_node,
                            None,
                            false,
                            hover_config,
                            edition,
                            display_target,
                        )
                    }),
                    definition: nav.as_ref().map(|it| FileRange {
                        file_id: it.file_id,
                        range: it.focus_or_full_range(),
                    }),
                    definition_body: nav.as_ref().map(|it| FileRange {
                        file_id: it.file_id,
                        range: definition_range_excluding_trivia(&sema, it.file_id, it.full_range),
                    }),
                    references: vec![],
                    moniker: current_crate.and_then(|cc| def_to_moniker(db, def, cc)),
                    display_name: def.name(db).map(|name| name.display(db, edition).to_string()),
                    signature: Some(def.label(db, display_target)),
                    kind: def_to_kind(db, def),
                })
            });
            let mut token = token_store.get_mut(id).unwrap();
            token.references.push(ReferenceData {
                range: FileRange { range, file_id },
                is_definition: match def.try_to_nav(&sema).map(UpmappingResult::call_site) {
                    Some(it) => it.file_id == file_id && it.focus_or_full_range() == range,
                    None => false,
                },
            });
            result
                .entry(file_id)
                .or_insert_with(|| StaticIndexedFile { file_id, folds: vec![], tokens: vec![] })
                .tokens
                .push((range, id));
        };

    if let Some(module) = sema.file_to_module_def(file_id) {
        let def = Definition::Module(module);
        let range = root.text_range();
        add_token(def, file_id, range, &root);
    }

    // The loop below will traverse all macro expansions, but only record tokens from the current file and include! calls.
    #[derive(PartialEq, Eq, Clone, Copy)]
    enum RootKind {
        OnlyTraverse,
        RecordTokens,
    }

    let mut roots = VecDeque::from([(root, RootKind::RecordTokens)]);
    while let Some((root, root_kind)) = roots.pop_front() {
        for node_or_token in root.descendants_with_tokens() {
            match node_or_token {
                NodeOrToken::Token(token) => {
                    if root_kind != RootKind::RecordTokens {
                        continue;
                    }

                    let Some(node) = token.parent() else {
                        continue;
                    };
                    let Some(defs) = hir::attach_db(db, || get_definitions(&sema, token.clone()))
                    else {
                        continue;
                    };

                    let file_range = InFile::new(sema.hir_file_for(&node), token.text_range())
                        .original_node_file_range_rooted(db);

                    for (def, _) in defs {
                        let range = file_range.range;
                        let file_id = file_range.file_id.file_id(db);
                        add_token(def, file_id, range, &node);
                    }
                }

                NodeOrToken::Node(node) => {
                    let Some(macro_call) = MacroCall::cast(node) else {
                        continue;
                    };
                    let Some(macro_call) = sema.to_def(&macro_call) else {
                        continue;
                    };

                    let expansion = sema.parse_or_expand(macro_call.into());
                    let root_kind = if macro_call.is_include_macro(db) {
                        RootKind::RecordTokens
                    } else {
                        RootKind::OnlyTraverse
                    };
                    roots.push_front((expansion, root_kind));
                }
            }
        }
    }

    result
}

impl StaticIndex {
    pub fn compute(
        analysis: &Analysis,
        vendored_libs_config: VendoredLibrariesConfig<'_>,
        include_hovers: bool,
        include_folds: bool,
        num_threads: usize,
    ) -> Self {
        let db = &analysis.db;

        let files_to_index = {
            let mut files_to_index: Vec<_> = hir::attach_db(db, || {
                let modules = all_modules(db).into_iter();

                let modules_to_index = modules.filter(|module| {
                    let file_id = module.definition_source_file_id(db).original_file(db);
                    let source_root = db.file_source_root(file_id.file_id(db)).source_root_id(db);
                    let source_root = db.source_root(source_root).source_root(db);
                    let is_vendored = match vendored_libs_config {
                        VendoredLibrariesConfig::Included { workspace_root } => source_root
                            .path_for_file(&file_id.file_id(db))
                            .is_some_and(|module_path| module_path.starts_with(workspace_root)),
                        VendoredLibrariesConfig::Excluded => false,
                    };

                    !source_root.is_library || is_vendored
                });

                let files_to_index = modules_to_index.map(|module| {
                    module.definition_source_file_id(db).original_file(db).file_id(db)
                });

                files_to_index.collect()
            });
            files_to_index.sort();
            files_to_index.dedup();
            files_to_index
        };

        // Because Analysis is not Sync, each thread needs to get exclusive access to its own Analysis clone.
        // The Definitions which are used as keys in def_map inherit the lifetime of these Analysis clones, so analyses should live longer than def_map.
        // NOTE this assumes that Definitions yielded from multiple untouched Analysis clones still compare equal.
        let mut analyses: Vec<_> = (0..num_threads).map(|_| analysis.clone()).collect();

        let def_map = Default::default();
        let tokens = Default::default();
        let files = std::thread::scope(|s| {
            let threads: Vec<_> = analyses
                .iter_mut()
                .enumerate()
                .map(|(thread_index, analysis)| {
                    let files_to_index = files_to_index
                        .iter()
                        .copied()
                        .enumerate()
                        .filter(move |(index, _)| index % num_threads == thread_index)
                        .map(|(_, file_id)| file_id);

                    s.spawn(|| {
                        let analysis = analysis;
                        hir::attach_db(&analysis.db, || {
                            files_to_index
                                .into_iter()
                                .flat_map(|file_id| {
                                    index_file(
                                        analysis,
                                        &tokens,
                                        &def_map,
                                        include_hovers,
                                        include_folds,
                                        file_id,
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                })
                .collect();

            let files_with_duplicates =
                threads.into_iter().flat_map(|join_handle| join_handle.join().unwrap());

            let mut result = FxHashMap::default();
            for (file_id, mut file) in files_with_duplicates {
                result
                    .entry(file_id)
                    .and_modify(|existing_file: &mut StaticIndexedFile| {
                        existing_file.tokens.extend(std::mem::take(&mut file.tokens))
                    })
                    .or_insert(file);
            }

            result.into_values().collect()
        });

        StaticIndex { files, tokens }
    }
}

fn definition_range_excluding_trivia(
    sema: &Semantics<'_, RootDatabase>,
    file_id: FileId,
    range: TextRange,
) -> TextRange {
    let root = sema.parse_guess_edition(file_id).syntax().clone();
    if range == root.text_range() {
        return range;
    }
    if !root.text_range().contains_range(range) {
        return range;
    }

    let element = root.covering_element(range);
    let tokens = match element {
        NodeOrToken::Node(node) => Either::Left(node.descendants_with_tokens().filter_map(|it| {
            let token = it.into_token()?;
            range.contains_range(token.text_range()).then_some(token)
        })),
        NodeOrToken::Token(token) => Either::Right(std::iter::once(token)),
    };

    let mut first = None;
    let mut last = None;
    for token in tokens {
        if first.is_none() && !is_leading_trivia_excluding_docs(&token) {
            first = Some(token.clone());
        }
        if !is_trailing_trivia(&token) {
            last = Some(token);
        }
    }

    match (first, last) {
        (Some(first), Some(last)) => {
            TextRange::new(first.text_range().start(), last.text_range().end())
        }
        _ => range,
    }
}

fn is_leading_trivia_excluding_docs(token: &SyntaxToken) -> bool {
    matches!(
        token.kind(),
        SyntaxKind::WHITESPACE | SyntaxKind::COMMENT | SyntaxKind::INNER_DOC_COMMENT
    )
}

fn is_trailing_trivia(token: &SyntaxToken) -> bool {
    matches!(
        token.kind(),
        SyntaxKind::WHITESPACE
            | SyntaxKind::COMMENT
            | SyntaxKind::INNER_DOC_COMMENT
            | SyntaxKind::OUTER_DOC_COMMENT
    )
}

#[cfg(test)]
mod tests {
    use crate::{StaticIndex, fixture};
    use ide_db::{FileRange, FxHashMap, FxHashSet, base_db::VfsPath};
    use syntax::TextSize;

    use super::VendoredLibrariesConfig;

    fn check_all_ranges(
        #[rust_analyzer::rust_fixture] ra_fixture: &str,
        vendored_libs_config: VendoredLibrariesConfig<'_>,
    ) {
        let (analysis, ranges) = fixture::annotations_without_marker(ra_fixture);
        let s = StaticIndex::compute(&analysis, vendored_libs_config, true, true, 1);
        let mut range_set: FxHashSet<_> = ranges.iter().map(|it| it.0).collect();
        for f in s.files {
            for (range, _) in f.tokens {
                if range.start() == TextSize::from(0) {
                    // ignore whole file range corresponding to module definition
                    continue;
                }
                let it = FileRange { file_id: f.file_id, range };
                if !range_set.contains(&it) {
                    panic!("additional range {it:?}");
                }
                range_set.remove(&it);
            }
        }
        if !range_set.is_empty() {
            panic!("unfound ranges {range_set:?}");
        }
    }

    #[track_caller]
    fn check_definitions(
        #[rust_analyzer::rust_fixture] ra_fixture: &str,
        vendored_libs_config: VendoredLibrariesConfig<'_>,
    ) {
        let (analysis, ranges) = fixture::annotations_without_marker(ra_fixture);
        let s = StaticIndex::compute(&analysis, vendored_libs_config, true, true, 1);
        let mut range_set: FxHashSet<_> = ranges.iter().map(|it| it.0).collect();
        for (_, t) in s.tokens.iter() {
            if let Some(t) = t.definition {
                if t.range.start() == TextSize::from(0) {
                    // ignore definitions that are whole of file
                    continue;
                }
                if !range_set.contains(&t) {
                    panic!("additional definition {t:?}");
                }
                range_set.remove(&t);
            }
        }
        if !range_set.is_empty() {
            panic!("unfound definitions {range_set:?}");
        }
    }

    #[track_caller]
    fn check_references(
        #[rust_analyzer::rust_fixture] ra_fixture: &str,
        vendored_libs_config: VendoredLibrariesConfig<'_>,
    ) {
        let (analysis, ranges) = fixture::annotations_without_marker(ra_fixture);
        let s = StaticIndex::compute(&analysis, vendored_libs_config, true, true, 1);
        let mut range_set: FxHashMap<_, i32> = ranges.iter().map(|it| (it.0, 0)).collect();

        // Make sure that all references have at least one range. We use a HashMap instead of a
        // a HashSet so that we can have more than one reference at the same range.
        for (_, t) in s.tokens.iter() {
            for r in &t.references {
                if r.is_definition {
                    continue;
                }
                if r.range.range.start() == TextSize::from(0) {
                    // ignore whole file range corresponding to module definition
                    continue;
                }
                match range_set.entry(r.range) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        let count = entry.get_mut();
                        *count += 1;
                    }
                    std::collections::hash_map::Entry::Vacant(_) => {
                        panic!("additional reference {r:?}");
                    }
                }
            }
        }
        for (range, count) in range_set.iter() {
            if *count == 0 {
                panic!("unfound reference {range:?}");
            }
        }
    }

    #[test]
    fn field_initialization() {
        check_references(
            r#"
struct Point {
    x: f64,
     //^^^
    y: f64,
     //^^^
}
    fn foo() {
        let x = 5.;
        let y = 10.;
        let mut p = Point { x, y };
                  //^^^^^   ^  ^
        p.x = 9.;
      //^ ^
        p.y = 10.;
      //^ ^
    }
"#,
            VendoredLibrariesConfig::Included {
                workspace_root: &VfsPath::new_virtual_path("/workspace".to_owned()),
            },
        );
    }

    #[test]
    fn struct_and_enum() {
        check_all_ranges(
            r#"
struct Foo;
     //^^^
enum E { X(Foo) }
   //^   ^ ^^^
"#,
            VendoredLibrariesConfig::Included {
                workspace_root: &VfsPath::new_virtual_path("/workspace".to_owned()),
            },
        );
        check_definitions(
            r#"
struct Foo;
     //^^^
enum E { X(Foo) }
   //^   ^
"#,
            VendoredLibrariesConfig::Included {
                workspace_root: &VfsPath::new_virtual_path("/workspace".to_owned()),
            },
        );

        check_references(
            r#"
struct Foo;
enum E { X(Foo) }
   //      ^^^
"#,
            VendoredLibrariesConfig::Included {
                workspace_root: &VfsPath::new_virtual_path("/workspace".to_owned()),
            },
        );
    }

    #[test]
    fn multi_crate() {
        check_definitions(
            r#"
//- /workspace/main.rs crate:main deps:foo


use foo::func;

fn main() {
 //^^^^
    func();
}
//- /workspace/foo/lib.rs crate:foo

pub func() {

}
"#,
            VendoredLibrariesConfig::Included {
                workspace_root: &VfsPath::new_virtual_path("/workspace".to_owned()),
            },
        );
    }

    #[test]
    fn vendored_crate() {
        check_all_ranges(
            r#"
//- /workspace/main.rs crate:main deps:external,vendored
struct Main(i32);
     //^^^^ ^^^

//- /external/lib.rs new_source_root:library crate:external@0.1.0,https://a.b/foo.git library
struct ExternalLibrary(i32);

//- /workspace/vendored/lib.rs new_source_root:library crate:vendored@0.1.0,https://a.b/bar.git library
struct VendoredLibrary(i32);
     //^^^^^^^^^^^^^^^ ^^^
"#,
            VendoredLibrariesConfig::Included {
                workspace_root: &VfsPath::new_virtual_path("/workspace".to_owned()),
            },
        );
    }

    #[test]
    fn vendored_crate_excluded() {
        check_all_ranges(
            r#"
//- /workspace/main.rs crate:main deps:external,vendored
struct Main(i32);
     //^^^^ ^^^

//- /external/lib.rs new_source_root:library crate:external@0.1.0,https://a.b/foo.git library
struct ExternalLibrary(i32);

//- /workspace/vendored/lib.rs new_source_root:library crate:vendored@0.1.0,https://a.b/bar.git library
struct VendoredLibrary(i32);
"#,
            VendoredLibrariesConfig::Excluded,
        )
    }

    #[test]
    fn derives() {
        check_all_ranges(
            r#"
//- minicore:derive
#[rustc_builtin_macro]
//^^^^^^^^^^^^^^^^^^^
pub macro Copy {}
        //^^^^
#[derive(Copy)]
//^^^^^^ ^^^^
struct Hello(i32);
     //^^^^^ ^^^
"#,
            VendoredLibrariesConfig::Included {
                workspace_root: &VfsPath::new_virtual_path("/workspace".to_owned()),
            },
        );
    }
}
