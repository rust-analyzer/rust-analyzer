//! Utility module for converting between hir_def ids and code_model wrappers.
//!
//! It's unclear if we need this long-term, but it's definitely useful while we
//! are splitting the hir.

use hir_def::{
    AdtId, AssocItemId, BuiltinDeriveImplId, DefWithBodyId, ExpressionStoreOwnerId, FieldId,
    FunctionId, GenericDefId, GenericParamId, ImplId, ModuleDefId, VariantId,
    hir::{BindingId, LabelId},
    item_scope::ItemInNs as ItemInNsId,
};
use hir_ide::next_solver::AnyImplId;
use stdx::impl_from;

use crate::{
    Adt, AnyFunctionId, AssocItem, BuiltinType, DefWithBody, ExpressionStoreOwner, Field, Function,
    GenericDef, GenericParam, Impl, ItemInNs, Label, Local, ModuleDef, Variant,
};

macro_rules! from_id {
    ($(($id:path, $ty:path)),* $(,)?) => {$(
        impl From<$id> for $ty {
            fn from(id: $id) -> $ty {
                $ty { id }
            }
        }
        impl From<$ty> for $id {
            fn from(ty: $ty) -> $id {
                ty.id
            }
        }
    )*}
}

from_id![
    (base_db::Crate, crate::Crate),
    (hir_def::ModuleId, crate::Module),
    (hir_def::StructId, crate::Struct),
    (hir_def::UnionId, crate::Union),
    (hir_def::EnumId, crate::Enum),
    (hir_def::TypeAliasId, crate::TypeAlias),
    (hir_def::TraitId, crate::Trait),
    (hir_def::StaticId, crate::Static),
    (hir_def::ConstId, crate::Const),
    (hir_def::TypeOrConstParamId, crate::TypeOrConstParam),
    (hir_def::TypeParamId, crate::TypeParam),
    (hir_def::ConstParamId, crate::ConstParam),
    (hir_def::LifetimeParamId, crate::LifetimeParam),
    (hir_def::MacroId, crate::Macro),
    (hir_def::EnumVariantId, crate::EnumVariant),
    (hir_def::ExternCrateId, crate::ExternCrateDecl),
    (hir_def::ExternBlockId, crate::ExternBlock),
];
impl<'db> From<crate::AnyFunctionId<'db>> for crate::Function<'db> {
    fn from(id: crate::AnyFunctionId<'db>) -> Self {
        Self { id }
    }
}
impl<'db> From<crate::Function<'db>> for crate::AnyFunctionId<'db> {
    fn from(ty: crate::Function<'db>) -> Self {
        ty.id
    }
}
impl<'db> From<hir_ide::next_solver::AnyImplId<'db>> for crate::Impl<'db> {
    fn from(id: hir_ide::next_solver::AnyImplId<'db>) -> Self {
        Self { id }
    }
}
impl<'db> From<crate::Impl<'db>> for hir_ide::next_solver::AnyImplId<'db> {
    fn from(ty: crate::Impl<'db>) -> Self {
        ty.id
    }
}

impl_from!(AdtId { StructId => Struct, UnionId => Union, EnumId => Enum } for Adt);
impl_from!(Adt { Struct => StructId, Union => UnionId, Enum => EnumId } for AdtId);
impl_from!(
    VariantId { EnumVariantId => EnumVariant, StructId => Struct, UnionId => Union }
    for Variant
);
impl_from!(
    GenericParamId {
        TypeParamId => TypeParam,
        ConstParamId => ConstParam,
        LifetimeParamId => LifetimeParam,
    }
    for GenericParam
);
impl_from!(
    GenericParam {
        LifetimeParam => LifetimeParamId,
        ConstParam => ConstParamId,
        TypeParam => TypeParamId,
    }
    for GenericParamId
);

impl_from!(
    impl<'db>
    ModuleDefId {
        ModuleId => Module,
        FunctionId => Function,
        AdtId => Adt,
        EnumVariantId => EnumVariant,
        ConstId => Const,
        StaticId => Static,
        TraitId => Trait,
        TypeAliasId => TypeAlias,
        BuiltinType => BuiltinType,
        MacroId => Macro,
    }
    for ModuleDef<'db>
);

impl TryFrom<ModuleDef<'_>> for ModuleDefId {
    type Error = ();
    fn try_from(id: ModuleDef<'_>) -> Result<Self, Self::Error> {
        Ok(match id {
            ModuleDef::Module(it) => ModuleDefId::ModuleId(it.into()),
            ModuleDef::Function(it) => match it.id {
                AnyFunctionId::FunctionId(it) => it.into(),
                AnyFunctionId::BuiltinDeriveImplMethod { .. } => return Err(()),
            },
            ModuleDef::Adt(it) => ModuleDefId::AdtId(it.into()),
            ModuleDef::EnumVariant(it) => ModuleDefId::EnumVariantId(it.into()),
            ModuleDef::Const(it) => ModuleDefId::ConstId(it.into()),
            ModuleDef::Static(it) => ModuleDefId::StaticId(it.into()),
            ModuleDef::Trait(it) => ModuleDefId::TraitId(it.into()),
            ModuleDef::TypeAlias(it) => ModuleDefId::TypeAliasId(it.into()),
            ModuleDef::BuiltinType(it) => ModuleDefId::BuiltinType(it.into()),
            ModuleDef::Macro(it) => ModuleDefId::MacroId(it.into()),
        })
    }
}

impl TryFrom<DefWithBody<'_>> for DefWithBodyId {
    type Error = ();
    fn try_from(def: DefWithBody<'_>) -> Result<Self, ()> {
        Ok(match def {
            DefWithBody::Function(it) => match it.id {
                AnyFunctionId::FunctionId(it) => it.into(),
                AnyFunctionId::BuiltinDeriveImplMethod { .. } => return Err(()),
            },
            DefWithBody::Static(it) => DefWithBodyId::StaticId(it.id),
            DefWithBody::Const(it) => DefWithBodyId::ConstId(it.id),
            DefWithBody::EnumVariant(it) => DefWithBodyId::VariantId(it.into()),
        })
    }
}

impl_from!(
    impl<'db>
    DefWithBodyId {
        FunctionId => Function,
        StaticId => Static,
        ConstId => Const,
        VariantId => EnumVariant,
    }
    for DefWithBody<'db>
);
impl_from!(
    impl<'db>
    AssocItemId { FunctionId => Function, TypeAliasId => TypeAlias, ConstId => Const }
    for AssocItem<'db>
);

impl TryFrom<GenericDef<'_>> for GenericDefId {
    type Error = ();

    fn try_from(def: GenericDef<'_>) -> Result<Self, Self::Error> {
        def.id().ok_or(())
    }
}

impl_from!(
    impl<'db>
    GenericDefId {
        FunctionId => Function,
        AdtId => Adt,
        TraitId => Trait,
        TypeAliasId => TypeAlias,
        ImplId => Impl,
        ConstId => Const,
        StaticId => Static,
    }
    for GenericDef<'db>
);

impl From<Adt> for GenericDefId {
    fn from(id: Adt) -> Self {
        match id {
            Adt::Struct(it) => it.id.into(),
            Adt::Union(it) => it.id.into(),
            Adt::Enum(it) => it.id.into(),
        }
    }
}

impl_from!(
    Variant { Struct => StructId, EnumVariant => EnumVariantId, Union => UnionId }
    for VariantId
);

impl From<Field> for FieldId {
    fn from(def: Field) -> Self {
        FieldId { parent: def.parent.into(), local_id: def.id }
    }
}

impl From<FieldId> for Field {
    fn from(def: FieldId) -> Self {
        Field { parent: def.parent.into(), id: def.local_id }
    }
}

impl TryFrom<AssocItem<'_>> for GenericDefId {
    type Error = ();
    fn try_from(item: AssocItem<'_>) -> Result<Self, Self::Error> {
        Ok(match item {
            AssocItem::Function(f) => match f.id {
                AnyFunctionId::FunctionId(it) => it.into(),
                AnyFunctionId::BuiltinDeriveImplMethod { .. } => return Err(()),
            },
            AssocItem::Const(c) => c.id.into(),
            AssocItem::TypeAlias(t) => t.id.into(),
        })
    }
}

impl<'db> From<(DefWithBodyId, BindingId)> for Local<'db> {
    fn from((parent, binding_id): (DefWithBodyId, BindingId)) -> Self {
        Local { parent: parent.into(), parent_infer: parent.into(), binding_id }
    }
}

impl From<(ExpressionStoreOwnerId, LabelId)> for Label {
    fn from((parent, label_id): (ExpressionStoreOwnerId, LabelId)) -> Self {
        Label { parent, label_id }
    }
}

impl_from!(impl<'db> ItemInNsId { Types => Types, Values => Values, Macros => Macros } for ItemInNs<'db>);

impl TryFrom<ItemInNs<'_>> for hir_def::item_scope::ItemInNs {
    type Error = ();
    fn try_from(it: ItemInNs<'_>) -> Result<Self, Self::Error> {
        Ok(match it {
            ItemInNs::Types(it) => Self::Types(it.try_into()?),
            ItemInNs::Values(it) => Self::Values(it.try_into()?),
            ItemInNs::Macros(it) => Self::Macros(it.into()),
        })
    }
}

impl From<hir_def::builtin_type::BuiltinType> for BuiltinType {
    fn from(inner: hir_def::builtin_type::BuiltinType) -> Self {
        Self { inner }
    }
}

impl From<BuiltinType> for hir_def::builtin_type::BuiltinType {
    fn from(it: BuiltinType) -> Self {
        it.inner
    }
}

impl<'db> From<hir_def::ImplId> for crate::Impl<'db> {
    fn from(value: hir_def::ImplId) -> Self {
        crate::Impl { id: AnyImplId::ImplId(value) }
    }
}

impl<'db> From<BuiltinDeriveImplId<'db>> for crate::Impl<'db> {
    fn from(value: BuiltinDeriveImplId<'db>) -> Self {
        crate::Impl { id: AnyImplId::BuiltinDeriveImplId(value) }
    }
}

impl<'db> From<hir_def::FunctionId> for crate::Function<'db> {
    fn from(value: hir_def::FunctionId) -> Self {
        crate::Function { id: AnyFunctionId::FunctionId(value) }
    }
}

impl<'db> TryFrom<ExpressionStoreOwner<'db>> for ExpressionStoreOwnerId {
    type Error = ();

    fn try_from(v: ExpressionStoreOwner<'db>) -> Result<Self, Self::Error> {
        match v {
            ExpressionStoreOwner::Signature(generic_def_id) => {
                Ok(Self::Signature(generic_def_id.try_into()?))
            }
            ExpressionStoreOwner::Body(def_with_body_id) => {
                Ok(Self::Body(def_with_body_id.try_into()?))
            }
            ExpressionStoreOwner::VariantFields(variant_id) => {
                Ok(Self::VariantFields(variant_id.into()))
            }
        }
    }
}

impl TryFrom<Function<'_>> for FunctionId {
    type Error = ();

    fn try_from(v: Function<'_>) -> Result<Self, Self::Error> {
        match v.id {
            AnyFunctionId::FunctionId(id) => Ok(id),
            _ => Err(()),
        }
    }
}

impl TryFrom<Impl<'_>> for ImplId {
    type Error = ();

    fn try_from(v: Impl<'_>) -> Result<Self, Self::Error> {
        match v.id {
            AnyImplId::ImplId(id) => Ok(id),
            _ => Err(()),
        }
    }
}
