# docs/draft/ — M1-0 决策冻结评审草案

> **角色**：M1-0「决策冻结（2 周）」门槛的评审件。五份草案对应 PRD §3.7.1 中状态为「待冻结 / 待落地」的五项决策。
> **上游**：`docs/prd.md` v0.4.7（D-0~D-21）
> **日期**：2026-09-06 · **评审人**：墨染柒（BDFL）+ 编译器团队（Orcha）

---

## 文件清单

| 文件 | 决策 | 内容 | 状态 |
|---|---|---|---|
| [d0-memory-layout.md](d0-memory-layout.md) | **D-0** | 共享内存布局规范（IR/HIR 共享层，LAY-1 版本化） | 提议冻结 |
| [d3-dialect-hir-interop.md](d3-dialect-hir-interop.md) | **D-3** | 共享方言 + 统一 HIR 互操作契约（薄前端边界、无损性定义、降级表） | 提议冻结 |
| [d4-unsafe-gate.md](d4-unsafe-gate.md) | **D-4** | `unsafe` 等价判定**落地**：禁止操作清单定稿（F-01~F-15）+ `unsafe_examples/` 用例集规格 + `--unsafe-allowed` CI 门禁 | 落地草案 |
| [d5-mlir-decision.md](d5-mlir-decision.md) | **D-5** | 是否引入 MLIR —— **提议：不引入**（附复评触发条件） | 提议冻结 |
| [d6-call-topology.md](d6-call-topology.md) | **D-6** | 调用拓扑与开销口径（边界定义 B1~B3、编译器义务、O(模块数) 承诺） | 提议冻结 |

## 毕业流程（评审通过后）

```
docs/draft/*.md ──评审通过──► ① 规范内容迁入 spec/（正文）
                              ② PRD §3.7.1 决策状态改为「已冻结（M1-0）」
                              ③ docs/CHANGELOG.md 记 v0.4.8+
                              ④ 本目录文件标注「已毕业」归档
```

## 评审检查单（逐文件）

- [ ] 与 PRD 已定决策（D-1/D-2/D-7/D-8/D-9/D-12/D-13/D-15/D-19/D-20/D-21）零冲突
- [ ] 错误码不与既有注册表（spec/unsafe.md §4）冲突、编号连续
- [ ] 每条规范性规则有对应的检查层级（HIR / MIR / 类型检查 / 链接期）
- [ ] 未决问题逐条列明、不冒充已定
