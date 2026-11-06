//! This module describes hir-level representation of expressions.
//!
//! This representation is:
//!
//! 1. Identity-based. Each expression has an `id`, so we can distinguish
//!    between different `1` in `1 + 1`.
//! 2. Independent of syntax. Though syntactic provenance information can be
//!    attached separately via id-based side map.
//! 3. Unresolved. Paths are stored as sequences of names, and not as defs the
//!    names refer to.
//! 4. Desugared. There's no `if let`.
//!
//! See also a neighboring [`body`] module.
//!
//! [`body`]: crate::expr_store::body

pub mod format_args;
pub mod generics;
pub mod type_ref;

use std::{fmt, marker::PhantomData, mem};

use hir_expand::{MacroDefId, name::Name};
use intern::Symbol;
use la_arena::{Idx, RawIdx};
use rustc_apfloat::ieee::{Double, Half, Quad, Single};
use salsa::SalsaValue;
use syntax::ast;
use thin_vec::ThinVec;
use type_ref::TypeRefId;

use crate::{
    BlockId,
    builtin_type::{BuiltinFloat, BuiltinInt, BuiltinUint},
    expr_store::{
        HygieneId,
        path::{GenericArgs, Path},
    },
    type_ref::{Mutability, Rawness},
};

pub use syntax::ast::{ArithOp, BinaryOp, CmpOp, LogicOp, Ordering, RangeOp, UnaryOp};

pub type BindingId = Idx<Binding>;

pub type ExprId<'db> = Idx<Expr<'db>>;

pub type PatId<'db> = Idx<Pat<'db>>;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, SalsaValue)]
pub enum ExprOrPatId<'db> {
    ExprId(ExprId<'db>),
    PatId(PatId<'db>),
}

impl<'db> ExprOrPatId<'db> {
    pub fn as_expr(self) -> Option<ExprId<'db>> {
        match self {
            Self::ExprId(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_expr(&self) -> bool {
        matches!(self, Self::ExprId(_))
    }

    pub fn as_pat(self) -> Option<PatId<'db>> {
        match self {
            Self::PatId(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_pat(&self) -> bool {
        matches!(self, Self::PatId(_))
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, salsa::SalsaValue)]
pub struct ExprOrPatIdPacked(u32);

const _: () = assert!(mem::size_of::<ExprOrPatIdPacked>() == mem::size_of::<u32>());

impl ExprOrPatIdPacked {
    const PAT_BIT: u32 = 1 << (u32::BITS - 1);
    const INDEX_MASK: u32 = !Self::PAT_BIT;

    pub fn unpack(self) -> ExprOrPatId<'static> {
        match self.is_expr() {
            true => ExprOrPatId::ExprId(ExprId::from_raw(RawIdx::from_u32(self.0))),
            false => {
                ExprOrPatId::PatId(PatId::from_raw(RawIdx::from_u32(self.0 & Self::INDEX_MASK)))
            }
        }
    }

    #[inline]
    pub fn as_expr(self) -> Option<ExprId<'static>> {
        self.is_expr().then(|| ExprId::from_raw(RawIdx::from_u32(self.0)))
    }

    #[inline]
    pub fn is_expr(&self) -> bool {
        self.0 & Self::PAT_BIT == 0
    }

    #[inline]
    pub fn as_pat(self) -> Option<PatId<'static>> {
        self.is_pat().then(|| PatId::from_raw(RawIdx::from_u32(self.0 & Self::INDEX_MASK)))
    }

    #[inline]
    pub fn is_pat(&self) -> bool {
        self.0 & Self::PAT_BIT != 0
    }
}

impl fmt::Debug for ExprOrPatIdPacked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.unpack() {
            ExprOrPatId::ExprId(id) => f.debug_tuple("ExprId").field(&id).finish(),
            ExprOrPatId::PatId(id) => f.debug_tuple("PatId").field(&id).finish(),
        }
    }
}

impl<'db> From<ExprId<'db>> for ExprOrPatIdPacked {
    fn from(value: ExprId<'db>) -> Self {
        let value = value.into_raw().into_u32();
        // virtually impossible to have IDs that high
        debug_assert_eq!(value & Self::PAT_BIT, 0);
        Self(value)
    }
}

impl<'db> From<PatId<'db>> for ExprOrPatId<'db> {
    fn from(value: PatId<'db>) -> Self {
        ExprOrPatId::PatId(value)
    }
}
impl<'db> From<ExprId<'db>> for ExprOrPatId<'db> {
    fn from(value: ExprId<'db>) -> Self {
        ExprOrPatId::ExprId(value)
    }
}

impl<'db> From<PatId<'db>> for ExprOrPatIdPacked {
    fn from(value: PatId<'db>) -> Self {
        let value = value.into_raw().into_u32();
        // virtually impossible to have IDs that high
        debug_assert_eq!(value & Self::PAT_BIT, 0);
        Self(value | Self::PAT_BIT)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Label {
    pub name: Name,
}
pub type LabelId = Idx<Label>;

// We leave float values as a string to avoid double rounding.
// For PartialEq, string comparison should work, as ordering is not important
// https://github.com/rust-lang/rust-analyzer/issues/12380#issuecomment-1137284360
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FloatTypeWrapper(Symbol);

// FIXME(#17451): Use builtin types once stabilised.
impl FloatTypeWrapper {
    pub fn new(sym: Symbol) -> Self {
        Self(sym)
    }

    pub fn to_f128(&self) -> Quad {
        self.0.as_str().parse().unwrap_or_default()
    }

    pub fn to_f64(&self) -> Double {
        self.0.as_str().parse().unwrap_or_default()
    }

    pub fn to_f32(&self) -> Single {
        self.0.as_str().parse().unwrap_or_default()
    }

    pub fn to_f16(&self) -> Half {
        self.0.as_str().parse().unwrap_or_default()
    }
}

impl fmt::Display for FloatTypeWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Literal {
    String(Symbol),
    ByteString(Box<[u8]>),
    CString(Box<[u8]>),
    Char(char),
    Bool(bool),
    Int(i128, Option<BuiltinInt>),
    Uint(u128, Option<BuiltinUint>),
    // Here we are using a wrapper around float because float primitives do not implement Eq, so they
    // could not be used directly here, to understand how the wrapper works go to definition of
    // FloatTypeWrapper
    Float(FloatTypeWrapper, Option<BuiltinFloat>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Used in range patterns.
pub enum LiteralOrConst<'db> {
    Literal(Literal),
    Const(PatId<'db>),
}

impl Literal {
    pub fn negate(self) -> Option<Self> {
        if let Literal::Int(i, k) = self { Some(Literal::Int(-i, k)) } else { None }
    }
}

impl From<ast::LiteralKind> for Literal {
    fn from(ast_lit_kind: ast::LiteralKind) -> Self {
        use ast::LiteralKind;
        match ast_lit_kind {
            LiteralKind::IntNumber(lit) => {
                if let builtin @ Some(_) = lit.suffix().and_then(BuiltinFloat::from_suffix) {
                    Literal::Float(
                        FloatTypeWrapper::new(Symbol::intern(&lit.value_string())),
                        builtin,
                    )
                } else if let builtin @ Some(_) = lit.suffix().and_then(BuiltinUint::from_suffix) {
                    Literal::Uint(lit.value().unwrap_or(0), builtin)
                } else {
                    let builtin = lit.suffix().and_then(BuiltinInt::from_suffix);
                    Literal::Int(lit.value().unwrap_or(0) as i128, builtin)
                }
            }
            LiteralKind::FloatNumber(lit) => {
                let ty = lit.suffix().and_then(BuiltinFloat::from_suffix);
                Literal::Float(FloatTypeWrapper::new(Symbol::intern(&lit.value_string())), ty)
            }
            LiteralKind::ByteString(bs) => {
                let text = bs.value().map_or_else(|_| Default::default(), Box::from);
                Literal::ByteString(text)
            }
            LiteralKind::String(s) => {
                let text = s.value().map_or_else(|_| Symbol::empty(), |it| Symbol::intern(&it));
                Literal::String(text)
            }
            LiteralKind::CString(s) => {
                let text = s.value().map_or_else(|_| Default::default(), Box::from);
                Literal::CString(text)
            }
            LiteralKind::Byte(b) => {
                Literal::Uint(b.value().unwrap_or_default() as u128, Some(BuiltinUint::U8))
            }
            LiteralKind::Char(c) => Literal::Char(c.value().unwrap_or_default()),
            LiteralKind::Bool(val) => Literal::Bool(val),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, SalsaValue)]
pub enum RecordSpread<'db> {
    None,
    FieldDefaults,
    Expr(ExprId<'db>),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Unsafe {
    Yes,
    No,
}

#[derive(Debug, Clone, Eq, PartialEq, SalsaValue)]
pub enum Expr<'db> {
    /// This is produced if the syntax tree does not have a required expression piece.
    Missing,
    Path(Path<'db>),
    If {
        condition: ExprId<'db>,
        then_branch: ExprId<'db>,
        else_branch: Option<ExprId<'db>>,
    },
    Let {
        pat: PatId<'db>,
        expr: ExprId<'db>,
    },
    Block {
        id: Option<BlockId>,
        statements: Box<[Statement<'db>]>,
        tail: Option<ExprId<'db>>,
        label: Option<LabelId>,
        unsafe_: Unsafe,
    },
    Const(ExprId<'db>),
    Loop {
        body: ExprId<'db>,
        label: Option<LabelId>,
        source: LoopSource,
    },
    Call {
        callee: ExprId<'db>,
        args: Box<[ExprId<'db>]>,
    },
    MethodCall {
        receiver: ExprId<'db>,
        method_name: Name,
        args: Box<[ExprId<'db>]>,
        generic_args: Option<Box<GenericArgs<'db>>>,
    },
    Match {
        expr: ExprId<'db>,
        arms: Box<[MatchArm<'db>]>,
    },
    Continue {
        label: Option<LabelId>,
    },
    Break {
        expr: Option<ExprId<'db>>,
        label: Option<LabelId>,
    },
    Return {
        expr: Option<ExprId<'db>>,
    },
    Become {
        expr: ExprId<'db>,
    },
    Yield {
        expr: Option<ExprId<'db>>,
    },
    Yeet {
        expr: Option<ExprId<'db>>,
    },
    RecordLit {
        path: Path<'db>,
        fields: ThinVec<RecordLitField<'db>>,
        spread: RecordSpread<'db>,
    },
    Field {
        expr: ExprId<'db>,
        name: Name,
    },
    Await {
        expr: ExprId<'db>,
    },
    Cast {
        expr: ExprId<'db>,
        type_ref: TypeRefId<'db>,
    },
    Ref {
        expr: ExprId<'db>,
        rawness: Rawness,
        mutability: Mutability,
    },
    UnaryOp {
        expr: ExprId<'db>,
        op: UnaryOp,
    },
    /// `op` cannot be bare `=` (but can be `op=`), these are lowered to `Assignment` instead.
    BinaryOp {
        lhs: ExprId<'db>,
        rhs: ExprId<'db>,
        op: Option<BinaryOp>,
    },
    // Assignments need a special treatment because of destructuring assignment.
    Assignment {
        target: PatId<'db>,
        value: ExprId<'db>,
    },
    Index {
        base: ExprId<'db>,
        index: ExprId<'db>,
    },
    Closure {
        args: Box<[PatId<'db>]>,
        arg_types: Box<[Option<TypeRefId<'db>>]>,
        ret_type: Option<TypeRefId<'db>>,
        body: ExprId<'db>,
        closure_kind: ClosureKind,
        capture_by: CaptureBy,
    },
    Tuple {
        exprs: Box<[ExprId<'db>]>,
    },
    Array(Array<'db>),
    Literal(Literal),
    Underscore,
    OffsetOf(OffsetOf<'db>),
    InlineAsm(InlineAsm<'db>),
    IncludeBytes,
}

#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Expr<'_>>() == 48);

impl<'db> Expr<'db> {
    pub fn precedence(&self) -> ast::prec::ExprPrecedence {
        use ast::prec::ExprPrecedence;

        match self {
            Expr::Array(_)
            | Expr::InlineAsm(_)
            | Expr::Block { .. }
            | Expr::Const(_)
            | Expr::If { .. }
            | Expr::Literal(_)
            | Expr::Loop { .. }
            | Expr::Match { .. }
            | Expr::Missing
            | Expr::Path(_)
            | Expr::RecordLit { .. }
            | Expr::Tuple { .. }
            | Expr::OffsetOf(_)
            | Expr::Underscore
            | Expr::IncludeBytes => ExprPrecedence::Unambiguous,

            Expr::Await { .. }
            | Expr::Call { .. }
            | Expr::Field { .. }
            | Expr::Index { .. }
            | Expr::MethodCall { .. } => ExprPrecedence::Postfix,

            Expr::Let { .. } | Expr::UnaryOp { .. } | Expr::Ref { .. } => ExprPrecedence::Prefix,

            Expr::Cast { .. } => ExprPrecedence::Cast,

            Expr::BinaryOp { op, .. } => match op {
                None => ExprPrecedence::Unambiguous,
                Some(BinaryOp::LogicOp(LogicOp::Or)) => ExprPrecedence::LOr,
                Some(BinaryOp::LogicOp(LogicOp::And)) => ExprPrecedence::LAnd,
                Some(BinaryOp::CmpOp(_)) => ExprPrecedence::Compare,
                Some(BinaryOp::Assignment { .. }) => ExprPrecedence::Assign,
                Some(BinaryOp::ArithOp(arith_op)) => match arith_op {
                    ArithOp::Add | ArithOp::Sub => ExprPrecedence::Sum,
                    ArithOp::Mul | ArithOp::Div | ArithOp::Rem => ExprPrecedence::Product,
                    ArithOp::Shl | ArithOp::Shr => ExprPrecedence::Shift,
                    ArithOp::BitXor => ExprPrecedence::BitXor,
                    ArithOp::BitOr => ExprPrecedence::BitOr,
                    ArithOp::BitAnd => ExprPrecedence::BitAnd,
                },
            },

            Expr::Assignment { .. } => ExprPrecedence::Assign,

            Expr::Become { .. }
            | Expr::Break { .. }
            | Expr::Closure { .. }
            | Expr::Return { .. }
            | Expr::Yeet { .. }
            | Expr::Yield { .. } => ExprPrecedence::Jump,

            Expr::Continue { .. } => ExprPrecedence::Unambiguous,
        }
    }
}

/// The loop type that yielded an `Expr::Loop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopSource {
    /// A `loop { .. }` loop.
    Loop,
    /// A `while _ { .. }` loop.
    While,
    /// A `for _ in _ { .. }` loop.
    ForLoop,
}

#[derive(Debug, Clone, PartialEq, Eq, SalsaValue)]
pub struct OffsetOf<'db> {
    pub container: TypeRefId<'db>,
    pub fields: Box<[Name]>,
}

#[derive(Debug, Clone, PartialEq, Eq, SalsaValue)]
pub struct InlineAsm<'db> {
    pub operands: Box<[(Option<Name>, AsmOperand<'db>)]>,
    pub options: AsmOptions,
    pub kind: InlineAsmKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineAsmKind {
    /// `asm!()`.
    Asm,
    /// `global_asm!()`.
    GlobalAsm,
    /// `naked_asm!()`.
    NakedAsm,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsmOptions(u16);
bitflags::bitflags! {
    impl AsmOptions: u16 {
        const PURE            = 1 << 0;
        const NOMEM           = 1 << 1;
        const READONLY        = 1 << 2;
        const PRESERVES_FLAGS = 1 << 3;
        const NORETURN        = 1 << 4;
        const NOSTACK         = 1 << 5;
        const ATT_SYNTAX      = 1 << 6;
        const RAW             = 1 << 7;
        const MAY_UNWIND      = 1 << 8;
    }
}

impl AsmOptions {
    pub const COUNT: usize = Self::all().bits().count_ones() as usize;

    pub const GLOBAL_OPTIONS: Self = Self::ATT_SYNTAX.union(Self::RAW);
    pub const NAKED_OPTIONS: Self = Self::ATT_SYNTAX.union(Self::RAW).union(Self::NORETURN);

    pub fn human_readable_names(&self) -> Vec<&'static str> {
        let mut options = vec![];

        if self.contains(AsmOptions::PURE) {
            options.push("pure");
        }
        if self.contains(AsmOptions::NOMEM) {
            options.push("nomem");
        }
        if self.contains(AsmOptions::READONLY) {
            options.push("readonly");
        }
        if self.contains(AsmOptions::PRESERVES_FLAGS) {
            options.push("preserves_flags");
        }
        if self.contains(AsmOptions::NORETURN) {
            options.push("noreturn");
        }
        if self.contains(AsmOptions::NOSTACK) {
            options.push("nostack");
        }
        if self.contains(AsmOptions::ATT_SYNTAX) {
            options.push("att_syntax");
        }
        if self.contains(AsmOptions::RAW) {
            options.push("raw");
        }
        if self.contains(AsmOptions::MAY_UNWIND) {
            options.push("may_unwind");
        }

        options
    }
}

impl std::fmt::Debug for AsmOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, SalsaValue)]
pub enum AsmOperand<'db> {
    In {
        reg: InlineAsmRegOrRegClass,
        expr: ExprId<'db>,
    },
    Out {
        reg: InlineAsmRegOrRegClass,
        expr: Option<ExprId<'db>>,
        late: bool,
    },
    InOut {
        reg: InlineAsmRegOrRegClass,
        expr: ExprId<'db>,
        late: bool,
    },
    SplitInOut {
        reg: InlineAsmRegOrRegClass,
        in_expr: ExprId<'db>,
        out_expr: Option<ExprId<'db>>,
        late: bool,
    },
    Label(ExprId<'db>),
    Const(ExprId<'db>),
    Sym(Path<'db>),
}

impl<'db> AsmOperand<'db> {
    pub fn reg(&self) -> Option<&InlineAsmRegOrRegClass> {
        match self {
            Self::In { reg, .. }
            | Self::Out { reg, .. }
            | Self::InOut { reg, .. }
            | Self::SplitInOut { reg, .. } => Some(reg),
            Self::Const { .. } | Self::Sym { .. } | Self::Label { .. } => None,
        }
    }

    pub fn is_clobber(&self) -> bool {
        matches!(self, AsmOperand::Out { reg: InlineAsmRegOrRegClass::Reg(_), late: _, expr: None })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum InlineAsmRegOrRegClass {
    Reg(Symbol),
    RegClass(Symbol),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoroutineKind {
    Async,
    Gen,
    AsyncGen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClosureKind {
    Closure,
    OldCoroutine(Movability),
    Coroutine { kind: CoroutineKind, source: CoroutineSource },
    CoroutineClosure(CoroutineKind),
}

/// In the case of a coroutine created as part of an async/gen construct,
/// which kind of async/gen construct caused it to be created?
///
/// This helps error messages but is also used to drive coercions in
/// type-checking (see #60424).
#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy)]
pub enum CoroutineSource {
    /// An explicit `async`/`gen` block written by the user.
    Block,

    /// An explicit `async`/`gen` closure written by the user.
    Closure,

    /// The `async`/`gen` block generated as the body of an async/gen function.
    Fn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBy {
    /// `move |x| y + x`.
    Value,
    /// `move` keyword was not specified.
    Ref,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Movability {
    Static,
    Movable,
}

#[derive(Debug, Clone, Eq, PartialEq, SalsaValue)]
pub enum Array<'db> {
    ElementList { elements: Box<[ExprId<'db>]> },
    Repeat { initializer: ExprId<'db>, repeat: ExprId<'db> },
}

#[derive(Debug, Clone, Eq, PartialEq, SalsaValue)]
pub struct MatchArm<'db> {
    pub pat: PatId<'db>,
    pub guard: Option<ExprId<'db>>,
    pub expr: ExprId<'db>,
}

#[derive(Debug, Clone, Eq, PartialEq, SalsaValue)]
pub struct RecordLitField<'db> {
    pub name: Name,
    pub expr: ExprId<'db>,
}

#[derive(Debug, Clone, Eq, PartialEq, SalsaValue)]
pub enum Statement<'db> {
    Let {
        pat: PatId<'db>,
        type_ref: Option<TypeRefId<'db>>,
        initializer: Option<ExprId<'db>>,
        else_branch: Option<ExprId<'db>>,
    },
    Expr {
        expr: ExprId<'db>,
        has_semi: bool,
    },
    Item(Item<'db>),
}

#[derive(Debug, Clone, PartialEq, Eq, SalsaValue)]
pub enum Item<'db> {
    // FIXME(ada4a): the `PhantomData` is a stand-in for the lifetime that `MacroDefId` will get
    // once we add a lifetime to `MacroCallId`. Remove it when we do.
    MacroDef(Box<MacroDefId>, PhantomData<&'db ()>),
    Other,
}

/// Explicit binding annotations given in the HIR for a binding. Note
/// that this is not the final binding *mode* that we infer after type
/// inference.
#[derive(Clone, PartialEq, Eq, Debug, Copy)]
pub enum BindingAnnotation {
    /// No binding annotation given: this means that the final binding mode
    /// will depend on whether we have skipped through a `&` reference
    /// when matching. For example, the `x` in `Some(x)` will have binding
    /// mode `None`; if you do `let Some(x) = &Some(22)`, it will
    /// ultimately be inferred to be by-reference.
    Unannotated,

    /// Annotated with `mut x` -- could be either ref or not, similar to `None`.
    Mutable,

    /// Annotated as `ref`, like `ref x`
    Ref,

    /// Annotated as `ref mut x`.
    RefMut,
}

impl BindingAnnotation {
    pub fn new(is_mutable: bool, is_ref: bool) -> Self {
        match (is_mutable, is_ref) {
            (true, true) => BindingAnnotation::RefMut,
            (false, true) => BindingAnnotation::Ref,
            (true, false) => BindingAnnotation::Mutable,
            (false, false) => BindingAnnotation::Unannotated,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BindingProblems {
    /// <https://doc.rust-lang.org/stable/error_codes/E0416.html>
    BoundMoreThanOnce,
    /// <https://doc.rust-lang.org/stable/error_codes/E0409.html>
    BoundInconsistently,
    /// <https://doc.rust-lang.org/stable/error_codes/E0408.html>
    NotBoundAcrossAll,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Binding {
    pub name: Name,
    pub mode: BindingAnnotation,
    pub problems: Option<BindingProblems>,
    /// Note that this may not be the direct `SyntaxContextId` of the binding's expansion, because transparent
    /// expansions are attributed to their parent expansion (recursively).
    pub hygiene: HygieneId,
}

#[derive(Debug, Clone, Eq, PartialEq, SalsaValue)]
pub struct RecordFieldPat<'db> {
    pub name: Name,
    pub pat: PatId<'db>,
}

/// Close relative to rustc's hir::PatKind
#[derive(Debug, Clone, Eq, PartialEq, SalsaValue)]
pub enum Pat<'db> {
    Missing,
    /// A rest pattern. Not valid outside special context.
    Rest,
    Wild,
    Tuple {
        args: Box<[PatId<'db>]>,
        ellipsis: Option<u32>,
    },
    Or(Box<[PatId<'db>]>),
    Record {
        path: Path<'db>,
        args: Box<[RecordFieldPat<'db>]>,
        ellipsis: bool,
    },
    Range {
        start: Option<ExprId<'db>>,
        end: Option<ExprId<'db>>,
        range_type: RangeOp,
    },
    Slice {
        prefix: Box<[PatId<'db>]>,
        slice: Option<PatId<'db>>,
        suffix: Box<[PatId<'db>]>,
    },
    Path(Path<'db>),
    Lit(ExprId<'db>),
    Bind {
        id: BindingId,
        subpat: Option<PatId<'db>>,
    },
    TupleStruct {
        path: Path<'db>,
        args: Box<[PatId<'db>]>,
        ellipsis: Option<u32>,
    },
    Ref {
        pat: PatId<'db>,
        mutability: Mutability,
    },
    Box {
        inner: PatId<'db>,
    },
    Deref {
        inner: PatId<'db>,
    },
    NotNull,
    /// An expression inside a pattern. That can only occur inside assignments.
    ///
    /// E.g. in `(a, *b) = (1, &mut 2)`, `*b` is an expression.
    Expr(ExprId<'db>),
}

impl<'db> Pat<'db> {
    pub fn walk_child_pats(&self, mut f: impl FnMut(PatId<'db>)) {
        match self {
            Pat::Range { .. }
            | Pat::Lit(..)
            | Pat::Path(..)
            | Pat::Wild
            | Pat::Missing
            | Pat::Rest
            | Pat::Expr(_)
            | Pat::NotNull => {}
            Pat::Bind { subpat, .. } => {
                subpat.iter().copied().for_each(f);
            }
            Pat::Or(args) | Pat::Tuple { args, .. } | Pat::TupleStruct { args, .. } => {
                args.iter().copied().for_each(f);
            }
            Pat::Ref { pat, .. } => f(*pat),
            Pat::Slice { prefix, slice, suffix } => {
                let total_iter = prefix.iter().chain(slice.iter()).chain(suffix.iter());
                total_iter.copied().for_each(f);
            }
            Pat::Record { args, .. } => {
                args.iter().map(|f| f.pat).for_each(f);
            }
            Pat::Box { inner } | Pat::Deref { inner } => f(*inner),
        }
    }
}
