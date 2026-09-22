//! Pre-type IR item generics
use std::{ops, sync::LazyLock};

use base_db::SourceDatabase;
use hir_expand::name::Name;
use la_arena::{Arena, Idx, RawIdx};
use salsa::SalsaValue;
use stdx::impl_from;
use thin_vec::ThinVec;

use crate::{
    AdtId, ConstParamId, GenericDefId, LifetimeParamId, TypeOrConstParamId, TypeParamId,
    expr_store::{ExpressionStore, ExpressionStoreSourceMap},
    signatures::{
        ConstSignature, EnumSignature, FunctionSignature, ImplSignature, StaticSignature,
        StructSignature, TraitSignature, TypeAliasSignature, UnionSignature,
    },
    type_ref::{ConstRef, LifetimeRefId, TypeBound, TypeRefId},
};

pub type LocalTypeOrConstParamId<'db> = Idx<TypeOrConstParamData<'db>>;
pub type LocalLifetimeParamId = Idx<LifetimeParamData>;

/// Data about a generic type parameter (to a function, struct, impl, ...).
#[derive(Clone, PartialEq, Eq, Debug, Hash, SalsaValue)]
pub struct TypeParamData<'db> {
    /// [`None`] only if the type ref is an [`crate::type_ref::TypeRef::ImplTrait`].
    pub name: Option<Name>,
    pub default: Option<TypeRefId<'db>>,
    pub provenance: TypeParamProvenance,
}

/// Data about a generic lifetime parameter (to a function, struct, impl, ...).
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct LifetimeParamData {
    pub name: Name,
    pub bound_type: LifetimeBoundType,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum LifetimeBoundType {
    EarlyBound,
    LateBound,
}

impl LifetimeParamData {
    #[inline]
    pub fn is_late_bound(&self) -> bool {
        self.bound_type == LifetimeBoundType::LateBound
    }

    #[inline]
    pub fn is_early_bound(&self) -> bool {
        self.bound_type == LifetimeBoundType::EarlyBound
    }
}

/// Data about a generic const parameter (to a function, struct, impl, ...).
#[derive(Clone, PartialEq, Eq, Debug, Hash, SalsaValue)]
pub struct ConstParamData<'db> {
    pub name: Name,
    pub ty: TypeRefId<'db>,
    pub default: Option<ConstRef<'db>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum TypeParamProvenance {
    TypeParamList,
    TraitSelf,
    ArgumentImplTrait,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, SalsaValue)]
pub enum TypeOrConstParamData<'db> {
    TypeParamData(TypeParamData<'db>),
    ConstParamData(ConstParamData<'db>),
}

impl<'db> TypeOrConstParamData<'db> {
    pub fn name(&self) -> Option<&Name> {
        match self {
            TypeOrConstParamData::TypeParamData(it) => it.name.as_ref(),
            TypeOrConstParamData::ConstParamData(it) => Some(&it.name),
        }
    }

    pub fn has_default(&self) -> bool {
        match self {
            TypeOrConstParamData::TypeParamData(it) => it.default.is_some(),
            TypeOrConstParamData::ConstParamData(it) => it.default.is_some(),
        }
    }

    pub fn type_param(&self) -> Option<&TypeParamData<'db>> {
        match self {
            TypeOrConstParamData::TypeParamData(it) => Some(it),
            TypeOrConstParamData::ConstParamData(_) => None,
        }
    }

    pub fn const_param(&self) -> Option<&ConstParamData<'db>> {
        match self {
            TypeOrConstParamData::TypeParamData(_) => None,
            TypeOrConstParamData::ConstParamData(it) => Some(it),
        }
    }

    pub fn is_trait_self(&self) -> bool {
        match self {
            TypeOrConstParamData::TypeParamData(it) => {
                it.provenance == TypeParamProvenance::TraitSelf
            }
            TypeOrConstParamData::ConstParamData(_) => false,
        }
    }
}

impl_from!(impl<'db> TypeParamData<'db>, ConstParamData<'db> for TypeOrConstParamData<'db>);

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum GenericParamData<'db> {
    TypeParamData(TypeParamData<'db>),
    ConstParamData(ConstParamData<'db>),
    LifetimeParamData(LifetimeParamData),
}

impl<'db> GenericParamData<'db> {
    pub fn name(&self) -> Option<&Name> {
        match self {
            GenericParamData::TypeParamData(it) => it.name.as_ref(),
            GenericParamData::ConstParamData(it) => Some(&it.name),
            GenericParamData::LifetimeParamData(it) => Some(&it.name),
        }
    }

    pub fn type_param(&self) -> Option<&TypeParamData<'db>> {
        match self {
            GenericParamData::TypeParamData(it) => Some(it),
            _ => None,
        }
    }

    pub fn const_param(&self) -> Option<&ConstParamData<'db>> {
        match self {
            GenericParamData::ConstParamData(it) => Some(it),
            _ => None,
        }
    }

    pub fn lifetime_param(&self) -> Option<&LifetimeParamData> {
        match self {
            GenericParamData::LifetimeParamData(it) => Some(it),
            _ => None,
        }
    }
}

impl_from!(impl<'db> TypeParamData<'db>, ConstParamData<'db>, LifetimeParamData for GenericParamData<'db>);

#[derive(Debug, Clone, Copy)]
pub enum GenericParamDataRef<'db> {
    TypeParamData(&'db TypeParamData<'db>),
    ConstParamData(&'db ConstParamData<'db>),
    LifetimeParamData(&'db LifetimeParamData),
}

/// Data about the generic parameters of a function, struct, impl, etc.
#[derive(PartialEq, Eq, Debug, Hash, Default, SalsaValue)]
pub struct GenericParams<'db> {
    pub(crate) type_or_consts: Arena<TypeOrConstParamData<'db>>,
    pub(crate) lifetimes: Arena<LifetimeParamData>,
    pub(crate) where_predicates: Box<[WherePredicate<'db>]>,
    pub(crate) early_bound_lifetimes_len: usize,
}

impl<'db> ops::Index<LocalTypeOrConstParamId<'db>> for GenericParams<'db> {
    type Output = TypeOrConstParamData<'db>;
    fn index(&self, index: LocalTypeOrConstParamId<'db>) -> &TypeOrConstParamData<'db> {
        &self.type_or_consts[index]
    }
}

impl<'db> ops::Index<LocalLifetimeParamId> for GenericParams<'db> {
    type Output = LifetimeParamData;
    fn index(&self, index: LocalLifetimeParamId) -> &LifetimeParamData {
        &self.lifetimes[index]
    }
}

/// A single predicate from a where clause, i.e. `where Type: Trait`. Combined
/// where clauses like `where T: Foo + Bar` are turned into multiple of these.
/// It might still result in multiple actual predicates though, because of
/// associated type bindings like `Iterator<Item = u32>`.
#[derive(Clone, PartialEq, Eq, Debug, Hash, SalsaValue)]
pub enum WherePredicate<'db> {
    TypeBound { lifetimes: Option<ThinVec<Name>>, target: TypeRefId<'db>, bound: TypeBound<'db> },
    Lifetime { target: LifetimeRefId, bound: LifetimeRefId },
}

static EMPTY: LazyLock<GenericParams<'static>> = LazyLock::new(|| GenericParams {
    type_or_consts: Arena::default(),
    lifetimes: Arena::default(),
    where_predicates: Box::default(),
    early_bound_lifetimes_len: 0,
});

impl<'db> GenericParams<'db> {
    /// The index of the self param in the generic of the non-parent definition.
    pub const SELF_PARAM_ID_IN_SELF: la_arena::Idx<TypeOrConstParamData<'db>> =
        LocalTypeOrConstParamId::from_raw(RawIdx::from_u32(0));

    #[inline]
    pub fn empty() -> &'static GenericParams<'static> {
        LazyLock::force(&EMPTY)
    }

    pub fn of(db: &'db dyn SourceDatabase, def: GenericDefId) -> &'db GenericParams<'db> {
        Self::with_store(db, def).0
    }

    pub fn with_store(
        db: &'db dyn SourceDatabase,
        def: GenericDefId,
    ) -> (&'db GenericParams<'db>, &'db ExpressionStore<'db>) {
        match def {
            GenericDefId::AdtId(AdtId::EnumId(id)) => {
                let sig = EnumSignature::of(db, id);
                (&sig.generic_params, &sig.store)
            }
            GenericDefId::AdtId(AdtId::StructId(id)) => {
                let sig = StructSignature::of(db, id);
                (&sig.generic_params, &sig.store)
            }
            GenericDefId::AdtId(AdtId::UnionId(id)) => {
                let sig = UnionSignature::of(db, id);
                (&sig.generic_params, &sig.store)
            }
            GenericDefId::ConstId(id) => {
                let sig = ConstSignature::of(db, id);
                (&EMPTY, &sig.store)
            }
            GenericDefId::FunctionId(id) => {
                let sig = FunctionSignature::of(db, id);
                (&sig.generic_params, &sig.store)
            }
            GenericDefId::ImplId(id) => {
                let sig = ImplSignature::of(db, id);
                (&sig.generic_params, &sig.store)
            }
            GenericDefId::StaticId(id) => {
                let sig = StaticSignature::of(db, id);
                (&EMPTY, &sig.store)
            }
            GenericDefId::TraitId(id) => {
                let sig = TraitSignature::of(db, id);
                (&sig.generic_params, &sig.store)
            }
            GenericDefId::TypeAliasId(id) => {
                let sig = TypeAliasSignature::of(db, id);
                (&sig.generic_params, &sig.store)
            }
        }
    }

    pub fn with_source_map(
        db: &'db dyn SourceDatabase,
        def: GenericDefId,
    ) -> (&'db GenericParams<'db>, &'db ExpressionStore<'db>, &'db ExpressionStoreSourceMap<'db>)
    {
        match def {
            GenericDefId::AdtId(AdtId::EnumId(id)) => {
                let (sig, sm) = EnumSignature::with_source_map(db, id);
                (&sig.generic_params, &sig.store, sm)
            }
            GenericDefId::AdtId(AdtId::StructId(id)) => {
                let (sig, sm) = StructSignature::with_source_map(db, id);
                (&sig.generic_params, &sig.store, sm)
            }
            GenericDefId::AdtId(AdtId::UnionId(id)) => {
                let (sig, sm) = UnionSignature::with_source_map(db, id);
                (&sig.generic_params, &sig.store, sm)
            }
            GenericDefId::ConstId(id) => {
                let (sig, sm) = ConstSignature::with_source_map(db, id);
                (&EMPTY, &sig.store, sm)
            }
            GenericDefId::FunctionId(id) => {
                let (sig, sm) = FunctionSignature::with_source_map(db, id);
                (&sig.generic_params, &sig.store, sm)
            }
            GenericDefId::ImplId(id) => {
                let (sig, sm) = ImplSignature::with_source_map(db, id);
                (&sig.generic_params, &sig.store, sm)
            }
            GenericDefId::StaticId(id) => {
                let (sig, sm) = StaticSignature::with_source_map(db, id);
                (&EMPTY, &sig.store, sm)
            }
            GenericDefId::TraitId(id) => {
                let (sig, sm) = TraitSignature::with_source_map(db, id);
                (&sig.generic_params, &sig.store, sm)
            }
            GenericDefId::TypeAliasId(id) => {
                let (sig, sm) = TypeAliasSignature::with_source_map(db, id);
                (&sig.generic_params, &sig.store, sm)
            }
        }
    }

    /// Number of Generic parameters (type_or_consts + lifetimes)
    #[inline]
    pub fn len(&self) -> usize {
        self.type_or_consts.len() + self.lifetimes.len()
    }

    #[inline]
    pub fn len_no_late(&self) -> usize {
        self.type_or_consts.len() + self.early_bound_lifetimes_len
    }

    #[inline]
    pub fn len_lifetimes(&self) -> usize {
        self.lifetimes.len()
    }

    #[inline]
    pub fn len_early_bound_lifetimes(&self) -> usize {
        self.early_bound_lifetimes_len
    }

    #[inline]
    pub fn len_late_bound_lifetimes(&self) -> usize {
        self.len_lifetimes() - self.len_early_bound_lifetimes()
    }

    #[inline]
    pub fn len_type_or_consts(&self) -> usize {
        self.type_or_consts.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn has_no_predicates(&self) -> bool {
        self.where_predicates.is_empty()
    }

    #[inline]
    pub fn where_predicates(&self) -> &[WherePredicate<'db>] {
        &self.where_predicates
    }

    /// Iterator of type_or_consts field
    #[inline]
    pub fn iter_type_or_consts(
        &self,
    ) -> impl DoubleEndedIterator<Item = (LocalTypeOrConstParamId<'db>, &TypeOrConstParamData<'db>)>
    {
        self.type_or_consts.iter()
    }

    /// Iterator of lifetimes field
    #[inline]
    pub fn iter_lt(
        &self,
    ) -> impl DoubleEndedIterator<Item = (LocalLifetimeParamId, &LifetimeParamData)> {
        self.lifetimes.iter()
    }

    #[inline]
    pub fn iter_early_bound_lt(
        &self,
    ) -> impl DoubleEndedIterator<Item = (LocalLifetimeParamId, &LifetimeParamData)> {
        self.lifetimes.iter().filter(|(_, p)| p.bound_type == LifetimeBoundType::EarlyBound)
    }

    #[inline]
    pub fn iter_late_bound_lt(
        &self,
    ) -> impl DoubleEndedIterator<Item = (LocalLifetimeParamId, &LifetimeParamData)> {
        self.lifetimes.iter().filter(|(_, p)| p.bound_type == LifetimeBoundType::LateBound)
    }

    pub fn find_type_by_name(&self, name: &Name, parent: GenericDefId) -> Option<TypeParamId<'db>> {
        self.type_or_consts.iter().find_map(|(id, p)| {
            if p.name().as_ref() == Some(&name) && p.type_param().is_some() {
                Some(TypeParamId::from_unchecked(TypeOrConstParamId { local_id: id, parent }))
            } else {
                None
            }
        })
    }

    pub fn find_const_by_name(
        &self,
        name: &Name,
        parent: GenericDefId,
    ) -> Option<ConstParamId<'db>> {
        self.type_or_consts.iter().find_map(|(id, p)| {
            if p.name().as_ref() == Some(&name) && p.const_param().is_some() {
                Some(ConstParamId::from_unchecked(TypeOrConstParamId { local_id: id, parent }))
            } else {
                None
            }
        })
    }

    #[inline]
    pub fn trait_self_param(&self) -> Option<LocalTypeOrConstParamId<'db>> {
        if self.type_or_consts.is_empty() {
            return None;
        }
        matches!(
            self.type_or_consts[Self::SELF_PARAM_ID_IN_SELF],
            TypeOrConstParamData::TypeParamData(TypeParamData {
                provenance: TypeParamProvenance::TraitSelf,
                ..
            })
        )
        .then(|| Self::SELF_PARAM_ID_IN_SELF)
    }

    pub fn find_lifetime_by_name(
        &self,
        name: &Name,
        parent: GenericDefId,
    ) -> Option<LifetimeParamId> {
        self.lifetimes.iter().find_map(|(id, p)| {
            if &p.name == name { Some(LifetimeParamId { local_id: id, parent }) } else { None }
        })
    }

    pub fn lifetime_param_idx(
        &self,
        lifetime_param_id: &LocalLifetimeParamId,
    ) -> Option<(usize, bool)> {
        let mut late_bound_idx = 0;
        self.iter_lt().enumerate().find_map(|(idx, (param_id, param_data))| {
            let idx = if param_data.is_late_bound() {
                let prev = late_bound_idx;
                late_bound_idx += 1;
                prev
            } else {
                idx - late_bound_idx
            };

            (param_id == *lifetime_param_id).then(|| (idx, param_data.is_late_bound()))
        })
    }
}
