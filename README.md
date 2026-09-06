# MusLang
一个更好的编程语言。A better language like Rust.

## 仓库结构

- `docs/` — PRD（唯一权威）、CHANGELOG、进度看板、M1-0 决策冻结草案
- `spec/` — 规格草案：语法 EBNF、内存模型、C99 后端、std/sys、unsafe
- `crates/` — 编译器工程（Rust，阶段一 Bootstrap）
  - `muslang-syntax` — 词法器 + 语法器 + AST，依据 `spec/grammar.ebnf` v0.1；
    对 ⚠ 未定项的临时裁定记录于 crate 文档 `DEVIATIONS`，供 M1-1 评审定稿
  - `muslangc` — 编译器 CLI（`lex` / `parse` / `version`）

## 快速开始

```bash
cargo test --workspace
cargo run -p muslangc -- parse crates/muslang-syntax/tests/fixtures/hello.mus
cargo run -p muslangc -- version
```

> muslangc v0.1 仅读取当前工作目录树内的 `.mus` 源文件（路径穿越防护）。
