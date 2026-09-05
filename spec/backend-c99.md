# MusLang C99 后端规格（backend-c99）— v0.1 草案

| 项 | 内容 |
|---|---|
| 文件角色 | M1-0 决策冻结输入；MVP 默认后端（FR-032 / D-2）的映射规范 |
| 上游 | `docs/prd.md` v0.4.6（§3.4、§3.9、§5.3、§3.15.6、§3.12.4、D-2/D-13/D-16/D-19） |
| 状态 | 🔄 草案（M1-0 评审项） |
| 更新日期 | 2026-09-05 |

---

## 1. 定位

- **C99 为 MVP 默认后端**（D-2）：保证极小二进制、避免锁定未 1.0 的 Zig；LLVM IR = P1（FR-032b）；Zig = P2 远期复评（FR-032c）。
- 编译管线：`.mus` → muslangc（前端 + HIR + 单态化 + MIR 检查）→ **C99 源码** → cc/clang → `.o` → muslink / LLD → ELF64。
- 后端只见到**单态化后的具体函数**（D-13：单态化在 HIR 层完成），无需理解泛型语法。

## 2. 语言构造 → C99 映射规则

| MusLang 构造 | C99 输出（M1-5 v0.1 基线） |
|---|---|
| `Result<T, E>` | 返回值 + `errno` / 输出参数（§5.3） |
| `Option<T>` | 哨兵值或 `_Bool ok` + payload |
| `?` 传播 | `goto err` / 早期返回标签 |
| `panic!(..)` | `abort()`（FR-010：无栈展开） |
| `defer` / `errdefer` | 作用域出口清理段：**全路径出口**（正常 label + 错误 label 共享同一条 LIFO 清理序列，`errdefer` 项仅在错误出口执行——对应 D-12 规则一/二） |
| `Box` / `Vec` / `HashMap` 隐式 free | 编译器在 HIR 已重写为显式 `defer`（D-20），后端按普通 defer 生成；所有分配经 `__default_allocator` 静态引用 |
| `async fn` | 状态机 `struct` + `poll(ctx, waker)` 函数；运行时驱动由 C 实现的 event loop API 承担（D-19）——**后端不生成运行时**，只生成状态机 + 调用点 |
| `.await` | 状态机挂起点：保存局部、返回 `Pending`、记录续延 |
| 单态化实例命名 | `模块路径_函数名_类型哈希`（**稳定类型哈希**，不含时间戳/路径——D-16 §3.12.4 M2 要求） |
| `#[repr(C)]` 类型 | 一一对应 C 布局（rt-c / FFI 边界） |
| 非 repr(C) 类型（rt 自有布局） | 按 muslang-rt 布局规范生成（胖指针 / vtable 槽位，D-8）——**布局表待 D-0 冻结** ⚠ |

## 3. defer / errdefer 代码生成（与 D-12 一一对应）

```c
/* fn process() { let v = Box::new(42); defer cleanup(); ... } 的骨架示意 */
 MusLang 作用域                        C99 生成
 ─────────────────────────            ─────────────────────────────────────
 let v = Box::new(42);                Box_i32 v = box_new_in(&__default_allocator);
 defer cleanup();                     /* 注册：cleanup 先于 v.free 执行 */
 ...
 正常出口 / 错误出口（? → goto）  →    cleanup(); v_free(&v);   /* 单一 LIFO 清理段 */
```

- 正常出口与错误出口**共用**清理序列；`errdefer` 项仅编入错误出口分支。
- panic 路径不进入清理段（abort 语义，D-12 规则四）。

## 4. 体积预算与膨胀控制（三层，配合 D-13 / D-9）

1. std 按需链接：未实例化类型组合不进入编译单元；
2. C 编译器内联：单态小函数被调用方吸收；
3. `--gc-sections`：每个单态实例独立 section，未引用即丢弃。

测量协议（§4.1）：`x86_64-linux-musl`、`-Oz -flto`、静态链接、无动态加载；L1 core 裸启动 < **8KB**（M1-8 中期口径 < 16KB）。

## 5. 确定性输出（D-16 §3.12.4，M2 交付）

- [ ] 临时变量名去随机化；单态命名用稳定类型哈希（无时间戳 / 路径）；
- [ ] `muslangc --reproducible`：固化哈希表遍历顺序、section 顺序；
- [ ] Stage 2 验收：同一源码 Stage 1 与 Stage 2 产物 SHA-256 一致（白名单差异：debug 路径、嵌入时间戳）。

## 6. 未决问题（M1-0 评审）

- [ ] muslang-rt 自有布局（胖指针 / vtable / 借用信息）的 C 表示——依赖 D-0 冻结。
- [ ] `Result<T,E>` 大对象返回的 ABI（寄存器 vs sret）约定。
- [ ] async 状态机的 C 表示：协程帧布局、`Waker` vtable 形状（依赖 D-0 + FR-023）。
- [ ] 错误出口共用清理段时 `errno` 的保存/恢复次序。
- [ ] 与 `@cImport` 生成代码的命名空间隔离规则。
