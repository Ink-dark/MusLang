//! 词法器单元测试：grammar.ebnf §7-§8 的关键行为。

use muslang_syntax::lexer::lex;
use muslang_syntax::token::TokenKind as T;

fn kinds(src: &str) -> Vec<T> {
    let (tokens, diags) = lex(src);
    assert!(diags.is_empty(), "unexpected diags: {diags:?}");
    tokens
        .iter()
        .filter(|t| t.kind != T::Eof)
        .map(|t| t.kind)
        .collect()
}

fn texts(src: &str) -> Vec<String> {
    let (tokens, diags) = lex(src);
    assert!(diags.is_empty(), "unexpected diags: {diags:?}");
    tokens
        .iter()
        .filter(|t| t.kind != T::Eof)
        .map(|t| t.text.clone())
        .collect()
}

#[test]
fn keywords_and_idents() {
    let ks = texts("fn let mut const static struct enum trait impl mod use pub");
    assert_eq!(ks.len(), 12);
    let k = kinds("defer errdefer async await move where extern self super crate as");
    assert!(matches!(k[0], T::Defer));
    assert!(matches!(k[10], T::As));
    // allowzero / anyopaque 是保留字
    assert!(matches!(kinds("allowzero anyopaque")[0], T::Allowzero));
    // unsafe / dyn 暂为普通标识符（grammar §8 待定项）
    assert!(matches!(kinds("unsafe dyn")[0], T::Ident));
    // type 保留字（DEVIATIONS #11）
    assert!(matches!(kinds("type")[0], T::TypeKw));
}

#[test]
fn int_literals() {
    assert_eq!(
        texts("42 1_000 0xFF 0o777 0b1010"),
        ["42", "1_000", "0xFF", "0o777", "0b1010"]
    );
    assert!(kinds("42u32").iter().all(|&k| k == T::IntLit));
    assert_eq!(texts("42u32")[0], "42u32");
    // 坏后缀报错
    let (_, diags) = lex("123abc");
    assert!(!diags.is_empty());
    // 坏数字报错（八进制里的 9 → 被当作后缀拒绝）
    let (_, diags) = lex("0o19");
    assert!(!diags.is_empty());
}

#[test]
fn float_literals() {
    assert!(
        kinds("1.0 1e5 1.5e-3 2f32 3.25f64")
            .iter()
            .all(|&k| k == T::FloatLit)
    );
    // `1..2` 是两个区间点，不是浮点
    let k = kinds("1..2");
    assert!(matches!(k[0], T::IntLit));
    assert!(matches!(k[1], T::DotDot));
    assert!(matches!(k[2], T::IntLit));
}

#[test]
fn char_lifetime_strings() {
    // 'a 是生命周期，'a' 是字符
    assert!(matches!(kinds("'a")[0], T::Lifetime));
    assert!(matches!(kinds("'a'")[0], T::CharLit));
    assert!(matches!(kinds("'static")[0], T::Lifetime));
    assert!(matches!(kinds("'\\n'")[0], T::CharLit));
    assert!(matches!(kinds("'\\u{1F600}'")[0], T::CharLit));
    // 多字符字面量报错
    assert!(!lex("'ab'").1.is_empty());
    assert_eq!(texts("\"hi\\n\"")[0], "\"hi\\n\"");
    assert!(matches!(kinds("\"hi\"")[0], T::StrLit));
}

#[test]
fn raw_strings() {
    assert!(matches!(kinds("r\"raw\"")[0], T::RawStrLit));
    assert!(matches!(kinds("r#\"a \"quote\" b\"#")[0], T::RawStrLit));
    assert_eq!(texts("r#\"x\"#")[0], "r#\"x\"#");
    // 未闭合
    assert!(!lex("r\"unclosed").1.is_empty());
}

#[test]
fn nested_block_comments() {
    let k = kinds("a /* /* nested */ still */ b");
    assert_eq!(k.len(), 2); // a、b（Eof 已被 helper 过滤）
    assert!(!lex("/* unterminated").1.is_empty());
    // 行注释吞掉 //
    assert_eq!(kinds("a // comment\nb").len(), 2);
}

#[test]
fn operators_maximal_munch() {
    assert!(matches!(kinds("<<=")[0], T::ShlEq));
    assert!(matches!(kinds(">>=")[0], T::ShrEq));
    assert!(matches!(kinds("..=")[0], T::DotDotEq));
    assert!(matches!(kinds("::")[0], T::ColonColon));
    assert!(matches!(kinds("-> => != == <= >=")[0], T::Arrow));
    // && 单独成 token
    assert!(matches!(kinds("&&")[0], T::AmpAmp));
}

#[test]
fn spans_and_line_col() {
    let src = "fn main() {\n    let x = 1;\n}";
    let (tokens, diags) = lex(src);
    assert!(diags.is_empty());
    let last = &tokens[tokens.len() - 2]; // RBrace
    assert_eq!(last.kind, T::RBrace);
    let (line, col) = muslang_syntax::diag::line_col(src, last.span.start as usize);
    assert_eq!((line, col), (3, 1));
}
