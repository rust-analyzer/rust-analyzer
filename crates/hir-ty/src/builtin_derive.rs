//! Implementation of builtin derive impls.

use hir_def::{
    AdtId, BuiltinDeriveImplId, BuiltinDeriveImplLoc, HasModule, LocalFieldId, TraitId,
    TypeOrConstParamId, TypeParamId,
    builtin_derive::BuiltinDeriveImplTrait,
    hir::generics::{GenericParams, TypeOrConstParamData},
};
use la_arena::ArenaMap;
use rustc_type_ir::{
    AliasTyKind, BoundVar, BoundVarIndexKind, DebruijnIndex, INNERMOST, TypeFlags, TypeFoldable,
    TypeFolder, TypeSuperFoldable, TypeSuperVisitable, TypeVisitable, TypeVisitableExt,
    TypeVisitor, Upcast,
    inherent::{GenericArgs as _, IntoKind, Ty as _},
};

use crate::{
    FieldType, GenericPredicates,
    db::HirDatabase,
    next_solver::{
        AliasTy, Binder, BoundConst, BoundRegion, BoundTy, BoundVarKinds, Clause, Clauses, Const,
        ConstKind, DbInterner, EarlyBinder, GenericArgs, ParamEnv, Region, RegionKind,
        StoredEarlyBinder, TraitRef, Ty, TyKind, Unnormalized, generics::Generics,
    },
};

fn trait_args(trait_: BuiltinDeriveImplTrait, self_ty: Ty<'_>) -> GenericArgs<'_> {
    match trait_ {
        BuiltinDeriveImplTrait::Copy
        | BuiltinDeriveImplTrait::Reborrow
        | BuiltinDeriveImplTrait::Clone
        | BuiltinDeriveImplTrait::Default
        | BuiltinDeriveImplTrait::Debug
        | BuiltinDeriveImplTrait::Hash
        | BuiltinDeriveImplTrait::Eq
        | BuiltinDeriveImplTrait::Ord => GenericArgs::new_from_slice(&[self_ty.into()]),
        BuiltinDeriveImplTrait::PartialOrd | BuiltinDeriveImplTrait::PartialEq => {
            GenericArgs::new_from_slice(&[self_ty.into(), self_ty.into()])
        }
    }
}

pub(crate) fn generics_of<'db>(
    interner: DbInterner<'db>,
    id: BuiltinDeriveImplId,
) -> Generics<'db> {
    let db = interner.db;
    let loc = id.loc(db);
    match loc.trait_ {
        BuiltinDeriveImplTrait::Copy
        | BuiltinDeriveImplTrait::Reborrow
        | BuiltinDeriveImplTrait::Clone
        | BuiltinDeriveImplTrait::Default
        | BuiltinDeriveImplTrait::Debug
        | BuiltinDeriveImplTrait::Hash
        | BuiltinDeriveImplTrait::Ord
        | BuiltinDeriveImplTrait::PartialOrd
        | BuiltinDeriveImplTrait::Eq
        | BuiltinDeriveImplTrait::PartialEq => {
            Generics::from_generic_def(db, loc.adt.into(), false)
        }
    }
}

pub fn generic_params_count(db: &dyn HirDatabase, id: BuiltinDeriveImplId) -> usize {
    let loc = id.loc(db);
    let adt_params = GenericParams::of(db, loc.adt.into());
    adt_params.len()
}

pub fn impl_trait<'db>(
    interner: DbInterner<'db>,
    id: BuiltinDeriveImplId,
) -> EarlyBinder<'db, TraitRef<'db>> {
    let db = interner.db;
    let loc = id.loc(db);
    let trait_id = loc
        .trait_
        .get_id(interner.lang_items())
        .expect("we don't pass the impl to the solver if we can't resolve the trait");
    match loc.trait_ {
        BuiltinDeriveImplTrait::Copy
        | BuiltinDeriveImplTrait::Reborrow
        | BuiltinDeriveImplTrait::Clone
        | BuiltinDeriveImplTrait::Default
        | BuiltinDeriveImplTrait::Debug
        | BuiltinDeriveImplTrait::Hash
        | BuiltinDeriveImplTrait::Ord
        | BuiltinDeriveImplTrait::Eq
        | BuiltinDeriveImplTrait::PartialOrd
        | BuiltinDeriveImplTrait::PartialEq => {
            let self_ty = Ty::new_adt(
                interner,
                loc.adt,
                GenericArgs::identity_for_item(interner, loc.adt.into()),
            );
            EarlyBinder::bind(TraitRef::new_from_args(
                interner,
                trait_id.into(),
                trait_args(loc.trait_, self_ty),
            ))
        }
    }
}

#[inline]
pub fn impl_super_outlives<'db>(
    _interner: DbInterner<'db>,
    _id: BuiltinDeriveImplId,
) -> EarlyBinder<'db, &'db [Clause<'db>]> {
    // Currently no builtin-derived traits have super outlive bounds.
    EarlyBinder::bind(&[])
}

#[salsa::tracked(returns(ref))]
pub fn predicates(db: &dyn HirDatabase, impl_: BuiltinDeriveImplId) -> GenericPredicates {
    let loc = impl_.loc(db);
    let generic_params = GenericParams::of(db, loc.adt.into());
    let interner = DbInterner::new_with(db, loc.module(db).krate(db));
    let adt_predicates = GenericPredicates::query(db, loc.adt.into());
    let trait_id = loc
        .trait_
        .get_id(interner.lang_items())
        .expect("we don't pass the impl to the solver if we can't resolve the trait");
    match loc.trait_ {
        BuiltinDeriveImplTrait::Copy
        | BuiltinDeriveImplTrait::Clone
        | BuiltinDeriveImplTrait::Debug
        | BuiltinDeriveImplTrait::Hash
        | BuiltinDeriveImplTrait::Ord
        | BuiltinDeriveImplTrait::PartialOrd
        | BuiltinDeriveImplTrait::Eq
        | BuiltinDeriveImplTrait::PartialEq => {
            simple_trait_predicates(interner, loc, generic_params, adt_predicates, trait_id)
        }
        BuiltinDeriveImplTrait::Reborrow => {
            explicit_own_predicates(adt_predicates.own_explicit_predicates())
        }
        BuiltinDeriveImplTrait::Default => {
            if matches!(loc.adt, AdtId::EnumId(_)) {
                // Enums don't have extra bounds.
                explicit_own_predicates(adt_predicates.own_explicit_predicates())
            } else {
                simple_trait_predicates(interner, loc, generic_params, adt_predicates, trait_id)
            }
        }
    }
}

fn explicit_own_predicates<'db>(
    predicates: EarlyBinder<'db, impl Iterator<Item = Clause<'db>>>,
) -> GenericPredicates {
    GenericPredicates::from_explicit_own_predicates(StoredEarlyBinder::bind(
        Clauses::new_from_iter(predicates.skip_binder()).store(),
    ))
}

/// Not cached in a query, currently used in `hir` only. If you need this in `hir-ty` consider introducing a query.
pub fn param_env<'db>(interner: DbInterner<'db>, id: BuiltinDeriveImplId) -> ParamEnv<'db> {
    let predicates = predicates(interner.db, id);
    crate::lower::param_env_from_predicates(interner, predicates)
}

fn simple_trait_predicates<'db>(
    interner: DbInterner<'db>,
    loc: &BuiltinDeriveImplLoc,
    generic_params: &GenericParams,
    adt_predicates: &GenericPredicates,
    trait_id: TraitId,
) -> GenericPredicates {
    let extra_predicates = generic_params
        .iter_type_or_consts()
        .filter(|(_, data)| matches!(data, TypeOrConstParamData::TypeParamData(_)))
        .map(|(param_idx, _)| {
            let param_id = TypeParamId::from_unchecked(TypeOrConstParamId {
                parent: loc.adt.into(),
                local_id: param_idx,
            });
            let param_idx = param_idx.into_raw().into_u32()
                + (generic_params.len_early_bound_lifetimes() as u32);
            let param_ty = Ty::new_param(interner, param_id, param_idx);
            let trait_args = trait_args(loc.trait_, param_ty);
            let trait_ref = TraitRef::new_from_args(interner, trait_id.into(), trait_args);
            trait_ref.upcast(interner)
        });
    let mut assoc_type_bounds = Vec::new();
    match loc.adt {
        AdtId::StructId(id) => extend_assoc_type_bounds(
            interner,
            &mut assoc_type_bounds,
            interner.db.field_types(id.into()),
            trait_id,
            loc.trait_,
        ),
        AdtId::UnionId(id) => extend_assoc_type_bounds(
            interner,
            &mut assoc_type_bounds,
            interner.db.field_types(id.into()),
            trait_id,
            loc.trait_,
        ),
        AdtId::EnumId(id) => {
            for &(variant_id, _) in id.enum_variants(interner.db).variants.values() {
                extend_assoc_type_bounds(
                    interner,
                    &mut assoc_type_bounds,
                    interner.db.field_types(variant_id.into()),
                    trait_id,
                    loc.trait_,
                )
            }
        }
    }
    GenericPredicates::from_explicit_own_predicates(StoredEarlyBinder::bind(
        Clauses::new_from_iter(
            adt_predicates
                .explicit_predicates()
                .iter_identity()
                .map(Unnormalized::skip_norm_wip)
                .chain(extra_predicates)
                .chain(assoc_type_bounds),
        )
        .store(),
    ))
}

fn extend_assoc_type_bounds<'db>(
    interner: DbInterner<'db>,
    assoc_type_bounds: &mut Vec<Clause<'db>>,
    fields: &ArenaMap<LocalFieldId, FieldType>,
    trait_id: TraitId,
    trait_: BuiltinDeriveImplTrait,
) {
    struct ProjectionFinder<'a, 'db> {
        interner: DbInterner<'db>,
        assoc_type_bounds: &'a mut Vec<Clause<'db>>,
        trait_id: TraitId,
        trait_: BuiltinDeriveImplTrait,
        bound_generic_params_stack: Vec<BoundVarKinds<'db>>,
    }

    impl<'db> TypeVisitor<DbInterner<'db>> for ProjectionFinder<'_, 'db> {
        type Result = ();

        fn visit_ty(&mut self, t: Ty<'db>) -> Self::Result {
            if !t.has_type_flags(TypeFlags::HAS_TY_PROJECTION) {
                return;
            }
            if let TyKind::Alias(alias @ AliasTy { kind: AliasTyKind::Projection { .. }, .. }) =
                t.kind()
                && let TyKind::Param(_) = alias.self_ty().kind()
            {
                // rustc flattens binders before lowering, so the corresponding rustc
                // logic just has a flat vec already:
                // https://github.com/rust-lang/rust/blob/cabb1ea4ced589dfee40c08aeffff3cac78af43b/compiler/rustc_builtin_macros/src/deriving/generic/mod.rs#L396
                let t = t.fold_with(&mut FlattenBinders {
                    interner: self.interner,
                    enclosing_binders: &self.bound_generic_params_stack,
                    nested_binder_depth: INNERMOST,
                });
                let bound_vars = BoundVarKinds::new_from_iter(
                    self.bound_generic_params_stack.iter().flat_map(|it| it.iter()),
                );
                self.assoc_type_bounds.push(
                    Binder::bind_with_vars(
                        TraitRef::new_from_args(
                            self.interner,
                            self.trait_id.into(),
                            trait_args(self.trait_, t),
                        ),
                        bound_vars,
                    )
                    .upcast(self.interner),
                );
            }

            t.super_visit_with(self)
        }

        // Place bound generic params on a stack, to extract them when a type is encountered.
        fn visit_binder<T: TypeVisitable<DbInterner<'db>>>(
            &mut self,
            binder: &Binder<'db, T>,
        ) -> Self::Result {
            let stack_len = self.bound_generic_params_stack.len();
            // Given a field `x: for<'a> fn(T::SomeType<'a>)`, we want to account for `'a` so
            // that we generate `where for<'a> T::SomeType<'a>: ::core::clone::Clone`. #122622
            self.bound_generic_params_stack.push(binder.bound_vars());

            binder.super_visit_with(self);

            self.bound_generic_params_stack.truncate(stack_len);
        }
    }

    let mut visitor = ProjectionFinder {
        interner,
        assoc_type_bounds,
        trait_id,
        trait_,
        bound_generic_params_stack: Vec::new(),
    };
    for (_, field) in fields.iter() {
        field.ty().instantiate_identity().skip_norm_wip().visit_with(&mut visitor);
    }
}

/// Transform a projection `for<'a> for<'b> F::Foo<'a, 'b, X>` into
/// `for<'a, 'b> F::Foo<'a, 'b, X>`. Update de Bruijn indexes for
/// enclosing binders, but preserve nested binders.
struct FlattenBinders<'a, 'db> {
    interner: DbInterner<'db>,
    enclosing_binders: &'a [BoundVarKinds<'db>],
    nested_binder_depth: DebruijnIndex,
}

impl FlattenBinders<'_, '_> {
    fn flattened_var(&self, debruijn: DebruijnIndex, var: BoundVar) -> Option<BoundVar> {
        let depth = debruijn.as_usize().checked_sub(self.nested_binder_depth.as_usize())?;
        let binder = self.enclosing_binders.len().checked_sub(depth + 1)?;
        let offset = self.enclosing_binders[..binder].iter().map(|it| it.len()).sum::<usize>();
        Some(BoundVar::from_usize(offset + var.as_usize()))
    }
}

impl<'db> TypeFolder<DbInterner<'db>> for FlattenBinders<'_, 'db> {
    fn cx(&self) -> DbInterner<'db> {
        self.interner
    }

    fn fold_binder<T: TypeFoldable<DbInterner<'db>>>(
        &mut self,
        binder: Binder<'db, T>,
    ) -> Binder<'db, T> {
        self.nested_binder_depth.shift_in(1);
        let binder = binder.super_fold_with(self);
        self.nested_binder_depth.shift_out(1);
        binder
    }

    fn fold_ty(&mut self, ty: Ty<'db>) -> Ty<'db> {
        if !ty.has_bound_vars() {
            return ty;
        }
        if let TyKind::Bound(BoundVarIndexKind::Bound(debruijn), bound_ty) = ty.kind()
            && let Some(var) = self.flattened_var(debruijn, bound_ty.var)
        {
            return Ty::new_bound(
                self.interner,
                self.nested_binder_depth,
                BoundTy { var, kind: bound_ty.kind },
            );
        }
        ty.super_fold_with(self)
    }

    fn fold_region(&mut self, region: Region<'db>) -> Region<'db> {
        if !region.has_bound_vars() {
            return region;
        }
        if let RegionKind::ReBound(BoundVarIndexKind::Bound(debruijn), bound_region) = region.kind()
            && let Some(var) = self.flattened_var(debruijn, bound_region.var)
        {
            return Region::new_bound(
                self.interner,
                self.nested_binder_depth,
                BoundRegion { var, kind: bound_region.kind },
            );
        }
        region
    }

    fn fold_const(&mut self, konst: Const<'db>) -> Const<'db> {
        if !konst.has_bound_vars() {
            return konst;
        }
        if let ConstKind::Bound(BoundVarIndexKind::Bound(debruijn), bound_const) = konst.kind()
            && let Some(var) = self.flattened_var(debruijn, bound_const.var)
        {
            return Const::new_bound(self.interner, self.nested_binder_depth, BoundConst::new(var));
        }
        konst.super_fold_with(self)
    }
}
