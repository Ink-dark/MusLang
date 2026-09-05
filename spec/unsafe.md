# MusLang `unsafe` 等价判定与审计（Unsafe Spec）— v0.1 草案

| 项 | 内容 |
|---|---|
| 文件角色 | M1-0 决策冻结输入；**D-4 的落地产物骨架**（D-4 状态：P0，待落地） |
| 上游 | `docs/prd.md` v0.4.6（§3.2.1、§4.3.2、§3.8.6、D-4） |
| 状态 | 🔄 草案（禁止操作清单为**待定稿**核心项） |
| 更新日期 | 2026-09-05 |

---

## 1. 原则（P2 / P3）

1. **无 `unsafe` 块**（FR-002）：不存在 `unsafe { ... }` 语法（`spec/grammar.ebnf` 仅保留词法冲突检测项）。
2. **类型即安全边界**：不安全操作通过类型标注与关键字表达，审计粒度精确到签名与类型。
3. 目标：借用检查 + 别名集跟踪，使"跨 FFI 内存安全"成为**可证性质**，而非宣称值。

## 2. 类型级不安全标志（审计触发器）

| 标志 | 语义 | 来源 |
|---|---|---|
| `*allowzero T` | 可空裸指针；与 `*const/*mut` 不可隐式转换 | FR-003 |
| `*anyopaque` | 无类型信息指针（C `void*` 等价） | FR-004 |
| `*const T` / `*mut T` 解引用 | 不可为空裸指针的解引用 | §3.2.1 |
| `extern "C"` / `extern "C++"` 块 | FFI 边界声明，块内默认 `repr(C)` | FR-005 |
| `@cImport(...)` | 编译期 C/C++ 头解析边界 | FR-006 |
| `#[muslang::take/borrow/borrow_mut/ret]` | 跨边界归属标注（D-7 C 策略） | §3.8 |
| `static mut` | 可变全局（⚠ 未定：MVP 是否保留） | spec/memory-model.md §10 |

编译器生成 **FFI 审计清单**（`muslangc --emit audit`）：逐调用点记录文件、行号、函数、不安全类型/边界、理由；同时列出所有隐式 `Box` free 调用点（D-20）。

## 3. 禁止操作清单（D-4 核心待定稿项）

> 以下为**候选清单**（M1-0 评审定稿；每条须给出：检查层级、错误码、`unsafe_examples/` 用例）。

| # | 候选禁止操作 | 检查层级（拟） |
|---|---|---|
| F-01 | 伪造引用生命周期（裸指针 → `&T` 无约束转换） | HIR / MIR |
| F-02 | 跨分配器分配与释放（未配对 `from_c`） | HIR（`E_ALLOC_MISMATCH`） |
| F-03 | 同函数 A/C 所有权策略混用 | HIR FFI pass（`E_FFI_MIXED_STRATEGY`） |
| F-04 | `defer` 中引用已跨 FFI 移交句柄 | HIR（`E_DEFER_USED_AFTER_TRANSFER`） |
| F-05 | `defer` 体内 `.await` | HIR（`E_DEFER_AWAIT_FORBIDDEN`） |
| F-06 | 泛型递归单态化 > 25 层 | HIR（`E_MONO_DEPTH_EXCEEDED` / `E_MONO_CYCLE`） |
| F-07 | HRTB `for<'a>`（MVP 不支持） | 类型检查（`E_HRTB_NOT_SUPPORTED`） |
| F-08 | C++ 异常跨越 FFI 边界（throwing 签名） | 类型检查（D-15，编译期拒绝） |
| F-09 | 结构体/枚举字段引用生命周期省略 | 类型检查（`E_LIFETIME_STRUCT_OMITTED`） |
| F-10 | L1 core 中使用无兜底的 `Box::new` / `Vec::new` | HIR（`E_ALLOC_NO_DEFAULT`） |
| F-11 | （待补充）裸指针算术、指针对齐伪造、别名集违规 | MIR ⚠ 待定稿 |

## 4. 固定错误码总表（各子规范引用，纳入 CI 门禁）

| 错误码 | 类别 | 定义处 |
|---|---|---|
| `E_DEFER_AWAIT_FORBIDDEN` / `E_ERRDEFER_INFALLIBLE` / `E_ERRDEFER_TRY_FORBIDDEN` / `E_DEFER_CAPTURE_MOVED` / `E_DEFER_USE_AFTER_DROP` / `E_DEFER_USED_AFTER_TRANSFER` | defer 语义 | D-12 / §3.8.6 |
| `E_FFI_MIXED_STRATEGY` / `E_FFI_MISSING_ATTRIBUTE` / `E_FFI_BAD_ATTRIBUTE` | FFI 所有权 | D-7 / §3.8.6 |
| `W_FFI_ALLOCATOR_MISMATCH`（可升级）/ `W_ARC_LEAK`（P1） | 分配器配对 | §3.8.4 |
| `E_MONO_DEPTH_EXCEEDED` / `E_MONO_CYCLE` / `E_MONO_UNINSTANTIATED` | 单态化 | D-13 / §3.9.3 |
| `E_LIFETIME_AMBIGUOUS` / `E_LIFETIME_STRUCT_OMITTED` / `E_LIFETIME_FFI_OMITTED` / `E_HRTB_NOT_SUPPORTED` | 生命周期 | D-14 / §3.10.3 |
| `E_ALLOC_NO_DEFAULT` / `E_ALLOC_MISMATCH` / `E_DEFAULT_ALLOCATOR_UNRESOLVED` / `W_ALLOC_LEAK`（可升级） | 分配器模型 | D-20 / §3.16.8 |

错误码一经发布**固定不可复用**；新增语义须使用新编号（D-0 IR 冻结精神）。

## 5. `unsafe_examples/` 用例集（目录规格）

```
unsafe_examples/
├── README.md                 # 用例索引：编号 ↔ 错误码 ↔ 检查层级 ↔ PRD 章节
├── allowzero_mmio/           # *allowzero 合法场景（内核 MMIO）
├── ffi_move_across/          # A 策略移交（合法）+ 移交后使用（F-04 违规）
├── ffi_mixed_strategy/       # A/C 混用（F-03 违规）
├── allocator_pairing/        # from_c 配对（合法）+ 不配对（W_FFI_ALLOCATOR_MISMATCH）
├── defer_semantics/          # defer/errdefer 全路径 + 取消 + panic 路径
└── mono_depth/               # 25 层边界用例（24 通过 / 25 拒绝）
```

- 每个用例 = 最小可编译/可拒编译片段 + 预期结果（`ok` / `error(E_XXX)`）+ 断言说明。
- 用例集为**回归资产**：CI 中全量运行，任何错误码变更必须同步用例。

## 6. CI 门禁

- `muslangc --unsafe-allowed=false`（默认值即 false）：任何绕过类型级检查的后门关闭；开启需显式 flag 并在审计报告中标记。
- 门禁通过条件：`--emit audit` 无未说明理由的边界项 + `unsafe_examples/` 全部按预期通过。
- defer / 单态化 / 生命周期检查**默认开启、不可关闭**（D-12）。

## 7. 未决问题（M1-0 评审）

- [ ] 禁止操作清单 F-01~F-11 定稿（每条补齐错误码与用例）。
- [ ] `--emit audit` 输出格式（YAML schema）冻结。
- [ ] `static mut` 的去留。
- [ ] 审计清单与 hypo SBOM（D-18）的字段映射。
