//! 词法单元（token）定义，对应 grammar.ebnf §7-§8。

use crate::diag::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // ---- 字面量 ----
    IntLit,
    FloatLit,
    CharLit,
    StrLit,
    RawStrLit,

    // ---- 标识符与生命周期 ----
    Ident,
    Lifetime, // 'a / 'static（text 含前导 '）

    // ---- 严格保留字（grammar.ebnf §8）----
    Fn,
    Let,
    Mut,
    Const,
    Static,
    Struct,
    Enum,
    Trait,
    Impl,
    Mod,
    Use,
    Pub,
    Match,
    If,
    Else,
    While,
    Loop,
    For,
    In,
    Return,
    Break,
    Continue,
    Defer,
    Errdefer,
    Async,
    Await,
    Move,
    Where,
    Extern,
    SelfKw,
    Super,
    Crate,
    As,
    TypeKw, // "type"（type_alias；grammar.ebnf §8 保留字表遗漏，M1-1 补录）
    True,
    False,
    Allowzero, // *allowzero 指针标注（FR-003）
    Anyopaque, // *anyopaque 指针标注（FR-004）

    // ---- 符号 ----
    Amp,      // &
    AmpAmp,   // &&
    Pipe,     // |
    PipePipe, // ||
    Caret,    // ^
    Shl,      // <<
    Shr,      // >>
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    Eq,       // =
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
    EqEq,       // ==
    Ne,         // !=
    Lt,         // <
    Gt,         // >
    Le,         // <=
    Ge,         // >=
    Bang,       // !
    Question,   // ?
    Dot,        // .
    DotDot,     // ..
    DotDotEq,   // ..=
    ColonColon, // ::
    Comma,      // ,
    Semi,       // ;
    Colon,      // :
    Arrow,      // ->
    FatArrow,   // =>
    Hash,       // #
    At,         // @
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Eof,
}

impl TokenKind {
    pub fn describe(self) -> &'static str {
        use TokenKind::*;
        match self {
            IntLit => "整数字面量",
            FloatLit => "浮点字面量",
            CharLit => "字符字面量",
            StrLit => "字符串字面量",
            RawStrLit => "原始字符串字面量",
            Ident => "标识符",
            Lifetime => "生命周期",
            Eof => "文件结束",
            _ => "记号",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

/// 严格保留字表（grammar.ebnf §8）。`unsafe` / `dyn` 待定项未保留，
/// 当前按普通标识符处理（M1-1 评审项）。
pub fn keyword(text: &str) -> Option<TokenKind> {
    use TokenKind::*;
    Some(match text {
        "fn" => Fn,
        "let" => Let,
        "mut" => Mut,
        "const" => Const,
        "static" => Static,
        "struct" => Struct,
        "enum" => Enum,
        "trait" => Trait,
        "impl" => Impl,
        "mod" => Mod,
        "use" => Use,
        "pub" => Pub,
        "match" => Match,
        "if" => If,
        "else" => Else,
        "while" => While,
        "loop" => Loop,
        "for" => For,
        "in" => In,
        "return" => Return,
        "break" => Break,
        "continue" => Continue,
        "defer" => Defer,
        "errdefer" => Errdefer,
        "async" => Async,
        "await" => Await,
        "move" => Move,
        "where" => Where,
        "extern" => Extern,
        "self" => SelfKw,
        "super" => Super,
        "crate" => Crate,
        "as" => As,
        "type" => TypeKw,
        "true" => True,
        "false" => False,
        "allowzero" => Allowzero,
        "anyopaque" => Anyopaque,
        _ => return None,
    })
}
