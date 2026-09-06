//! muslangc v0.1 —— 端到端前端管线 CLI。
//!
//! 子命令：
//! - `lex <file>`   打印 token 流
//! - `parse <file>` 解析并打印 AST（Debug 格式）
//! - `version`      打印版本
//!
//! 输入沙箱：仅接受当前工作目录树内的 `.mus` 源文件；路径在 CLI 边界
//! （main）一次性完成 canonicalize + 根目录前缀校验，通过后存入
//! `SOURCE`，worker 函数只消费已校验路径。

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

use muslang_syntax::{Diagnostic, parse};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 经边界校验的源文件路径（validate_source_path 成功后唯一写入点）。
static SOURCE: OnceLock<PathBuf> = OnceLock::new();

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("lex") if args.len() == 2 => {
            if let Err(code) = validate_source_path(&args[1]) {
                return code;
            }
            run_lex()
        }
        Some("parse") if args.len() == 2 => {
            if let Err(code) = validate_source_path(&args[1]) {
                return code;
            }
            run_parse()
        }
        Some("version") | Some("--version") | Some("-V") => {
            println!("muslangc {VERSION}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("muslangc v{VERSION} — MusLang 编译器（M1 前端管线）");
            eprintln!();
            eprintln!("用法: muslangc <命令> <file.mus>");
            eprintln!("  lex <file>    打印 token 流");
            eprintln!("  parse <file>  解析并打印 AST");
            eprintln!("  version       打印版本");
            ExitCode::from(2)
        }
    }
}

/// CLI 边界校验：仅接受 `.mus` 扩展名、且 canonicalize 后位于当前工作
/// 目录树内的路径（物理消解 `..` 与符号链接，防路径穿越）。
/// 通过后写入 `SOURCE`。
fn validate_source_path(raw: &str) -> Result<(), ExitCode> {
    let p = std::path::Path::new(raw);
    if p.extension().map(|e| e != "mus").unwrap_or(true) {
        eprintln!("muslangc: 仅接受 .mus 源文件: {raw}");
        return Err(ExitCode::from(2));
    }
    let canonical = p.canonicalize().map_err(|e| {
        eprintln!("muslangc: 无法解析路径 {raw}: {e}");
        ExitCode::from(2)
    })?;
    let root = std::env::current_dir()
        .map_err(|e| {
            eprintln!("muslangc: 无法确定工作目录: {e}");
            ExitCode::from(2)
        })?
        .canonicalize()
        .map_err(|e| {
            eprintln!("muslangc: 无法规范化工作目录: {e}");
            ExitCode::from(2)
        })?;
    if !canonical.starts_with(&root) {
        eprintln!("muslangc: 源文件必须位于当前工作目录内（v0.1 读取范围）: {raw}");
        return Err(ExitCode::from(2));
    }
    SOURCE.set(canonical).map_err(|_| {
        eprintln!("muslangc: 源文件路径已设置");
        ExitCode::from(2)
    })
}

fn report_diags(src: &str, diags: &[Diagnostic]) -> bool {
    for d in diags {
        eprintln!("muslangc: {}", d.render(src));
    }
    !diags.is_empty()
}

fn run_lex() -> ExitCode {
    let Some(path) = SOURCE.get() else {
        return ExitCode::from(2);
    };
    let Ok(src) = std::fs::read_to_string(path) else {
        eprintln!("muslangc: 无法读取 {}", path.display());
        return ExitCode::from(2);
    };
    let (tokens, diags) = muslang_syntax::lexer::lex(&src);
    let mut out = String::new();
    for t in &tokens {
        let text = if t.text.is_empty() {
            String::new()
        } else {
            format!(" {:?}", t.text)
        };
        let _ = writeln!(
            out,
            "{:?}{} @ {}..{}",
            t.kind, text, t.span.start, t.span.end
        );
    }
    print!("{out}");
    if report_diags(&src, &diags) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_parse() -> ExitCode {
    let Some(path) = SOURCE.get() else {
        return ExitCode::from(2);
    };
    let Ok(src) = std::fs::read_to_string(path) else {
        eprintln!("muslangc: 无法读取 {}", path.display());
        return ExitCode::from(2);
    };
    let (program, diags) = parse(&src);
    println!("{program:#?}");
    if report_diags(&src, &diags) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
