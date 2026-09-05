# MusLang 进度看板 & Todolist

> **同步版本**：PRD v0.4.6（2026-09-05）
> **权威来源**：决策状态以 `docs/prd.md` §3.7.1 为准；里程碑定义以 `docs/prd.md` §8 为准
> **图例**：✅ 已完成 · 🔄 进行中 · ⏸️ 受阻 · 📋 未开始（Backlog）

---

## 看板总览

| 看板列 | 条目 |
|---|---|
| ✅ **已完成** | PRD v0.1 → v0.4.6（D-0~D-20 决策记录 + D-12 语义修订）；`docs/CHANGELOG.md` 同步；`spec/` 五份 M1-0 规格草案 v0.1 落库；本看板建立 |
| 🔄 **进行中** | **M1-0 决策冻结**（见 Todolist 第一组） |
| ⏸️ **受阻** | 无 |
| 📋 **未开始** | M1-1 ~ M1-8、阶段二（M2-1~M2-6）、阶段三（M3-1~M3-7）、§12 开放问题（comptime / WASM） |

---

## Todolist

### ① 当前里程碑：M1-0 决策冻结（2 周窗口，阶段一放行门槛）

- [ ] **D-0**（共享内存布局规范 IR/HIR）冻结
- [ ] **D-3**（共享方言 + 统一 HIR 互操作）冻结
- [ ] **D-5**（是否引入 MLIR）决策并冻结（若引入需同步上修 `<8KB` 指标）
- [ ] **D-6**（调用拓扑与开销口径）冻结
- [ ] **D-4 落地**：禁止操作清单定稿 + `unsafe_examples/` 用例集 + `--unsafe-allowed=false` CI 门禁实现（骨架见 `spec/unsafe.md`）
- [ ] **D-11 确认**：L1 是否含 libc（建议默认 musl，影响 rt-c 设计下限）
- [ ] `spec/grammar.ebnf`：从 v0.1 骨架补完为可测语法（M1-1 的直接输入；⚠ 标注项逐条评审）
- [ ] `spec/memory-model.md` / `spec/unsafe.md` / `spec/backend-c99.md` / `spec/std-sys.md`：从草案评审至定稿
- [ ] §3.7.1 决策表状态复核（已定 15 / 待冻结 4 / 待落地 1 / 需确认 1）

### ② M1-1 ~ M1-8（阶段一 Bootstrap，2026 Q4-2027 Q1）

- [ ] **M1-1** 语法定义：grammar.ebnf 定稿 + 100% 语法测试通过（FR-001a/001b）
- [ ] **M1-2** 所有权检查器 v0.1（通过 Rust 测试用例子集，NLL 除外）
- [ ] **M1-3** `*allowzero` 类型系统：类型检查 + 代码生成 + FFI 审计清单
- [ ] **M1-4** `@cImport`：能解析 MusKitty 现有 C 头文件
- [ ] **M1-5** C99 后端代码生成 v0.1：hello world 编译运行
- [ ] **M1-6** `net` 标准库第一版：TCP/HTTP 客户端，HTTP/1.1 访问真实服务
- [ ] **M1-7** muslangc v0.1（Rust 写，端到端编译 + 测试）
- [ ] **M1-8** 链接器集成（Zig LLD + `--gc-sections`）：hello world 静态链接 < 16KB（M1 中期口径；终态 = L1 core < 8KB）

### ③ 阶段二：MusKitty 内核开发（2027 Q2-Q3）

- [ ] **M2-1** MusKitty Layer 1-4（内存、进程、IPC）
- [ ] **M2-2** MusKitty Layer 5（TCP/UDP 可用，HTTP 服务可运行）
- [ ] **M2-3** MusKitty Layer 6（V8/SpiderMonkey 内核态适配）——**触发 §12.2 WASM 评估**
- [ ] **M2-4** 统信 UOS / 银河麒麟真机适配
- [ ] **M2-5** 国密 SM2/SM3/SM4 集成
- [ ] **M2-6** Freestanding 链接 + 自定义链接脚本
- [ ] muslangc 增量编译 + `--reproducible` 确定性输出（D-16 §3.12.4 的 M2 项）
- [ ] `mktplace` 雏形试用（D-17 过渡期：Cargo 为主）
- [ ] hypo 试点集成（内核镜像经 hypo 打包分发，D-18）

### ④ 阶段三：自举与信创（2027 Q4-2028 Q4）

- [ ] **M3-1** 编译器前端 MusLang 化（Stage 1：lexer/parser/AST/类型检查/借用检查）
- [ ] **M3-2** 完整编译器（含 C99 后端 MusLang 化）
- [ ] **M3-3** 自持闭环（Stage 2：SHA-256 一致，可重现构建）——**触发 §12.1 comptime 重评**
- [ ] **M3-4** 信创测评送测（SBOM + SM2 签名由 hypo 承担，D-18）
- [ ] **M3-5** 100% 信创替代达标（工具链零外部语言依赖）
- [ ] **M3-6** `muslink` v0.1（MusLang 写的极简 ELF64 链接器）
- [ ] **M3-7** 全工具链纯自主验证（muslangc + muslink）

### ⑤ 远期 / 决策节点

- [ ] §12.1 `comptime`：M3-3 达成后重评
- [ ] §12.2 WASM 后端：M2-3 启动时评估
- [ ] `muslink` 动态链接：v2.0（2028+）评估（与 L3 纯自主模式的衔接见 §12.5 说明）
- [ ] io_uring：P1 立项条件 = epoll 在 LoongArch64/ARM64 实测达不到目标吞吐（§3.15.8）
- [ ] Cargo → `mktplace` 迁移：三门槛全绿（hypo lockfile+mirror+签名 / 20 个真实包 / 信创离线全通过，§3.13.3）

---

## 决策冻结跟踪（D-0 ~ D-20，共 21 项）

| 状态 | 决策 |
|---|---|
| ✅ 已定（15） | D-1 语法基线、D-2 后端 C99、D-7 所有权移交 A+C、D-8 双 runtime、D-9 std 按需链接、D-10 rt-c 唯一系统接口、D-12 `defer` 语义（v0.4.6 修订）、D-13 单态化、D-14 生命周期、D-15 C++ 边界、D-16 自举 staging、D-17 包管理器、D-18 软件分发、D-19 net 运行时绑定、D-20 分配器模型 |
| ⏸️ 待冻结（4） | D-0（IR/HIR 布局规范）、D-3（互操作机制）、D-5（MLIR 与否）、D-6（调用拓扑口径） |
| 🔧 待落地（1） | D-4（`unsafe` 等价判定 + `unsafe_examples/` + CI 门禁） |
| ❓ 需确认（1） | D-11（L1 是否含 libc；建议 musl） |

---

## 文档体系进度（对应 PRD §13）

| 文档 | 状态 | 说明 |
|---|---|---|
| `docs/prd.md` | ✅ v0.4.6 | 产品需求文档（唯一权威） |
| `docs/CHANGELOG.md` | ✅ 同步 v0.4.6 | PRD 总变更记录 |
| `docs/PROGRESS.md` | ✅ 本文件 | 进度看板 + Todolist |
| `spec/grammar.ebnf` | 🔄 v0.1 骨架 | M1-0/M1-1 输入，⚠ 未定项待评审 |
| `spec/memory-model.md` | 🔄 v0.1 草案 | 所有权 / defer / 分配器模型 |
| `spec/unsafe.md` | 🔄 v0.1 草案 | D-4 骨架，禁止清单待定稿 |
| `spec/backend-c99.md` | 🔄 v0.1 草案 | C99 后端映射规则 |
| `spec/std-sys.md` | 🔄 v0.1 草案 | std 拆分 / sys 三端 / event loop |
| `language-reference/`、`std-lib/`、`tutorials/`、`internals/`、`rfc/` | 📋 未开始 | 随 M1 推进建立 |

---

## 变更记录

| 日期 | 变更 |
|---|---|
| 2026-09-05 | 建立看板；M1-0 五份 spec 草案（`grammar.ebnf` / `memory-model.md` / `unsafe.md` / `backend-c99.md` / `std-sys.md`）落库；同步 PRD v0.4.6（D-12 语义修订 + 自洽性清理） |
