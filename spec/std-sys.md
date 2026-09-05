# MusLang 标准库与 sys 层规格（std-sys）— v0.1 草案

| 项 | 内容 |
|---|---|
| 文件角色 | M1-0 决策冻结输入；std 拆分、sys 三端分发、event loop、分配器后端的规范 |
| 上游 | `docs/prd.md` v0.4.6（§3.3、§3.7.3、§3.7.4、§3.15、§3.16.4，D-8/D-9/D-10/D-19/D-20） |
| 状态 | 🔄 草案（M1-0 评审项） |
| 更新日期 | 2026-09-05 |

---

## 1. std crate 拆分（D-9：按需链接）

| 子系统 crate | 优先级 | 对标 / 说明 |
|---|---|---|
| `std::net`（TCP/UDP/HTTP） | P0 | Go `net`（TLS 见 `std::net::tls`，P1，FR-016） |
| `std::net::http` | P0 | Go `net/http`：HTTP/1.1 + HTTP/2，async（FR-015，API 见 §3.3.1） |
| `std::fs` | P0 | Go `os`/`io` |
| `std::strings` | P0 | Go `strings` |
| `std::collections` | P0 | `Vec` / `HashMap` / `BTreeMap` |
| `std::alloc` | P0 | 分配器接口（`Allocator` trait / `MusAllocator`，FR-021） |
| `std::c` | P0 | C ABI 类型（`c_int` / `c_void` / …，FR-022） |
| `std::async` | P0 | `Future` / `Waker` / `FfiFuture`（FR-023） |
| `std::io` | P0 | 统一 I/O 接口（FR-024） |
| `std::channel` | P1 | mpsc / broadcast（FR-020） |
| `std::time` / `std::encoding` / `std::crypto` / `std::net::tls` | P1 | FR-025/026/027/016（crypto 含国密 SM2/SM3/SM4） |

- **用哪个链哪个**：未 `use` 的 crate 整块被 `--gc-sections` 剔除；体积仅由实际使用决定。
- M1 分发打包：单 crate `muslang-std` + features 控制源码参与编译（分发口径，语言层仍逐子系统按需链接，D-17 §3.13.1）。

## 2. sys 层与三端分发（D-9）

```
std facade（统一 API）
   ├── std::os::linux     ← 当前实装
   ├── std::os::macos     ← trait + stub（架构就绪）
   └── std::os::windows   ← trait + stub
            │
     muslang-rt-c  ──►  libc（glibc / musl / …）
```

- `sys` 按 `target_os` 分发；**当前仅 Linux 实装**，macOS/Windows 仅承诺架构可扩展，不承诺 API 可用（§3.7.4）。
- **rt-c = std 底层 + C 互操作边界 = 唯一系统接口**（D-10）：std 走 rt-c，本身不产生 FFI 开销。
- 双 runtime：`muslang-rt`（自有布局 / 类型化 panic）/ `muslang-rt-c`（`repr(C)` / `malloc-free` / `errno`）；L2 hosted 二选一，L3 compat 可并存（D-8）。

## 3. `std::net` 与 event loop（D-19）

1. **事件循环是 `std::net` 的内部依赖**：`use std::net` 即自动链接；不提供运行时选择、不引入 `#[entry]` / `block_on`；`fn main` 为同步入口。
2. MVP 规格：**单线程 epoll**（Linux）/ kqueue（macOS，经 sys 分发）；每连接一个状态机 future（挂起不占运行槽）；就绪队列固定 256 深、无动态扩容、超出背压；无锁；体积 < 20KB（不含 TLS / HTTP 解析）。
3. 事件循环调度核心、`Waker`、任务类型属 **muslang-rt**；syscall 经 `std::sys` 发出（D-10）；「C 实现」指编译产物为 C99（D-2）。
4. executor 内部统一 `Box<dyn Future>`（D-13 第一个例外）。
5. 不用 `std::net` ⇒ event loop 完全不链接（`#![no_std]` / 内核场景）。
6. 替换路径：嵌入已有 C event loop → 不用 `std::net`，基于 `std::sys` + `FfiFuture`（FR-044）自行实现。
7. 多线程 work-stealing（FR-047）/ io_uring = P1，仅经 `Server::with_config(Threads(n))` 配置，不暴露 Executor 注入接口。

## 4. 分配器后端（D-20 §3.16.4）

| runtime | 兜底分配器 | 说明 |
|---|---|---|
| `muslang-rt` | `GeneralPurposeAllocator` | FR-021；调试模式泄漏检测（`W_ALLOC_LEAK`） |
| `muslang-rt-c` | `malloc/free` via `MusAllocator::from_c` | deallocator 配对，C 侧可直接 `free` MusLang 的 `Box`（D-7） |
| L1 core（`#![no_std]` + `#![no_runtime]`） | **无兜底** | `Box::new` 报 `E_ALLOC_NO_DEFAULT`；bump / pool 由用户经 `#[default_allocator]` 显式提供 |

- `std` 自身（含 `alloc` crate）必须用显式 `#[default_allocator]`——避免自举期"兜底尚未编译完成"（D-16 §3.16.9）。

## 5. 按需链接实现约束

- 每个子系统 / 单态实例独立 section，配合 `--gc-sections`；
- event loop 与 `std::net` 同 section 归属（D-19）；
- L1 core 验收口径：hello world 裸启动 < 8KB（`x86_64-linux-musl`、`-Oz -flto`、静态链接）。

## 6. 未决问题（M1-0 评审）

- [ ] 各子系统 crate 的最终命名与依赖图（`std::` 前缀 vs 顶层 `net`/`fs`——PRD 示例已统一为 `std::`）。
- [ ] `std::io` 与 Zig 新 Io 模型的接口对齐程度（FR-024）。
- [ ] macOS kqueue 分发的 trait 边界（`std::os` API 形状）。
- [ ] `FfiFuture` 的 `repr(C)` 布局（依赖 D-0 冻结）。
- [ ] M1 `muslang-std` features 的初始清单。
