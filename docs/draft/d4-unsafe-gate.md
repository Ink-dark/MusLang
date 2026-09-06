# D-4 落地：`unsafe` 等价判定 — 禁止操作清单定稿 + 用例集规格 + CI 门禁

| 项 | 内容 |
|---|---|
| 决策编号 | D-4（PRD §3.7.1，状态：P0 待落地 → 本文件为落地件） |
| 文件角色 | M1-0 评审件；评审通过后 ① §1/§2 定稿行迁入 `spec/unsafe.md` §3（替换候选表）② §3/§4 作为 `unsafe_examples/` 与 CI 的实现契约 |
| 上游 | `docs/prd.md` v0.4.7（§3.2.1、§3.8.6、§3.9.3、§3.10.3、§3.16.8、§3.17.2）；`spec/unsafe.md`（候选清单与错误码注册表）；`docs/draft/d6-call-topology.md`（边界 = 审计点） |
| 日期 | 2026-09-06 |

---

## 1. 禁止操作清单（定稿，F-01 ~ F-15）

> 定稿原则：每条 = **唯一错误码 + 唯一检查层级 + 至少一个用例**。错误码一经发布固定不可复用（spec/unsafe.md §4 注册表为唯一权威，本表为定稿来源）。

| 编号 | 禁止操作 | 检查层级 | 错误码 | 用例目录 |
|---|---|---|---|---|
| F-01 | `@ptrCast` 直接产生引用类型（伪造引用生命周期） | MIR | `E_PTR_TO_REF_FORBIDDEN`（新定稿） | `ref_cast/` |
| F-02 | 跨分配器分配与释放（未配对 `from_c`） | HIR | `E_ALLOC_MISMATCH` | `allocator_pairing/` |
| F-03 | 同 `extern` 函数内 A / C 所有权策略混用 | HIR FFI pass | `E_FFI_MIXED_STRATEGY` | `ffi_mixed_strategy/` |
| F-04 | `defer` 引用已跨 FFI 移交句柄 | HIR | `E_DEFER_USED_AFTER_TRANSFER` | `ffi_move_across/` |
| F-05 | `defer` 体内 `.await` | HIR | `E_DEFER_AWAIT_FORBIDDEN` | `defer_semantics/` |
| F-06 | 递归单态化深度 > 25 层 | HIR 构建期 | `E_MONO_DEPTH_EXCEEDED` / `E_MONO_CYCLE` | `mono_depth/` |
| F-07 | HRTB `for<'a>`（MVP 不支持） | 类型检查 | `E_HRTB_NOT_SUPPORTED` | `lifetime_hrtb/` |
| F-08 | C++ throwing 签名跨边界 | 类型检查 | `E_FFI_THROWING_ABI`（新定稿） | `cpp_exceptions/` |
| F-09 | struct / enum 字段引用生命周期省略 | 类型检查 | `E_LIFETIME_STRUCT_OMITTED` | `lifetime_struct/` |
| F-10 | L1 core 无兜底时 `Box::new` / `Vec::new` | HIR | `E_ALLOC_NO_DEFAULT` | `l1_no_allocator/` |
| F-11 | 指针算术无来源证明（越出分配对象即解引用） | MIR | `E_PTR_ARITH_UNPROVEN`（新定稿） | `ptr_offset/` |
| F-12 | 对齐伪造（`@ptrCast` 至更高对齐且无来源保证） | MIR | `E_ALIGN_FORGE`（新定稿） | `ptr_align/` |
| F-13 | `#[constant_time]` 函数内秘密依赖分支 / 查表 / 提前返回 | MIR | `E_NON_CONSTANT_TIME` | `crypto_ct/` |
| F-14 | `errdefer` 体返回 `Result` / 使用 `?` | 类型检查 | `E_ERRDEFER_INFALLIBLE` / `E_ERRDEFER_TRY_FORBIDDEN` | `defer_semantics/` |
| F-15 | `extern` 签名引用参数生命周期省略 | 类型检查 | `E_LIFETIME_FFI_OMITTED` | `ffi_lifetime/` |

**警告级（可升级，同表管理）**：

| 编号 | 条件 | 错误码 | 升级 flag |
|---|---|---|---|
| W-01 | 密钥材料未经 `SecretBox` 暴露 | `W_SECRET_PLAINTEXT` | `--forbid-secret-plaintext` |
| W-02 | Allocator 配对不一致 | `W_FFI_ALLOCATOR_MISMATCH` | `--forbid-unsafe-allocator` |
| W-03 | GPA 调试模式泄漏 | `W_ALLOC_LEAK` | CI 内默认升级为错误 |
| W-04 | `Arc`（B 策略）未配对 `arc_drop`（P1） | `W_ARC_LEAK` | — |

**新增定稿错误码 4 枚**：`E_PTR_TO_REF_FORBIDDEN`、`E_FFI_THROWING_ABI`、`E_PTR_ARITH_UNPROVEN`、`E_ALIGN_FORGE`（随本清单入注册表）。

### 1.1 F-01 / F-11 / F-12 的合法路径（escape-hatch 内建，全部为审计点）

裸指针操作是 FFI 与内核的现实需求——清单禁止的是**无证明的转换**，合法路径收敛为**审计内建白名单**（封闭集合，禁止用户自定义同类内建）：

| 内建 | 语义 | 审计要求 |
|---|---|---|
| `@ref_cast<P, T>(p: P) -> &T` | 裸指针 → 引用（要求非空 + 对齐 + 生命周期绑定借用区） | 调用点须 `#[audit(reason="…")]` |
| `@ptr_offset(p, n)` | 指针算术（生成区间注记，供 MIR 证明） | 同上 |
| `@volatile_read / @volatile_write` | MMIO / 寄存器访问（仅接受 `*allowzero`/`*mut`） | 同上 |
| `@bitcast<T, U>(v)` | 同尺寸 Copy 类型位转换（`sizeof<T> == sizeof<U>` 编译期断言） | 同上 |
| `MusAllocator::from_c` | 分配器桥接（§3.8.4） | 配对检查（W-02） |

- **`--unsafe-allowed=false`（默认）时**：白名单内建**仍可用**，但每个调用点必须携带 `#[audit(reason="…")]` 注解，否则报 `E_AUDIT_JUSTIFICATION_MISSING`（新定稿，第 5 枚新增码）；
- **`--unsafe-allowed=true` 时**：`reason` 可省略（降为 warning）——**该 flag 不放开任何 F-01~F-15 语言级检查**（MusLang 无 `unsafe` 块可开），唯一作用是把"已声明、已审计的逃逸口"从 CI 阻断项降为例外登记项。

## 2. `unsafe_examples/` 用例集规格

### 2.1 目录与格式（冻结）

```
unsafe_examples/
├── README.md                     # 索引表：用例 ↔ F 编号 ↔ 错误码 ↔ PRD 章节
└── <case_id>/                    # case_id = snake_case，与 §1 表"用例目录"列一致
    ├── case.mus                  # 最小片段（可编译或可拒编译，禁止依赖外部 crate）
    ├── EXPECT.toml               # 机器可读预期
    └── rationale.md              # 为什么（PRD/spec 章节引用 + 人类可读说明）
```

```toml
# EXPECT.toml schema（冻结）
expected = "ok"            # "ok" | "error"
code = ""                  # expected=error 时必填：错误码，须已在注册表
layer = ""                 # "parse" | "hir" | "typecheck" | "mir" | "link"
audited_hatches = []       # case.mus 中出现的审计内建（供门禁交叉核对）
```

### 2.2 运行与验收

- 运行器：`muslangc test-examples unsafe_examples/ --gate`
  - 逐用例编译并比对实际结果与 `EXPECT.toml`（错误码**逐字节一致**，不允许"报了错但码不对"）；
  - `--gate` 追加两项：① 每个 `error` 用例的错误码已在注册表；② `audited_hatches` 与 case.mus 中实际内建调用一致（防漏标）；
- **覆盖要求**：F-01~F-15 每条 ≥ 1 用例；D-12 defer 语义（正常/错误/取消/panic 四路径）≥ 4 用例；F-06 边界用例须含 24 层（通过）与 25 层（拒绝）两枚；
- **规模承诺**：M1-0 内 ≥ **20** 用例落库（骨架 + 已实现检查项的用例），其余随检查器实现逐条补齐（每实现一条 F-xx 检查，同 PR 合入对应用例——**检查与用例不得分批落库**）；
- 用例集为回归资产：错误码语义变更必须同步用例，`test-examples` 在 CI 每次 PR 全量运行。

## 3. `--unsafe-allowed` CI 门禁（实现契约）

### 3.1 flag 语义（冻结）

| flag | 行为 |
|---|---|
| `--unsafe-allowed=false`（**默认**） | F-01~F-15 全部硬错误；W 级按升级表；审计内建须带 `#[audit(reason)]`；`--emit audit` 完整输出 |
| `--unsafe-allowed=true` | 仅放宽 `#[audit(reason)]` 必填性（降 warning）；语言级检查**不放宽**；审计输出追加 `UNSAFE_ALLOWED` 标记 |

### 3.2 CI 流水线（GitHub Actions，M1-0 建骨架）

```yaml
# .github/workflows/unsafe-gate.yml（骨架）
jobs:
  unsafe-gate:
    steps:
      - run: muslangc test-examples unsafe_examples/ --gate        # §2.2
      - run: muslangc build --emit audit --unsafe-allowed=false    # 全仓审计
      - run: audit-lint audit.yml                                  # ① 每个 B1/B2/B3 边界项有 reason
                                                                   # ② 零开销域（d6 §1）内无边界项（负向断言）
                                                                   # ③ 错误码引用均在注册表
      - run: audit-lint --forbid-secret-plaintext --escalate W_ALLOC_LEAK
```

- **通过条件**：三步全绿 + 审计清单中所有边界项 reason 非空；
- 门禁随 M1-2 / M1-3 检查器落地逐步启用：M1-0 建流水线 + 用例运行器（用例集允许仅覆盖已完成检查项），M1-3（`*allowzero` 类型系统 + FFI 审计清单）起全量生效。

## 4. 错误码注册表（流程性冻结）

- `spec/unsafe.md` §4 为唯一注册表；本文件 §1 为**定稿来源**；
- 注册规则：新码必须绑定 F/W 编号 + 检查层级 + 用例，三者齐备才可入表；编号不复用、不重定义；
- 文档化：每码一节（触发条件、示例、修复建议），随 `spec/unsafe.md` 维护。

## 5. 与既有决策的衔接

| 决策 | 衔接 |
|---|---|
| D-12 | F-04/F-05/F-14 即 defer 校验的错误码体系（§3.2.3.1） |
| D-7 | F-02/F-03 + 审计内建 `from_c`（§3.8.4/§3.8.6） |
| D-13 | F-06（25 层） |
| D-14 | F-07/F-09/F-15 |
| D-15 | F-08（throwing 拒绝） |
| D-20 | F-10 + W-03 |
| D-21 | F-13 + W-01（`SecretBox` / `#[constant_time]`） |
| D-6 | 边界（B1/B2/B3）= 审计触发器；零开销域负向断言 |
| D-3 | FFI 项的 `E_FFI_*` 与方言降级表共用注册表 |

## 6. 未决问题（M1-0 评审）

- [ ] `--emit audit` 输出 YAML schema 冻结（与 D-0 `--emit layout`、D-3 `--emit hir` 同批评审）；
- [ ] `static mut` 去留（决定 F 清单是否追加 F-16）；
- [ ] `#[audit(reason)]` 注解的 reason 格式（自由文本 vs 受控词表；倾向自由文本 + 强制非空）；
- [ ] `E_AUDIT_JUSTIFICATION_MISSING` / `E_NON_FFI_LAYOUT` 等"新增定稿码"的最终编号位次（注册表追加顺序）；
- [ ] 用例运行器在 muslangc v0.1（M1-7）前的宿主实现（拟：M1-0/M1-3 期间由 Rust 侧 Stage 0 提供子命令）。
