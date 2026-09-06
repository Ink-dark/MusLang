//! AST：grammar.ebnf §1-§6 的直接对应。
//!
//! 每个节点携带 `Span`（源码字节区间）；字符串一律使用源码原文切片
//! （字面量不做脱转义，留给语义层/后端）。

use crate::diag::Span;
use crate::token::TokenKind;

// ============================================================ 属性

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub inner: bool, // #![...] 为 true
    pub path: Vec<String>,
    pub payload: Vec<AttrToken>,
    pub span: Span,
}

/// 属性载荷 token（透传给编译期属性解析，不展开为表达式）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrToken {
    Ident(String),
    Str(String),
    Int(String),
    Punct(String),
}

impl AttrToken {
    pub fn from_token(t: &crate::token::Token) -> Self {
        match t.kind {
            TokenKind::IntLit => AttrToken::Int(t.text.clone()),
            TokenKind::StrLit | TokenKind::RawStrLit => AttrToken::Str(t.text.clone()),
            TokenKind::Ident => AttrToken::Ident(t.text.clone()),
            _ => AttrToken::Punct(t.text.clone()),
        }
    }
}

// ============================================================ 顶层

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub inner_attrs: Vec<Attribute>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: ItemKind,
    pub attrs: Vec<Attribute>, // 外围属性 #[...]
    pub vis: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Pub,
    PubCrate,
    PubSelf,
    PubSuper,
    PubIn(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Fn(FnItem),
    Struct(StructItem),
    Enum(EnumItem),
    Trait(TraitItem),
    Impl(ImplItem),
    Mod(ModItem),
    Use(UseItem),
    ExternBlock(ExternBlock),
    TypeAlias(TypeAliasItem),
    Const(ConstItem),
    Static(StaticItem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnItem {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub where_clause: Vec<WherePred>,
    pub body: Option<Block>, // None = 仅声明（trait / extern 块内）
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// `self` 参数（⚠ 未定项裁定：独立形式，见 crate DEVIATIONS #2）
    pub kind: ParamKind,
    pub attrs: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamKind {
    SelfVal {
        is_mut: bool,
    },
    SelfRef {
        is_mut: bool,
        lifetime: Option<String>,
    },
    /// self: Type
    SelfTyped {
        ty: Box<Type>,
        is_mut: bool,
    },
    Pattern {
        pat: Pattern,
        ty: Type,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParam {
    pub name: String, // 'a 时含前导 '
    pub bounds: Vec<GenericBound>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericBound {
    Trait(PathType),
    Lifetime(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WherePred {
    pub subject: WhereSubject,
    pub bounds: Vec<GenericBound>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhereSubject {
    Type(Type),
    /// 'a（含前导 '）
    Lifetime(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructItem {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<Field>, // 空 + 无花哨 = 单元结构体（分号声明）
    pub where_clause: Vec<WherePred>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub vis: Visibility,
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumItem {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub where_clause: Vec<WherePred>,
    pub variants: Vec<Variant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub vis: Visibility,
    pub name: String,
    pub payload: Option<VariantPayload>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantPayload {
    Tuple(Vec<Type>),
    Struct(Vec<Field>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitItem {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub where_clause: Vec<WherePred>,
    pub members: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplItem {
    pub trait_path: Option<PathType>,
    pub self_ty: Type,
    pub generics: Vec<GenericParam>,
    pub where_clause: Vec<WherePred>,
    pub members: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModItem {
    pub name: String,
    pub body: Option<Vec<Item>>, // None = `mod x;` 外部文件
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseItem {
    pub tree: UseTree,
    pub span: Span,
}

/// use 树；`as default` 仅对分配器类型合法（D-20，语义层校验）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseTree {
    Name { name: String, alias: Option<String> },
    Glob, // *
    Nested(Vec<UseTree>),
    Path { prefix: String, rest: Box<UseTree> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternBlock {
    pub abi: Option<String>,
    pub fns: Vec<ExternFn>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternFn {
    pub name: String,
    pub params: Vec<(Pattern, Type)>,
    pub ret: Option<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasItem {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstItem {
    pub name: String,
    pub ty: Type,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticItem {
    pub name: String,
    pub is_mut: bool,
    pub ty: Type,
    pub value: Expr,
    pub span: Span,
}

// ============================================================ 类型

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Ref {
        lifetime: Option<String>,
        is_mut: bool,
        inner: Box<Type>,
    },
    Ptr(PtrKind),
    Array {
        elem: Box<Type>,
        len: Box<Expr>,
    },
    Slice(Box<Type>),
    Tuple(Vec<Type>),
    Path(PathType),
    Never,
    Inferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtrKind {
    Const(Box<Type>),
    Mut(Box<Type>),
    /// *allowzero const T（FR-003）
    AllowzeroConst(Box<Type>),
    /// *allowzero mut T
    AllowzeroMut(Box<Type>),
    /// *anyopaque（FR-004）
    Anyopaque,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathType {
    /// 段：名称 + 可选泛型实参（turbofish 在表达式路径上以 `::` 显式引导）
    pub segments: Vec<PathSegment>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    pub name: String,
    pub generic_args: Vec<Type>,
}

// ============================================================ 模式

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternKind {
    Int(String),
    Float(String),
    Char(String),
    Str(String),
    Bool(bool),
    /// [ref] [mut] 绑定（含 `_` 以外的裸标识符）
    Binding {
        reference: bool,
        is_mut: bool,
        name: String,
    },
    Wildcard,
    Tuple(Vec<Pattern>),
    Struct {
        path: PathType,
        fields: Vec<(String, Option<Pattern>)>,
    },
    Enum {
        path: PathType,
        args: Vec<Pattern>,
    },
    Range {
        start: String,
        inclusive: bool,
        end: String,
    },
    Ref(Box<Pattern>),
}

// ============================================================ 语句与块

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Let {
        pat: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
    },
    Defer(ExprOrBlock),
    Errdefer(ExprOrBlock),
    Expr {
        expr: Expr,
        requires_semi: bool,
    },
    Item(Box<Item>),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprOrBlock {
    Expr(Expr),
    Block(Block),
}

// ============================================================ 表达式

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    BitOr,
    BitXor,
    BitAnd,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Deref,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Int(String),
    Float(String),
    Char(String),
    Str(String),
    RawStr(String),
    Bool(bool),
    Path(PathType),
    Group(Box<Expr>),
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    ArrayRepeat {
        elem: Box<Expr>,
        len: Box<Expr>,
    },
    Block(Block),
    If {
        cond: Box<Expr>,
        then: Block,
        els: Option<Box<Expr>>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Loop(Block),
    While {
        cond: Box<Expr>,
        body: Block,
    },
    WhileLet {
        pat: Pattern,
        expr: Box<Expr>,
        body: Block,
    },
    For {
        pat: Pattern,
        iter: Box<Expr>,
        body: Block,
    },
    Closure {
        is_move: bool,
        params: Vec<Pattern>,
        ret: Option<Type>,
        body: Box<Expr>,
    },
    AsyncBlock {
        is_move: bool,
        body: Block,
    },
    Builtin {
        name: String,
        args: Vec<Expr>,
    }, // @cImport(...) / @ptrCast 等
    Macro {
        name: String,
        body: MacroBody,
    }, // MVP 白名单：panic!/assert!/assert_eq!/unimplemented!
    Return(Option<Box<Expr>>),
    Break(Option<Box<Expr>>),
    Continue,
    Assign {
        op: AssignOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Range {
        op: RangeOp,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    Borrow {
        is_mut: bool,
        inner: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Cast {
        expr: Box<Expr>,
        ty: Box<Type>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Field {
        expr: Box<Expr>,
        name: String,
    },
    TupleField {
        expr: Box<Expr>,
        index: u32,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    Try(Box<Expr>),   // ?（FR-011）
    Await(Box<Expr>), // .await（FR-043）
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeOp {
    Exclusive, // ..
    Inclusive, // ..=
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pat: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

/// 宏体：MVP 仅 `name!(tokens)` 与 `name![tokens]` / `name!{tokens}`（透传 token）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroBody {
    Paren(Vec<AttrToken>),
    Bracket(Vec<AttrToken>),
    Brace(Vec<crate::token::Token>),
}
