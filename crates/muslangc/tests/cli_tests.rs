//! muslangc CLI 集成测试（经 CARGO_BIN_EXE 调用真实二进制）。
//!
//! v0.1 读取范围沙箱：muslangc 仅接受当前工作目录树内的 .mus 文件；
//! cargo test 的工作目录是本包根，故夹具放在 `tests/fixtures/` 下。

use std::process::Command;

fn muslangc() -> Command {
    let bin = env!("CARGO_BIN_EXE_muslangc");
    Command::new(bin)
}

fn fixture(name: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn version_flag() {
    let out = muslangc().arg("version").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn parse_smoke_fixture() {
    let out = muslangc()
        .arg("parse")
        .arg(fixture("smoke.mus"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Program"));
    assert!(stdout.contains(r#""seed""#));
}

#[test]
fn parse_bad_source_fails_with_diag() {
    // target/ 位于包内（CWD 沙箱范围内）且被 gitignore
    let bad = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("muslangc_bad_test.mus");
    std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
    std::fs::write(&bad, "fn broken( {").unwrap();
    let out = muslangc().arg("parse").arg(&bad).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("expected"), "stderr: {stderr}");
    let _ = std::fs::remove_file(&bad);
}

#[test]
fn lex_prints_tokens() {
    let out = muslangc()
        .arg("lex")
        .arg(fixture("smoke.mus"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Ident"));
    assert!(stdout.contains("Eof"));
}

#[test]
fn rejects_non_mus_extension() {
    let tmp = std::env::temp_dir().join("muslangc_note.txt");
    std::fs::write(&tmp, "fn f() {}").unwrap();
    let out = muslangc().arg("parse").arg(&tmp).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn rejects_file_outside_workdir() {
    // 存在的 .mus 但位于 CWD 之外 → 沙箱拒绝（exit 2）
    let outside = std::env::temp_dir().join("muslangc_outside_test.mus");
    std::fs::write(&outside, "fn f() {}").unwrap();
    let out = muslangc().arg("parse").arg(&outside).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&outside);
}

#[test]
fn rejects_dotdot_path() {
    // `..` 分量 → 拒绝（目标不存在时在 canonicalize 阶段即 exit 2）
    let out = muslangc()
        .arg("parse")
        .arg("sub/dir/../ok.mus")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}
