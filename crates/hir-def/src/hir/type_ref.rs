//! HIR for references to types. Paths in these are not yet resolved. They can
//! be directly created from an ast::TypeRef, without further queries.

use hir_expand::name::Name;
use la_arena::Idx;
use rustc_abi::ExternAbi;
use salsa::SalsaValue;
use thin_vec::ThinVec;

use crate::{
    LifetimeParamId, TypeParamId,
    expr_store::{ExpressionStore, path::Path},
    hir::{ExprId, PatId},
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Mutability {
    Shared,
    Mut,
}

impl Mutability {
    pub fn from_mutable(mutable: bool) -> Mutability {
        if mutable { Mutability::Mut } else { Mutability::Shared }
    }

    pub fn as_keyword_for_ref(self) -> &'static str {
        match self {
            Mutability::Shared => "",
            Mutability::Mut => "mut ",
        }
    }

    pub fn as_keyword_for_ptr(self) -> &'static str {
        match self {
            Mutability::Shared => "const ",
            Mutability::Mut => "mut ",
        }
    }

    /// Returns `true` if the mutability is [`Mut`].
    ///
    /// [`Mut`]: Mutability::Mut
    #[must_use]
    pub fn is_mut(&self) -> bool {
        matches!(self, Self::Mut)
    }

    /// Returns `true` if the mutability is [`Shared`].
    ///
    /// [`Shared`]: Mutability::Shared
    #[must_use]
    pub fn is_shared(&self) -> bool {
        matches!(self, Self::Shared)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Rawness {
    RawPtr,
    Ref,
}

impl Rawness {
    pub fn from_raw(is_raw: bool) -> Rawness {
        if is_raw { Rawness::RawPtr } else { Rawness::Ref }
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, Self::RawPtr)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, SalsaValue)]
/// A `TypeRefId` that is guaranteed to always be `TypeRef::Path`. We use this for things like
/// impl's trait, that are always paths but need to be traced back to source code.
pub struct PathId<'db>(TypeRefId<'db>);

impl<'db> PathId<'db> {
    #[inline]
    pub fn from_type_ref_unchecked(type_ref: TypeRefId<'db>) -> Self {
        Self(type_ref)
    }

    #[inline]
    pub fn type_ref(self) -> TypeRefId<'db> {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, SalsaValue)]
pub struct TraitRef<'db> {
    pub path: PathId<'db>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, SalsaValue)]
pub struct FnType<'db> {
    pub binder: Option<Box<[Name]>>,
    pub params: Box<[(Option<Name>, TypeRefId<'db>)]>,
    pub is_varargs: bool,
    pub is_unsafe: bool,
    pub abi: ExternAbi,
}

impl<'db> FnType<'db> {
    #[inline]
    pub fn split_params_and_ret(&self) -> (&[(Option<Name>, TypeRefId<'db>)], TypeRefId<'db>) {
        let (ret, params) = self.params.split_last().expect("should have at least return type");
        (params, ret.1)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, SalsaValue)]
pub struct ArrayType<'db> {
    pub ty: TypeRefId<'db>,
    pub len: ConstRef<'db>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, SalsaValue)]
pub struct RefType<'db> {
    pub ty: TypeRefId<'db>,
    pub lifetime: Option<LifetimeRefId>,
    pub mutability: Mutability,
}

/// Compare ty::Ty
#[derive(Clone, PartialEq, Eq, Hash, Debug, SalsaValue)]
pub enum TypeRef<'db> {
    Never,
    Placeholder,
    Tuple(ThinVec<TypeRefId<'db>>),
    Path(Path<'db>),
    RawPtr(TypeRefId<'db>, Mutability),
    // FIXME: Unbox this once `Idx` has a niche,
    // as `RefType` should shrink by 4 bytes then
    Reference(Box<RefType<'db>>),
    Array(ArrayType<'db>),
    Slice(TypeRefId<'db>),
    /// A fn pointer. Last element of the vector is the return type.
    Fn(Box<FnType<'db>>),
    ImplTrait(ThinVec<TypeBound<'db>>),
    DynTrait(ThinVec<TypeBound<'db>>),
    TypeParam(TypeParamId<'db>),
    PatternType(TypeRefId<'db>, PatId<'db>),
    Error,
}

#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
const _: () = assert!(size_of::<TypeRef<'_>>() == 24);

pub type TypeRefId<'db> = Idx<TypeRef<'db>>;

pub type LifetimeRefId = Idx<LifetimeRef>;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum LifetimeRef {
    Named(Name),
    Static,
    Placeholder,
    Param(LifetimeParamId),
    Error,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, SalsaValue)]
pub enum TypeBound<'db> {
    Path(PathId<'db>, TraitBoundModifier),
    ForLifetime(ThinVec<Name>, PathId<'db>),
    Lifetime(LifetimeRefId),
    Use(ThinVec<UseArgRef>),
    Error,
}

#[cfg(target_pointer_width = "64")]
const _: [(); 16] = [(); size_of::<TypeBound<'_>>()];

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum UseArgRef {
    Name(Name),
    Lifetime(LifetimeRefId),
}

/// A modifier on a bound, currently this is only used for `?Sized`, where the
/// modifier is `Maybe`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TraitBoundModifier {
    None,
    Maybe,
}

impl<'db> TypeRef<'db> {
    pub(crate) fn unit() -> TypeRef<'db> {
        TypeRef::Tuple(ThinVec::new())
    }
}

impl<'db> TypeBound<'db> {
    pub fn as_path<'a>(
        &self,
        map: &'a ExpressionStore<'db>,
    ) -> Option<(&'a Path<'db>, TraitBoundModifier)> {
        match self {
            &TypeBound::Path(p, m) => Some((&map[p], m)),
            &TypeBound::ForLifetime(_, p) => Some((&map[p], TraitBoundModifier::None)),
            TypeBound::Lifetime(_) | TypeBound::Error | TypeBound::Use(_) => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, SalsaValue)]
pub struct ConstRef<'db> {
    pub expr: ExprId<'db>,
}
