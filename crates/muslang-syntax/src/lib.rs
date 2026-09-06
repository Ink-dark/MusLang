//! MusLang 前端语法层：词法器 + 语法器 + AST。
//!
//! 依据 `spec/grammar.ebnf` v0.1（上游 PRD v0.4.7）。凡 grammar.ebnf 中标注
//! `⚠ 未定` 的产生式，默认按注释倾向实现并在下方记录；此处任何偏离语法的
//! 决定都必须记录在 DEVIATIONS 中，供 M1-1 评审逐条定稿。
//!
//! # DEVIATIONS（grammar.ebnf v0.1 → 本实现的临时裁定）
//!
//! 1. `use ... as default`（D-20 模块级默认分配器）：grammar.ebnf 的
//!    `use_segment` 产生式未含 `as`，但 PRD §3.16 要求该语法。按 PRD 实现
//!    `use_path` 段后可跟 `as 标识符|default`。
//! 2. `self` / `&self` / `&mut self` / `mut self` / `self: T` 参数：grammar.ebnf
//!    标注 ⚠ 未定（隐式首参 vs 独立产生式）。按类 Rust 惯例实现为独立参数形式。
//! 3. 整数字面量 `0b` / `0o` / `0x` 前缀：grammar.ebnf 标注 ⚠「默认纳入」→ 已纳入。
//! 4. 嵌套块注释与 `///` `//!` 文档注释：按 Rust 惯例支持；文档注释当前作为
//!    普通注释忽略（trivia），不进入 AST。
//! 5. `cast_expr` 严格单次 `as`（grammar.ebnf 不允许 `x as a as b` 链）。
//! 6. 作用域出口表达式：block 允许 `defer`/`errdefer` 之后的尾部表达式（grammar
//!    的 block_expr 未禁止；`defer` 与尾表达式共存语义由 M1 语义层裁定）。
//! 7. grammar.ebnf v0.1 **没有结构体字面量表达式产生式**（仅有 struct pattern）。
//!    本实现不擅自补充；`Foo { .. }` 作为表达式将报错，待 M1-1 评审补充产生式。
//! 8. `struct` / `enum` 允许 `where` 子句：grammar.ebnf 产生式缺失，按 FR-001a
//!    （类 Rust 视觉相似）补齐，M1-1 定稿确认。
//! 9. 类型 `(T)`（单元素、无尾逗号）按 Rust 惯例解释为括号类型而非一元组；
//!    一元组须写作 `(T,)`。grammar.ebnf 的 tuple_type 未区分。
//! 10. 借位前缀 `&` / `&mut` / `&&` 并入一元层：grammar.ebnf 把 borrow_op 放在
//!     or_expr 之前会使 `&a || b` 解析为 `&(a || b)`，按 Rust 惯例修正。
//! 11. `type` 作为保留字（type_alias 产生式用到，grammar.ebnf §8 保留字表遗漏）。
//! 12. 属性可附加在任意层级的 item 上：grammar.ebnf 仅允许 program 头部的
//!     属性 run，放宽为类 Rust 惯例。
//! 13. 路径首段允许 `self` / `super` / `crate`（type_path 产生式仅 identifier）。

pub mod ast;
pub mod diag;
pub mod lexer;
pub mod parser;
pub mod token;

pub use diag::{Diagnostic, Span};
pub use parser::parse;

/// 词法 + 语法一站式入口：返回 AST 与全部诊断（词法错误与语法错误合并）。
pub fn parse_source(src: &str) -> (ast::Program, Vec<Diagnostic>) {
    parse(src)
}
