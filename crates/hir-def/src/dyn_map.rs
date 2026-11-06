//! This module defines a `DynMap` -- a container for heterogeneous maps.
//!
//! This means that `DynMap` stores a bunch of hash maps inside, and those maps
//! can be of different types.
//!
//! It is used like this:
//!
//! ```ignore
//! # use hir_def::dyn_map::DynMap;
//! # use hir_def::dyn_map::Key;
//! // keys define submaps of a `DynMap`
//! const STRING_TO_U32: StaticKey<String, u32> = Key::new();
//! const U32_TO_VEC: StaticKey<u32, Vec<bool>> = Key::new();
//!
//! // Note: concrete type, no type params!
//! let mut map = DynMap::new();
//!
//! // To access a specific map, index the `DynMap` by `Key`:
//! map[STRING_TO_U32].insert("hello".to_string(), 92);
//! let value = map[U32_TO_VEC].get(92);
//! assert!(value.is_none());
//! ```
//!
//! This is a work of fiction. Any similarities to Kotlin's `BindingContext` are
//! a coincidence.

pub mod keys {
    use either::Either;
    use hir_expand::{MacroCallId, attrs::AttrId};
    use syntax::ast;

    use crate::{
        BlockId, BuiltinDeriveImplId, ConstId, EnumId, EnumVariantId, ExternBlockId, ExternCrateId,
        FieldId, FunctionId, ImplId, LifetimeParamId, Macro2Id, MacroRulesId, ProcMacroId,
        StaticId, StructId, TraitId, TypeAliasId, TypeOrConstParamId, UnionId, UseId,
        dyn_map::{Key, ValueTrait},
    };

    macro_rules! declare_keys {
        {
            $vis:vis const $key_name:ident<$key:ty, for<$db_lt:lifetime> $value:ty $(,)?>;
            $( $rest:tt )*
        } => {
            $vis const $key_name: Key<$key, dyn for<$db_lt> ValueTrait<$db_lt, Output = $value>> = Key::new();
            declare_keys!( $($rest)* );
        };
        {
            $vis:vis const $key_name:ident<$key:ty, $value:ty $(,)?>;
            $( $rest:tt )*
        } => {
            declare_keys! {
            $vis const $key_name<$key, for<'db> $value>;
            $( $rest )*
            }
        };
        // Recursion base case.
        () => {};
    }

    declare_keys! {
        pub const BLOCK<ast::BlockExpr, BlockId>;
        pub const FUNCTION<ast::Fn, FunctionId>;
        pub const CONST<ast::Const, ConstId>;
        pub const STATIC<ast::Static, StaticId>;
        pub const TYPE_ALIAS<ast::TypeAlias, TypeAliasId>;
        pub const IMPL<ast::Impl, ImplId>;
        pub const EXTERN_BLOCK<ast::ExternBlock, ExternBlockId>;
        pub const TRAIT<ast::Trait, TraitId>;
        pub const STRUCT<ast::Struct, StructId>;
        pub const UNION<ast::Union, UnionId>;
        pub const ENUM<ast::Enum, EnumId>;
        pub const EXTERN_CRATE<ast::ExternCrate, ExternCrateId>;
        pub const USE<ast::Use, UseId>;

        pub const ENUM_VARIANT<ast::Variant, EnumVariantId>;
        pub const TUPLE_FIELD<ast::TupleField, FieldId>;
        pub const RECORD_FIELD<ast::RecordField, FieldId>;
        pub const TYPE_PARAM<ast::TypeParam, TypeOrConstParamId>;
        pub const CONST_PARAM<ast::ConstParam, TypeOrConstParamId>;
        pub const LIFETIME_PARAM<ast::LifetimeParam, LifetimeParamId>;

        pub const MACRO_RULES<ast::MacroRules, MacroRulesId>;
        pub const MACRO2<ast::MacroDef, Macro2Id>;
        pub const PROC_MACRO<ast::Fn, ProcMacroId>;
        pub const MACRO_CALL<ast::MacroCall, MacroCallId>;
        pub const ATTR_MACRO_CALL<ast::Item, MacroCallId>;
        pub const DERIVE_MACRO_CALL<
            ast::Meta,
            for<'db> (
                AttrId,
                /* derive() */ MacroCallId,
                /* actual derive macros */
                Box<[Option<Either<MacroCallId, BuiltinDeriveImplId<'db>>>]>,
            ),
        >;
    }
}

use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use rustc_hash::FxHashMap;
use stdx::anymap::Map;

pub trait ValueTrait<'db> {
    type Output;
}

type Value<'db, V> = <V as ValueTrait<'db>>::Output;

use syntax::{AstNode, AstPtr};

pub type StaticKey<K, V> = Key<K, dyn for<'db> ValueTrait<'db, Output = V>>;

pub struct Key<K, V: ?Sized> {
    _phantom: PhantomData<(K, V)>,
}

impl<K: 'static, V: ?Sized> Key<K, V> {
    pub(crate) const fn new() -> Key<K, V>
    where
        V: for<'db> ValueTrait<'db>,
        Value<'static, V>: 'static,
    {
        Key { _phantom: PhantomData }
    }
}

impl<K, V: ?Sized> Copy for Key<K, V> {}

impl<K, V: ?Sized> Clone for Key<K, V> {
    fn clone(&self) -> Key<K, V> {
        *self
    }
}

#[derive(Default)]
pub struct DynMap<'db> {
    pub(crate) map: Map,
    _marker: PhantomData<&'db ()>,
}

#[repr(transparent)]
pub struct KeyMap<'db, KEY> {
    map: DynMap<'db>,
    _phantom: PhantomData<KEY>,
}

// XXX: AST Nodes and SyntaxNodes have identity equality semantics: nodes are
// equal if they point to exactly the same object.
//
// In general, we do not guarantee that we have exactly one instance of a
// syntax tree for each file. We probably should add such guarantee, but, for
// the time being, we will use identity-less AstPtr comparison.
impl<'db, K, V: ?Sized> KeyMap<'db, Key<K, V>>
where
    K: AstNode + 'static,
    V: for<'db_> ValueTrait<'db_>,
    Value<'static, V>: 'static,
{
    #[inline]
    pub fn insert(&mut self, key: AstPtr<K>, value: Value<'db, V>) {
        // SAFETY: We only retrieve it with lifetime `'db`.
        let value = unsafe { std::mem::transmute::<Value<'db, V>, Value<'static, V>>(value) };
        self.map
            .map
            .entry::<FxHashMap<AstPtr<K>, Value<'static, V>>>()
            .or_insert_with(Default::default)
            .insert(key, value);
    }

    #[inline]
    pub fn get(&self, key: &AstPtr<K>) -> Option<&Value<'db, V>> {
        let result = self.map.map.get::<FxHashMap<AstPtr<K>, Value<'static, V>>>()?.get(key);
        // SAFETY: We only store with lifetime `'db`.
        unsafe { std::mem::transmute::<Option<&Value<'static, V>>, Option<&Value<'db, V>>>(result) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.map.get::<FxHashMap<AstPtr<K>, Value<'static, V>>>().is_none_or(|it| it.is_empty())
    }
}

impl<'db, K, V: ?Sized> Index<Key<K, V>> for DynMap<'db>
where
    K: AstNode + 'static,
    V: for<'db_> ValueTrait<'db_>,
    Value<'static, V>: 'static,
{
    type Output = KeyMap<'db, Key<K, V>>;
    #[inline]
    fn index(&self, _key: Key<K, V>) -> &Self::Output {
        // SAFETY: Safe due to `#[repr(transparent)]`.
        unsafe { std::mem::transmute::<&DynMap<'db>, &KeyMap<'db, Key<K, V>>>(self) }
    }
}

impl<'db, K, V: ?Sized> IndexMut<Key<K, V>> for DynMap<'db>
where
    K: AstNode + 'static,
    V: for<'db_> ValueTrait<'db_>,
    Value<'static, V>: 'static,
{
    #[inline]
    fn index_mut(&mut self, _key: Key<K, V>) -> &mut Self::Output {
        // SAFETY: Safe due to `#[repr(transparent)]`.
        unsafe { std::mem::transmute::<&mut DynMap<'db>, &mut KeyMap<'db, Key<K, V>>>(self) }
    }
}
