//! 解析器单元测试：AST 形状与错误恢复。

use muslang_syntax::ast::*;
use muslang_syntax::parse;

fn parse_ok(src: &str) -> Program {
    let (prog, diags) = parse(src);
    assert!(diags.is_empty(), "unexpected diags: {diags:?}");
    prog
}

#[test]
fn hello_program_shape() {
    let src = r#"
extern "C" {
    fn v8_init() -> i32;
}

fn main() {
    let ret = v8_init();
    if ret != 0 {
        panic!("v8 init failed");
    }
}
"#;
    let prog = parse_ok(src);
    assert!(prog.inner_attrs.is_empty());
    assert_eq!(prog.items.len(), 2);
    let ItemKind::ExternBlock(ext) = &prog.items[0].kind else {
        panic!("expected extern block")
    };
    assert_eq!(ext.abi.as_deref(), Some("\"C\""));
    assert_eq!(ext.fns[0].name, "v8_init");
    let ItemKind::Fn(f) = &prog.items[1].kind else {
        panic!("expected fn")
    };
    assert_eq!(f.name, "main");
    let StmtKind::Let {
        init: Some(init), ..
    } = &f.body.as_ref().unwrap().stmts[0].kind
    else {
        panic!("expected let")
    };
    match &init.kind {
        ExprKind::Call { callee, args } => {
            assert!(matches!(callee.kind, ExprKind::Path(_)));
            assert!(args.is_empty());
        }
        _ => panic!("expected call, got {init:?}"),
    }
}

#[test]
fn precedence_tree() {
    let src = "fn f() { let x = 1 + 2 * 3 == 7 && true || false; }";
    let prog = parse_ok(src);
    let ItemKind::Fn(f) = &prog.items[0].kind else {
        panic!()
    };
    let StmtKind::Let {
        init: Some(init), ..
    } = &f.body.as_ref().unwrap().stmts[0].kind
    else {
        panic!()
    };
    // || 最低
    let ExprKind::Binary {
        op: BinOp::Or, lhs, ..
    } = &init.kind
    else {
        panic!("expected || at root, got {init:?}")
    };
    // && 次之
    let ExprKind::Binary { op: BinOp::And, .. } = &lhs.kind else {
        panic!("expected && on left, got {lhs:?}")
    };
}

#[test]
fn assign_is_right_assoc_range_binds_tighter() {
    let src = "fn f() { a = b..c; d = e = g; }";
    let prog = parse_ok(src);
    let ItemKind::Fn(f) = &prog.items[0].kind else {
        panic!()
    };
    let stmts = &f.body.as_ref().unwrap().stmts;
    let StmtKind::Expr { expr, .. } = &stmts[0].kind else {
        panic!()
    };
    let ExprKind::Assign { rhs, .. } = &expr.kind else {
        panic!()
    };
    assert!(matches!(rhs.kind, ExprKind::Range { .. }));
    let StmtKind::Expr { expr, .. } = &stmts[1].kind else {
        panic!()
    };
    let ExprKind::Assign { rhs, .. } = &expr.kind else {
        panic!()
    };
    assert!(matches!(rhs.kind, ExprKind::Assign { .. }));
}

#[test]
fn types_and_pointers() {
    let src = r#"
fn f(a: *const u8, b: *allowzero mut u8, c: *anyopaque, d: &[String], e: (i32,)) {}
"#;
    let prog = parse_ok(src);
    let ItemKind::Fn(f) = &prog.items[0].kind else {
        panic!()
    };
    assert_eq!(f.params.len(), 5);
    let ParamKind::Pattern { ty, .. } = &f.params[1].kind else {
        panic!()
    };
    assert!(matches!(&ty.kind, TypeKind::Ptr(PtrKind::AllowzeroMut(_))));
    let ParamKind::Pattern { ty, .. } = &f.params[2].kind else {
        panic!()
    };
    assert!(matches!(&ty.kind, TypeKind::Ptr(PtrKind::Anyopaque)));
    let ParamKind::Pattern { ty, .. } = &f.params[4].kind else {
        panic!()
    };
    // (i32,) 是一元组；(i32) 是括号类型
    assert!(matches!(&ty.kind, TypeKind::Tuple(_)));
}

#[test]
fn nested_generics_close() {
    // >> 必须被劈成两个 >
    let src = "fn f() { let v: Vec::<Vec::<u8>> = v; }";
    parse_ok(src);
}

#[test]
fn defer_errdefer_and_control() {
    let src = r#"
fn f() {
    defer cleanup();
    errdefer { rollback(); };
    for x in 0..10 {
        defer x = x + 1;
    }
    while let Some(v) = pop() {
        if v > 0 { continue; } else { break; }
    }
}
"#;
    let prog = parse_ok(src);
    let ItemKind::Fn(f) = &prog.items[0].kind else {
        panic!()
    };
    let stmts = &f.body.as_ref().unwrap().stmts;
    assert!(matches!(stmts[0].kind, StmtKind::Defer(_)));
    assert!(matches!(stmts[1].kind, StmtKind::Errdefer(_)));
    let StmtKind::Expr { expr, .. } = &stmts[2].kind else {
        panic!()
    };
    assert!(matches!(expr.kind, ExprKind::For { .. }));
}

#[test]
fn closures_async_macros() {
    let src = r#"
fn f() {
    let cl = move |x: i32, y| -> i32 { x + y };
    let z = || 5;
    let h = |req| async {
        resp()
    };
    panic!("boom {}", 1);
    assert_eq!(a, b);
}
"#;
    parse_ok(src);
}

#[test]
fn use_trees_with_default_alias() {
    let src = r#"
use std::alloc::Gpa as default;
use a::b::{c, d::*};
use e::{f, g::h};
"#;
    let prog = parse_ok(src);
    assert_eq!(prog.items.len(), 3);
    // std::alloc::Gpa as default → Path{std, Path{alloc, Name{Gpa, alias=default}}}
    let ItemKind::Use(u) = &prog.items[0].kind else {
        panic!()
    };
    let UseTree::Path { rest, .. } = &u.tree else {
        panic!()
    };
    let UseTree::Path { rest, .. } = &**rest else {
        panic!()
    };
    assert!(
        matches!(&**rest, UseTree::Name { alias: Some(alias), .. } if alias == "default"),
        "expected `as default` alias, got {rest:?}"
    );
}

#[test]
fn error_recovery_reports_and_continues() {
    let src = "fn a( {}
fn b() {}
struct S { x: i32 }
";
    let (prog, diags) = parse(src);
    assert!(!diags.is_empty(), "expected diagnostics for broken fn a");
    // fn a 报错后恢复，fn b 与 struct S 仍在
    assert!(
        prog.items
            .iter()
            .any(|i| matches!(&i.kind, ItemKind::Fn(f) if f.name == "b"))
    );
    assert!(
        prog.items
            .iter()
            .any(|i| matches!(&i.kind, ItemKind::Struct(s) if s.name == "S"))
    );
}

#[test]
fn non_whitelisted_macro_is_diagnostic() {
    let src = "fn f() { vec!(1); }";
    let (_, diags) = parse(src);
    assert!(diags.iter().any(|d| d.message.contains("白名单")));
}

#[test]
fn struct_literal_expr_rejected_per_grammar() {
    // DEVIATIONS #7：grammar v0.1 无结构体字面量产生式
    let src = "fn f() { let p = Point { x: 1 }; }";
    let (_, diags) = parse(src);
    assert!(!diags.is_empty());
}

#[test]
fn method_self_params_and_impl() {
    let src = r#"
impl Draw for Shape {
    fn draw(&self) -> i32 { 0 }
    fn set(&mut self, v: i32) { self.v = v; }
    fn owned(mut self) {}
    fn typed(self: *const Self) {}
}
"#;
    let prog = parse_ok(src);
    let ItemKind::Impl(im) = &prog.items[0].kind else {
        panic!()
    };
    assert!(im.trait_path.is_some());
    assert_eq!(im.members.len(), 4);
}
