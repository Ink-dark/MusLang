# D-3 共享方言 + 统一 HIR 互操作契约 — 提议冻结

| 项 | 内容 |
|---|---|
| 决策编号 | D-3（PRD §3.7.1 / §3.7.2，状态：待冻结 → 本文件提议冻结） |
| 文件角色 | M1-0 评审件；评审通过后正文迁入 `spec/dialect-hir.md` |
| 上游 | `docs/prd.md` v0.4.7（§3.7.2、§3.11 D-15、§3.9 D-13、§3.8 D-7）；`docs/draft/d0-memory-layout.md`（LAY-1）、`d6-call-topology.md`（边界） |
| 日期 | 2026-09-06 |

---

## 1. 契约总纲（一句话）

**方言层 = 薄前端（仅解析 + 类型化 + lowering 到 HIR），HIR = 唯一语义基准**；三个方言（Rust / C / C++）与 MusLang 源码最终汇入同一 HIR，共享同一套类型检查、借用检查、单态化（D-13）与代码生成——跨语言调用即"同一模块内函数调用"，无独立 FFI 层（PRD §3.7.2）。

**冻结承诺**：
1. 方言层**不得**新增 HIR 无法表达的语义（方言私有 pass 禁止）——凡 HIR 表达不了的方言特性，一律按 §4 降级表拒绝或降级，不做静默翻译；
2. HIR 是兼容性承诺主体：HIR 公共形态一经冻结（与 D-0 LAY-1 同批）按**追加式演进**，方言重写不追溯破坏；
3. 三方言 + MusLang 的单态化 / 借用检查行为**必须一致**（同一实现，不接受方言特判）。

## 2. HIR 承载的语义（唯一基准清单）

| 语义域 | HIR 形态（冻结范围） | 与既有决策衔接 |
|---|---|---|
| 数据布局 | D-0 LAY-1 全部 F 级布局 | d0-memory-layout.md |
| 函数签名 | 显式生命周期（FFI 强制，D-14）、所有权策略标注（D-7 A/C/B） | §3.8 / §3.10 |
| 错误模型 | `Result<T, E>` / `FfiResult`（F 级）/ `Option<T>` | §5 / d0 §4.7 |
| 泛型 | 单态化前泛型（HIR 层展开，深度 25，D-13）；无 `dyn`（例外仅 D-19/D-20 运行时组件） | §3.9 |
| 并发 | `Send`/`Sync` marker trait、`Future`/`Waker`（d0 §4.6） | §3.6 / FR-044 |
| 不安全边界 | `*allowzero` / `*anyopaque` / 审计内建调用点（d4 F 清单） | §3.2.1 / D-4 |

## 3. 无损性定义（"编译期无损互操作"的验收口径）

**无损 = 满足全部三条**：
1. **数据无损**：跨边界类型为 D-0 F 级，两侧 sizeof/alignof/offset 一致（交叉断言测试）；
2. **签名无损**：对方言公开 API 的每个签名，存在语义等价的 HIR 表示（不等价 → 按 §4 降级表处理，不静默丢语义）；
3. **错误无损**：C 错误码 → `Result` 包装规则固定（§5.4 映射表）；Rust `Result` → MusLang `Result` 直接映射；C++ 仅 non-throwing（D-15）。

**明确不承诺无损**（PRD 已定的降级面）：Rust 宏、`const` 泛型与特化、`Pin`（P1）、C++ 异常 / RTTI / 模板实例化（D-15）、Rust 内部 `UnsafeCell` 语义。

## 4. 方言降级表（冻结：每行的处理方式为编译期强制）

| 方言特性 | HIR 对应 | 处理 | 错误码 / 机制 |
|---|---|---|---|
| **C**：全部 K&R/ANSI 声明 | `extern "C"` 签名 | ✅ 直接映射 | — |
| C：函数式宏 / 常量宏 | 常量宏 → HIR const；函数式宏 → 拒绝 | ⚠ 部分支持 | 宏展开 P1（FR 表 §3.4.2） |
| C：`union` / 位域 | F 级 union / ⚠ 位域 | 位域 P1 | `E_DIALECT_UNSUPPORTED`（占位：位域 MVP 拒绝） |
| **Rust**：`Send`/`Sync` bound | MusLang marker trait（同名） | ✅ MVP 起映射 | 语义审查清单 |
| Rust：`Pin`、`?Sized` 约束 | 无（P1） | ❌ 拒绝 | `E_DIALECT_UNSUPPORTED` |
| Rust：宏展开后的 API | — | ❌ 拒绝（要求 Rust 侧先降宏为具体 API） | `E_DIALECT_UNSUPPORTED` |
| Rust：`const` 泛型 / 特化 / HRTB | 无（HRTB = D-14 推 1.0） | ❌ 拒绝 | `E_DIALECT_UNSUPPORTED` / `E_HRTB_NOT_SUPPORTED` |
| Rust：闭包 / `Fn` trait 对象 | MusLang 闭包（语法层，⚠ 未定项见 grammar） | ⚠ MVP 仅函数指针跨界 | — |
| **C++**：non-throwing 自由函数 / POD / Itanium 虚函数 | D-15 边界 | ✅ 按 §3.11 | — |
| C++：throwing 签名 / RTTI / 模板 | 无 | ❌ 拒绝 | D-15 编译期拒绝 |
| C++：`std::shared_ptr` 跨界 | 无（远期 B 策略） | ❌ 拒绝 | §3.8 / D-15 |

> **Rust→C 单向导入（M1 范围，PRD §8 裁剪建议）**：M1 方言层仅实现"Rust 侧声明经方言层导入为 HIR 的 `extern` 项 + `repr(C)` 结构"，双向 Rust 方言编译推迟至 P1；本表为完整 1.0 契约，M1 实现为其中标 ✅ 的子集。

## 5. 错误码映射表（C → MusLang，冻结）

| C 约定 | MusLang 包装（标准库 `std::c` 提供模板） |
|---|---|
| 返回 `int`，`-1` = 错误 | `Result<T, IoError::from_errno()>` |
| 返回指针，`NULL` = 错误 | `Result<*allowzero T, Errno>` → 调用侧再归一为不可空 |
| 返回 `int` 0/非 0 布尔 | `bool` |
| `errno` 线程局部 | 读取点经 `std::c::errno()`（rt-c 直通，无缓存） |

## 6. 验收：差分测试（方言等价性）

1. **HIR 等价测试**：同一 API 的方言声明与 MusLang 源声明，产出**逐字节相同**的 HIR dump（`muslangc --emit hir`；HIR hash 进入差分基线）——保证"方言只是薄糖"；
2. **跨界矩阵**：MusLang↔C / MusLang↔Rust / MusLang↔C++（D-15 范围内）双向调用 + 数据传递用例（`tests/integration/ffi_*`），全部走 D-6 边界口径统计（无隐藏拷贝）；
3. **单态化一致性**：三方言各自调用同一泛型 std 函数，`--emit mono` 实例集合一致（D-13）。

## 7. 未决问题（M1-0 评审）

- [ ] `E_DIALECT_UNSUPPORTED` 是否按方言细分（R_ / C_ / CXX_ 前缀）；
- [ ] C 位域的 MVP 态度（拒绝 vs 降级为 `#[repr(C)]` 手工位段宏）；
- [ ] Rust 闭包跨界的 MVP 边界（仅 `fn` 指针是否够 M1-4 `@cImport` 用例）；
- [ ] HIR dump 的冻结 schema（与 D-0 `--emit layout` 同批评审）；
- [ ] C 方言中 `restrict` 语义的映射（拟：忽略 + 审计注记）。
