//! 夹具集成测试：PRD §9 hello.mus 必须零诊断通过；kitchen_sink 覆盖全 item 面。

use muslang_syntax::ast::*;
use muslang_syntax::parse;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(path).expect("fixture readable")
}

#[test]
fn prd_hello_mus_parses_clean() {
    let src = fixture("hello.mus");
    let (prog, diags) = parse(&src);
    assert!(diags.is_empty(), "hello.mus must parse clean: {diags:?}");
    assert_eq!(prog.items.len(), 3);
    // use std::net::http;
    assert!(matches!(prog.items[0].kind, ItemKind::Use(_)));
    // extern "C" { fn v8_init() -> i32; }
    let ItemKind::ExternBlock(ext) = &prog.items[1].kind else {
        panic!()
    };
    assert_eq!(ext.fns.len(), 1);
    // fn main
    let ItemKind::Fn(f) = &prog.items[2].kind else {
        panic!()
    };
    assert_eq!(f.name, "main");
    // 闭包体为 async block（§3.15.3 Handler blanket impl）
    let ItemKind::Fn(_) = &prog.items[2].kind else {
        panic!()
    };
    let f = match &prog.items[2].kind {
        ItemKind::Fn(f) => f,
        _ => unreachable!(),
    };
    let has_async_block = f.body.as_ref().unwrap().stmts.iter().any(|s| {
        matches!(
            &s.kind,
            StmtKind::Expr { expr, .. }
                if matches!(&expr.kind,
                    ExprKind::Call { args, .. }
                        if args.iter().any(|a| matches!(a.kind, ExprKind::Closure { .. })))
        )
    });
    assert!(has_async_block, "server.handle 的第二实参应为闭包");
}

#[test]
fn kitchen_sink_parses_clean() {
    let src = fixture("kitchen_sink.mus");
    let (prog, diags) = parse(&src);
    assert!(diags.is_empty(), "kitchen_sink must parse clean: {diags:?}");
    // #![no_std] 内部属性
    assert_eq!(prog.inner_attrs.len(), 1);
    let names: Vec<&str> = prog
        .items
        .iter()
        .filter_map(|i| match &i.kind {
            ItemKind::Fn(f) => Some(f.name.as_str()),
            ItemKind::Struct(s) => Some(s.name.as_str()),
            ItemKind::Enum(e) => Some(e.name.as_str()),
            ItemKind::Trait(t) => Some(t.name.as_str()),
            ItemKind::Mod(m) => Some(m.name.as_str()),
            _ => None,
        })
        .collect();
    for expected in ["alloc_map", "Point", "Shape", "Draw", "control", "fetch"] {
        assert!(
            names.contains(&expected),
            "missing item {expected}, got {names:?}"
        );
    }
}
