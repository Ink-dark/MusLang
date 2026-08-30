# MusLang — 产品需求文档（PRD）

---

## 1. 产品概述

### 1.1 产品定位

MusLang 是 MusCat 生态的原生系统编程语言。语法与 Rust 100% 一致，安全模型以 Zig 的类型区分取代 `unsafe` 块，内置 Go 级网络标准库，编译产物与 Zig 同级轻量（零运行时、零 GC、极小二进制）。MusLang 是 MusCat 全栈自研的最后一环——从语言到编译器到内核到浏览器，全栈自主可控。

### 1.2 核心价值主张

| 维度 | 传统方案 | MusLang |
|------|---------|---------|
| 语法 | Rust（复杂）/ Go（简单）/ Zig（类C） | **Rust 语法，零学习成本** |
| 内存安全 | Rust（`unsafe` 块）/ Zig（手动）/ Go（GC） | **类型即标志，无 `unsafe` 块** |
| C/C++ 互操作 | Rust（`bindgen`）/ Go（cgo）/ Zig（`@cImport`） | **`@cImport` 同级，直接吃头** |
| 网络 | Rust（tokio）/ Go（net/http）/ Zig（无） | **内置 `net`，Go 级体验** |
| 运行时 | Go（GC）/ Rust（panic handler） | **零运行时** |
| 二进制体积 | Rust（中等）/ Go（中等）/ Zig（~4KB） | **~4KB hello world** |
| 编译速度 | Rust（慢）/ Go（快）/ Zig（极快） | **极快（对标 Zig）** |
| 自持 | Rust（自举）/ Go（自举） | **自举，全栈自主** |

### 1.3 目标用户

- MusCat 生态开发者（内核、dll、MCP Server、Cookie 协议）
- 信创场景下的系统级开发者
- AI Agent 工具开发者（通过 MCP Server 接口）

---

## 2. 用户场景与需求

### 2.1 场景一：MusKitty 内核开发

**角色**：MusCat 核心开发者
**需求**：用一门语言编写浏览器内核，要求内存安全、C/C++ 互操作、零 GC、产物极小。
**痛点**：Rust 编译慢、二进制大、`unsafe` 块难以审计；Zig 无内存安全保证；C++ 过于复杂。
**MusLang 解法**：Rust 语法写内核代码，所有权强制保证安全，`@cImport` 直接吃 Chromium/V8 头，编译到 Zig 后端生成极小二进制。

### 2.2 场景二：MCP Server 实现

**角色**：AI Agent 工具开发者
**需求**：实现 MCP Server 工具，处理高并发 HTTP 请求，需要登录态审批接口。
**痛点**：Go 的 GC 在高并发下 STW 停顿；Rust 的 async 生态复杂；C++ 开发效率低。
**MusLang 解法**：内置 `net/http`，async/await 语法，高并发稳定，零 GC 停顿。

### 2.3 场景三：信创环境构建

**角色**：信创测评机构 / 国产 OS 适配工程师
**需求**：全栈自主可控，无外部语言依赖，能在 LoongArch64/ARM64 上构建。
**痛点**：Rust 工具链在国产 CPU 上支持不完整；Go 运行时依赖；C++ 标准库依赖。
**MusLang 解法**：自举编译器，编译产物零依赖，交叉编译到国产架构。

### 2.4 场景四：第三方内核 dll 适配

**角色**：Chromium/Gecko 后端适配开发者
**需求**：编写适配层 dll，桥接 MusCat 主程序与外部内核。
**痛点**：C ABI 兼容、类型映射复杂、手动管理内存易出错。
**MusLang 解法**：`@cImport` 直接吃 C++ 头，类型自动映射，所有权管理内存，导出 `extern "C"` 符号。

---

## 3. 功能需求

### 3.1 语法层（FR-001 ~ FR-010）

| 编号 | 功能 | 描述 | 优先级 |
|------|------|------|--------|
| FR-001 | Rust 风格语法 | 结构体、枚举、trait、泛型、模式匹配、async/await，与 Rust 语法 100% 一致 | P0 |
| FR-002 | 无 `unsafe` 块 | 移除 Rust 的 `unsafe {}` 块语法，不安全通过类型标注表达 | P0 |
| FR-003 | `*allowzero T` 类型 | 可空裸指针类型，是"不安全"的类型级标志，替代 `unsafe` 块 | P0 |
| FR-004 | `*anyopaque` 类型 | 等价于 C 的 `void*`，类型不安全，需显式转换 | P0 |
| FR-005 | `extern "C"` 块 | 声明外部 C/C++ 函数，是 FFI 的标志，替代 `unsafe extern` | P0 |
| FR-006 | `@cImport` 指令 | 编译期解析 C/C++ 头文件，类型自动映射，与 Zig 同级 | P0 |
| FR-007 | 所有权与借用 | 编译期所有权检查，移动语义，借用检查，无 GC | P0 |
| FR-008 | 显式分配器 | 分配器作为参数显式传递，Zig 风格，无隐式堆分配 | P0 |
| FR-009 | `defer` 语句 | 作用域结束时执行清理，替代 RAII 析构函数 | P0 |
| FR-010 | 默认 abort | panic 时直接 abort，无栈展开，无运行时开销 | P0 |

### 3.2 标准库（FR-011 ~ FR-020）

| 编号 | 功能 | 描述 | 优先级 |
|------|------|------|--------|
| FR-011 | `net` 包 | TCP/UDP/HTTP/TLS，API 对标 Go `net/http` | P0 |
| FR-012 | `net::http` | HTTP/1.1 + HTTP/2 客户端与服务器，async | P0 |
| FR-013 | `net::tls` | TLS 支持，信创场景集成国密 SM2/SM3/SM4 | P1 |
| FR-014 | `fs` 包 | 文件操作，对标 Go `os`/`io` | P0 |
| FR-015 | `strings` 包 | 字符串处理 | P0 |
| FR-016 | `collections` 包 | Vec、HashMap、BTreeMap 等 | P0 |
| FR-017 | `channel` 包 | mpsc/broadcast 通道，类型安全 | P1 |
| FR-018 | `alloc` 包 | 分配器接口，GeneralPurposeAllocator | P0 |
| FR-019 | `c` 包 | C ABI 类型定义（c_int, c_void 等） | P0 |
| FR-020 | `async` 包 | Future trait, Waker, FfiFuture 等 | P0 |

### 3.3 编译器（FR-021 ~ FR-030）

| 编号 | 功能 | 描述 | 优先级 |
|------|------|------|--------|
| FR-021 | 前端解析 | 词法分析、语法分析、AST 构建 | P0 |
| FR-022 | 所有权检查器 | 编译期检查所有权、借用、生命周期 | P0 |
| FR-023 | 类型检查 | 类型推断、泛型单态化、trait 解析 | P0 |
| FR-024 | `@cImport` 实现 | 编译期 C/C++ 头文件解析与类型映射 | P0 |
| FR-025 | Zig 后端 | 生成 Zig 源码，保证极小二进制 | P0 |
| FR-026 | Rust 后端 | 生成 Rust 源码，复用 Rust 生态 | P1 |
| FR-027 | C ABI 中间层 | 生成 C 源码 + 头文件，所有后端基础 | P0 |
| FR-028 | 增量编译 | 秒级增量编译，对标 Zig | P1 |
| FR-029 | 交叉编译 | 支持 LoongArch64/ARM64/RISC-V/x86_64 | P1 |
| FR-030 | 自举 | 编译器用 MusLang 自身编写，自持闭环 | P2 |

### 3.4 异步模型（FR-031 ~ FR-035）

| 编号 | 功能 | 描述 | 优先级 |
|------|------|------|--------|
| FR-031 | async/await | 语法与 Rust 一致，编译期展开为状态机 | P0 |
| FR-032 | FfiFuture | FFI-safe Future，repr(C)，跨语言异步统一 | P0 |
| FR-033 | 轻量级任务 | 栈池复用，无 GC，无栈协程 | P0 |
| FR-034 | 事件驱动 | epoll/kqueue/io_uring 后端 | P0 |
| FR-035 | 调度器 | work-stealing，可选，用户显式启用 | P1 |

---

## 4. 非功能需求

### 4.1 性能

| 指标 | 目标 | 参考 |
|------|------|------|
| 编译速度 | 增量 < 1s，全量 < 10s（中等项目） | Zig 同级 |
| 二进制体积 | hello world < 8KB（静态链接） | Zig ~4KB |
| 运行时开销 | 零（无 GC、无运行时） | Zig 同级 |
| 内存安全 | 编译期保证，零 unsafe 块 | Rust 同级 |
| 并发吞吐 | 10万连接 < 500MB | 优于 Go（无 GC 压力） |

### 4.2 兼容性

| 维度 | 要求 |
|------|------|
| C ABI | 100% 兼容 System V AMD64 / AAPCS64 |
| C++ 互操作 | 支持 C 风格接口，复杂类型用不透明指针 |
| 交叉编译 | x86_64, ARM64, LoongArch64, RISC-V |
| 操作系统 | Linux（统信 UOS/银河麒麟）、macOS、Windows |

### 4.3 安全

| 维度 | 要求 |
|------|------|
| 内存安全 | 所有权强制，无 `unsafe` 块，零 GC |
| FFI 安全 | `extern` 声明即标志，类型系统区分安全/不安全 |
| 供应链安全 | 自举后零外部语言依赖 |
| 审计 | 所有 FFI 调用可审计，Cookie 协议可审计 |

### 4.4 可用性

| 维度 | 要求 |
|------|------|
| 学习曲线 | Rust 开发者零学习成本，Zig 开发者 1 天上手 |
| 错误信息 | 友好、精准、带修复建议（对标 Rust） |
| 文档 | 语言参考、标准库文档、教程、示例 |
| 工具链 | 格式化、LSP、调试信息（DWARF） |

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
│  构建链：MusLang 源码 → MusLang 编译器 → Zig 后端 → 二进制   │
│  自举：MusLang 编译器（MusLang 写）→ 自持闭环               │
│  信创：LoongArch64/ARM64 + 国密 SM2/SM3/SM4                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. 开发阶段与时间线

### 阶段一：Bootstrap（2026 Q1-Q2）

| 里程碑 | 交付物 | 依赖 |
|--------|--------|------|
| M1-1 | MusLang 语法定义（Rust 风格） | 无 |
| M1-2 | 所有权检查器 v0.1 | M1-1 |
| M1-3 | `*allowzero` 类型系统 | M1-2 |
| M1-4 | `@cImport` C/C++ 头解析 | M1-1 |
| M1-5 | Zig 后端代码生成 v0.1 | M1-2, M1-3 |
| M1-6 | `net` 标准库第一版（TCP/HTTP 客户端） | M1-5 |
| M1-7 | MusLang 编译器 v0.1（Rust 写） | M1-5 |

### 阶段二：MusKitty 内核开发（2026 Q3-Q4）

| 里程碑 | 交付物 | 依赖 |
|--------|--------|------|
| M2-1 | MusKitty Layer 1-4（基础） | M1-7 |
| M2-2 | MusKitty Layer 5（网络接驳） | M2-1, M1-6 |
| M2-3 | MusKitty Layer 6（JS 引擎适配） | M2-2 |
| M2-4 | 统信 UOS / 银河麒麟适配 | M2-3 |
| M2-5 | 国密 SM2/SM3/SM4 集成 | M2-2 |

### 阶段三：自举与信创（2027 Q1-Q4）

| 里程碑 | 交付物 | 依赖 |
|--------|--------|------|
| M3-1 | MusLang 编译器 v0.2（MusLang 写，前端） | M2-1 |
| M3-2 | MusLang 编译器 v0.3（完整功能） | M3-1 |
| M3-3 | 自持闭环（v0.3 编译 v0.3） | M3-2 |
| M3-4 | 信创测评送测 | M2-5, M3-3 |
| M3-5 | 100% 信创替代达标 | M3-4 |

---

## 7. 成功指标

| 指标 | 目标值 | 测量方式 |
|------|--------|----------|
| 信创测评通过 | 100% | 官方测评报告 |
| MusKitty 内核内存安全 | 零 CVE（内存安全类） | 静态分析 + 审计 |
| 二进制体积 | hello world < 8KB | 编译产物测量 |
| 编译速度 | 增量 < 1s | 基准测试 |
| MCP Server 并发 | 10万连接稳定 | 压力测试 |
| 自持 | 编译器自举成功 | 构建验证 |

---

## 8. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Zig 后端不稳定（Zig 未 1.0） | 编译产物可能变化 | 锁定 Zig 版本，维护兼容层 |
| 所有权检查器复杂度超预期 | 开发延迟 | 简化设计，先支持核心特性 |
| 信创 Deadline 紧迫 | 时间不足 | 聚焦 MusKitty，MusLang 先 Rust 后端 |
| 自举编译器调试困难 | 开发效率降低 | 差分测试，Rust 版对照 |
| C++ 复杂类型映射不全 | FFI 受限 | 不透明指针 + 包装函数 |

---

## 9. 开放问题

| 问题 | 状态 | 备注 |
|------|------|------|
| 是否需要 `comptime` 级元编程？ | 待定 | 可能用泛型 + 宏替代 |
| 是否需要 WASM 后端？ | 待定 | 信创场景可能不需要 |
| 包管理器设计？ | 待定 | 可能复用 Cargo 或自研 |
| 调试信息格式？ | 待定 | DWARF 是默认选择 |

---

## 10. 附录

### A. MusLang 代码示例

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

extern "C" {
    fn v8_init() -> i32;
}

fn main() {
    let ret = v8_init();  // 调用 extern 函数，不需要 unsafe 块
    if ret != 0 {
        panic!("v8 init failed");
    }
    
    let server = Server::new("0.0.0.0:8080");
    server.listen().await;
}
```

### B. 与 Rust 的语法对比

| 特性 | Rust | MusLang | 差异 |
|------|------|---------|------|
| 结构体 | `struct S { x: i32 }` | `struct S { x: i32 }` | 无 |
| 枚举 | `enum E { A, B }` | `enum E { A, B }` | 无 |
| trait | `trait T { fn f(); }` | `trait T { fn f(); }` | 无 |
| 泛型 | `fn f<T>(x: T) {}` | `fn f<T>(x: T) {}` | 无 |
| 所有权 | `let y = x;` 移动 | `let y = x;` 移动 | 无 |
| 借用 | `&x` / `&mut x` | `&x` / `&mut x` | 无 |
| 裸指针 | `*mut T` / `*const T` | `*allowzero T` / `*const T` | 类型名不同 |
| unsafe 块 | `unsafe { ... }` | ❌ 无 | 移除 |
| FFI | `unsafe extern "C" { ... }` | `extern "C" { ... }` | 无 `unsafe` |
| 网络 | `tokio::net` | `net::tcp` | 内置标准库 |

### C. 参考资料

- https://ziglang.org/documentation/master/
- https://doc.rust-lang.org/reference/
- https://pkg.go.dev/net/http
- https://github.com/asynchronics/async-ffi
- https://github.com/mozilla/uniffi-rs
- ./muscat-architecture.md

---

**文档版本**：v0.1
**创建日期**：2026-08-30
**作者**：墨染柒（Ink-dark）
**状态**：草稿，待评审

---

*MusLang — Rust 的壳，Zig 的灵魂，Go 的网络，MusCat 的心脏。🐱*
