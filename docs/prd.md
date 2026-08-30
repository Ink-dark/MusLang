# MusLang — 产品需求文档（PRD）

> **项目代号**：MusLang-Qomolongma
> **仓库**：https://gitee.com/moranqidarkseven/MusLang
> **所属生态**：MusCat 浏览器的原生系统编程语言
> **文档版本**：v0.2
> **创建日期**：2026-08-30
> **更新日期**：2026-08-30
> **作者**：墨染柒（Ink-dark）
> **状态**：更新中

---

## 1. 产品概述

### 1.1 一句话定位

**MusLang 是一门语法与 Rust 100% 一致、安全模型以 Zig 的类型区分取代 `unsafe` 块、内置 Go 级网络标准库、编译产物与 Zig 同级轻量的系统编程语言。** 它是 MusCat 生态全栈自研的最后一环——从语言到编译器到链接器到内核到浏览器，全栈自主可控。

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

### 1.3 目标用户

- MusCat 生态开发者（MusKitty 内核、dll 适配层、MCP Server、Cookie 协议）
- 信创场景下的系统级开发者
- AI Agent 工具开发者（通过 MCP Server 接口）

---

## 2. 用户场景

| 场景 | 角色 | 核心需求 | MusLang 解法 |
|---|---|---|---|
| MusKitty 内核开发 | 墨染柒 + Orcha | 内存安全、C/C++ 互操作、零 GC、极小二进制 | Rust 语法 + 所有权 + `@cImport` + Zig 后端 |
| MCP Server 实现 | Agent 工具开发者 | 高并发 HTTP、登录态审批接口 | 内置 `net/http` + async/await + 零 GC |
| 信创环境构建 | 国产 OS 适配工程师 | 全栈自主、无外部语言依赖、LoongArch64/ARM64 | 自举编译器 + 自研链接器 + 零运行时 |
| 第三方内核 dll 适配 | Chromium/Gecko 适配开发者 | C ABI 兼容、类型映射、内存安全 | `@cImport` + 所有权 + `extern "C"` 导出 |

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

### 3.2 标准库（P0/P1）

| 编号 | 功能 | 优先级 |
|---|---|---|
| FR-011 | `net` 包：TCP/UDP/HTTP/TLS，API 对标 Go `net/http` | P0 |
| FR-012 | `net::http`：HTTP/1.1 + HTTP/2 客户端与服务器，async | P0 |
| FR-013 | `net::tls`：TLS 支持，信创集成国密 SM2/SM3/SM4 | P1 |
| FR-014 | `fs` 包：文件操作，对标 Go `os`/`io` | P0 |
| FR-015 | `strings` 包：字符串处理 | P0 |
| FR-016 | `collections` 包：Vec、HashMap、BTreeMap | P0 |
| FR-017 | `channel` 包：mpsc/broadcast，类型安全 | P1 |
| FR-018 | `alloc` 包：分配器接口（GeneralPurposeAllocator） | P0 |
| FR-019 | `c` 包：C ABI 类型定义（c_int, c_void 等） | P0 |
| FR-020 | `async` 包：Future trait、Waker、FfiFuture | P0 |

### 3.3 编译器（P0/P1/P2）

| 编号 | 功能 | 优先级 |
|---|---|---|
| FR-021 | 前端：词法/语法分析、AST 构建 | P0 |
| FR-022 | 所有权检查器：编译期所有权/借用/生命周期检查 | P0 |
| FR-023 | 类型检查：类型推断、泛型单态化、trait 解析 | P0 |
| FR-024 | `@cImport` 实现：编译期 C/C++ 头解析与类型映射 | P0 |
| FR-025 | **Zig 后端**：生成 Zig 源码，保证极小二进制（默认后端） | P0 |
| FR-026 | Rust 后端：生成 Rust 源码，复用 Rust 生态 | P1 |
| FR-027 | C ABI 中间层：生成 C 源码 + 头文件 | P0 |
| FR-028 | 增量编译：秒级，对标 Zig | P1 |
| FR-029 | 交叉编译：LoongArch64/ARM64/RISC-V/x86_64 | P1 |
| FR-030 | **自举**：编译器用 MusLang 自身编写，自持闭环 | P2 |

### 3.4 链接器（P0/P1/P2） 🆕

| 编号 | 功能 | 优先级 |
|---|---|---|
| FR-031 | **链接器集成**：默认调用 Zig 的 LLD 分支，支持 `--gc-sections` 死代码消除 | P0 |
| FR-032 | **Freestanding 链接**：不依赖 libc，支持自定义入口 `_start`，生成纯裸二进制 | P0 |
| FR-033 | **链接脚本**：支持 `-T script.ld`，自定义段布局（`.text`/`.data`/`.bss`） | P1 |
| FR-034 | **UEFI 输出**：生成 PE 格式 + UEFI 子系统，支持 Secure Boot | P1 |
| FR-035 | **`muslink` 自研链接器**：用 MusLang 写的极简 ELF64 链接器，作为信创纯自主后端 | P2 |

### 3.5 异步模型（P0）

| 编号 | 功能 | 描述 |
|---|---|---|
| FR-036 | async/await | 语法与 Rust 一致，编译期展开为状态机 |
| FR-037 | FfiFuture | FFI-safe Future，`repr(C)`，跨语言异步统一 |
| FR-038 | 轻量级任务 | 栈池复用，无 GC，无栈协程 |
| FR-039 | 事件驱动 | epoll/kqueue/io_uring 后端 |
| FR-040 | 调度器 | work-stealing，可选，用户显式启用 |

---

## 4. 非功能需求

### 4.1 性能指标

| 指标 | 目标 | 参考 |
|---|---|---|
| 编译速度 | 增量 < 1s，全量 < 10s（中等项目） | Zig 同级 |
| 链接速度 | < 500ms（中等项目，muslink） | mold/lld 同级 |
| 二进制体积 | hello world < 8KB（静态链接） | Zig ~4KB |
| 运行时开销 | 零（无 GC、无运行时） | Zig 同级 |
| 内存安全 | 编译期保证，零 `unsafe` 块 | Rust 同级 |
| 并发吞吐 | 10 万连接 < 500MB | 优于 Go（无 GC 压力）|

### 4.2 兼容性

- C ABI：100% 兼容 System V AMD64 / AAPCS64
- C++ 互操作：支持 C 风格接口，复杂类型用不透明指针 + 包装函数
- 交叉编译：x86_64、ARM64、LoongArch64、RISC-V
- 操作系统：Linux（统信 UOS/银河麒麟）、macOS、Windows

### 4.3 安全

- 内存安全：所有权强制，无 `unsafe` 块，零 GC
- FFI 安全：`extern` 声明即标志，类型系统区分安全/不安全
- 供应链安全：自举后零外部语言依赖
- 审计：所有 FFI 调用可审计

### 4.4 可用性

- 学习曲线：Rust 开发者零学习成本，Zig 开发者 1 天上手
- 错误信息：友好、精准、带修复建议（对标 Rust）
- 文档：语言参考、标准库文档、教程、示例
- 工具链：格式化、LSP、DWARF 调试信息

---

## 5. 与 MusCat 生态的集成

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

## 6. 开发阶段与时间线

### 阶段一：Bootstrap（2026 Q1-Q2）

| 里程碑 | 交付物 |
|---|---|
| M1-1 | MusLang 语法定义（Rust 风格）|
| M1-2 | 所有权检查器 v0.1 |
| M1-3 | `*allowzero` 类型系统 |
| M1-4 | `@cImport` C/C++ 头解析 |
| M1-5 | Zig 后端代码生成 v0.1 |
| M1-6 | `net` 标准库第一版（TCP/HTTP 客户端）|
| M1-7 | MusLang 编译器 v0.1（Rust 写）|
| M1-8 | 链接器集成：调用 Zig LLD，支持 `--gc-sections` |

### 阶段二：MusKitty 内核开发（2026 Q3-Q4）

| 里程碑 | 交付物 |
|---|---|
| M2-1 | MusKitty Layer 1-4（基础）|
| M2-2 | MusKitty Layer 5（网络接驳）|
| M2-3 | MusKitty Layer 6（JS 引擎适配）|
| M2-4 | 统信 UOS / 银河麒麟适配 |
| M2-5 | 国密 SM2/SM3/SM4 集成 |
| M2-6 | Freestanding 链接 + 自定义链接脚本 |

### 阶段三：自举与信创（2027 Q1-Q4）

| 里程碑 | 交付物 |
|---|---|
| M3-1 | MusLang 编译器 v0.2（MusLang 写，前端）|
| M3-2 | MusLang 编译器 v0.3（完整功能）|
| M3-3 | **自持闭环**（v0.3 编译 v0.3）|
| M3-4 | 信创测评送测 |
| M3-5 | 100% 信创替代达标 |
| M3-6 | `muslink` v0.1（MusLang 写的极简 ELF64 链接器）🆕 |
| M3-7 | 全工具链纯自主验证（muslangc + muslink，零外部依赖）🆕 |

---

## 7. 语法示例

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

---

## 8. 成功指标

| 指标 | 目标值 |
|---|---|
| 信创测评通过 | 100% |
| MusKitty 内核内存安全 | 零 CVE（内存安全类）|
| 二进制体积 | hello world < 8KB |
| 编译速度 | 增量 < 1s |
| 链接速度 | < 500ms（中等项目）|
| MCP Server 并发 | 10 万连接稳定 |
| 自持 | 编译器自举成功 |
| 链接器自主 | muslink 可用，信创纯自主模式可选 🆕 |

---

## 9. 风险与缓解

| 风险 | 缓解措施 |
|---|---|
| Zig 后端不稳定（Zig 未 1.0） | 锁定 Zig 版本，维护兼容层 |
| 所有权检查器复杂度超预期 | 简化设计，先支持核心特性 |
| 信创 Deadline 紧迫 | 聚焦 MusKitty，MusLang 先 Rust 后端 |
| 自举编译器调试困难 | 差分测试，Rust 版对照 |
| C++ 复杂类型映射不全 | 不透明指针 + 包装函数 |
| LLD 依赖断供风险 | Fork Zig LLD 到 Gitee；muslink 作为纯自主后备 🆕 |
| muslink 开发延期 | 阶段一/二先用 Zig LLD，muslink 作为阶段三目标 🆕 |

---

## 10. 开放问题

| 问题 | 状态 |
|---|---|
| 是否需要 `comptime` 级元编程？ | 待定，可能用泛型 + 宏替代 |
| 是否需要 WASM 后端？ | 待定 |
| 包管理器设计？ | 待定，可能复用 Cargo 或自研 |
| 调试信息格式？ | DWARF（默认）|
| muslink 是否支持动态链接？ | 否，默认静态链接，动态链接不在 v1.0 范围 🆕 |
| muslink 支持哪些重定位类型？ | ELF64 基础集（R_X86_64_64, R_AARCH64_CALL26 等）🆕 |

---

## 11. 附录：参考资源

- https://ziglang.org/documentation/master/
- https://doc.rust-lang.org/reference/
- https://pkg.go.dev/net/http
- https://github.com/asynchronics/async-ffi
- https://github.com/mozilla/uniffi-rs
- https://lld.llvm.org/
- https://ziglang.org/news/
- https://refspecs.linuxfoundation.org/elf/elf.pdf
- https://github.com/Ink-dark/MusKitty

---

> **MusLang — Rust 的壳，Zig 的灵魂，类似 Go 的网络，MusCat 的心脏。**