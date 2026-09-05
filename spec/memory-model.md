# MusLang 内存模型（Memory Model）— v0.1 草案

| 项 | 内容 |
|---|---|
| 文件角色 | M1-0 决策冻结输入；所有权/借用/资源管理的规范性定义 |
| 上游 | `docs/prd.md` v0.4.6（§3.2、§3.2.3、§3.2.3.1、§3.8、§3.16、§4.3） |
| 状态 | 🔄 草案（M1-0 评审项） |
| 更新日期 | 2026-09-05 |

---

## 1. 总纲

- 内存安全**编译期**保证：所有权 + 移动语义 + 借用检查（NLL），无 GC（FR-007）。
- **无 RAII / `Drop` trait**：资源清理由 `defer`（全路径）与 `errdefer`（错误路径补充）显式承担（FR-009，D-12）。
- 不安全能力不通过 `unsafe` 块表达，而通过**类型标注**表达（FR-002，安全边界见 `spec/unsafe.md`）。

## 2. 所有权与移动

1. 每个值有唯一所有者；赋值、传参、返回默认**移动**（move）。
2. 实现了 `Copy` 的类型按位复制（自动派生规则与 Rust 一致，§3.2.2）。
3. 移动后原绑定失效，编译器在 MIR 层拒绝 use-after-move。
4. 智能指针 MVP 仅 `Box<T>`；`Rc`/`Arc` 为 P1（`Arc` 跨 FFI 仅限 D-7 B 策略）。

## 3. 借用与生命周期

- 借用：`&T` / `&mut T`，别名规则与 Rust NLL 一致；借用检查在 MIR 层完成，生成代码**无运行时 borrow tracker**。
- 生命周期标注采用 **D-14 方案 B**：
  1. 函数签名默认全省略；返回值只引用一个输入参数时自动绑定（推断失败 → `E_LIFETIME_AMBIGUOUS` + 标注建议）；
  2. 结构体/枚举字段必须显式标注（`E_LIFETIME_STRUCT_OMITTED`）；
  3. `'static` 保留，语义同 Rust；HRTB 推迟 1.0（`E_HRTB_NOT_SUPPORTED`）；
  4. `extern` 边界强制显式标注（`E_LIFETIME_FFI_OMITTED`）。
- 借用检查器与所有权检查的 MVP 验收基线：通过 Rust 测试用例子集（NLL 除外，M1-2）。

## 4. 指针类型安全分级（§3.2.1）

| 类型 | 分级 | 约束 |
|---|---|---|
| `&T` / `&mut T` | 安全 | 借用检查器保证 |
| `*const T` / `*mut T` | 不可为空裸指针 | 解引用属不安全操作，进审计清单 |
| `*allowzero T` | 可空裸指针 | 类型级不安全标志（内核 MMIO、C 兼容）；与 `*const/*mut` 不可隐式转换，须经 `@ptrCast` |
| `*anyopaque` | 不透明指针 | 无类型信息，显式转换 |

- 空值表达：可空性首选 `Option<T>` / `Option<&T>`；`*T` 永不为空（§4.3.1）。
- 边界检查：切片**全模式**边界检查，越界 → panic → abort（FR-010）；性能逃逸必须走裸指针并进审计清单。

## 5. 资源管理：`defer` / `errdefer`（D-12，v0.4.6 修订）

> 与 Zig 语义对齐：`defer` **全路径执行**，`errdefer` 仅错误路径**额外**执行。

| 规则 | 内容 |
|---|---|
| 一 | 正常退出（自然结束 / `return` / `break`）：仅 `defer` 执行，LIFO；`errdefer` 不执行 |
| 二 | 错误退出（`?` 或 `return Err`）：`defer` 与 `errdefer` **均执行**，同一条 LIFO 栈、按注册逆序交错 |
| 三 | `async fn` 在 `.await` 点被取消：`defer` 的同步部分执行；**取消非错误返回，`errdefer` 不执行**；`defer` 体内禁止 `.await`（`E_DEFER_AWAIT_FORBIDDEN`），异步清理必须显式 `close().await` |
| 四 | 多个 `defer` 按文本逆序；`defer` 体内 panic 不被捕获（panic → abort，panic 路径不执行任何 `defer`） |

编译期校验（不可关闭，纳入 D-4 门禁）：

| 校验 | 错误码 |
|---|---|
| `defer` 体含 `.await` | `E_DEFER_AWAIT_FORBIDDEN` |
| `errdefer` 体返回 `Result` | `E_ERRDEFER_INFALLIBLE` |
| `errdefer` 体内 `?` | `E_ERRDEFER_TRY_FORBIDDEN` |
| 跨 `.await` 捕获已移动变量 | `E_DEFER_CAPTURE_MOVED` |
| 取消后 `defer` 访问已 drop 局部 | `E_DEFER_USE_AFTER_DROP` |
| `defer` 引用已跨 FFI 移交句柄 | `E_DEFER_USED_AFTER_TRANSFER` |

## 6. 内置容器的自动释放（D-20）

- `Box` / `Vec` / `HashMap` 等**标准库拥有型容器**在作用域退出时**自动 free**：底层为编译器隐式插入 `defer`（非 RAII `Drop`）。
- 执行顺序：隐式 `defer box.free()` 与用户显式 `defer` 共享同一条 LIFO 栈（注册点 = 创建点）→ 其后注册的用户 `defer` 先执行，可安全访问 `Box` 内容。
- 错误路径：`defer` 全路径执行 ⇒ `?` 提前返回时 `Box` 同样被释放，不泄漏。
- 显式放弃所有权：`Box::into_raw` / `Box::leak`（对应 D-7 移交语义）；此后释放责任归接收方。
- **用户自定义类型不享有隐式 free**，仍须手写 `defer`（P1 零隐藏控制流）。
- `muslangc --emit audit` 列出所有隐式 free 调用点。

## 7. 分配器（D-20）

1. 默认写法 `Box::new(x)` / `Vec::new()` / `HashMap::new()`：编译器在 HIR 层注入当前作用域默认分配器（重写为 `*_in(.., __default_allocator)`，不产生泛型参数、不新增单态化实例）。
2. 三层解析（编译期确定）：`#[default_allocator(alloc)]` 函数注解 > 模块级 `use ... as default` > 全局兜底。
3. 全局兜底：`muslang-rt` = `GeneralPurposeAllocator`（FR-021，调试模式带泄漏检测）；`muslang-rt-c` = `malloc/free`（经 `MusAllocator::from_c` 桥接，deallocator 配对）。
4. **L1 core（`#![no_std]` + `#![no_runtime]`）无兜底**：`Box::new` 报 `E_ALLOC_NO_DEFAULT`，必须 `#[default_allocator]` / `new_in` 显式提供（bump / pool）。
5. 显式覆盖：`Box::new_in(x, &alloc)` / `Vec::with_capacity_in(cap, &alloc)` / `HashMap::with_allocator(&alloc)`；集合 `clone`/`extend` 沿用目标集合的分配器。

| 错误码 | 触发 |
|---|---|
| `E_ALLOC_NO_DEFAULT` | L1 core 中无默认分配器使用 `Box::new` / `Vec::new` |
| `E_ALLOC_MISMATCH` | 跨分配器分配/释放未配对 |
| `E_DEFAULT_ALLOCATOR_UNRESOLVED` | 注解引用的分配器不可见 / 生命周期不足 |
| `W_ALLOC_LEAK`（可升级） | GPA 调试模式检测到泄漏 |

## 8. 跨 FFI 所有权（摘要，全文见 PRD §3.8 / D-7）

- **A 严格移交**（默认）：跨边界即 move，释放责任随所有权转移；唯一可被借用检查器实质保证的模式。
- **C 标注协议**：`#[muslang::take]` / `#[muslang::borrow]` / `#[muslang::borrow_mut]` / `#[muslang::ret]` 逐参数标注；**同函数内 A/C 不得混用**（`E_FFI_MIXED_STRATEGY`）。
- **B `Arc` across FFI**：P1 显式设施，须配对 `arc_drop`（`W_ARC_LEAK`）。
- `defer` 清理必须在所有者一侧执行；跨边界对象的析构不得两侧都注册。

## 9. 并发与取消

- 数据竞争自由：`Send` / `Sync`（§4.3.1）；`channel` 包 P1（FR-020）。
- 取消模型：协作式，仅发生在 `.await` 点（D-12 规则三）；标准库异步类型须文档化 cancel-safety（对标 Tokio）。
- event loop drop 未完成 future 时触发 `defer` 清理（D-19 §3.15.6）。

## 10. 未决问题（M1-0 评审）

- [ ] `static mut` 是否在 MVP 保留（类型级不安全，进审计清单）。
- [ ] `Box` 隐式 free 与用户显式 `box.free()` 并存时是否报错（重复释放风险）。
- [ ] `Copy` 派生规则是否与 Rust 完全一致（含 `&T` auto-copy 边界）。
- [ ] 切片胖指针布局是否携带生命周期参数（影响 D-0 冻结）。
- [ ] `W_ALLOC_LEAK` 默认为警告的升级条件（CI 中何时强制为错误）。
