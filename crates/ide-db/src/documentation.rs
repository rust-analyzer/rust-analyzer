//! Documentation attribute related utilities.
use std::borrow::Cow;

use hir::{HasAttrs, db::HirDatabase, resolve_doc_path_on};

/// Holds documentation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Documentation<'db>(Cow<'db, str>);

impl<'db> Documentation<'db> {
    #[inline]
    pub fn new_owned(s: String) -> Self {
        Documentation(Cow::Owned(s))
    }

    #[inline]
    pub fn new_borrowed(s: &'db str) -> Self {
        Documentation(Cow::Borrowed(s))
    }

    #[inline]
    pub fn into_owned(self) -> Documentation<'static> {
        Documentation::new_owned(self.0.into_owned())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub trait HasDocs: HasAttrs + Copy {
    fn docs(self, db: &dyn HirDatabase) -> Option<Documentation<'_>> {
        let docs = match self.docs_with_rangemap(db)? {
            Cow::Borrowed(docs) => Documentation::new_borrowed(docs.docs()),
            Cow::Owned(docs) => Documentation::new_owned(docs.into_docs()),
        };
        Some(docs)
    }
    fn docs_with_rangemap(self, db: &dyn HirDatabase) -> Option<Cow<'_, hir::Docs>> {
        self.hir_docs(db).map(Cow::Borrowed)
    }
    fn resolve_doc_path<'db>(
        self,
        db: &'db dyn HirDatabase,
        link: &str,
        ns: Option<hir::Namespace>,
        is_inner_doc: hir::IsInnerDoc,
    ) -> Option<hir::DocLinkDef<'db>> {
        resolve_doc_path_on(db, self, link, ns, is_inner_doc)
    }
}

macro_rules! impl_has_docs {
    ($($def:ty,)*) => {$(
        impl HasDocs for $def {}
    )*};
}

impl_has_docs![
    hir::EnumVariant,
    hir::Field,
    hir::Static,
    hir::Const,
    hir::Trait,
    hir::TypeAlias,
    hir::Macro,
    hir::Function<'_>,
    hir::Adt,
    hir::Module,
    hir::Impl<'_>,
    hir::Crate,
    hir::AssocItem<'_>,
    hir::Struct,
    hir::Union,
    hir::Enum,
];

impl HasDocs for hir::ExternCrateDecl {
    fn docs(self, db: &dyn HirDatabase) -> Option<Documentation<'_>> {
        let crate_docs = self.resolved_crate(db)?.hir_docs(db);
        let decl_docs = self.hir_docs(db);
        match (decl_docs, crate_docs) {
            (None, None) => None,
            (Some(docs), None) | (None, Some(docs)) => {
                Some(Documentation::new_borrowed(docs.docs()))
            }
            (Some(decl_docs), Some(crate_docs)) => {
                let mut docs = String::with_capacity(
                    decl_docs.docs().len() + "\n\n".len() + crate_docs.docs().len(),
                );
                docs.push_str(decl_docs.docs());
                docs.push_str("\n\n");
                docs.push_str(crate_docs.docs());
                Some(Documentation::new_owned(docs))
            }
        }
    }

    fn docs_with_rangemap(self, db: &dyn HirDatabase) -> Option<Cow<'_, hir::Docs>> {
        let crate_docs = self.resolved_crate(db)?.hir_docs(db);
        let decl_docs = self.hir_docs(db);
        match (decl_docs, crate_docs) {
            (None, None) => None,
            (Some(docs), None) | (None, Some(docs)) => Some(Cow::Borrowed(docs)),
            (Some(decl_docs), Some(crate_docs)) => {
                let mut docs = decl_docs.clone();
                docs.append_str("\n\n");
                docs.append(crate_docs);
                Some(Cow::Owned(docs))
            }
        }
    }
}
