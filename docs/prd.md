# MusLang — 产品需求文档（PRD）

> **项目代号**：MusLang-Qomolangma
> **仓库**：https://gitee.com/moranqidarkseven/MusLang
> **所属生态**：MusCat 浏览器的原生系统编程语言
> **文档版本**：v0.4.1
> **创建日期**：2026-08-30
> **更新日期**：2026-09-04
> **作者**：墨染柒（Ink-dark）
> **状态**：评审中
> **变更记录**：
> - v0.1（2026-08-30）：初稿
> - v0.2（2026-08-30）：补充链接器章节、muslink 设计
> - v0.3（2026-09-01）：全面完善——补齐错误处理、测试策略、API 规范、安全模型形式化、开放问题分析框架、模块系统、文档体系、CI/CD、治理模型
> - v0.4（2026-09-04）：架构决策落地（§3.7 架构决策记录、后端 C99 默认、双 runtime、std 按需链接 + 三端、二进制体积口径定稿、M1-0 决策冻结）
> - v0.4.1（2026-09-04）：语义定稿——① `defer` 与错误传播 / 异步取消的交互规则（§3.2.3.1，D-12）；② 跨 `.so` 所有权移交协议 A+C 混合、函数级策略一致性（§3.8，D-7 定稿）；③ 配套引用补充（§15.1）

---

## 1. 产品概述

### 1.1 一句话定位

**MusLang 是一门语法以 Rust 为参照（不保证源码级兼容）、安全模型以 Zig 的类型区分取代 `unsafe` 块、内置 Go 级网络标准库、编译产物与 Zig 同级轻量的系统编程语言。** Rust、C/C++ 三方通过**共享内存布局规范与统一 HIR** 实现编译期无损互操作（无运行时 FFI 层）；标准库按子系统拆分为独立 crate、按需链接，并通过 `sys` 层支持 Linux / macOS / Windows 三端扩展（当前仅 Linux 实装）。MusLang 是 MusCat 生态全栈自研的最后一环——从语言到编译器到链接器到内核到浏览器，全栈自主可控。

> **说明（v0.4）**：本段仅反映已冻结/已决策的架构方向，详见新增 §3.7「架构决策记录」。尚未决策的事项（元编程具体形态、包管理器、WASM 等）仍见 §12，保持「待定」不动。

### 1.2 核心价值：三角融合

```
         Go 网络体验
        ╱────────────╲
       ╱              ╲
      ╱    MusLang     ╲
     ╱                  ╲
    ╱────────────────────╲
   Zig 轻量+C/C++兼容    Rust 内存安全
```

| 维度 | Rust | Go | Zig | **MusLang** |
|---|---|---|---|---|
| 语法 | Rust | Go | 类C | **Rust 风格** |
| 内存安全 | 所有权 + `unsafe` 块 | GC | 手动 | **所有权，无 `unsafe` 块** |
| C/C++ 互操作 | bindgen | cgo（重） | `@cImport` | **`@cImport` 同级** |
| 网络标准库 | tokio/axum | `net/http` 内置 | 无 | **内置，对标 Go** |
| 运行时 | panic handler | GC | 零 | **零** |
| 二进制体积 | 中等 | 中等 | ~4KB hello | **~4KB hello** |
| 编译速度 | 慢 | 快 | 极快 | **极快** |
| 自持 | 自举 | 自举 | 进行中 | **自举** |
| 链接器 | 依赖外部 ld/lld | 内置 | 内置 LLD | **内置 + 自研 muslink** |
| 元编程 | 泛型 + 宏 | 泛型 | comptime | **待定（见 §12）** |

### 1.3 目标用户

- MusCat 生态开发者（MusKitty 内核、dll 适配层、MCP Server、Cookie 协议）
- 信创场景下的系统级开发者
- AI Agent 工具开发者（通过 MCP Server 接口）

### 1.4 设计原则

> **核心哲学**：Rust 的壳，Zig 的灵魂，类似 Go 的网络，MusCat 的心脏。

以下设计原则贯穿所有决策：

| # | 原则 | 说明 |
|---|---|---|
| P1 | **零隐藏控制流** | 无隐式分配、无隐式转换、无异常——所有控制流在源码中可见 |
| P2 | **所有权强制** | 编译期保证内存安全，不允许 `unsafe` 逃逸 hatch |
| P3 | **类型即安全边界** | 不安全操作通过类型标注（`*allowzero`、`*anyopaque`、`extern`）显式标记，审计粒度精确到类型 |
| P4 | **显式优于隐式** | 分配器作为参数、错误传播用 `?`、并发模型显式选择 |
| P5 | **零运行时** | 无 GC、无运行时库依赖、panic 直接 abort |
| P6 | **C 是第一公民** | `@cImport` 是一等特性，C/C++ 互操作零摩擦 |
| P7 | **网络开箱即用** | 内置 `net` 标准库，对标 Go `net/http` 体验 |
| P8 | **渐进式复杂度** | Hello World 只需 5 行；内核开发才需要理解分配器、裸指针、链接脚本 |

---

## 2. 用户场景

| 场景 | 角色 | 核心需求 | MusLang 解法 |
|---|---|---|---|
| MusKitty 内核开发 | 墨染柒 + Orcha | 内存安全、C/C++ 互操作、零 GC、极小二进制 | Rust 语法 + 所有权 + `@cImport` + Zig 后端 |
| MCP Server 实现 | Agent 工具开发者 | 高并发 HTTP、登录态审批接口 | 内置 `net/http` + async/await + 零 GC |
| 信创环境构建 | 国产 OS 适配工程师 | 全栈自主、无外部语言依赖、LoongArch64/ARM64 | 自举编译器 + 自研链接器 + 零运行时 |
| 第三方内核 dll 适配 | Chromium/Gecko 适配开发者 | C ABI 兼容、类型映射、内存安全 | `@cImport` + 所有权 + `extern "C"` 导出 |
| 嵌入式/裸机开发 | 固件工程师 | 极小体积、无 libc、自定义入口 | Freestanding 链接 + 显式分配器 + 无运行时 |

---

## 3. 功能需求

### 3.1 语法层（P0）

| 编号 | 功能 | 描述 |
|---|---|---|
| FR-001 | Rust 风格语法 | 结构体、枚举、trait、泛型、模式匹配、async/await，与 Rust 语法 100% 一致 |
| FR-002 | **无 `unsafe` 块** | 移除 Rust 的 `unsafe {}` 块，不安全通过类型标注表达 |
| FR-003 | `*allowzero T` 类型 | 可空裸指针类型，是"不安全"的类型级标志，替代 `unsafe` 块 |
| FR-004 | `*anyopaque` 类型 | 等价于 C 的 `void*`，类型不安全，需显式转换 |
| FR-005 | `extern "C"` 块 | 声明外部 C/C++ 函数，是 FFI 的标志（Rust 中是 `unsafe extern "C"`） |
| FR-006 | `@cImport` 指令 | 编译期解析 C/C++ 头文件，类型自动映射，与 Zig 同级 |
| FR-007 | 所有权与借用 | 编译期所有权检查，移动语义，借用检查，无 GC |
| FR-008 | 显式分配器 | 分配器作为参数显式传递，Zig 风格 |
| FR-009 | `defer` 语句 | 作用域结束时执行清理，替代 RAII 析构函数 |
| FR-010 | 默认 abort | panic 时直接 abort，无栈展开，零运行时开销 |
| FR-011 | `?` 错误传播 | 函数返回 `Result<T, E>`，`?` 运算符传播错误（同 Rust） |
| FR-012 | `Option<T>` 类型 | 可选值，替代可空指针 |
| FR-013 | **模块系统** | `mod` / `use` 声明，与 Rust 一致；包管理见 §12 开放问题 |

### 3.2 类型系统

#### 3.2.1 安全类型 vs 不安全类型

> **核心创新**：MusLang 将 Rust 的"块级 `unsafe`"提升为"类型级安全标注"，使安全边界在类型签名中可见。

| 类型类别 | 示例 | 安全性 | 使用场景 |
|---|---|---|---|
| **安全指针** | `&T`、`&mut T` | ✅ 借用检查器保证 | 日常引用 |
| **安全裸指针** | `*const T`、`*mut T` | ⚠️ 不可为空，需 `unsafe` 操作 | FFI 参数/返回值 |
| **可空裸指针** | `*allowzero T` | ❌ 可为地址零，类型级不安全标志 | 内核 MMIO、C 兼容 |
| **不透明指针** | `*anyopaque` | ❌ 无类型信息 | C `void*` 互操作 |
| **安全抽象** | `Box<T>`、`Vec<T>` | ✅ 所有权管理 | 堆分配容器 |

**关键规则**：

1. `*allowzero T` 与 `*const T` / `*mut T` **不可隐式转换**，必须显式 `@ptrCast`
2. `extern "C"` 块中的函数签名出现 `*allowzero` / `*anyopaque`，即标志其为"安全边界"（块内默认 `repr(C)`）
3. 编译器生成"FFI 审计清单"，列出所有跨越安全边界的调用点
4. **（v0.4，D-4）** MusLang 侧如需等价 Rust `unsafe` 的能力，须**显式标注**并配套可判定规范：`unsafe_examples/` 用例集 + 固定错误码 + `--unsafe-allowed=false` CI 门禁；目标是通过借用检查 + 别名集跟踪，将"跨 FFI 内存安全"变为**可证性质**，而非宣称值（详见 §3.7 D-4）

#### 3.2.2 所有权与借用规则

与 Rust 一致的移动语义、借用规则（NLL）、生命周期省略。差异点：

| 特性 | Rust | MusLang |
|---|---|---|
| `Copy` trait | 自动派生 | 同 |
| `Drop` | RAII 析构 | ❌ 使用 `defer`（见 §3.2.3） |
| 智能指针 | `Box`/`Rc`/`Arc` | `Box`（P0）；`Rc`/`Arc`（P1） |
| Pin | 支持 | P1 |

#### 3.2.3 资源管理：`defer` 替代 RAII

```rust
// MusLang：defer 显式清理
fn process_file(path: &str) -> Result<(), IoError> {
    let file = fs::open(path)?;
    defer file.close();  // 作用域退出时执行，无论正常还是错误路径

    let data = file.read_all()?;
    // ... 处理数据
    Ok(())
}
```

**设计理由**：
- 与 Zig 一致，显式优于隐式
- 避免 RAII 析构函数在 FFI 边界的复杂性
- 内核场景中资源清理路径必须可见

#### 3.2.3.1 `defer` 与错误传播、异步取消的交互（D-12）

> **v0.4.1 定稿**。MusLang 无 RAII / `Drop` trait，资源清理由 `defer`（正常路径）与 `errdefer`（错误路径）显式承担。本小节定义其与 `?` 提前返回、以及 `async fn` 在 `.await` 点被取消时的精确语义。**规则为函数级、编译期强制，无运行时开销。**

**规则一：正常退出 → `defer` / `errdefer` 均执行（LIFO）**

作用域无论以何种**非错误**方式退出（自然结束、`return`、`break` 跳出块），已注册的 `defer` 与 `errdefer` 按**后进先出**顺序执行：

```rust
fn process(path: &str) -> Result<(), IoError> {
    let file = fs::open(path)?;      // 若 ? 提前返回，走规则二
    defer file.close();              // 仅正常退出路径执行
    errdefer log_failure(path);      // 仅错误退出路径执行（见规则二）

    let data = file.read_all()?;     // ? 触发 → 仅 errdefer 执行
    Ok(())
}   // 正常到达此处 → 仅 defer 执行
```

**规则二：`?` 提前返回（错误路径）→ 仅 `errdefer` 执行**

`?` 将错误从当前作用域返回时，该作用域内已注册的 `defer` **不执行**，仅 `errdefer` 按 LIFO 执行。这是 Zig 的 `errdefer` 语义 ：错误路径上的资源回滚显式标注，符合 P1「零隐藏控制流」——清理行为由关键字即可读出，无需追溯整个作用域。

- **多个 `errdefer`**：按注册的逆序（LIFO）执行；
- **`defer` 与 `errdefer` 混合**：错误路径只跑 `errdefer` 链，正常路径只跑 `defer` 链，两者**互不交叉**；
- **`errdefer` 块本身允许 `?`** 吗？不允许——`errdefer` 体必须为 infallible（不返回 `Result`）。否则清理失败会形成「错误路径上再出错」的不可判定状态，由编译器拒绝。

**规则三：`async fn` 在 `.await` 点被取消 → `defer` / `errdefer` 的同步部分执行**

`async fn` 编译期展开为状态机（FR-043）；当该 future 在 `.await` 点被丢弃（取消）时：

1. **取消只发生在 `.await` 点**（协作式），与 Rust 的取消模型一致 ；进入长同步循环而无 `.await` 的 `async fn` **不可被取消**——此限制须在文档与 LSP 提示中明示；
2. 状态机按**逆构造顺序**丢弃所有跨 `.await` 点存活的局部变量，`defer` / `errdefer` 中**不含 `.await` 的同步部分**照常执行（规则一同态）；
3. **`defer` 块内禁止 `.await`**（编译期硬拒绝）。理由：异步析构（async Drop）是 Rust 折腾多年仍未稳定的难题 ，MusLang MVP 不应踩此坑——**需要异步清理的资源必须显式调用 `close().await`，不得隐藏在 `defer` 中**；
4. 异步资源的显式关闭接口（如 `TcpStream::close(self) -> impl Future`）须在标准库中统一提供，`defer stream.close()` 此类**同步**调用仅关闭底层 fd、不等待优雅关闭，属用户知情的选择。

> **取消安全（cancel-safety）**：取消后已执行的 `defer` / `errdefer` 必须保持资源不变量——标准库异步类型须文档化其在 `.await` 点被取消时的行为（对标 Tokio 的 cancel-safe 约定 ）。

**规则四：多个 `defer` 的执行顺序与可见性**

- 同一作用域内多个 `defer`：按**文本逆序**（后注册先执行），与 Zig、Go 一致；
- `defer` 可引用外层变量，但**不可捕获跨 `.await` 点被移动的变量**（借用检查器在 HIR 阶段按状态机拆分点校验，防止 use-after-move）；
- `defer` 体内的 panic：**不**被自动捕获，直接向上传播并终止同作用域剩余 `defer`——避免「清理失败静默」。

**函数级校验（编译器强制）**

| 校验项 | 阶段 | 违反时行为 |
|---|---|---|
| `defer` 体含 `.await` | HIR / 类型检查 | 报错 `E_DEFER_AWAIT_FORBIDDEN` |
| `errdefer` 体返回 `Result` | 类型检查 | 报错 `E_ERRDEFER_INFALLIBLE` |
| `?` 用于 `errdefer` 体内 | 类型检查 | 报错 `E_ERRDEFER_TRY_FORBIDDEN` |
| 跨 `.await` 捕获被移动变量 | 借用检查（MIR） | 报错 `E_DEFER_CAPTURE_MOVED` |
| 取消后 `defer` 访问已 drop 局部 | 借用检查（MIR） | 报错 `E_DEFER_USE_AFTER_DROP` |

错误码固定、文档化，纳入 D-4 的 `unsafe_examples/` + `--unsafe-allowed=false` CI 门禁覆盖范围（`defer` 相关检查默认开启、不可关闭）。

**与既有设计的衔接**

- **§3.2.3**：`defer` 替代 RAII 的总纲，本小节为其语义细化，不冲突；
- **§3.6 / FR-045~047**：取消为协作式、仅 `.await` 点，调度器（work-stealing，P1）可随时 drop 未轮询 future——本规则保证 drop 时资源不变量仍成立；
- **§3.7 D-4**：`defer` / `errdefer` 检查属编译期安全机制，不引入运行时；
- **§3.8 D-7**：跨 `.so` 边界传入/返回的资源句柄，其 `defer` 清理**必须在同侧完成**（见「策略一致性」），C 侧持有的句柄不得由 MusLang 侧的 `defer` 关闭，反之亦然。

### 3.3 标准库（P0/P1）

| 编号 | 功能 | 优先级 | 对标 |
|---|---|---|---|
| FR-014 | `net` 包：TCP/UDP/HTTP/TLS | P0 | Go `net` |
| FR-015 | `net::http`：HTTP/1.1 + HTTP/2 客户端与服务器，async | P0 | Go `net/http` |
| FR-016 | `net::tls`：TLS 支持，信创集成国密 SM2/SM3/SM4 | P1 | rustls / BoringSSL |
| FR-017 | `fs` 包：文件操作 | P0 | Go `os`/`io` |
| FR-018 | `strings` 包：字符串处理 | P0 | Go `strings` |
| FR-019 | `collections` 包：Vec、HashMap、BTreeMap | P0 | Rust `std::collections` |
| FR-020 | `channel` 包：mpsc/broadcast，类型安全 | P1 | Go channels / Rust `std::sync::mpsc` |
| FR-021 | `alloc` 包：分配器接口（GeneralPurposeAllocator） | P0 | Zig `std.heap` |
| FR-022 | `c` 包：C ABI 类型定义（c_int, c_void 等） | P0 | — |
| FR-023 | `async` 包：Future trait、Waker、FfiFuture | P0 | Rust `std::future` |
| FR-024 | `io` 包：统一 I/O 接口（对标 Zig 新 Io 模型） | P0 | Zig `std.io` |
| FR-025 | `time` 包：时间/定时器 | P1 | Go `time` |
| FR-026 | `encoding` 包：JSON/CBOR/XML | P1 | Go `encoding/json` |
| FR-027 | `crypto` 包：哈希、对称加密、国密 | P1 | — |

#### 3.3.1 `net::http` API 设计规范

> **设计目标**：对标 Go `net/http` 的简洁性 + Rust 的类型安全性。

```rust
// 服务端：对标 Go http.Handler + ServeMux
trait Handler {
    async fn handle(&self, req: &Request, res: &mut Response) -> Result<(), HttpError>;
}

struct ServeMux {
    // 路由表：method + path pattern → handler
}

impl ServeMux {
    fn new() -> Self;
    fn handle(&mut self, pattern: &str, handler: impl Handler);
    fn handle_func(&mut self, pattern: &str, f: fn(req: &Request) -> Response);
}

// 客户端：对标 Go http.Client
struct Client {
    timeout: Duration,
    // ...
}

impl Client {
    async fn get(&self, url: &str) -> Result<Response, HttpError>;
    async fn post(&self, url: &str, body: &[u8]) -> Result<Response, HttpError>;
}

// 中间件：函数组合（对标 Go 闭包模式）
fn logging_middleware(next: impl Handler) -> impl Handler {
    // ...
}
```

**关键设计决策**：
- 每个连接一个轻量级任务（对标 Go goroutine-per-connection，但零 GC）
- `Request` 携带 `Context`（对标 Go `context.Context`），支持超时/取消
- 路由支持 `GET /users/{id}` 模式匹配（对标 Go 1.22+ ServeMux）
- 中间件通过函数组合实现，无反射

### 3.4 编译器（P0/P1/P2）

| 编号 | 功能 | 优先级 |
|---|---|---|
| FR-028 | 前端：词法/语法分析、AST 构建 | P0 |
| FR-029 | 所有权检查器：编译期所有权/借用/生命周期检查 | P0 |
| FR-030 | 类型检查：类型推断、泛型单态化、trait 解析 | P0 |
| FR-031 | `@cImport` 实现：编译期 C/C++ 头解析与类型映射 | P0 |
| FR-032 | **C99 后端**：生成 C99 源码（**MVP 默认后端**，保证极小二进制、避免锁定未 1.0 的 Zig） | P0 |
| FR-032b | LLVM IR 后端：性能档 | P1 |
| FR-032c | **Zig 后端**：生成 Zig 源码（**P2，远期**——Zig 0.15+ 移除 async/await 且 `@Frame` 语义漂移，待其 async 模型稳定后复评） | P2 |
| FR-033 | Rust 后端：生成 Rust 源码，复用 Rust 生态 | P1 |
| FR-034 | C ABI 中间层：生成 C 源码 + 头文件 | P0 |
| FR-035 | 增量编译：秒级，对标 Zig | P1 |
| FR-036 | 交叉编译：LoongArch64/ARM64/RISC-V/x86_64 | P1 |
| FR-037 | **自举**：编译器用 MusLang 自身编写，自持闭环 | P2 |

#### 3.4.1 编译器架构

```
┌─────────────────────────────────────────────────────────────┐
│                   muslangc 架构                              │
│                                                             │
│  源码 (.mus)                                                │
│    │                                                        │
│    ▼                                                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ 词法分析  │→│ 语法分析  │→│ AST      │→│ 宏展开    │    │
│  └──────────┘  └──────────┘  └──────────┘  └────┬─────┘    │
│                                                  │          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐       │          │
│  │ HIR      │←│ 名称解析  │←│ 类型推断  │←─────┘          │
│  └────┬─────┘  └──────────┘  └──────────┘                  │
│       │                                                     │
│  ┌────┴─────┐  ┌──────────┐  ┌──────────┐                  │
│  │ MIR      │→│ 所有权检查 │→│ 借用检查  │                  │
│  │ (控制流)  │  │ (Move)   │  │ (Borrow) │                  │
│  └────┬─────┘  └──────────┘  └────┬─────┘                  │
│       │                            │                       │
│  ┌────┴─────┐  ┌──────────┐  ┌────┴─────┐                  │
│  │ THIR     │→│ 泛型单态化 │→│ 代码生成   │                  │
│  │(类型级IR) │  └──────────┘  │ (Zig/Rust/C)│                 │
│  └──────────┘                 └────┬─────┘                  │
│                                    │                        │
│                              ┌─────┴─────┐                  │
│                              │ .zig/.rs/.c │                  │
│                              └───────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

#### 3.4.2 `@cImport` 技术规范

| 能力 | 描述 | 优先级 |
|---|---|---|
| C 头文件解析 | 支持 `#include`、宏定义、`typedef`、结构体/联合体 | P0 |
| 类型映射 | C 类型 → MusLang 类型（自动推导） | P0 |
| 函数声明 | 自动生成 `extern "C"` 声明 | P0 |
| C++ 支持 | 通过 `extern "C++"` 块，支持命名空间、类（不透明指针 + 包装） | P1 |
| 宏展开 | 有限支持：编译期常量宏可展开 | P1 |
| 回调函数 | C 函数指针 ↔ MusLang 闭包 | P0 |

### 3.5 链接器（P0/P1/P2）

| 编号 | 功能 | 优先级 |
|---|---|---|
| FR-038 | **链接器集成**：默认调用 Zig 的 LLD 分支，支持 `--gc-sections` 死代码消除 | P0 |
| FR-039 | **Freestanding 链接**：不依赖 libc，支持自定义入口 `_start`，生成纯裸二进制 | P0 |
| FR-040 | **链接脚本**：支持 `-T script.ld`，自定义段布局（`.text`/`.data`/`.bss`） | P1 |
| FR-041 | **UEFI 输出**：生成 PE 格式 + UEFI 子系统，支持 Secure Boot | P1 |
| FR-042 | **`muslink` 自研链接器**：用 MusLang 写的极简 ELF64 链接器，作为信创纯自主后端 | P2 |

#### 3.5.1 `muslink` 技术规范

| 规格项 | 决策 |
|---|---|
| 输入格式 | ELF64 可重定位目标文件（`.o`） |
| 输出格式 | ELF64 可执行文件 / 静态库（`.a`） |
| 支持架构 | x86_64 → ARM64 → LoongArch64 → RISC-V64 |
| 重定位类型 | R_X86_64_64, R_X86_64_PC32, R_AARCH64_CALL26, R_AARCH64_ADR_PREL_PG_HI21, R_LARCH_32, R_LARCH_64, R_RISCV_64, R_RISCV_CALL |
| 动态链接 | ❌ 不支持（v1.0 范围外） |
| 静态链接 | ✅ 默认 |
| `--gc-sections` | ✅ 死代码消除 |
| 链接脚本 | ✅ 支持 GNU ld 脚本子集 |
| TLS 支持 | P1（初始执行器模型） |

### 3.6 异步模型（P0）

| 编号 | 功能 | 描述 |
|---|---|---|
| FR-043 | async/await | 语法与 Rust 一致，编译期展开为状态机 |
| FR-044 | FfiFuture | FFI-safe Future，`repr(C)`，跨语言异步统一 |
| FR-045 | 轻量级任务 | 栈池复用，无 GC，无栈协程 |
| FR-046 | 事件驱动 | epoll/kqueue/io_uring 后端 |
| FR-047 | 调度器 | work-stealing，可选，用户显式启用 |

#### 3.6.1 异步实现模型

> **决策背景**：MusLang **保留 Rust 风格 async/await 语法**（FR-043），编译期展开为状态机。底层运行时（epoll/kqueue/io_uring）通过 `Io` 接口抽象驱动，与具体后端解耦——因 MVP 默认后端为 C99（见 FR-032 修订），不依赖任何单一语言的 async 语义。

**实现映射**：

```
MusLang async/await  ──编译期──→  状态机 struct
                                  ├── poll(ctx, waker) → Poll<T>
                                  └── 底层由 Io 接口驱动（epoll/io_uring，
                                                        可替换为其他事件循环）
```

> **跨语言异步**：`FfiFuture`（FR-044）以 `repr(C)` 定义，作为 rt-c 边界上的稳定 ABI，使 C/C++/Rust 侧的 future 可在 MusLang 事件循环中驱动（详见 §3.7 D-6、D-7 的所有权移交协议）。

### 3.7 架构决策记录（v0.4）

> 本节为 v0.4 新增，收录经设计阶段审阅后**已收敛的架构决策**。每项标注编号（D-x）、结论与状态；未在此列出的事项仍按原章节「待定」处理，**保持不变**。完整审阅依据见仓库 `docs/audit/`（审计报告 v1→v9）。

#### 3.7.1 决策清单

| 编号 | 决策 | 结论 | 状态 |
|---|---|---|---|
| D-0 | 共享内存布局规范（IR/HIR） | 与语法**并列**为 v0.4 放行门槛；语法可改，IR 一旦发布不可改 | 待冻结 |
| D-1 | 语法基线 | **类 Rust**：保留 `fn/let/match/struct/impl` 可读性，自有正式语法，**不保证源码级兼容**（FR-001 拆为 001a 视觉相似 + 001b 明确不兼容） | 已定 |
| D-2 | 编译器后端 | MVP 默认 **C99**；LLVM = P1；**Zig = P2**（Zig 0.15+ 移除 async/await，`@Frame` 语义漂移，且避免锁定未 1.0 的语言） | 已定 |
| D-3 | 互操作机制 | Rust/C/C++ 通过**共享方言 + 统一 HIR** 实现编译期无损互操作，**无运行时 FFI 层**；方言层 = 薄前端（仅解析 + 类型化 + lowering） | 待冻结 |
| D-4 | `unsafe` 等价判定 | 定义 MusLang 侧禁止操作清单、检查层级、固定错误码；配套 `unsafe_examples/` + `--unsafe-allowed=false` CI 门禁 | P0，待落地 |
| D-5 | 是否引入 MLIR | 未决（D-5 定前不引入）；若引入需同步上修 `<8KB` 指标 | 待冻结 |
| D-6 | 调用拓扑 | 同类 runtime 内部调用 **0 开销**；仅**跨 rt / rt-c 边界**产生一次 ABI 成本 | 待冻结 |
| D-7 | 跨 `.so` 所有权移交协议 | **A（严格移交）+ C（标注协议）混合允许、函数级策略一致**（同函数内禁止 A/C 混用）；B（`Arc` across FFI）为 P1 可选；`MusAllocator::from_c` 桥接 deallocator 配对（详见 §3.8） | 已定（v0.4.1） |
| D-8 | 双 runtime | **muslang-rt**（MusLang 原生，自有布局/borrow/类型化 panic）+ **muslang-rt-c**（C 兼容，`repr(C)`、`malloc/free`、`errno`、完整用户态语义） | 已定 |
| D-12 | `defer` 语义（含 `?` / `async`） | `defer`/`errdefer` 分离（错误路径仅 `errdefer`、LIFO）；`?` 提前返回只跑 `errdefer`；`async fn` 取消仅在 `.await` 点、协作式，`defer` 禁止 `.await`、清理归调用方显式 `close().await`；5 类错误码（详见 §3.2.3.1） | 已定（v0.4.1） |
| D-9 | 标准库 | **按需链接**：每子系统为独立 crate（用哪个链哪个）；`sys` 层按 `target_os` 分发，**当前仅 Linux 实装**，macOS/Windows 留 trait + stub | 已定 |
| D-10 | 系统接口 | **rt-c = std 底层 + C 互操作边界 = 唯一系统接口**；std 走 rt-c，本身不产生 FFI 开销 | 已定 |
| D-11 | L1 范围 | `no_std` 用户态（**kernel 为远期，不在当前规划**）；是否含 libc 待确认（影响 rt-c 设计下限，建议默认 musl） | 需确认 |

#### 3.7.2 互操作：编译期机制（D-3）

```
C/C++ ─┐
Rust  ─┼──► 方言层（薄前端：解析 + 类型化 + lowering）
       │       │
       │       ▼
       │    统一 HIR ──► 共享借用检查器 / 类型系统 / 代码生成
       │       │
       │       ▼
       └──► C99（MVP 默认）/ LLVM（P1）/ Zig（P2，远期复评）

跨语言调用 = 同一模块内函数调用，无 ABI 边界 → 零开销
实现顺序：Rust > C > C++（Rust 最难，先啃硬骨头）
```

- **无损 = 共享内存布局**，非绑定生成器；`#[repr(C)]` 在 `extern` 块内默认；
- **C++ 推迟**：MVP 仅普通 struct + 函数指针 + 单继承；模板单态化（需重演类型推导）、虚继承、概念约束推至 1.0；
- Rust 侧最重：借用/`Send`/`Sync`/`Pin` 的语义降级，是方言层唯一实质负担。

#### 3.7.3 双 Runtime（D-8）

| | **muslang-rt** | **muslang-rt-c** |
|---|---|---|
| 布局 | 自有（vtable / 胖指针 / 借用信息） | `repr(C)`，与 C 一一对应 |
| 内存 | `MusAllocator` | `malloc/free`，可桥接 `MusAllocator::from_c` |
| 错误 | 类型化 `Result` | `errno` / 返回码 / `setjmp` |
| Panic | MusLang unwind | C `longjmp`（**用户态完整语义**） |
| 场景 | 纯 MusLang 插件生态 | 既有 C/C++ `.so`、std 底层、互操作边界 |

- runtime **可选、可动态链接、可在 L1 中剔除**——不强制绑定；
- 预编译 `.so` **可以使用**，代价仅为 **ABI 描述符体积（约数 KB）**，加载期一次性，调用期为 0；
- **L1 core**：无 runtime、无动态加载、`no_std` 用户态（kernel 远期，见 D-11）；
- **L2 hosted**：rt 或 rt-c **二选一**（单 runtime 实例，无"双堆税"）；
- **L3 compat**：rt + rt-c 并存，支持任意第三方 `.so`（反复跨两类边界的热点建议 `#[repr(C)]` 钉在 rt-c 侧）。

#### 3.7.4 标准库：按需链接 + 三端可扩展（D-9 / D-10）

```
your_app.mus
   use std::net::TcpStream;   ← 只声明用到的
        │
   std facade（统一 API，零成本抽象）
   fs / net / thread / time / io / fmt / ...
   （每子系统为独立 crate，可独立启用/剔除）
        │
   ┌──────────┬─────────────┬──────────────┐
   │std:os:linux│std:os:macos│std:os:windows │
   │ (当前实装) │ (trait+stub)│ (trait+stub)  │
   └──────────┴─────────────┴──────────────┘
        │
   muslang-rt-c  ──►  libc（glibc / musl / ...）
   （唯一系统接口 = std 底层 + C 互操作边界）
```

- **用哪个链哪个**：不使用的 crate（如 `net`）会被链接器整块 GC 剔除，体积仅由"实际 use 了什么"决定；
- **三端**：当前仅 **1 端（Linux）** 实装，macOS/Windows 仅保证**架构可扩展**（trait + 分发机制就绪），实装留后续；**三端工作量在实装不在架构**，建议 MVP 仅承诺"架构支持"，不承诺全 API 可用。

#### 3.7.5 调用拓扑与开销口径（D-6 / D-7）

| 场景 | 开销 |
|---|---|
| 同镜像、同类 runtime 内部调用 | **0**（全内联，无 ABI 边界） |
| 跨 rt / rt-c 边界 | 一次 C ABI 调用（≈一次函数指针调用，零序列化/拷贝） |
| 加载预编译 `.so`（L3） | 加载期：符号解析 + vtable 重映射（一次性）；调用期 **0** |
| 镜像体积（L3） | + ABI 描述符（约数 KB） |

> **关键澄清**：标准 `extern "C"` 调用本身**无序列化、无拷贝**；所谓"FFI 开销"只在**动态加载边界**与**跨两类 runtime** 时出现。将符号隔离从"每个调用现场"推到"模块/镜像边界"，FFI 开销即从 O(调用次数) 降为 O(模块数)。

#### 3.8 跨语言所有权移交协议（D-7 定稿，v0.4.1）

> **问题**：MusLang 侧 `Box<T>` + 借用检查（编译期，运行时无 borrow tracker）与 C 侧 `malloc/free` + 裸指针（运行时不管归属）之间，对象**跨 `.so` 边界**传递时须回答三个问题：① 谁分配、谁释放；② 借用安全能否跨越边界；③ `defer` / 清理逻辑在哪一侧生效。本小节为 D-7 定稿，**编译期强制，零运行时开销**。

##### 3.8.1 三种移交策略

| 策略 | 核心思路 | 释放责任 | 适用 |
|---|---|---|---|
| **A. 严格移交（Move across FFI）** | 跨边界时所有权**完整转移**，接收方全权负责释放 | 调用后**移交方不再持有** | 默认；简单、零开销、与 Rust FFI 一致 |
| **C. 标注协议（`#[muslang::*]` 属性）** | `extern "C"` 签名上**逐参数**标注归属（`#[muslang::take]` / `#[muslang::borrow]` / `#[muslang::ret]`） | 按标注在 MusLang 侧 / C 侧各负其责 | 复杂生命周期、C 回调、长生命周期共享 |
| **B. 共享所有权（`Arc` across FFI）** | `Arc<T>::into_raw` / `from_raw`，引用计数跨语言 | 双方均可安全持有，**须显式 `arc_drop`** | P1 标准库设施（`std::sync::ffi_arc`），**非默认** |

**MusLang 默认不隐式引入引用计数**——A / C 均为零开销，B 仅在确需共享时显式选用。

##### 3.8.2 核心规则：A + C 混合允许，但**函数级策略一致**（v0.4.1 决策）

> **一个 `extern "C"` 函数内，所有跨边界参数的所有权策略必须统一——要么全部 A（严格移交），要么全部 C（标注协议），不允许混用。**

```rust
// ✅ 合法：整函数统一为 C（每个参数均有标注）
extern "C" {
    #[muslang::strategy(C)]
    fn parse_request(
        req:  *const Request,   #[muslang::borrow]   // 调用期借用，MusLang 仍拥有
        out:  *mut *mut Response, #[muslang::ret]    // 返回值归 C 侧释放
    ) -> c_int;
}

// ✅ 合法：整函数统一为 A（默认，无需标注，全移交）
extern "C" {
    #[muslang::strategy(A)]
    fn take_buffer(buf: *mut u8, len: usize);   // 接收后 MusLang 全权负责
}

// ❌ 非法：同一函数内 A / C 混用 → 编译错误 E_FFI_MIXED_STRATEGY
extern "C" {
    fn bad(buf:  *mut u8,                 // 无标注 → 默认 A
          ctx:  *const Context  #[muslang::borrow]);  // 显式 C → 冲突
}
```

**函数级原子性的理由**：调用方在一次 FFI 调用中只需记住**一套**释放规则，避免出现「第一个参数你 free、第二个参数你别碰」的心智负担与审计盲区。这与 P1「零隐藏控制流」、P3「类型即安全边界」一致——归属信息**显式落在签名上**。

**推论（须明确，避免后续歧义）**：

1. **混合生命周期的 API 必须拆函数**——若一个 API 天然需要混合语义（如「输入 buffer 归调用方、输出对象归被调用方」），拆为两个 `extern "C"` 函数，各自选 A 或 C；不允许在同一签名内并列两种策略；
2. **跨边界借用仅 C 策略支持**——`#[muslang::borrow]` / `#[muslang::borrow_mut]` 表示调用期临时借用、MusLang 侧保留所有权；A 策略下**不存在借用**，移交即不可再触碰；
3. **回调（C → MusLang）统一走 C 策略**——C 侧长期持有句柄后回调 MusLang 的场景，句柄归属须由 `#[muslang::ret]` / `#[muslang::take]` 显式声明，禁止 A 策略下的「C 回调后又交还」模式；
4. **B（`Arc`）不受函数级一致性约束**——`Arc<T>` 本身即为跨语言共享的显式句柄（`Arc::into_raw` 产生 `*const T`、`Arc::from_raw` 回收），选用 B 即表示「双方共享、谁最后用完谁 drop」；但 B **须**配对 `arc_drop`，否则 leak。B 为 P1，MVP 不强制实现。

##### 3.8.3 借用安全跨越边界

借用检查为**编译期**机制，生成代码**无运行时 borrow tracker**；因此跨 `.so` 的借用本质是**约定**而非可检查项：

- **C 策略 + `#[muslang::borrow]`**：MusLang 侧在调用期间保证借用有效（编译期 NLL），C 侧拿到裸指针后**编译器无法约束其用法**——属「约定安全」，须由 §3.2.3.1 的 FFI 审计清单记录并在 `unsafe_examples/` 中覆盖；
- **A 策略**：移交后 MusLang 侧**立即失效**该句柄（类型系统将其视为 moved），C 侧为唯一所有者——这是唯一可被借用检查器**实质保证**的跨边界模式；
- **`#[repr(C)]` 类型跨边界**：布局与 C 一一对应（见 D-8 双 runtime 表），无需借用转换，是「借用安全」的最简情形。

> **结论**：要借用检查真正生效 → 用 A（严格移交）；要灵活共享 → 用 C（标注 + 约定 + 审计）。二者在函数级不混用，是安全与灵活的明确分界。

##### 3.8.4 分配器桥接：`MusAllocator::from_c`

为使「MusLang 分配的对象能被 C 侧 `free`」与「C 分配的对象能被 MusLang 的 `Box` 接管」**deallocator 配对**，要求 MusLang 侧**所有堆分配经统一 `MusAllocator`**，并提供 C 侧对接：

```rust
// 标准库接口（FR-021，P0）
pub trait Allocator {
    fn alloc(&self, layout: Layout) -> Result<*mut u8, AllocError>;
    fn dealloc(&self, ptr: *mut u8, layout: Layout);
}

impl MusAllocator {
    /// 用 C 的 malloc/free 作为 MusLang 的分配器 → MusLang 分配的对象可被 C 侧 free
    pub fn from_c(malloc: unsafe fn(usize) -> *mut c_void,
                  free:   unsafe fn(*mut c_void)) -> Self;
}
```

- **rt-c 默认对接 libc 的 `malloc/free`**（见 D-10），故 MusLang 的 `Box<T>` 在 rt-c 侧分配的对象，C 侧可直接 `free`——**deallocator 一致，无 double-free**；
- **若 C 侧用自定义 allocator**（如 jemalloc、分区池），须在 `extern "C"` 块中通过 `MusAllocator::from_c` 显式桥接，**双方必须配对同一个 allocator**；配对失败 → 编译期警告 `W_FFI_ALLOCATOR_MISMATCH`（可由 `--forbid-unsafe-allocator` 升为错误）；
- **L1 core（无 libc）**：由 `no_std` 显式分配器（bump / pool）接管，`from_c` 不可用，跨 `.so` 移交仅支持 A 策略 + `#[repr(C)]` 类型。

##### 3.8.5 清理逻辑归属（与 §3.2.3.1 衔接）

`defer` / `errdefer` 的清理**必须在资源所有者的那一侧执行**：

- 对象经 A 策略移交给 C → **MusLang 侧不得再 `defer` 关闭它**；关闭责任随所有权转移；
- 对象经 C 策略 `#[muslang::ret]` 返回给 C → C 侧负责，`defer` 仅在 C 侧（C 的 `defer` / `goto err`）；
- **跨边界对象的析构不得在两侧都注册**——否则 double-drop / use-after-free。编译器在 HIR FFI 校验 pass 中检查「移交方是否在 `defer` 中仍引用已移交句柄」，违反报 `E_DEFER_USED_AFTER_TRANSFER`（属 D-12 错误码体系）。

##### 3.8.6 编译期校验汇总（HIR FFI 校验 pass）

| 校验项 | 错误码 | 说明 |
|---|---|---|
| 同函数 A / C 策略混用 | `E_FFI_MIXED_STRATEGY` | §3.8.2 核心规则 |
| 未标注参数（非 A 策略上下文） | `E_FFI_MISSING_ATTRIBUTE` | C 策略须逐参数标注 |
| `#[muslang::borrow]` 标注于非指针类型 | `E_FFI_BAD_ATTRIBUTE` | 仅裸指针可用借用标注 |
| 移交方在 `defer` 中仍引用已移交句柄 | `E_DEFER_USED_AFTER_TRANSFER` | 与 §3.2.3.1 联动 |
| Allocator 配对失败 | `W_FFI_ALLOCATOR_MISMATCH`（可升级为错误） | §3.8.4 |
| B（`Arc`）未配对 `arc_drop` | `W_ARC_LEAK`（P1，动态检测可选） | §3.8.1 |

错误码固定、文档化，纳入 D-4 CI 门禁与 `--emit audit` 审计清单（每个跨边界调用点记录策略、归属、allocator 配对情况）。

##### 3.8.7 与 Rust FFI 的对应关系（供实现参照）

| MusLang | Rust FFI 等价 | 说明 |
|---|---|---|
| A：`#[muslang::strategy(A)]` | `Box::into_raw` / `Box::from_raw` | 移交即 moved，两侧不共享 |
| C：`#[muslang::borrow]` | `&T` / `&mut T` 穿过 `extern "C"` | 调用期借用约定 |
| C：`#[muslang::take]` | 接收 ` *mut T`，接管所有权 | 同 `Box::from_raw` |
| C：`#[muslang::ret]` | 返回 ` *mut T`，调用方负责 | 同 `Box::into_raw` |
| B：`Arc::into_raw` / `from_raw` | `Arc::into_raw` / `Arc::from_raw` | 引用计数跨语言 |

> 实现优先级：**A 策略（P0，M1）→ C 策略（P0，M1，含属性解析 + 审计清单）→ B（P1，随 `std::sync`）**。C++ `std::shared_ptr` 跨边界映射**推至 1.0**（见 §3.7.2 C++ 推迟清单），v0.4.1 不承诺。

---

## 4. 非功能需求

### 4.1 性能指标

| 指标 | 目标 | 参考 |
|---|---|---|
| 编译速度 | 增量 < 1s，全量 < 10s（中等项目） | Zig 同级 |
| 链接速度 | < 500ms（中等项目，muslink） | mold/lld 同级 |
| 二进制体积 | **L1 core**（不含 std、不含编译器）裸启动 **< 8KB**；测量协议：`x86_64-linux-musl`、`-Oz -flto`、静态链接、无动态加载（详见 §3.7 D-11） | Zig ~4KB |
| 运行时开销 | 零（无 GC、无运行时） | Zig 同级 |
| 内存安全 | 编译期保证，零 `unsafe` 块 | Rust 同级 |
| 并发吞吐 | 10 万连接 < 500MB | 优于 Go（无 GC 压力）|

### 4.2 兼容性

- C ABI：100% 兼容 System V AMD64 / AAPCS64
- C++ 互操作：支持 C 风格接口，复杂类型用不透明指针 + 包装函数
- 交叉编译：x86_64、ARM64、LoongArch64、RISC-V
- 操作系统：Linux（统信 UOS/银河麒麟）、macOS、Windows

### 4.3 安全模型

#### 4.3.1 内存安全保证

| 保证 | 机制 | 与 Rust 差异 |
|---|---|---|
| 无 use-after-free | 借用检查 + 生命周期 | 同 |
| 无双重释放 | 移动语义（值不可复制时） | 同 |
| 无缓冲区溢出 | 切片边界检查（debug 模式） | 同 |
| 无空指针解引用 | `Option<T>` + 不可为空指针 | **Zig 风格：`*T` 不可空** |
| 无数据竞争 | `Send`/`Sync` trait | 同 |

#### 4.3.2 安全边界审计

> **关键差异**：Rust 使用 `unsafe` 块标记不安全代码，MusLang 使用**类型标注**标记不安全边界。

```rust
// Rust：块级标注
unsafe {
    let ptr = 0xDEADBEEF as *mut i32;  // 不安全的块
    *ptr = 42;
}

// MusLang：类型级标注
fn kernel_mmap(addr: usize) -> *allowzero u8 {  // 返回类型即标注
    // ...
}

// 调用处无需 unsafe 块，但类型标注使审计精确到签名
let ptr: *allowzero u8 = kernel_mmap(0x1000);  // 审计清单记录此调用
```

**审计清单生成**（编译器内置）：

```yaml
# muslangc --emit audit 输出
ffi_boundaries:
  - file: kernel/mmap.mus
    line: 42
    function: kernel_mmap
    unsafe_type: "*allowzero u8"
    reason: "MMIO 映射，地址可为 0"
  - file: ffi/v8.mus
    line: 15
    function: v8_init
    extern: "C"
    reason: "V8 引擎初始化"
```

#### 4.3.3 其他安全维度

- FFI 安全：`extern` 声明即标志，类型系统区分安全/不安全
- 供应链安全：自举后零外部语言依赖
- 审计：所有 FFI 调用可审计（见 §4.3.2）

### 4.4 可用性

| 维度 | 目标 |
|---|---|
| 学习曲线 | Rust 开发者零学习成本，Zig 开发者 1 天上手 |
| 错误信息 | 友好、精准、带修复建议（对标 Rust） |
| 文档 | 语言参考、标准库文档、教程、示例 |
| 工具链 | 格式化、LSP、DWARF 调试信息 |

---

## 5. 错误处理规范

> **新增章节**：明确 MusLang 的错误处理模型，确保与 Rust 一致且与 Zig 互操作兼容。

### 5.1 错误模型

MusLang 采用 **Rust 风格 Result + ？传播** 模型[citation:21][citation:24]：

```rust
// 函数返回 Result<T, E>
fn parse_config(path: &str) -> Result<Config, ConfigError> {
    let data = fs::read_to_string(path)?;       // ? 传播错误
    let config = serde_json::from_str(&data)?;  // ? 自动类型转换（如有 From 实现）
    Ok(config)
}

// 调用方：必须显式处理
match parse_config("app.json") {
    Ok(config) => use_config(config),
    Err(e) => log_error(e),
}
```

### 5.2 错误类型体系

| 类型 | 用途 | 对应 Zig |
|---|---|---|
| `Result<T, E>` | 可恢复错误（必须有 Error trait 实现） | `error union` (`!T`) |
| `Option<T>` | 值可能不存在 | `?T` |
| `panic!` | 不可恢复（编程错误） | `@panic` |

### 5.3 后端映射

> **MVP 默认后端为 C99**（见 FR-032 / D-2），错误模型直接落为 C 的 `errno` + 返回码约定；Zig 后端（P2）映射保留如下供远期参考。

**C99 后端（默认）**：`Result<T, E>` → 返回值 + `errno` / 输出参数；`Option<T>` → 哨兵值或 `_Bool ok`；`?` → `goto err` 或早期返回；`panic!()` → `abort()`。

**Zig 后端（P2，远期）**：

| MusLang | Zig 后端输出 |
|---|---|
| `Result<T, E>` | `E!T`（error union） |
| `Option<T>` | `?T` |
| `?`（传播） | `try` |
| `panic!()` | `@panic()` |

### 5.4 `extern` 函数的错误处理

```rust
// C 函数返回错误码 → 自动包装为 Result
extern "C" {
    fn open(path: *const c_char, flags: c_int) -> c_int;  // 返回 -1 = 错误
}

// MusLang 包装（编译器或手写）：
fn open(path: &str) -> Result<c_int, IoError> {
    let fd = unsafe_open(path, O_RDONLY);  // *allowzero 边界
    if fd < 0 {
        Err(IoError::from_errno())
    } else {
        Ok(fd)
    }
}
```

---

## 6. 测试策略

> **新增章节**：明确测试层级、工具链、CI 集成要求。

### 6.1 测试层级

| 层级 | 工具/框架 | 覆盖范围 |
|---|---|---|
| 单元测试 | 内置 `#[test]`（同 Rust） | 函数/模块级 |
| 集成测试 | `tests/` 目录 | 跨模块交互 |
| 属性测试 | `proptest` 移植（P1） | 泛型/协议 |
| FFI 测试 | `@cImport` 双向调用 | C/C++ 互操作 |
| 差分测试 | Rust 版本对照（自举阶段） | 编译器正确性 |
| 基准测试 | `#[bench]` + Criterion 移植 | 性能回归 |

### 6.2 编译器测试基础设施

```
tests/
├── unit/              # 单元测试（每个模块对应 .mus 文件）
│   ├── ownership/
│   ├── typecheck/
│   └── codegen/
├── integration/       # 集成测试（完整编译 + 运行）
│   ├── hello_world.mus
│   ├── ffi_c_call.mus
│   └── http_server.mus
├── fuzz/              # 模糊测试
│   ├── fuzz_parser/
│   └── fuzz_typecheck/
├── spec/              # 语言规范一致性测试（逐条对应 RFC）
└── bootstrap/         # 自举验证
    ├── stage1_rust/   # Rust 写的前端
    └── stage2_muslang/ # MusLang 写的前端（自举后）
```

### 6.3 CI/CD 要求

| 平台 | 编译器 | 目标架构 | 频率 |
|---|---|---|---|
| Linux x86_64 | Zig 0.16+ | x86_64, ARM64 (cross) | 每次提交 |
| Linux ARM64 | Zig 0.16+ | ARM64 | 每次 PR |
| macOS | Zig 0.16+ | x86_64, ARM64 | 每次 PR |
| Windows | Zig 0.16+ | x86_64 | 每次 PR |
| 统信 UOS (LoongArch64) | Zig + muslink | LoongArch64 | 每日 |

---

## 7. 与 MusCat 生态的集成

```
┌─────────────────────────────────────────────────────────────┐
│                    MusCat 生态全栈                           │
│                                                             │
│  语言层：MusLang（Rust 语法 + Zig 安全 + Go 网络）           │
│    ├── MusKitty 内核（Layer 1-6）                           │
│    ├── MusCat 主程序（壳 + 内核加载器）                      │
│    ├── 内核 dll 适配层（Chromium/Gecko）                     │
│    ├── 私有 Cookie 协议（CBOR + 加密 + 审计）                │
│    ├── MCP Server（共用，多租户隔离）                        │
│    └── JS 引擎后端（V8/SpiderMonkey 适配）                   │
│                                                             │
│  构建链：MusLang 源码 → muslangc → Zig 后端 → .o            │
│        → muslink（自研链接器）→ 极小 ELF64 二进制            │
│  自举：muslangc（MusLang 写）+ muslink（MusLang 写）         │
│  信创：LoongArch64/ARM64 + 国密 SM2/SM3/SM4                 │
└─────────────────────────────────────────────────────────────┘
```

**关键设计**：MusLang 的网络标准库直接复用 MusKitty Layer 5 的网络栈——一套实现，全生态复用。信创场景自动用国密，Agent 场景自动审计。

---

## 8. 开发阶段与时间线

### 阶段一：Bootstrap（2026 Q1-Q2）

| 里程碑 | 交付物 | 验收标准 |
|---|---|---|
| 里程碑 | 交付物 | 验收标准 |
|---|---|---|
| **M1-0** | **决策冻结（2 周）** | `spec/`：`grammar.ebnf`、`memory-model.md`、`unsafe.md`、`backend-c99.md`、`std-sys.md`；D-0~D-11 定稿（**前置门槛，未冻结不进 M1-1） |
| M1-1 | MusLang 语法定义（类 Rust，FR-001a+001b）| 完整 grammar 文件 + 100% 语法测试通过 |
| M1-2 | 所有权检查器 v0.1 | 通过 Rust 测试用例子集（NLL 除外） |
| M1-3 | `*allowzero` 类型系统 | 类型检查 + 代码生成 + FFI 审计清单 |
| M1-4 | `@cImport` C/C++ 头解析 | 能解析 MusKitty 现有 C 头文件 |
| M1-5 | **C99 后端**代码生成 v0.1 | hello world 编译运行 |
| M1-6 | `net` 标准库第一版（TCP/HTTP 客户端）| 能通过 HTTP/1.1 访问真实服务 |
| M1-7 | MusLang 编译器 v0.1（Rust 写）| 端到端编译 + 测试通过 |
| M1-8 | 链接器集成：调用 Zig LLD，支持 `--gc-sections` | hello world 静态链接 < 16KB |

> **v0.4 裁剪建议（独立成立）**：M1 仍偏重，建议仅保 **Rust→C 单向导入**；C++ / Zig 后端 / 自举推至 v1.x。即便 kernel 移出 MVP，此裁剪仍成立。

### 阶段二：MusKitty 内核开发（2026 Q3-Q4）

| 里程碑 | 交付物 | 验收标准 |
|---|---|---|
| M2-1 | MusKitty Layer 1-4（基础）| 内核态基本功能（内存、进程、IPC） |
| M2-2 | MusKitty Layer 5（网络接驳）| TCP/UDP 可用，HTTP 服务可运行 |
| M2-3 | MusKitty Layer 6（JS 引擎适配）| V8/SpiderMonkey 可在内核态运行 |
| M2-4 | 统信 UOS / 银河麒麟适配 | 在真机上启动并运行基础服务 |
| M2-5 | 国密 SM2/SM3/SM4 集成 | 通过国密算法一致性测试 |
| M2-6 | Freestanding 链接 + 自定义链接脚本 | 内核镜像可直接加载 |

### 阶段三：自举与信创（2027 Q1-Q4）

| 里程碑 | 交付物 | 验收标准 |
|---|---|---|
| M3-1 | MusLang 编译器 v0.2（MusLang 写，前端）| 能编译自身 AST + 类型检查 |
| M3-2 | MusLang 编译器 v0.3（完整功能）| 全量功能等价 Rust 版本 |
| M3-3 | **自持闭环**（v0.3 编译 v0.3）| 用 MusLang 版编译器重新编译自身，输出一致 |
| M3-4 | 信创测评送测 | 通过 100% 测试用例 |
| M3-5 | 100% 信创替代达标 | 工具链零外部语言依赖 |
| M3-6 | `muslink` v0.1（MusLang 写的极简 ELF64 链接器）| 能链接 hello world + 内核镜像 |
| M3-7 | 全工具链纯自主验证（muslangc + muslink，零外部依赖）| 从源码到二进制完全自主 |

---

## 9. 语法示例

```rust
// hello.mus
use net::http;

struct Server {
    addr: String,
}

impl Server {
    fn new(addr: &str) -> Self {
        Server { addr: addr.to_string() }
    }
    
    async fn handle(&self, req: http::Request) -> http::Response {
        http::Response::ok("Hello, MusLang!")
    }
}

// FFI：extern 就是标志，不需要 unsafe 块
extern "C" {
    fn v8_init() -> i32;
}

fn main() {
    let ret = v8_init();  // 调用 extern 函数，无 unsafe 块
    if ret != 0 {
        panic!("v8 init failed");
    }
    
    let server = Server::new("0.0.0.0:8080");
    server.listen().await;
}
```

### 与 Rust 的关键差异

| 特性 | Rust | MusLang |
|---|---|---|
| 裸指针 | `*mut T` / `*const T` | `*allowzero T` / `*const T` |
| `unsafe` 块 | `unsafe { ... }` | ❌ 无 |
| FFI | `unsafe extern "C" { ... }` | `extern "C" { ... }` |
| 网络 | `tokio::net` | `net::tcp`（内置标准库）|
| 分配器 | 隐式（Box/Vec） | 显式传递（Zig 风格）|
| 链接器 | 依赖外部 ld/lld | **内置 LLD + 自研 muslink** |
| 二进制体积 | 中等 | ~4KB hello |
| 资源管理 | RAII（Drop trait） | `defer` 语句 |
| 错误传播 | `?` | `?`（同） |
| 可选值 | `Option<T>` | `Option<T>`（同） |
| 元编程 | 泛型 + 宏 | 待定（见 §12） |

---

## 10. 成功指标

| 指标 | 目标值 | 测量方法 |
|---|---|---|
| 信创测评通过 | 100% | 信创测评报告 |
| MusKitty 内核内存安全 | 零 CVE（内存安全类）| Coverity + 模糊测试 |
| 二进制体积 | **L1 core** hello world < 8KB（不含 std/编译器，见 §4.1 测量协议）| `size` 命令 |
| 编译速度 | 增量 < 1s | 自动化基准 |
| 链接速度 | < 500ms（中等项目）| 自动化基准 |
| MCP Server 并发 | 10 万连接稳定 | 压力测试（wrk/vegeta） |
| 自持 | 编译器自举成功 | 三阶段自举验证 |
| 链接器自主 | muslink 可用，信创纯自主模式可选 | 完整链接测试 |

---

## 11. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| Zig 后端不稳定（Zig 未 1.0）| 高 | 高 | 锁定 Zig 版本，维护兼容层；保留 Rust 后端作为 fallback |
| Zig async 模型变更（0.15+ 移除 async 关键字）| 高 | 中 | MusLang 自有 async/await → 状态机 IR，后端映射灵活 |
| 所有权检查器复杂度超预期 | 中 | 高 | 简化设计，先支持核心特性（NLL 延后） |
| 信创 Deadline 紧迫 | 中 | 高 | 聚焦 MusKitty，MusLang 先 Rust 后端 |
| 自举编译器调试困难 | 高 | 中 | 差分测试，Rust 版对照；保留完整测试套件 |
| C++ 复杂类型映射不全 | 中 | 中 | 不透明指针 + 包装函数 |
| LLD 依赖断供风险 | 低 | 高 | Fork Zig LLD 到 Gitee；muslink 作为纯自主后备 |
| muslink 开发延期 | 中 | 中 | 阶段一/二先用 Zig LLD，muslink 作为阶段三目标 |
| 社区生态匮乏 | 高 | 中 | 阶段一/二不追求生态，聚焦 MusCat 内部使用；阶段三再开放社区 |

---

## 12. 开放问题分析框架

> **说明**：以下开放问题保持"待定"状态，但每个问题均已补充 **分析维度、评估标准、决策时间节点**，确保后续决策有据可依。

### 12.1 元编程：`comptime` vs 泛型 + 宏

| 维度 | 方案 A：Zig 风格 `comptime` | 方案 B：Rust 风格泛型 + 宏 | 方案 C：仅泛型（无宏） |
|---|---|---|---|
| 表达力 | 极高（完整语言子集） | 高（语法树操作） | 中（声明式泛型） |
| 学习成本 | 中（语言本身延伸） | 高（独立宏系统） | 低 |
| 编译速度 | 中（解释器开销） | 中（宏展开 + 单态化） | 快 |
| 错误信息 | 好（普通代码） | 中（需 cargo expand） | 好 |
| 与 Rust 语法兼容 | ❌ 语法差异大 | ✅ 完全兼容 | ✅ 完全兼容 |
| 与 Zig 后端映射 | ✅ 直接映射 | ⚠️ 需翻译 | ✅ 可映射 |
| 自举复杂度 | 高（需编译期解释器） | 高（宏展开管道） | 低 |

**推荐方向**：阶段一采用 **方案 C（仅泛型）**，阶段二评估是否引入 `comptime`（方案 A）。理由：
1. 语法与 Rust 100% 一致是 P0 承诺，泛型是必须的
2. `comptime` 需要在自举编译器中实现完整解释器，复杂度极高
3. Zig 的 comptime 本身仍在演进（0.15+ 大幅重构），不宜过早绑定

**决策节点**：M3-1 完成后（自举编译器稳定），重新评估。

### 12.2 WASM 后端

| 维度 | 支持 | 不支持 |
|---|---|---|
| MusCat 浏览器集成 | ✅ JS 引擎可直接调用 | ❌ 需额外 FFI 层 |
| 编译器复杂度 | 高（+1 后端） | 低 |
| 生态复用 | ✅ 可编译为 WASM 运行在浏览器 | — |
| 优先级依据 | MusCat 浏览器核心需求 | 非 P0 用户场景 |

**推荐方向**：P1（阶段二启动）。MusCat 浏览器需要 WASM 支持来运行 Web 内容，MusLang 编译为 WASM 可在浏览器中运行 MusLang 代码。

**决策节点**：M2-3（MusKitty Layer 6 JS 引擎适配）启动时评估。

### 12.3 包管理器设计

| 维度 | 方案 A：复用 Cargo | 方案 B：自研（类 Zig build） | 方案 C：Git 依赖（类 Zig） |
|---|---|---|---|
| 生态复用 | ✅ 100% 兼容 crates.io | ❌ 从零建设 | ❌ 无中央注册表 |
| 开发成本 | 低（复用 Cargo） | 高 | 低 |
| 自主可控 | ❌ 依赖 Cargo | ✅ 完全自主 | ✅ 去中心化 |
| 信创适配 | ⚠️ Cargo 依赖网络 | ✅ 完全可控 | ✅ 离线可用 |
| 版本管理 | ✅ 成熟 | ❌ 需自建 | ⚠️ 手动管理 |

**推荐方向**：阶段一采用 **方案 A（复用 Cargo）**——MusLang 语法与 Rust 100% 一致，可直接复用 Cargo 构建系统（将 `.mus` 文件视为 Rust 的某种"受限子集"或使用 build script 调用 muslangc）。阶段三信创纯自主时，切换到 **方案 B**。

**决策节点**：M1-7 完成后评估包管理器需求。

### 12.4 调试信息格式

**决策**：DWARF 5（默认）。理由：
- Linux/Unix 生态标准
- LLDB/GDB 原生支持
- Zig 已支持 DWARF 生成，后端可直接复用

### 12.5 `muslink` 动态链接支持

**决策**：不支持（v1.0 范围外）。默认静态链接。理由：
- 内核场景无需动态链接
- 信创场景静态链接更可控
- 动态链接器（`.interp`）实现复杂度高，ROI 低

**未来评估**：v2.0 阶段（2028+）根据用户需求决定是否支持。

### 12.6 `muslink` 重定位类型支持

| 架构 | 必需重定位 | 优先级 |
|---|---|---|
| x86_64 | R_X86_64_64, R_X86_64_PC32, R_X86_64_GOTPCREL | P0 |
| ARM64 | R_AARCH64_CALL26, R_AARCH64_ADR_PREL_PG_HI21, R_AARCH64_ADD_ABS_LO12_NC | P0 |
| LoongArch64 | R_LARCH_32, R_LARCH_64, R_LARCH_B26 | P0 |
| RISC-V64 | R_RISCV_64, R_RISCV_CALL, R_RISCV_PCREL_HI20 | P1 |

---

## 13. 文档体系

> **新增章节**：明确文档组织结构，确保可用性目标达成。

```
docs/
├── prd.md                  # 本文档
├── language-reference/     # 语言参考手册（对标 Rust Reference）
│   ├── syntax.md
│   ├── type-system.md
│   ├── ownership.md
│   ├── ffi.md
│   └── async.md
├── std-lib/                # 标准库文档（对标 Rust std docs）
│   ├── net.md
│   ├── fs.md
│   └── ...
├── tutorials/              # 教程（对标 Go Tour）
│   ├── 01_hello_world.md
│   ├── 02_ownership.md
│   ├── 03_ffi.md
│   └── 04_http_server.md
├── internals/              # 编译器内部文档
│   ├── architecture.md
│   ├── ir-design.md
│   └── bootstrap.md
└── rfc/                    # RFC 提案（对标 Rust RFC）
    ├── 0001-error-handling.md
    └── 0002-comptime-design.md
```

---

## 14. 治理与版本管理

> **新增章节**：明确语言演进治理模型。

### 14.1 版本策略

| 阶段 | 版本 | 稳定性承诺 |
|---|---|---|
| 开发中 | 0.x | 无稳定性承诺，API 可随时变更 |
| 稳定版 | 1.0 | SemVer 兼容，至少 5 年维护 |
| 语言修订 | 每 6 个月 | RFC 流程驱动 |

### 14.2 RFC 流程

```
提案 → 讨论（2 周）→ 修订 → 最终评论（1 周）→ 合并/拒绝
```

### 14.3 治理角色

| 角色 | 职责 | 当前人员 |
|---|---|---|
| BDFL | 最终决策 | 墨染柒（Ink-dark） |
| 编译器团队 | 编译器实现 + 维护 | 墨染柒 + Orcha |
| 标准库团队 | 标准库 API 设计 | TBD |
| 信创适配团队 | 国产平台适配 | TBD |
| 文档团队 | 文档维护 | TBD |

---

## 15. 附录

### 15.1 参考资源

- [Zig 官方文档](https://ziglang.org/documentation/master/)
- [Zig Devlog 2024](https://ziglang.org/devlog/2024/)
- [Zig Bootstrap 仓库](https://codeberg.org/ziglang/zig-bootstrap)
- [Rust 语言参考](https://doc.rust-lang.org/reference/)
- [Rust 错误模型](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Go net/http 文档](https://pkg.go.dev/net/http)
- [Go net/http ServeMux (Go 1.22+)](https://go.dev/blog/routing-enhancements)
- [The Error Model (Joe Duffy)](https://github.com/rust-lang/rfcs/blob/master/text/0000-the-error-model.md)
- [Zig 新 Io 模型讨论](https://github.com/ziglang/zig/issues)
- [LLD 文档](https://lld.llvm.org/)
- [ELF 规范](https://refspecs.linuxfoundation.org/elf/elf.pdf)
- [MusKitty 仓库](https://github.com/Ink-dark/MusKitty)
- [The Error Model Convergence (matklad)](https://matklad.github.io/2025/12/29/second-error-model-convergence.html)
- [Zig 官方文档：`defer` / `errdefer` 语义（defer 正常路径执行、errdefer 仅错误路径、LIFO）](https://ziglang.org/documentation/master/)
- [Zig 概述：`defer` / `errdefer` 资源管理定性](https://ziglang.com.cn/)
- [Rust Internals：Asynchronous Destructors（取消 = Drop 触发、取消仅发生在 `.await` 点、无 async Drop）](https://internals.rust-lang.org/)
- [Huawei Trusted Programming：Rust Async Drop 编译器实现推进（async Drop 仍未稳定）](https://github.com/)
- [Tokio 实践：cancel safety 与 Future 丢弃语义（协作式取消、`select!` 输家被 drop）](https://docs.rs/tokio/)

### 15.2 术语表

| 术语 | 定义 |
|---|---|
| `unsafe` 逃逸 hatch | 允许绕过安全检查的机制（MusLang 中不存在） |
| 类型级安全标注 | 通过类型系统（而非代码块）标记不安全操作 |
| FFI 审计清单 | 编译器生成的、列出所有跨越安全边界的调用点的报告 |
| 自持闭环 | 编译器能用自身语言编译自身，不依赖其他语言的编译器 |
| Stage1 | 用宿主语言（Rust）编写的引导编译器 |
| Stage2 | 用目标语言（MusLang）编写的自举编译器 |
| Freestanding | 不依赖任何操作系统服务的编译目标（如内核、裸机） |
| GC-sections | 链接器死代码消除（Garbage Collection of Sections） |

### 15.3 变更记录

| 版本 | 日期 | 变更内容 | 作者 |
|---|---|---|---|
| v0.1 | 2026-08-30 | 初稿 | 墨染柒 |
| v0.2 | 2026-08-30 | 补充链接器章节（§3.5）、muslink 设计、更新时间线 | 墨染柒 |
| v0.3 | 2026-09-01 | **全面完善**：① 新增设计原则（§1.4）；② 新增类型系统章节（§3.2）；③ 新增错误处理规范（§5）；④ 新增测试策略（§6）；⑤ 补充编译器架构图（§3.4.1）；⑥ 补充 `@cImport` 技术规范（§3.4.2）；⑦ 补充 muslink 技术规范（§3.5.1）；⑧ 补充异步模型与 Zig 新 Io 模型关系（§3.6.1）；⑨ 完善安全模型（§4.3）；⑩ 完善风险表（§11）；⑪ 开放问题补充分析框架（§12）；⑫ 新增文档体系（§13）；⑬ 新增治理与版本管理（§14）；⑭ 补充术语表（§15.2） | Yuanbao (AI) |
| v0.4 | 2026-09-04 | **架构决策落地**：① 定位改为「类 Rust，不保证源码兼容」（§1.1，D-1）；② 新增架构决策记录（§3.7）含互操作架构、共享方言+HIR、D-0~D-11 决策表、双 runtime（muslang-rt / muslang-rt-c）、std 三层按需链接+sys 三端可扩展；③ 补充 `unsafe` 等价判定与 CI 门禁（§3.2.1 K3，D-4）；④ 默认后端由 Zig 改为 C99（FR-032、§3.4.1、§5.3，D-2）；⑤ 异步模型改为后端无关实现（§3.6.1）；⑥ 二进制体积口径定稿为 L1 core 裸启动 `<8KB`（§4.1、§10）；⑦ 新增 M1-0 决策冻结阶段与 v0.4 裁剪建议（§8） | Yuanbao (AI) |
| v0.4.1 | 2026-09-04 | **语义定稿**：① `defer` 与 `?`、async 取消交互规则（§3.2.3.1，D-12）：`defer`/`errdefer` 分离（错误路径仅 `errdefer`、LIFO）、`?` 提前返回只跑 `errdefer`、`async fn` 取消仅在 `.await` 点（协作式）、`defer` 禁止 `.await`（异步清理须显式 `close().await`）、5 类错误码，并附 Zig 官方文档 + Rust Internals + Tokio 引用（§15.1）；② 跨 `.so` 所有权移交协议 A+C 混合、函数级策略一致性（§3.8，D-7 定稿）：A 严格移交 / C 标注协议（`#[muslang::strategy]`、`#[muslang::borrow/take/ret]`）/ B `Arc` 为 P1 可选、同函数禁止 A/C 混用、混合生命周期须拆函数、回调走 C、`MusAllocator::from_c` deallocator 配对、清理归属与 §3.2.3.1 衔接、6 项 HIR FFI 校验错误码、Rust FFI 对应关系表 | Yuanbao (AI) & 墨染柒 |

---

> **MusLang — Rust 的壳，Zig 的灵魂，类似 Go 的网络，MusCat 的心脏。**
