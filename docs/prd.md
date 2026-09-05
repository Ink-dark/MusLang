# MusLang — 产品需求文档（PRD）

> **项目代号**：MusLang-Qomolangma
> **仓库**：https://gitee.com/moranqidarkseven/MusLang
> **所属生态**：MusCat 浏览器的原生系统编程语言
> **文档版本**：v0.4.4（网络运行时绑定）
> **创建日期**：2026-08-30
> **更新日期**：2026-09-05
> **作者**：墨染柒（Ink-dark）
> **状态**：评审中
> **变更记录**：
> - v0.1（2026-08-30）：初稿
> - v0.2（2026-08-30）：补充链接器章节、muslink 设计
> - v0.3（2026-09-01）：全面完善——补齐错误处理、测试策略、API 规范、安全模型形式化、开放问题分析框架、模块系统、文档体系、CI/CD、治理模型
> - v0.4（2026-09-04）：架构决策落地（§3.7 架构决策记录、后端 C99 默认、双 runtime、std 按需链接 + 三端、二进制体积口径定稿、M1-0 决策冻结）
> - v0.4.1（2026-09-04）：语义定稿——① `defer` 与错误传播 / 异步取消的交互规则（§3.2.3.1，D-12）；② 跨 `.so` 所有权移交协议 A+C 混合、函数级策略一致性（§3.8，D-7 定稿）；③ 配套引用补充（§15.1）
> - v0.4.2（2026-09-04）：语言机制细化——① 泛型单态化策略：HIR 层单态化 + 递归深度硬限 25 层、超限为语法错误（§3.9，D-13）；② 生命周期标注策略：函数签名默认全省略、推断失败给建议报错、结构体/枚举不可省、HRTB 推迟 1.0、FFI 边界强制标注（§3.10，D-14）；③ C++ 互操作边界定稿：Itanium ABI、异常禁止跨越边界、RTTI 不支持、模板须 C++ 侧预实例化（§3.11，D-15）
> - v0.4.3（2026-09-05）：工程路径定稿——① 自举路径与 staging 策略（§3.12，D-16）：M1 部分自举（前端 MusLang 写 + 后端 Rust 写）、Stage 0 宿主语言 = Rust、先编译 std 再编译编译器、确定性输出（可重现构建）推迟至 M2；② 包管理器（§3.13，D-17）：M1 = Cargo 复用（不造轮子），远期 = `mktplace`（谐音 marketplace，MusLang 源码级包管理 + 工作区编排，设计参考 hypo 分发安全模型 + pets_tools 编排能力）；③ 软件分发（§3.14，D-18）：hypo 为独立的系统级软件分发工具，`mktplace` 管"源码/依赖/构建"、hypo 管"产物分发部署"，二者各司其职；§3.7.1 决策表追加 D-16/D-17/D-18，§12.3 重写为三阶段
> - v0.4.4（2026-09-05）：运行时绑定定稿——① `net::http` / 整个 `std::net` 子系统的事件循环（event loop / executor）**作为 `std::net` 的依赖自动带入**，用户 `use std::net` 即链接，不提供独立的"运行时选择"、不引入 `#[entry]` / `block_on` 之类的注入 API（§3.15，D-19）；② 事件循环 MVP = **单线程 epoll**（一个 loop 处理全部连接），多线程 / work-stealing 为 P1（FR-047）；③ 高级替换路径：不使用 `std::net`、自行基于 syscall 实现网络层（裸机 / 嵌入式场景）；④ 澄清 P5「零运行时」语义（§1.4、§3.15.4）

---

## 1. 产品概述

### 1.1 一句话定位

**MusLang 是一门语法以 Rust 为参照（不保证源码级兼容）、安全模型以 Zig 的类型区分取代 `unsafe` 块、内置 Go 级网络标准库、编译产物与 Zig 同级轻量的系统编程语言。** Rust、C/C++ 三方通过**共享内存布局规范与统一 HIR** 实现编译期无损互操作（无运行时 FFI 层）；标准库按子系统拆分为独立 crate、按需链接，并通过 `sys` 层支持 Linux / macOS / Windows 三端扩展（当前仅 Linux 实装）。MusLang 是 MusCat 生态全栈自研的最后一环——从语言到编译器到链接器到内核到浏览器，全栈自主可控。

> **说明（v0.4.4）**：本段仅反映已冻结/已决策的架构方向，详见 §3.7「架构决策记录」（D-0~D-19）。尚未决策的事项（WASM、分配器模型、调试信息、comptime 元编程等）仍见 §12，保持「待定」不动；自举 staging、包管理器、软件分发、async 运行时绑定已于 v0.4.3 / v0.4.4 定稿（§3.12、§3.13、§3.14、§3.15）。**async 运行时（event loop / executor）不作为独立"运行时选择"暴露给用户，而是 `std::net` 的内部依赖（D-19）**。

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
| FR-037 | **自举**：编译器用 MusLang 自身编写，自持闭环（**M1 部分自举**：前端 MusLang 写、后端 Rust 写，见 §3.12 / D-16；完整自举推迟至 M2/M3） | P2 |

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
| D-13 | 泛型单态化 | **HIR 层单态化**（后端只见到单态后具体函数）；**递归单态化深度硬限 25 层，超限 = 语法错误**；不引入 `dyn` 类型擦除；配合按需链接 + `--gc-sections` 控制膨胀 | 已定（v0.4.2） |
| D-14 | 生命周期标注 | **方案 B**：函数签名默认全省略、推断失败给建议报错；结构体/枚举**不可省**；`'static` 保留；**HRTB 推迟 1.0**；`extern` FFI 边界强制显式标注 | 已定（v0.4.2） |
| D-15 | C++ 互操作边界 | **1.0 边界定稿**：vtable = **Itanium C++ ABI**；**异常禁止跨越 FFI 边界**（编译器拒绝 throwing 签名）；**RTTI 不支持**；**模板须 C++ 侧预实例化**（MusLang 只见到具体类型） | 已定（v0.4.2） |
| D-16 | 自举路径（staging） | **M1 部分自举**：编译器前端（lexer/parser/AST/类型检查）用 MusLang 编写，后端（C99 代码生成 / LLVM 桥接）仍用 Rust 编写；**Stage 0 宿主语言 = Rust**；构建顺序**先编译 std 再编译编译器**（std 依赖 HIR 单态化 D-13 完整支持）；**确定性输出（可重现构建）推迟至 M2**；完整自举（含 C99 后端）推迟至 M2/M3 | 已定（v0.4.3） |
| D-17 | 包管理器 | **M1 = Cargo 复用**（不造轮子，`.mus` 作为 Rust 受限子集 + `build.rs` 驱动 `muslangc`）；**远期 = `mktplace`**（谐音 marketplace，MusLang 源码级包管理器 + 工作区编排器），设计参考 **hypo 的分发安全模型** + **pets_tools 的工作区编排能力**；`mktplace` 与 hypo **各司其职**（见 §3.13、§3.14） | 已定（v0.4.3） |
| D-18 | 软件分发 | **hypo 为独立的系统级软件分发工具**（非构建时包管理器）；MusLang 编译产物经 hypo 做系统级分发部署，**hypo 与 `mktplace` 分工明确**：`mktplace` 管"源码怎么组织、依赖怎么拉、怎么构建"，hypo 管"编好的二进制/库怎么分发部署到目标机" | 已定（v0.4.3） |
| D-19 | `net` 运行时绑定 | **事件循环（event loop / executor）作为 `std::net` 的内部依赖自动带入**：用户 `use std::net` 即链接，**不提供独立的"运行时选择"**、**不引入 `#[entry]` / `block_on` 注入 API**（对标 Go，P7「网络开箱即用」）；**MVP = 单线程 epoll**（一个 loop 处理全部连接），多线程 / work-stealing = P1（FR-047），io_uring = P1；executor 内部用 **`Box<dyn Future>`**（D-13 泛型单态化的**唯一例外**，避免 `Spawn<F>` 泛型爆炸）；不用 `std::net` 时 event loop 完全不链接，`<8KB` 仍可达（D-11）；嵌入已有 C 事件循环 / 内核场景**不使用 `std::net`**、自行基于 `std::sys` + `FfiFuture`（FR-044）实现；并**澄清 P5「零运行时」=「零强制运行时」**（可选、不用不链、用了也透明，§3.15.4） | 已定（v0.4.4） |

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

### 3.8 跨语言所有权移交协议（D-7 定稿，v0.4.1）

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

> 实现优先级：**A 策略（P0，M1）→ C 策略（P0，M1，含属性解析 + 审计清单）→ B（P1，随 `std::sync`）**。C++ `std::shared_ptr` 跨边界映射**推至 1.0**（见 §3.11，D-15），v0.4.2 不承诺。

---

### 3.9 泛型单态化策略（D-13，v0.4.2）

> **问题**：MVP 默认后端为 **C99**（D-2），而 C 编译器无法识别"同一泛型的多个实例"、也无法跨实例做去重/合并。若 std 大量使用泛型（`Vec<T>`、`Option<T>`、`Result<T,E>`、`HashMap<K,V>`…），直译成 C 会产生代码膨胀，直接威胁 L1 `<8KB` 目标（D-11）。本小节定义单态化的**时机**与**边界**，属编译期机制、零运行时开销。

#### 3.9.1 核心规则（v0.4.2 定稿）

1. **单态化在 HIR 层完成**。泛型参数在类型检查通过后、 lowering 到后端 IR 之前展开为具体类型实例；**C99 / LLVM / Zig 三个后端只见到单态后的具体函数**，无需各自理解泛型语法。三方言（Rust/C/MusLang）共享同一套单态化逻辑，行为一致，后端实现简单。

   ```
   MusLang 源码                HIR 层单态化                C99 后端见到
   ─────────────               ──────────────              ──────────────
   fn first<T>(s: &[T]) -> &T  ──►  first_i32(&[i32])->&i32   （具体 C 函数）
                                 first_f64(&[f64])->&f64
                                 first_str(&[&str])->&&str
   ```

2. **全单态化，不引入类型擦除**（`dyn Trait` / `void*` 退化）。理由：MusLang 承诺"类 Rust、零成本抽象"，引入运行时类型擦除即背叛该承诺；且单态化后的小函数在 C 编译器内联 + `--gc-sections` 作用下可被整体优化掉，实测膨胀量远小于直觉估计。

3. **递归单态化深度硬限 25 层**。**超限 = 语法错误**（在解析 / HIR 构建阶段直接拒绝，**不是**编译警告、也**不是**链接期报错——写了 `Vec<Vec<Vec<…>>>` 嵌套过深即视为源码不合法，与"写了 100 层嵌套 `if`"同类）。

   - 25 层为**语言层面硬上限**，不可通过编译选项放宽（避免"改个 flag 就行为不同"的可移植性问题）；
   - 计数器定义：`MonoDepth(e) = 1 + max(MonoDepth(t_i))`，对类型参数取各实参最大值求和；触发 `MonoDepth > 25` 即报错。

4. **不引入 `dyn Trait` 式类型擦除作为补充**（v0.4.2 决策）。若某 API 确实需在运行时持有多种类型，统一通过 `enum` + 显式分发表达；这是工程取舍，代价是少量样板代码，收益是语义透明、与 C ABI 天然兼容。

#### 3.9.2 膨胀控制：三层工程缓解（配合 D-13）

| 缓解层 | 机制 | 效果 |
|---|---|---|
| **std 按需链接**（D-9） | 未实例化的类型组合不进入编译单元 | 砍掉绝大多数潜在膨胀 |
| **C 编译器内联** | 单态后的小函数 inline 进调用者，函数体被优化消除 | 消除调用开销 + 冗余函数体 |
| **`--gc-sections`** | 每个单态函数独立 section，未引用者链接期丢弃 | 最终二进制仅含被调用的实例 |

> **结论**：L1 `<8KB` 目标（D-11）**可达**——前提是上述三层协同生效。若实测某 std crate 单态后膨胀超标，应通过**拆分 crate / 减少公共泛型表面**解决，而非放宽 D-13 规则。

#### 3.9.3 错误码（属 D-4 CI 门禁）

| 错误 | 触发条件 |
|---|---|
| `E_MONO_DEPTH_EXCEEDED` | 递归单态化深度 > 25 层 |
| `E_MONO_CYCLE` | 类型定义出现非终止的递归实例化（防无穷展开） |
| `E_MONO_UNINSTANTIATED` | 泛型未在可见范围内被具体实例化即被导出 |

---

### 3.10 生命周期标注策略（D-14，v0.4.2）

> **问题**：MusLang "类 Rust"（D-1），但 Rust 生命周期标注的学习门槛集中在：① 结构体里 `'a` 无处不在；② 报错要求用户标注但用户不知标什么；③ HRTB 的 `for<'a>` 反直觉。本小节在不背叛零成本抽象、不切断与 Rust 生态 mental model 的前提下，定义**省略规则与显式标注的边界**。规则为编译期，零运行时开销。

#### 3.10.1 核心规则（方案 B）

1. **函数签名：默认全省略**。所有输入引用参数由编译器自动分配独立生命周期变量；返回值若只引用一个参数，自动绑定到该参数（等价于 Rust 省略规则 #1）。

   ```rust
   // 用户书写（省略）
   fn longest(x: &str, y: &str) -> &str { if x.len() > y.len() { x } else { y } }

   // 编译器内部推断为
   fn longest<'a, 'b>(x: &'a str, y: &'b str) -> &'a str { … }
   ```

2. **推断失败 → 编译错误 + 建议**。当返回值引用**多个**输入参数、编译器无法确定绑定时，**报错并给出具体标注建议**（对标 Rust 的错误体验），而非静默生成错误代码：

   ```rust
   // 编译错误 E_LIFETIME_AMBIGUOUS：
   // "返回值同时引用 'a 与 'b，请显式标注生命周期参数"
   fn pick<'a, 'b>(x: &'a i32, y: &'b i32) -> &'a i32 { … }  // 建议写法
   ```

3. **结构体 / 枚举：不可省略**。类型定义中的生命周期参数影响内存布局（引用大小），不能靠推断；此点与 Rust 一致，**不妥协**。

   ```rust
   struct Ref<'a> {
       r: &'a i32,   // 'a 必须显式
   }
   ```

4. **`'static`：保留，语义与 Rust 一致**（`&'static str`、`'static` trait bound 等）。

5. **HRTB（`for<'a> Fn(&'a T)`）：推迟到 1.0**。MVP 不支持；需要高阶 trait 约束的场合（如部分 async trait）用具体生命周期替代。理由：使用频率极低（主要是泛型库 / async trait），MVP 不值得引入语法与推断复杂度。**D-14 明确 HRTB 为 1.0 项。**

6. **FFI 边界：强制显式标注**（与 D-4 审计清单衔接）。`extern "C"` / `extern "C++"` 函数签名中所有引用参数**必须**标注生命周期，不允许省略——FFI 边界的可读性优先于便利性，且便于审计清单精确定位安全边界。

#### 3.10.2 省略规则速查

| 场景 | 是否可省略 | 说明 |
|---|---|---|
| 函数参数 & 返回值（单引用） | ✅ | 自动绑定到唯一参数 |
| 函数参数 & 返回值（多引用） | ❌（推断失败时） | 须显式标注，编译器给建议 |
| 结构体 / 枚举字段 | ❌ | 影响布局，永远不可省 |
| trait 定义（`&self` 等） | ✅ | 沿用 Rust 省略惯例 |
| HRTB `for<'a>` | ❌（1.0 才引入） | MVP 不支持 |
| `extern "C/C++"` 签名 | ❌ | FFI 强制显式 |

#### 3.10.3 错误码（属 D-4 CI 门禁）

| 错误 | 触发条件 |
|---|---|
| `E_LIFETIME_AMBIGUOUS` | 返回值引用多个输入，需显式标注（附带建议） |
| `E_LIFETIME_STRUCT_OMITTED` | 结构体/枚举字段遗漏 `'a` |
| `E_LIFETIME_FFI_OMITTED` | `extern` 签名中引用参数未标注 |
| `E_HRTB_NOT_SUPPORTED` | MVP 使用 `for<'a>`（提示推迟至 1.0） |

---

### 3.11 C++ 互操作边界（D-15，v0.4.2）

> **问题**：C++ 此前定为"推迟到 1.0"（§3.7.2 推迟清单），但"推迟"不等于不设计——PRD 须明确 **1.0 到底支持什么、不支持什么**，否则为空头承诺。本小节定稿 C++ 互操作的三个核心子问题：**vtable 布局、异常、RTTI**，以及模板策略。**所有规则为编译期 / ABI 约定，零运行时开销。**

#### 3.11.1 核心规则（v0.4.2 定稿）

1. **vtable：绑定 Itanium C++ ABI**。MusLang 的虚函数布局与 GCC / Clang 的 Itanium ABI 对齐（vtable 结构、偏移、RTTI 指针槽位一致）。

   - **理由**：信创场景（统信 UOS / 银河麒麟）全栈 GCC / Clang，无需 MSVC 兼容；Windows 留作远期。MusLang 与 C++ 可直接互调虚函数，无需 thunk 层。
   - **单继承**：P0 支持（§3.7.2 已列）；**虚继承**：1.0 不支持（Itanium 虚继承的 vtable 多表结构复杂，MVP 回避）。

2. **异常：禁止跨越 FFI 边界**。

   | 选择 | 行为 |
   |---|---|
   | **编译期拦截（D-15 采用）** | `extern "C++"` 函数若可能 throwing，编译器**直接拒绝**；需抛异常的 C++ 代码必须在边界内 `catch` 并转为错误码 |
   | 运行时 `std::terminate` | ❌ 不采用——崩进程不如编译期拦 |
   | 异常转换（→ `Result` / panic） | ❌ 不采用——C++ 异常展开与 rt / rt-c unwind 语义不一致 |

   > **原则**：C++ 侧负责"把异常消化在边界内"，MusLang 侧假设所有 `extern "C++"` 调用都不会 unwind。此约定与 `-fno-exceptions` 编译 C++ 代码可配合使用。

3. **RTTI：`dynamic_cast` / `typeid` 完全不支持**（1.0）。

   - MusLang 侧不能 `dynamic_cast`，C++ 的 `typeid` 在 MusLang 不可见；
   - **理由**：RTTI 依赖 C++ 运行时（`libstdc++`/`libcxx`），与 L1 `<8KB` 目标（D-11）冲突；且系统编程中 `dynamic_cast` 本不该常用。向下转型统一通过**显式判别字段 + `match`** 表达。

4. **模板：须 C++ 侧预实例化**。MusLang **不能**实例化 C++ 模板，只能调用 C++ 侧已编译好的**具体类型 / 具体函数**。

   - 例：`std::vector<int>` 若 C++ 侧已实例化，MusLang 可通过 `extern "C++"` 声明其具体接口；`template<typename T> class Foo` 的通用形式不可见于 MusLang。
   - 这是 §3.7.2 推迟清单中"模板单态化（需重演类型推导）推至 1.0"的具体化。

#### 3.11.2 1.0 支持边界一览

| 能力 | 1.0 支持 | 说明 |
|---|---|---|
| C++ 普通函数（non-throwing） | ✅ | `extern "C++"` 声明 |
| POD struct 字段访问 | ✅ | `#[repr(C)]` 对齐 Itanium 布局 |
| 单继承 + 虚函数调用 | ✅ | Itanium vtable 布局一致 |
| 虚继承 | ❌ | 远期 |
| **异常跨越边界** | ❌ | **编译期禁止** |
| **RTTI / `dynamic_cast`** | ❌ | **完全不支持** |
| **模板（MusLang 侧实例化）** | ❌ | **须 C++ 侧预实例化** |
| `std::shared_ptr` 跨边界 | ❌ | 远期（B 策略，`Arc` across FFI，见 §3.8） |

> **一句话总结（1.0）**：能调用 C++ 的 **non-throwing 自由函数**、能访问 **POD struct 字段**、能通过 **Itanium ABI 调用虚函数**；**不能**让异常跨越边界、**不能**用 RTTI、**不能**让 MusLang 实例化 C++ 模板。C++ 的复杂特性统一通过**不透明指针 + 包装函数**（已在 §3.2 / §3.4.2 定义）降级为 C 兼容接口。

#### 3.11.3 与既有章节的衔接

- **与 D-3（共享方言+HIR）**：C++ 的虚函数 / 继承语义在方言层 lowering 为 MusLang trait 对象，vtable 布局须与 Itanium 一致，由 HIR 单态化（§3.9）统一处理；
- **与 D-7（所有权移交，§3.8）**：C++ 对象跨边界一律走 **A（严格移交）**，`std::shared_ptr` 等引用计数类型不跨边界（见上表）；


---

### 3.12 自举路径与 staging 策略（D-16，v0.4.3）

> **问题**：自举（bootstrap）是编译器项目的"成年礼"——能否用 MusLang 写 `muslangc` 并编译通过、跑起来，决定语言是否真正自洽。但**完整自举（含 C99 后端）工作量极大**，M1 时间窗（2026 Q1-Q2）难以承受；且 std 自身依赖 HIR 单态化（D-13）完整支持，Stage 0 必须具备完整语言实现。**本小节定义分阶段自举路径，属工程规划、不影响语言语义。**

#### 3.12.1 经典三阶段

| 阶段 | 做什么 | 产物 |
|---|---|---|
| **Stage 0** | 用宿主语言（Rust）写第一个 `muslangc` | 能编译 MusLang → C99 的引导编译器 |
| **Stage 1** | 用 MusLang 写 `muslangc` 源码，用 Stage 0 编译它 | 自举编译器二进制（由 MusLang 代码生成） |
| **Stage 2** | 用 Stage 1 二进制重新编译同一份 MusLang 源码 | 验证产出一致性（比特级或语义级） |

**Stage 2 通过 = 自举完成**，语言可以"自己养自己"（§10 成功指标"自持"）。

#### 3.12.2 核心难点（MusLang 特有）

1. **Stage 0 须实现完整语言**。Stage 1 的 MusLang 编译器源码**必然使用 MusLang 全部核心特性**（泛型、`defer`/`errdefer`、`?`、模块系统、可能还有 `unsafe` 类型标注），故 Stage 0 必须实现完整 MusLang——**不能只是子集**，否则 Stage 1 源码用到某特性 → Stage 0 不认识 → 编译失败。

2. **std 依赖链**。编译器需要 `Vec`/`HashMap`/`String`、`fs` 文件 I/O、`Result<T,E>` 到处皆是，可能还需 `alloc`（FR-021）。若 std 用 MusLang 编写（合理假设），自举链为：
   ```
   Stage 0 编译 std 源码 → 得到 std 的 C99 产物
   Stage 0 编译 muslangc 源码（依赖 std）→ 链接 std 产物 → 得 Stage 1
   ```
   **std 编译又依赖 HIR 单态化（D-13）完整支持**——Stage 0 必须把泛型单态化做对，否则 `Vec<T>` 在 std 里无法展开。此为 D-13 与 D-16 的直接耦合。

3. **泛型重灾区与膨胀**。编译器是泛型重度使用者（`AstNode<T>`、`ParseResult<T>`、`Vec<Token>`、`HashMap<Symbol, Def>`…），全部单态化为 C99 产物可能膨胀至数 MB。**须依赖 D-13 的三层缓解（按需链接 + C 内联 + `--gc-sections`）**，并可能要求 std 拆分 crate 以控制单态爆炸。

4. **可重现构建（Reproducible Build）**。Stage 1 与 Stage 2 产出应一致，但 C99 后端生成的 C 代码可能含临时变量名计数器、单态函数哈希命名（`Vec_i32_0xabc`）、时间戳/路径名，导致 Stage 1 ≠ Stage 2。**须定义"确定性输出"规范（见 §3.12.4）。**

#### 3.12.3 分阶段方案（D-16 定稿）

**务实路线：M1 部分自举，完整自举推迟至 M2/M3。** 具体：

| 阶段 | M1 目标 | M2/M3 目标 |
|---|---|---|
| Stage 0 | Rust 写的**完整**引导编译器（覆盖 MusLang 核心全集：泛型 + `defer`/`errdefer` + `?` + 模块 + HIR 单态化 D-13） | 完善至 100% 语言特性（含 C++ `extern`、HRTB 等） |
| Stage 1 | MusLang 写**编译器前端**（lexer/parser/AST/名称解析/类型检查/借用检查），**后端（C99 代码生成、LLVM 桥接）仍用 Rust** | 完整编译器（含 C99 后端）用 MusLang 编写 |
| Stage 2 | **M1 不做**（见 §3.12.4） | 完整自举验证 + 可重现构建 |

**选择"部分自举"的四点理由**：

1. **M1 核心是"能编译出 L1 `<8KB` 的程序"（D-11），不是"语言能自举"**——部分自举即可验证语言设计自洽；
2. **完整自举工作量 6~12 个月**，M1 时间窗扛不住；
3. **前后端分离**使"前端 MusLang 写"能尽早验证语言可用性，后端可沿用成熟的 Rust 生态（inkwell/LLVM crate）；
4. **与 D-17（Cargo 复用）协同**——M1 构建系统本身就是 Cargo，Stage 0/1 混合 Rust+MusLang 的 workspace 编排天然可行。

#### 3.12.4 四项工程决策（v0.4.3 定稿）

| # | 决策 | 结论 |
|---|---|---|
| 1 | **M1 自举目标** | **部分自举**：前端 MusLang 写 + 后端 Rust 写（见 §3.12.3 表格） |
| 2 | **Stage 0 宿主语言** | **Rust**（成熟生态、LLVM 绑定稳定；不选 C 因其开发效率低，不选 Zig 因其未 1.0 且 async 模型漂移，见 D-2） |
| 3 | **std 自举顺序** | **先编译 std 再编译编译器**（编译器内嵌 minimal runtime 方案**不采用**——std 必须独立可测） |
| 4 | **确定性输出** | **M2 再做**（M1 不要求 Stage 1 == Stage 2 比特级一致，仅要求 Stage 1 能正确编译通过） |

**确定性输出的 M2 要求（预定义，便于 M1 不踩坑）**：
- C99 后端须生成**确定性 C 代码**：临时变量名去随机化、单态函数命名用**稳定类型哈希**（不含时间戳/路径）；
- 引入 `muslangc --reproducible` 标志，固化排序（哈希表遍历顺序、section 顺序）；
- Stage 2 验证目标：**同一份 MusLang 源码，Stage 1 与 Stage 2 产出二进制 SHA-256 一致**（允许白名单差异：debug 路径、嵌入时间戳）。

#### 3.12.5 与既有章节的衔接

- **§3.4 / FR-037**：自举属 P2（完整自举），本小节将 P2 拆为"M1 部分（前端）+ M2/M3 完整"，与 §8 阶段三 M3-1~M3-3 对齐；
- **§3.9（D-13）**：std 编译依赖 HIR 单态化完整支持，Stage 0 必须先实现 D-13（含 25 层限制、`E_MONO_*` 错误码）；
- **§3.13（D-17）**：`mktplace` 远期需支持"MusLang 编写的编译器作为可构建包"的工作区编排；
- **§10**：成功指标"自持"对应 Stage 2 通过，验收口径为 Stage 2 产出与 Stage 1 **语义等价**（M1 放宽为"能编译通过"）。

> **风险提示**：若 M1 实测发现"前端 MusLang 写"受限于语言缺陷反复返工，可退化为"M1 仅写 lexer/parser（无泛型），M2 再迁移类型检查"——但**不建议作为首选**，因会推迟语言设计验证。

---

### 3.13 包管理器（D-17，v0.4.3）

> **问题**：MusLang 是独立语言（自有编译器、自有 std、自有模块系统），但 M1 阶段不愿投入数年造一个完整包管理器（依赖解析、版本锁定、registry、构建图……）。Cargo 现成可用，但**Cargo 为 Rust 设计**——如何让 `.mus` 文件搭上 Cargo 的车？远期又该如何走向纯自主？本小节定稿 M1 与远期的包管理策略，**明确 `mktplace` 的定位及其与 hypo 的关系**。

#### 3.13.1 M1：复用 Cargo（方案 A，不造轮子）

**做法**：`muslangc` 作为 Cargo `build.rs` 驱动的命令行工具，Cargo 管依赖/版本，`build.rs` 调用 `muslangc` 编译 `.mus` → `.c`，再由 Cargo 的 `cc` crate 把 C 编成静态库。

```toml
# Cargo.toml（MusLang 应用，M1）
[package]
name = "my-mus-app"
build = "build.rs"

[build-dependencies]
muslangc = "0.1"  # MusLang 编译器作为 build dep
```

```rust
// build.rs
fn main() {
    std::process::Command::new("muslangc")
        .args(&["compile", "src/main.mus", "-o", "target/mus/main.c"])
        .status().unwrap();
    println!("cargo:rustc-link-search=target/mus");
}
```

**选用 Cargo 的六点理由**：

| 维度 | 收益 |
|---|---|
| 依赖版本 / lockfile | 免费获得 Cargo 全套（`Cargo.lock`、语义化版本、features 门控） |
| 发布 / registry | 可直接 `cargo publish` 到 crates.io（MusLang 库作为特殊 crate） |
| 增量编译 | `cargo build` 增量 + `muslangc` 可按需支持 mtime/hash 增量 |
| 信创离线 | `cargo vendor` 把整个依赖树打包到本地目录，离线构建零网络 |
| 心智模型 | Rust 开发者（核心用户群）零额外学习 |
| 实现成本 | `muslangc` 只需是命令行工具（读 `.mus` → 写 `.c`），`build.rs` 负责调用 |

**M1 已知限制（接受）**：
- `build.rs` 的增量支持有限，可能全量重编 `.mus`——**M1 全量，M2 再做 `muslangc` 增量**（与 D-16 确定性输出同期）；
- 依赖粒度：MusLang 库发布为**独立的 crates.io crate**（每个库一个 crate），std 组件统一归入 `muslang-std` 大 crate（features 控制开哪些模块），**不采用"一个大 crate 包含所有 std"**；
- 本地 registry mirror：`cargo vendor` + 私有 registry 满足信创离线，**M1 不额外开发分发服务**。

#### 3.13.2 远期：`mktplace`（谐音 marketplace）

> **名称**：`mktplace`（读作 /ˈmɑːkɪt.pleɪs/，谐音 **marketplace**），非 `muspm` 或其他。

**定位**：MusCat 生态中 **MusLang 源码级的包管理器 + 工作区编排器**——

> **借鉴 hypo 的分发安全模型**（去中心化 registry、依赖锁定、GPG 签名、SBOM、构建沙箱）+ **pets_tools 的工作区编排能力**（多仓拓扑、并行构建、`repos.toml`、WPT 基线对比、CCache 式构建缓存），但 **`mktplace` 是独立的语言包管理器**，并非把 hypo 改个名。

**职责边界**：

| 能力 | 来源 | 说明 |
|---|---|---|
| 依赖解析 / 版本锁定 | hypo | SAT 求解、`mktplace.lock`、lockfile |
| 去中心化 registry | hypo | `git+https` URL、离线 mirror、GPG 签名、SBOM |
| 供应链安全 | hypo | 依赖锁定、签名验证、构建沙箱 |
| 工作区编排（`repos.toml`） | pets_tools | 多仓拓扑、并行构建、WPT 基线 |
| 构建图 + 缓存 | pets_tools | 增量构建、CCache 式缓存 |
| 编译器驱动 | 共有 | 调用 `muslangc` + `muslink` |

#### 3.13.3 三阶段演进

```
M1（2026 Q1-Q2）         过渡期                    v1.0+
─────────────────       ──────────────            ──────────────────
   Cargo + build.rs  ──►  Cargo 为主 +          ──►  mktplace 一步到位
   muslangc 作为         mktplace 雏形试用         替代 Cargo + 吸收
   build dep                                    pets_tools 编排能力
                                                    │
                    hypo 作为独立分发工具 ◄──────────┘
                    （管产物分发，非源码包管理）
```

**迁移触发条件（量化门槛，避免"永远在等"）**——满足**全部**即触发 Cargo → `mktplace` 迁移：

1. hypo 实现完整 lockfile + 离线 mirror + GPG 签名（三项全绿）；
2. 至少 **20 个**真实 MusLang 包能用 `mktplace` 构建通过；
3. 信创离线环境（UOS LoongArch64 / 银河麒麟 ARM64）下通过完整构建测试。

#### 3.13.4 `mktplace` 与 hypo 的关系（关键澄清）

> **hypo 是系统级软件分发工具（类似系统包管理器），不是构建时依赖管理器**——`mktplace` 与 hypo **各管各的**，二者协作但不重叠：

| 工具 | 问题域 | 典型场景 |
|---|---|---|
| **`mktplace`** | **源码级**：怎么组织代码、怎么拉依赖、怎么构建 | `mktplace add net`、`mktplace build`、`mktplace workspace` |
| **hypo** | **产物级**：编好的二进制/库怎么分发部署到目标机 | `hypo install muslang-std`、`hypo deploy my-app --target uos-loongarch64` |

**示例协作流**：
```
源码(.mus) ──mktplace build──► 二进制/库 ──hypo package──► 系统级分发/部署
                                  ▲                        ▲
                              mktplace 管这                hypo 管这
```

**错误理解（须避免）**：❌ "hypo 成熟后变成 MusLang 的包管理器"——**错**。hypo 始终是独立的分发工具；`mktplace` 才是源码包管理器，只是**设计上参考了 hypo 的分发安全模型**。

#### 3.13.5 决策表

| # | 决策 | 结论 |
|---|---|---|
| 1 | M1 包管理 | **Cargo 复用**（方案 A，`build.rs` 驱动 `muslangc`） |
| 2 | 增量编译 | **M1 全量**，M2 再做（`muslangc` 基于 mtime/hash） |
| 3 | 依赖粒度 | 每个 MusLang 库独立 crates.io crate；std = `muslang-std` 大 crate + features |
| 4 | 信创离线 | **`cargo vendor` 够用**（本地 mirror 推至 M2 / `mktplace` 阶段） |
| 5 | 远期工具名 | **`mktplace`**（谐音 marketplace） |
| 6 | `mktplace` 来源 | hypo 分发安全模型 + pets_tools 编排能力，**合并为单一工具** |
| 7 | 迁移触发 | hypo lockfile+mirror+签名 + 20 个真实包 + 信创离线全绿 |

#### 3.13.6 与既有章节的衔接

- **§3.12（D-16）**：`mktplace` 需编排"MusLang 编写的编译器前端"这类包的构建，与自举 staging 协同；
- **§3.14（D-18）**：hypo 负责产物分发，与 `mktplace` 分工（见 §3.13.4）；
- **§12.3**：本节为 v0.4.3 重写版（原三方案对比保留为历史），**§12.3 现为「见 §3.13」的跳转**；
- **§6.3 CI/CD**：`mktplace` 阶段需增加"依赖锁定 + SBOM 校验"门禁。

> **命令空间约定**：`mktplace add | build | workspace | lock | vendor | publish`，与 `muslangc` / `muslink` 同前缀（`muslang-*` / `mktplace` / `hypo`），避免与系统包管理器冲突。

---

### 3.14 软件分发（D-18，v0.4.3）

> **问题**：MusLang 编译产物的**系统级分发部署**（安装到目标机、依赖系统库、版本升级）由谁负责？本小节明确：**由 hypo 负责**，且与 §3.13 的 `mktplace`（源码包管理）严格分工。本小节为 **D-18 定稿**，M1 仅需"知晓 hypo 存在"，**不需实现集成**。

#### 3.14.1 核心规则

1. **hypo 是独立的系统级软件分发工具**，定位对标系统包管理器（如 `apt`/`dnf`/`pacman` 的现代化去中心化版本），**不是**构建时包管理器、**不是** `mktplace` 的别名。

2. **分工矩阵**：

   | 环节 | 工具 | 输入 | 输出 |
   |---|---|---|---|
   | 源码组织 / 依赖解析 | `mktplace` | `.mus` 源码 + `mktplace.toml` | 构建图 |
   | 编译 / 链接 | `muslangc` + `muslink` | 构建图 | 二进制 / 库（`.o`/`.a`/ELF） |
   | **系统级分发 / 部署** | **hypo** | 二进制 / 库 + `hypo-manifest` | 目标机可运行环境 |

3. **MusLang 对 hypo 的契约**：
   - 编译产物（ELF64 / 静态库）须符合 **FHS / LSB** 标准布局（`/usr/bin`、`/usr/lib`、`/etc`），便于 hypo 打包；
   - 提供 `hypo-manifest.toml`：声明包名、版本、架构（`x86_64`/`aarch64`/`loongarch64`/`riscv64`）、依赖系统库、签名密钥；
   - **L1 core（无 libc，见 D-11）产物**：hypo 分发能力**受限**——裸机/内核镜像由 `muslink` 直接产出，hypo 仅管"镜像打包与刷写描述"，不承诺完整依赖解析。

4. **hypo 的分发安全模型（供 `mktplace` 参考，见 §3.13.2）**：
   - 去中心化 registry（`git+https` URL，可指向 **Gitee** 私有仓库）；
   - 依赖锁定（lockfile，可复现安装）；
   - **GPG / 国密 SM2 签名**（信创场景用 SM2，对标 SM3/SM4，见 FR-016）；
   - **SBOM（软件物料清单）** 生成，满足信创测评供应链审计（§10）；
   - 构建沙箱（可复现构建，与 D-16 §3.12.4 确定性输出协同）。

#### 3.14.2 与 `mktplace` 的边界示例

```yaml
# mktplace.toml（源码级，mktplace 管）
name: my-mus-app
version: 0.1.0
dependencies:
  - muslang-std = "0.4"
  - muslang-net = "0.4"   # 由 mktplace 从 registry 拉取

# hypo-manifest.toml（产物级，hypo 管）
name: my-mus-app
version: 0.1.0
arch: [x86_64, aarch64, loongarch64]
depends:
  - glibc >= 2.28          # 系统库依赖，hypo 负责
files:
  /usr/bin/my-mus-app: target/release/my-mus-app
signature: SM2:xxxx...
```

> **一句话**：`mktplace.toml` 里**只出现 MusLang 源码依赖**；`hypo-manifest.toml` 里**只出现系统级依赖与文件布局**。两者通过"编译产物"衔接，互不侵入。

#### 3.14.3 M1 / 远期范围

| 阶段 | hypo 集成 | 说明 |
|---|---|---|
| **M1（2026 Q1-Q2）** | ⚠️ **仅知晓，不集成** | 编译产物手工部署即可；`hypo-manifest.toml` 规格**预定义**（本节），但工具本身不实现 |
| **M2（2026 Q3-Q4）** | 试点集成 | MusKitty 内核镜像通过 hypo 打包分发 |
| **v1.0+** | 完整支持 | hypo + `mktplace` 双工具协同，信创离线 mirror 全绿 |

#### 3.14.4 与既有章节的衔接

- **§3.13（D-17）**：`mktplace` 管源码、hypo 管产物，**§3.13.4 为本节的具体化**；
- **§3.5 / `muslink`**：静态链接产物（`.a`/ELF）是 hypo 分发的**输入**；
- **§10 成功指标**："信创测评通过 100%"隐含 SBOM + 签名要求，由 hypo 承担；
- **§12**：WASM、调试信息等仍待定，**不影响本节**（软件分发与这些项无耦合）。

> **原则**：MusLang 自身**不内置包管理器也不内置分发器**——`mktplace`（源码）+ `hypo`（产物）构成双工具链，遵循 P6「C 是第一公民」精神：**优先复用成熟工具，仅在自主可控（信创）诉求下自建**。

---

### 3.15 `net` 事件循环：作为 `std::net` 的依赖自动带入（D-19，v0.4.4）

> **问题**：§3.6 已定 async 语法（Rust 风格、编译期状态机），§3.3.1 已定 `net::http` API 形状（`Handler::handle` 为 `async`、`server.listen().await`）。但 **`listen().await` 由谁驱动、事件循环从哪来** 一直未回答。本小节定稿 **事件循环（event loop / executor）的归属与绑定方式**。**规则为编译期 / 链接期机制，无运行时选项开销（无 vtable 分支、无 feature 检测）**。

#### 3.15.1 核心决策：事件循环是 `std::net` 的依赖，不是用户的选择

**一句话**：事件循环（epoll / kqueue 封装 + 任务调度）是 `std::net` 的**内部实现依赖**——它随 `std::net` crate **自动链接进来**，**用户不需要、也无法选择运行时**。

这与 Rust 生态（Tokio / async-std / smol 用户自选 executor + `#[tokio::main]`）**刻意不同**：MusLang 承诺 P7「网络开箱即用」（对标 Go `net/http`），因此标准库**自带一个够用的默认实现**，不把"选运行时"暴露成用户的第一道门槛。

```
用户源码                 链接图（自动，用户不可见）
─────────                ─────────────────────────────────────────
use std::net::http;       main.mus
                            │
async fn handle(...) { … }   ├── std::net::http   (FR-015，用户显式 use)
                            │       │
fn main() {                 │       ├── std::net::tcp     (FR-014)
    let s = Server::new();  │       │       │
    s.handle(handle);       │       │       └── std::async::event_loop  ← 自动带入
    s.listen();  // block   │       │               │
}                           │       │               └── libc::epoll_* / io_uring
                            │       └── std::io (FR-024)
                            └── muslang-rt-c ──► libc

未 use std::net 的程序（裸机 / 内核 / CLI）：std::net 整块被 --gc-sections 剔除，
event_loop 完全不存在于最终二进制中。
```

**四条硬性结论**：

1. **不提供独立的"运行时选择"**——没有 `--runtime=tokio`、没有 `muslang-rt-async` 独立 crate、不在 std 中预留 `Executor` trait 让用户 impl。**标准库就是唯一实现**，要改自己改源码（见 §3.15.3 替换路径）。

2. **不引入注入 API**——没有 `#[entry]` / `#[async_main]` / `Runtime::new().block_on(main())` / `muslang::runtime::block_on`。`fn main()` 就是同步入口，`async fn` 由 `std::net` 内部的 event loop 在 `listen()` / `connect()` / `accept()` 等**阻塞点**驱动。这与 Go 的 `go func()` 无需 main 注入 runtime 一致。

3. **依赖是显式的、实现是隐式的**（与 P4「显式优于隐式」不冲突）：
   - **显式**：`use std::net::http`（用户清楚地声明"我要用网络"）；
   - **隐式**：event loop 是 `net` 的**底层实现细节**，如同 `Vec` 内部用 `alloc`、``println!`` 内部调 `write` syscall——**没有人觉得 `alloc` 应该让用户手动初始化**。

4. **不用 `net` 的人完全不关心**——`#![no_std]` 内核 / 裸机程序不 `use std::net`，event loop 不被链接，`--gc-sections` 整块剔除，**L1 `<8KB` 目标（D-11）不受影响**（见 §3.15.4）。

#### 3.15.2 事件循环规格（MVP）

| 属性 | MVP 规格（v0.4.4 定稿） | 远期 |
|---|---|---|
| 事件驱动 | **单线程 epoll**（Linux）/ kqueue（macOS，`sys` 层分发，D-9） | io_uring 可选（P1） |
| 线程模型 | **一个 event loop 处理全部连接**（goroutine-per-connection 的**协程**跑在单 loop 上，非"一个连接一个 OS 线程"） | 多线程 work-stealing（FR-047，P1） |
| 任务池 | **固定大小**（默认 256 任务槽），**无动态扩容** | 可配置 `with_config(Threads(n))` |
| 锁 | 无（单线程），避免原子 / Mutex 开销 | 多线程时按 connection 分片 |
| 体积 | < 20KB（不含 TLS / HTTP 解析） | — |
| 剔除方式 | `no_std` 或不 `use std::net` → 不链接 | 同 |

**选择单线程 MVP 的理由**：
- **体积小**：不需要锁、不需要 work-stealing 任务队列、不需要线程池——符合 L1 精神；
- **内核 / 嵌入式场景够用**：单 loop + 非阻塞 I/O 即可支撑 10 万连接（瓶颈在 fd 数量与内存，不在线程数）；
- **多线程可后续追加而不破坏 API**：`Server::listen()` 签名不变，仅内部从单 loop 扩为多 loop，`Handler` 代码零改动（P1 再做，不急 M1）。

#### 3.15.3 用户代码示例（Go 级体验，零 boilerplate）

```rust
// ✅ 完整可用的 HTTP 服务——无任何运行时注入、无 block_on、无 #[entry]
use std::net::http;

struct Server { addr: String }
impl Server {
    fn new(addr: &str) -> Self { Server { addr: addr.to_string() } }
    fn handle(&self, pattern: &str, h: impl http::Handler) { /* 注册路由 */ }
    fn listen(&self) { /* 内部：起 event loop、accept、spawn 协程、block 至此 */ }
}

trait Handler {
    async fn handle(&self, req: &http::Request, res: &mut http::Response)
        -> Result<(), http::HttpError>;
}

fn main() {
    let server = Server::new("0.0.0.0:8080");
    server.handle("GET /", |req| async {
        http::Response::ok("Hello, MusLang!")
    });
    server.listen();  // ← 阻塞：event loop 在此启动并驱动所有 async Handler
}
```

**对照 Rust（Tokio）**——MusLang 省掉的正是这几行：

```rust
// Rust 要求用户写的 boilerplate（MusLang 不需要）
#[tokio::main]          // ← MusLang 无此宏
async fn main() {        // ← MusLang 的 main 是同步的
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    // ...
}
```

#### 3.15.4 与 P5「零运行时」的语义澄清（重要）

**P5 不是"二进制里没有任何运行时组件"，而是"运行时可选、不用不链、用了也透明"。** 本次（D-19）明确该承诺的口径：

| 场景 | 是否含 event loop | 是否满足 P5 | 说明 |
|---|---|---|---|
| `Hello, World`（无 `use std::net`） | ❌ 无 | ✅ | `--gc-sections` 剔除，`<8KB` 可达（D-11） |
| `#![no_std]` 内核 / 裸机 | ❌ 无 | ✅ | 用户自行 syscall，不依赖 `std::net` |
| HTTP 服务（`use std::net`） | ✅ 有 | ✅ | event loop 是 `std::net` 内部依赖，**对用户不可见、不可配置、不污染全局**——与 `printf` 内部含 `write` syscall 封装同理 |
| 自定义事件循环（嵌入已有 C loop） | 可选 | ✅ | 见 §3.15.5 替换路径 |

> **结论**：P5 的准确表述应为 **"零**强制**运行时"**——运行时组件**只在被显式 `use` 的标准库子系统（`net`、可能的 `gui` 等）引入时才存在，且对用户透明。**这与 Zig 的"标准库可拆分、不用就不付代价"精神一致**，也与 D-9「按需链接」完全自洽。

#### 3.15.5 替换路径（高级 / 嵌入式场景）

标准库的实现是唯一默认，**但源码可改**——这是"不提供选择"与"完全锁死"之间的关键分野：

| 需求 | 做法 | 说明 |
|---|---|---|
| **嵌入到已有的 C 事件循环**（libuv、libevent、自研 epoll） | **不使用 `std::net`**，自行基于 `std::sys`（`std::os` syscall 封装）+ `std::async::Future` 实现网络层 | 用户代码完全控制 accept / poll / spawn；`FfiFuture`（FR-044，`repr(C)`）用于在 C loop 中驱动 MusLang future，与 D-8 双 runtime 衔接 |
| **内核 / 裸机网络栈** | `#![no_std]` + 自有网络实现（MusKitty Layer 5，见 §7） | 不走 `std::net`，event loop 概念由内核调度器承担 |
| **想要多线程 / io_uring / 自定义调度策略** | **Fork std 的 `net` crate 修改**，或等 P1（FR-047 work-stealing） | MVP 单线程是工程取舍，非语言层面禁止 |
| **完全同步阻塞 I/O**（某些嵌入式场景） | `std::sys::read` / `std::sys::write` 直接 syscall，不用 `async` | 不涉及 event loop |

> **原则**：MusLang **不预留抽象层供用户注入 executor**（不在 `std::async` 暴露 `Executor` trait），但**保留"不使用 std、自己写"这条退路**——这与 P8「渐进式复杂度」一致：5 行 Hello World 不感知任何运行时，写 HTTP 服务也只 `use std::net`，只有"把 MusLang 嵌进已有事件循环"这种高级场景才需要绕过 std。

#### 3.15.6 与既有决策的衔接

| 已有决策 | 本小节（D-19）的衔接 |
|---|---|
| **D-8 双 runtime**（§3.7.3） | event loop **属于 `muslang-rt`**（MusLang 原生布局：胖指针、`Waker`、类型化任务）；rt-c 侧由 C 的 event loop（libuv 等）通过 `FfiFuture`（FR-044）驱动 MusLang future，**两侧 executor 不共享任务队列**（避免"双堆税"，见 D-8 L2 hosted） |
| **D-2 默认 C99 后端**（FR-032） | event loop **用 C 实现**（`epoll` 直接 syscall、`fd_set` 操作），通过 `std::sys` 调用；MusLang 侧只定义 `Future` / `Waker` trait 与状态机 IR。C99 后端**不需要生成 async 运行时代码**，只需生成正确的状态机 + 调用 C 的 event loop API——**这是 C99 后端能落地 async 的关键** |
| **FR-044 FfiFuture** | rt / rt-c 边界处的 future 转换点；C 侧 event loop 驱动 MusLang future 的统一 ABI |
| **D-13 泛型单态化**（§3.9） | **event loop 内部统一使用 `Box<dyn Future>`（trait object），不做泛型单态化**——理由：① executor 是运行时组件，一次虚调用的开销相对 I/O 等待可忽略；② 避免 `Spawn<F>` 泛型爆炸（编译器前端、任务调度器本身是泛型重灾区，单态化会威胁 `<8KB` 与编译速度）。**这是 D-13「不引入类型擦除」规则的唯一明确例外**，因它属于运行时而非语言抽象层 |
| **§3.3.1 `net::http` API** | API 形状（Handler / ServeMux / Client）**完全不变**——D-19 只规定"谁来驱动这些 async 函数"，不改用户可见接口 |
| **§3.2.3.1 `defer` + async 取消**（D-12） | 取消仅在 `.await` 点、协作式；event loop 在连接关闭 / 超时（`Context` 取消）时 **drop 未完成的 future**，触发 `defer` / `errdefer` 清理（规则三），资源不变量由 D-12 保证 |
| **D-16 自举**（§3.12） | 编译器前端（MusLang 写）**不依赖 `std::net`**（编译期无需网络），自举链不受 event loop 影响；Stage 1 编译自身不要求 event loop 可用 |
| **D-9 按需链接** | event loop 与 `std::net` 同 section 归属，**不用即剔除**，是 D-9「用哪个链哪个」的直接实例 |

#### 3.15.7 决策表（D-19 定稿）

| # | 决策点 | 结论 |
|---|---|---|
| 1 | 事件循环的归属 | **`std::net` 的内部依赖**，随 `use std::net` 自动链接，**非独立 crate** |
| 2 | 用户是否可选运行时 | **不可**（对标 Go，不提供 Tokio 式多选） |
| 3 | 注入 API（`#[entry]` / `block_on`） | **不引入**；`fn main()` 为同步入口 |
| 4 | MVP 线程模型 | **单线程 epoll**，一个 loop 处理全部连接 |
| 5 | 多线程 / work-stealing | **P1**（FR-047），`Server::listen` 签名不变、内部扩展 |
| 6 | io_uring | **P1**，MVP 用 epoll |
| 7 | 任务池 | 固定 256 槽，**无动态扩容**（MVP） |
| 8 | Executor 内部用 trait object 还是单态化 | **`Box<dyn Future>`（trait object）**——D-13 的唯一例外，避免泛型爆炸 |
| 9 | 不用 `std::net` 时 | event loop **完全不链接**，`<8KB` 可达（D-11） |
| 10 | 嵌入 C 事件循环 / 内核场景 | **不使用 `std::net`**，自行基于 `std::sys` + `FfiFuture` 实现 |
| 11 | P5「零运行时」口径 | **零强制运行时**：可选、不用不链、用了也透明（§3.15.4） |

#### 3.15.8 待办（实现期跟进，非语言决策）

- [ ] **体积基准**：实测 `Hello, World`（无 `net`）vs `Hello, World` + `use std::net` + `listen` 的二进制增量，确认 event loop < 20KB（D-11 验收口径）；
- [ ] **任务池上限配置**：256 为拟定默认值，实现期若实测不够需通过 `Server::with_config` 暴露，**但 MVP 不允许动态扩容**（避免隐式分配，P1）；
- [ ] **取消传播测试**：连接被 event loop 关闭时，`defer` / `errdefer` 执行顺序须符合 §3.2.3.1 规则三（纳入 D-12 `unsafe_examples/`）；
- [ ] **io_uring 评估门槛**：P1 立项条件 = epoll 在 LoongArch64 / ARM64 上实测达不到目标吞吐（10 万连接 < 500MB，§4.1）。

---



| 指标 | 目标 | 参考 |
|---|---|---|
| 编译速度 | 增量 < 1s，全量 < 10s（中等项目） | Zig 同级 |
| 链接速度 | < 500ms（中等项目，muslink） | mold/lld 同级 |
| 二进制体积 | **L1 core**（不含 std、不含编译器）裸启动 **< 8KB**；测量协议：`x86_64-linux-musl`、`-Oz -flto`、静态链接、无动态加载（详见 §3.7 D-11） | Zig ~4KB |
| 运行时开销 | 零（无 GC、无运行时） | Zig 同级 |
| 内存安全 | 编译期保证，零 `unsafe` 块 | Rust 同级 |
| 并发吞吐 | 10 万连接 < 500MB | 优于 Go（无 GC 压力）|

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
- C++ 互操作：**1.0 边界**（详见 §3.11，D-15）：vtable = Itanium C++ ABI、异常禁止跨越 FFI 边界、RTTI 不支持、模板须 C++ 侧预实例化；复杂类型用不透明指针 + 包装函数
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
| M3-1 | MusLang 编译器 v0.2（MusLang 写，**前端**；后端仍 Rust，见 §3.12 D-16）| 能编译自身 AST + 类型检查，通过 Stage 1 构建 |
| M3-2 | MusLang 编译器 v0.3（**完整功能，含 C99 后端 MusLang 化**）| 全量功能等价 Rust 版本，Stage 1 == Stage 2 语义等价（确定性输出，见 §3.12.4）|
| M3-3 | **自持闭环**（v0.3 编译 v0.3，Stage 2）| 用 MusLang 版编译器重新编译自身，产出与 Stage 1 **SHA-256 一致**（可重现构建）|
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

**决策节点**：**M3-3 自持闭环达成后**（自举编译器稳定、确定性输出可用，见 §3.12 D-16 / §8 M3-3），重新评估 `comptime`。此节点与 §3.12.4「确定性输出推迟至 M2」协同——comptime 解释器本身也需可重现执行。

### 12.2 WASM 后端

| 维度 | 支持 | 不支持 |
|---|---|---|
| MusCat 浏览器集成 | ✅ JS 引擎可直接调用 | ❌ 需额外 FFI 层 |
| 编译器复杂度 | 高（+1 后端） | 低 |
| 生态复用 | ✅ 可编译为 WASM 运行在浏览器 | — |
| 优先级依据 | MusCat 浏览器核心需求 | 非 P0 用户场景 |

**推荐方向**：P1（阶段二启动）。MusCat 浏览器需要 WASM 支持来运行 Web 内容，MusLang 编译为 WASM 可在浏览器中运行 MusLang 代码。

**决策节点**：M2-3（MusKitty Layer 6 JS 引擎适配）启动时评估。

### 12.3 包管理器与软件分发（v0.4.3 已定，见 §3.13、§3.14）

> **v0.4.3 状态**：本节原「三方案对比」已于 v0.4.3 **定稿为 D-17（包管理器）+ D-18（软件分发）**，详见 §3.13（`mktplace`）、§3.14（hypo）。以下仅保留历史方案对比供追溯，**新设计以 §3.13 / §3.14 为准**。

**历史方案对比（阶段一评审用，已归档）**：

| 维度 | 方案 A：复用 Cargo | 方案 B：自研（类 Zig build） | 方案 C：Git 依赖（类 Zig） |
|---|---|---|---|
| 生态复用 | ✅ 100% 兼容 crates.io | ❌ 从零建设 | ❌ 无中央注册表 |
| 开发成本 | 低（复用 Cargo） | 高 | 低 |
| 自主可控 | ❌ 依赖 Cargo | ✅ 完全自主 | ✅ 去中心化 |
| 信创适配 | ⚠️ Cargo 依赖网络 | ✅ 完全可控 | ✅ 离线可用 |
| 版本管理 | ✅ 成熟 | ❌ 需自建 | ⚠️ 手动管理 |

**定稿结论（D-17 / D-18）**：
- **M1 = Cargo 复用**（原方案 A），`build.rs` 驱动 `muslangc`；
- **远期 = `mktplace`**（谐音 marketplace），参考 hypo 分发安全模型 + pets_tools 编排能力，**与 hypo 各司其职**；
- **hypo = 独立的系统级软件分发工具**，非构建时包管理器（D-18）。

**迁移触发条件**：hypo lockfile+mirror+签名全绿 + 20 个真实包 + 信创离线全绿（详见 §3.13.3）。

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

### 12.7 `net` 运行时绑定（v0.4.4 已定，见 §3.15）

> **v0.4.4 状态**：原「async 运行时绑定」待定项已于 v0.4.4 **定稿为 D-19**，详见 §3.15（`std::net` 事件循环作为依赖自动带入、单线程 epoll MVP、无注入 API、P5 语义澄清）。本节仅保留历史讨论供追溯，**新设计以 §3.15 为准**。

**历史方案对比（阶段一评审用，已归档）**：

| 方案 | 做法 | 结论 |
|---|---|---|
| **A. 运行时内置（Go 模型）** | `muslang-rt` 内含默认 executor，`#[entry]` 自动启动 | ❌ **不采用**——引入注入宏，违背"零 boilerplate"的初衷 |
| **B. 运行时可选注入（Rust 模型）** | std 只定义 `Future` trait，用户自选 executor（`muslang-tokio` 等） | ❌ **不采用**——把"选运行时"暴露成用户第一道门槛，违背 P7「网络开箱即用」 |
| **C. 默认内置 + 可选剔除** | 默认带精简 executor，高级用户 `no_rt` 剔除 | ⚠️ **部分采纳**——采纳"默认内置"，但**剔除方式不是 feature flag，而是"不用 `std::net` 即不链接"**（D-9 按需链接的天然结果），且**不提供 `no_rt` feature** |
| **✅ D. 作为 `net` 依赖自动带入（D-19 定稿）** | 事件循环 = `std::net` 内部实现细节，用户不可见、不可选、不污染全局 | ✅ **采用**——最符合 P7 + P4 + D-9「按需链接」，详见 §3.15 |

**定稿结论（D-19）**：
- 事件循环随 `use std::net` 自动链接，**不提供运行时选择**、**不引入 `#[entry]` / `block_on`**；
- MVP = **单线程 epoll**，多线程 / work-stealing / io_uring = P1（FR-047）；
- Executor 内部用 `Box<dyn Future>`（D-13 唯一例外）；
- P5「零运行时」澄清为 **「零强制运行时」**（§3.15.4）；
- 高级替换路径：**不使用 `std::net`**，自行基于 `std::sys` + `FfiFuture` 实现。

### 12.8 分配器模型（仍待定）

> **状态**：v0.4.4 未决策，保持待定。已知锚点：`alloc` 包 = `GeneralPurposeAllocator`（FR-021，P0）、分配器显式传递（FR-008，Zig 风格，P4）、`MusAllocator::from_c` 桥接 deallocator 配对（§3.8.4，D-7）。核心待定项：**零成本抽象边界**（allocator 参数是否单态化、与 D-13 25 层限制的交互）、**L1 core 无 libc 时的默认 allocator**（bump / pool / 静态分区）、**`no_std` 场景的 allocator 注入方式**。

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

| Stage 0 | 用宿主语言（Rust）编写的引导编译器，须实现完整 MusLang 语言 |
| Stage 1 | 用目标语言（MusLang）编写的编译器（M1 = 前端），由 Stage 0 编译 |
| Stage 2 | 用 Stage 1 二进制重新编译同一份源码，验证自举一致性 |
| `mktplace` | MusLang 源码级包管理器 + 工作区编排器，名称谐音 marketplace |
| hypo | 独立的系统级软件分发工具，与 `mktplace` 各司其职 |
| 可重现构建（Reproducible Build） | 同一源码多次构建产出比特一致的二进制，含确定性输出规范 |
| event loop（事件循环） | `std::net` 的内部组件，负责 epoll/kqueue 等待 + 就绪 fd 分发；**随 `use std::net` 自动链接，对用户不可见**（D-19） |
| executor | 任务调度器，驱动 `Future` 状态机的 `poll`；MusLang 中即 event loop 的一部分，**不暴露为独立 trait** |
| 单线程 epoll（MVP） | D-19 的 MVP 线程模型：一个 event loop 处理全部连接，无锁、无 work-stealing |
| work-stealing | P1 调度策略（FR-047）：多线程下任务跨 worker 窃取；MVP 不实现 |
| `Box<dyn Future>` | executor 内部的任务存储方式（D-19）；D-13「不引入类型擦除」规则的**唯一例外**，因 executor 属运行时 |
| P5「零强制运行时」 | D-19 澄清后的 P5 准确表述：运行时可选、不用不链、用了也透明（非"二进制里完全没有运行时组件"） |

### 15.3 变更记录

| 版本 | 日期 | 变更内容 | 作者 |
|---|---|---|---|
| v0.1 | 2026-08-30 | 初稿 | 墨染柒 |
| v0.2 | 2026-08-30 | 补充链接器章节（§3.5）、muslink 设计、更新时间线 | 墨染柒 |
| v0.3 | 2026-09-01 | **全面完善**：① 新增设计原则（§1.4）；② 新增类型系统章节（§3.2）；③ 新增错误处理规范（§5）；④ 新增测试策略（§6）；⑤ 补充编译器架构图（§3.4.1）；⑥ 补充 `@cImport` 技术规范（§3.4.2）；⑦ 补充 muslink 技术规范（§3.5.1）；⑧ 补充异步模型与 Zig 新 Io 模型关系（§3.6.1）；⑨ 完善安全模型（§4.3）；⑩ 完善风险表（§11）；⑪ 开放问题补充分析框架（§12）；⑫ 新增文档体系（§13）；⑬ 新增治理与版本管理（§14）；⑭ 补充术语表（§15.2） | Yuanbao (AI) |
| v0.4 | 2026-09-04 | **架构决策落地**：① 定位改为「类 Rust，不保证源码兼容」（§1.1，D-1）；② 新增架构决策记录（§3.7）含互操作架构、共享方言+HIR、D-0~D-11 决策表、双 runtime（muslang-rt / muslang-rt-c）、std 三层按需链接+sys 三端可扩展；③ 补充 `unsafe` 等价判定与 CI 门禁（§3.2.1 K3，D-4）；④ 默认后端由 Zig 改为 C99（FR-032、§3.4.1、§5.3，D-2）；⑤ 异步模型改为后端无关实现（§3.6.1）；⑥ 二进制体积口径定稿为 L1 core 裸启动 `<8KB`（§4.1、§10）；⑦ 新增 M1-0 决策冻结阶段与 v0.4 裁剪建议（§8） | Yuanbao (AI) |
| v0.4.1 | 2026-09-04 | **语义定稿**：① `defer` 与 `?`、async 取消交互规则（§3.2.3.1，D-12）：`defer`/`errdefer` 分离（错误路径仅 `errdefer`、LIFO）、`?` 提前返回只跑 `errdefer`、`async fn` 取消仅在 `.await` 点（协作式）、`defer` 禁止 `.await`（异步清理须显式 `close().await`）、5 类错误码，并附 Zig 官方文档 + Rust Internals + Tokio 引用（§15.1）；② 跨 `.so` 所有权移交协议 A+C 混合、函数级策略一致性（§3.8，D-7 定稿）：A 严格移交 / C 标注协议（`#[muslang::strategy]`、`#[muslang::borrow/take/ret]`）/ B `Arc` 为 P1 可选、同函数禁止 A/C 混用、混合生命周期须拆函数、回调走 C、`MusAllocator::from_c` deallocator 配对、清理归属与 §3.2.3.1 衔接、6 项 HIR FFI 校验错误码、Rust FFI 对应关系表 | Yuanbao (AI) |
| v0.4.2 | 2026-09-04 | **语言机制细化**：① 泛型单态化策略（§3.9，D-13）：**HIR 层单态化**（后端只见到单态后具体函数）、**递归单态化深度硬限 25 层、超限 = 语法错误**、不引入 `dyn` 类型擦除、配合按需链接 + `--gc-sections` 三层控制膨胀、3 类错误码（`E_MONO_*`）；② 生命周期标注策略（§3.10，D-14）：方案 B——函数签名默认全省略 / 推断失败给建议报错（`E_LIFETIME_AMBIGUOUS` 附建议）、**结构体·枚举不可省**、`'static` 保留、**HRTB 推迟 1.0**、`extern` FFI 边界强制显式标注；③ C++ 互操作边界定稿（§3.11，D-15）：**vtable = Itanium C++ ABI**、**异常禁止跨越 FFI 边界**（编译期拒绝 throwing 签名）、**RTTI 不支持**、**模板须 C++ 侧预实例化**（MusLang 只见到具体类型）、1.0 支持边界一览表、与 D-3/D-7/D-4 衔接 | Yuanbao (AI) |

| v0.4.3 | 2026-09-05 | **工程路径定稿**：① 自举路径与 staging 策略（§3.12，D-16）：M1 部分自举（前端 MusLang 写 + 后端 Rust 写）、Stage 0 宿主语言 = Rust、先编译 std 再编译编译器、确定性输出（可重现构建）推迟至 M2、完整自举含 C99 后端推迟至 M2/M3、与 D-13 耦合说明；② 包管理器（§3.13，D-17）：M1 = Cargo 复用（`build.rs` 驱动 muslangc）+ 三阶段演进（Cargo → hypo 成熟后 → `mktplace`），`mktplace` 谐音 marketplace、参考 hypo 分发安全模型 + pets_tools 编排能力、与 hypo 各司其职；③ 软件分发（§3.14，D-18）：hypo 为独立的系统级软件分发工具，`mktplace` 管源码/依赖/构建、hypo 管产物分发/部署，分工矩阵 + `mktplace.toml` vs `hypo-manifest.toml` + M1 仅知晓不集成；④ §3.7.1 追加 D-16/D-17/D-18；⑤ §12.3 重写为跳转 + 历史方案归档；⑥ §8 M3-1~M3-3 与 §3.12 Stage 0/1/2 对齐 | Yuanbao (AI) |
| v0.4.4 | 2026-09-05 | **运行时绑定定稿**：① `net` 事件循环（event loop / executor）**作为 `std::net` 的内部依赖自动带入**（§3.15，D-19）：用户 `use std::net` 即链接、**不提供独立的"运行时选择"**、**不引入 `#[entry]` / `block_on` 注入 API**（对标 Go，P7「网络开箱即用」）；② **MVP = 单线程 epoll**（一个 loop 处理全部连接），多线程 / work-stealing = P1（FR-047）、io_uring = P1；③ executor 内部用 **`Box<dyn Future>`**（D-13 泛型单态化的**唯一例外**，避免 `Spawn<F>` 泛型爆炸）；④ **不用 `std::net` 时 event loop 完全不链接**，`<8KB` 仍可达（D-11）；⑤ 高级替换路径：不使用 `std::net`、自行基于 `std::sys` + `FfiFuture`（FR-044）实现（嵌入 C loop / 内核场景）；⑥ **澄清 P5「零运行时」=「零强制运行时」**（可选、不用不链、用了也透明，§3.15.4）；⑦ §12.7 归档 A/B/C/D 四方案对比（最终采纳 D）；⑧ §15.2 术语表新增 event loop / executor / work-stealing / 单线程 epoll；⑨ §1.1、§3.7.1（D-19）同步 | Yuanbao (AI) |

---

> **MusLang — Rust 的壳，Zig 的灵魂，类似 Go 的网络，MusCat 的心脏。**
