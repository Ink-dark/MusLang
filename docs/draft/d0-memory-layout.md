# D-0 共享内存布局规范（Shared Memory Layout Spec）— 提议冻结 LAY-1

| 项 | 内容 |
|---|---|
| 决策编号 | D-0（PRD §3.7.1，状态：待冻结 → 本文件提议冻结） |
| 文件角色 | M1-0 评审件；评审通过后正文迁入 `spec/layout.md` 并成为**不可变更**规范 |
| 上游 | `docs/prd.md` v0.4.7（§3.7.2 D-3、§3.7.3 D-8、§3.11 D-15、§3.8 D-7、FR-044）；`spec/memory-model.md` |
| 关联 | D-8（双 runtime 布局）、D-15（Itanium C++ ABI）、D-13（单态化）、FR-044（FfiFuture）、D-21（`SecretBox` 清零语义） |
| 日期 | 2026-09-06 |

---

## 1. 冻结承诺（P0 放行门槛语义）

1. **语法可改，布局一经发布不可改**（PRD D-0 原文）。本文件定义冻结的版本号 **LAY-1**：M1-0 评审通过即发布 LAY-1，此后任何变更只允许**追加式演进**（LAY-2 引入新类型 / 新可选布局，不重新解释 LAY-1 已冻结项）。
2. 冻结范围 = **跨方言共享层**（C / Rust / MusLang 三方言可见、跨 `.so` / 跨 runtime 边界可传递的一切布局）。**muslang-rt 内部私有布局不在冻结范围**（如借用检查的调试注记、HashMap 桶结构）——内部布局变更不构成 LAY 破坏。
3. 违反冻结 = 语言级事故：`layout-diff` CI 门禁（§7）阻断任何对已冻结项的重解释。

## 2. 布局分级

| 级别 | 标注 | 冻结性 | 用途 |
|---|---|---|---|
| **F（Frozen）** | `#[repr(C)]` 或本文件列出的内建类型 | LAY 冻结，跨边界必用 | FFI / 共享数据 / `FfiFuture` / rt-c |
| **R（rt 内部）** | `#[repr(muslang)]`（默认，可省略） | ❌ 不冻结 | 纯 MusLang 镜像内部（D-8 muslang-rt 自有布局） |
| **P（Pinned）** | `#[repr(C)]` 显式钉在 rt-c 侧 | LAY 冻结 | L3 反复跨界热点（§3.7.5 建议） |

> 规则：**跨 B1/B2/B3 边界（见 d6-call-topology.md）传递的类型必须为 F 或 P 级**；R 级类型跨界 → 编译错误 `E_NON_FFI_LAYOUT`（新增错误码，注册进 spec/unsafe.md §4）。

## 3. 标量布局表（F 级，全平台冻结）

| 类型 | 大小 | 对齐 | 备注 |
|---|---|---|---|
| `bool` | 1 | 1 | 合法值 0/1；其他值 = 违约（跨界前须归一化） |
| `char` | 4 | 4 | Unicode 标量值（同 Rust），非 C 的 `char` |
| `i8/u8/i16/u16/i32/u32/i64/u64/i128/u128` | 同位宽 | 同位宽（≤8），i128 对齐 8 | 小端序平台按目标端序（跨界二进制协议自行声明端序） |
| `f32/f64` | 4/8 | 4/8 | IEEE-754 |
| `usize/isize` | 指针位宽 | 同 | x86_64/ARM64/LoongArch64/RISC-V64 全部 64 位 |
| `c_char/c_short/c_int/c_long/c_longlong` | 按 target ABI | 同 | LP64：c_long = 8；c_char = i8（Linux） |
| `c_size_t/c_ssize_t/c_ptrdiff_t` | 同 usize/isize | 同 | |
| `c_void` | — | — | 仅作指针点类型；`*anyopaque` 为 MusLang 侧等价物（FR-004） |

## 4. 复合布局规则

### 4.1 `#[repr(C)]` 结构体（F 级）

- 字段顺序、对齐、尾部填充 = C 编译器规则（System V AMD64 / AAPCS64 / LoongArch ABI 一致行为）；
- 对齐上限：标量字段自然对齐；**`#[repr(packed(N))]` / `#[repr(align(N))]` 为显式 opt-in**，不默认；
- 空结构体大小 = 1（C 行为），R 级可为 0（不跨界）。

### 4.2 枚举与判别值（F 级）

| 形态 | 冻结布局 |
|---|---|
| 字段级 enum（C 风格） | 默认判别类型 = `c_int`；`#[repr(i8/i16/.../i128)]` 显式覆盖 |
| 标签联合（Rust 风格 enum with data） | 跨界用 `#[repr(C)]` tag + union 手写；**编译器自动布局的 tag 形态属 R 级，不冻结** |
| 判别值显式指定 | `= N` 语法支持，跨界契约的一部分 |

### 4.3 `Option<T>` 空间优化（F 级 niche 清单）

| 类型 | niche 规则 | 级别 |
|---|---|---|
| `Option<&T>` / `Option<*const T>` / `Option<*mut T>` | null = None（`*allowzero` 同理） | **F，冻结** |
| `Option<Box<T>>` / `Option<Vec<T>>` | 同上（内部指针 niche） | F，冻结 |
| `Option<bool>` / `Option<char>` / `Option<NonZero*>` | 允许 niche，**布局不做跨边界承诺** | R |

### 4.4 胖指针形状（F 级）

| 类型 | 冻结布局（字段顺序） |
|---|---|
| `&[T]` / `&str` | `(ptr: *T, len: usize)` |
| `&dyn Trait`（MusLang 自有 trait object） | `(data: *c_void, vtable: *MusLangVTable)` |
| `Box<T>`（T: Sized） | 单裸指针（不可空） |
| `Box<T>`（T: ?Sized） | 同 `&dyn` 形状 |
| `Vec<T>` / `String` | `(ptr, cap, len)`——字段顺序冻结为 **ptr, cap, len** |

**`MusLangVTable`（F 级，MusLang 自有 trait object）**：

```
slot 0: usize    // size（对象大小，供 Box<dyn> 释放）
slot 1: usize    // align
slot 2..: 方法指针，按 trait 声明顺序
```

- MusLang 无 `Drop` trait（D-12）⇒ vtable **无 drop 槽位**——销毁一律经具体类型函数或 `defer`；
- `Box<dyn Future>`（D-19 executor 例外）即用此 vtable。

### 4.5 C++ 互操作布局（F 级，绑定 D-15）

1. 单继承 + 虚函数类：对象头 **vptr 于 offset 0**（Itanium ABI）；vtable 布局 = `[offset-to-top, typeinfo 指针, 虚函数槽...]`；
2. MusLang **不读** typeinfo 槽（RTTI 不支持，D-15），虚函数槽索引按 `extern "C++"` **声明顺序**计算；
3. MusLang 不调用 C++ 虚析构槽（D1/D2）——析构经 C++ 侧显式 API（与 D-7 清理归属一致）；
4. 一致性由 C++ `static_assert(offsetof(...))` 交叉测试保证（§7）。

### 4.6 `FfiFuture` 与 `Waker`（F 级，FR-044 / D-19）

```text
FfiFuture<T>:  repr(C) {
    frame:  *mut c_void,                        // 状态机帧（D-13 单态化后具体类型，跨界仅指针）
    poll:   fn(frame: *mut c_void, ctx: *mut Context) -> Poll,
}
Poll:          repr(C) { tag: c_int }           // 0 = Ready, 1 = Pending；payload 经独立 out 参数传递
Context:       repr(C) { waker: Waker }
Waker:         repr(C) { data: *mut c_void, vtable: *WakerVTable }
WakerVTable:   repr(C) { clone, wake, wake_by_ref, drop_fn }   // 4 槽，顺序冻结
```

> 说明：MusLang 无 `Drop`，`WakerVTable.drop_fn` 为 **Waker 资源回收**专用槽（非类型析构）；命名避免与关键字 `drop` 冲突。

### 4.7 跨界 `Result`（F 级）

- 语言内 `Result<T, E>` 为 R 级（tag 自动布局可优化）；
- **跨 B1~B3 边界**必须使用 `#[repr(C)]` 的 `FfiResult<T, E> = { tag: c_int, union { ok: T, err: E } }`（0 = Ok）或经 D-7 A/C 策略的输出参数约定（§3.8）。

### 4.8 不透明类型（F 级承诺最小化）

`HashMap` / `BTreeMap` / `GPA` 等**内部结构不冻结**（仅承诺存在性与 API）；跨边界一律以句柄 + 访问函数传递。`SecretBox<T>`（D-21）冻结承诺仅两条：布局同 `T`；作用域退出**先清零后 free**（行为冻结，进审计清单）。

## 5. 对齐与填充

1. 自然对齐规则同 C；i128 / 向量类型按 target ABI（x86_64: 16 / ARM64: 16 / LoongArch64 LSX: 16 / LASX: 32 ⚠ 待平台表确认）；
2. 填充字节内容**未定义**：跨边界序列化场景必须显式 `#[repr(packed)]` + 手工填充或按字段序列化——不允许直接 `memcmp`/hash 带填充结构体（含填充的结构体上 `PartialEq` 由编译器按字段生成，不算违反 P1）；
3. `usize` 对齐 = 指针对齐（8，64 位平台）。

## 6. 工具与流程

| 工具 | 作用 |
|---|---|
| `muslangc --emit layout` | 导出当前编译单元全部 F 级类型布局（JSON，含 size/align/offset/discriminant），LAY-1 schema |
| `layout-diff`（CI 门禁） | LAY-1 冻结后，对已发布类型与基线 diff：任何重解释（size/align/offset 变化）→ **阻断合并**；纯追加 → 要求人工确认后更新基线 |
| 交叉验证测试 | C 侧 `_Static_assert(offsetof/sizeof ...)` + C++ 侧 Itanium offsetof 断言 + Rust 侧 `memoffset` 断言——三语言对照同一份向量 |

## 7. 未决问题（M1-0 评审）

- [ ] 向量类型（LSX/LASX/SSE）对齐的平台表；
- [ ] `FfiResult` 的 niche（`tag` 与 union 重叠）是否允许——倾向**不允许**（tag 独立存储，换可预测性）；
- [ ] `char` niche（`Option<char>`）是否升 F 级；
- [ ] R 级布局是否需要 `--emit layout --all` 调试导出（倾向是，不进 CI 门禁）；
- [ ] LAY-1 基线文件入库位置（拟 `spec/layout-baseline/LAY-1.json`）。
