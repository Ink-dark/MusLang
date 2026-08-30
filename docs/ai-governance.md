# AI 生成（贡献）代码准入流程

> **文档版本**：v1.0
> **生效日期**：2026-08-30
> **适用项目**：MusLang、MusKitty 及 MusCat 生态所有仓库
> **作者**：墨染柒（Ink-dark）
> **状态**：生效

---

## 1. 总则

### 1.1 目的

本流程规范 AI 辅助生成代码从**生成 → 审查 → 测试 → 合并 → 追溯**的全生命周期管理，确保：

- 所有 AI 贡献代码**可审计、可追溯、可回滚**
- 核心模块的安全性不因 AI 生成而降级
- 满足信创测评对软件供应链安全的要求（T/CIIPA 00012-2024）

### 1.2 核心原则

| 原则 | 含义 |
|---|---|
| **人在回路（Human-in-the-Loop）** | AI 生成代码不得直接合入主干，必须经过人工审查 |
| **生成隔离** | AI 操作限定在独立分支和工作区内 |
| **机器先筛** | 自动化工具先过滤低级问题，人工只审逻辑和架构 |
| **红线不可越** | 核心安全模块禁止 AI 直接修改 |
| **全程留痕** | 每次 AI 贡献的生成模型、审查人、审批记录完整保留 |

---

## 2. 工具配置

### 2.1 AI 辅助工具选型

| 工具 | 用途 | 权限级别 |
|---|---|---|
| DSH (DeepSeek Harness) | 主力 AI 编码助手 | 工作区隔离 + 写操作审批 |
| 其他模型/工具 | 辅助分析、文档生成 | 只读模式，禁止直接写代码 |

### 2.2 DSH 配置要求

```
工作区绑定：仅允许访问项目指定目录
  允许：src/、std/、compiler/、muslink/、docs/
  禁止：~/.ssh、/etc、其他项目目录

默认模式：Minimal（只读分析）
   → 需要写代码时手动切换到 Standard 或 PTC 模式

写操作：全部弹窗审批，人工逐条确认

PTC 模式使用场景：批量重构、多文件联动修改（流程更可控、可审计）
```

### 2.3 禁止配置

- ❌ 禁止 DSH 绑定外网暴露端口
- ❌ 禁止 AI 工具持有 commit/push 权限
- ❌ 禁止 AI 自动生成后直接执行 `git commit`
- ❌ 禁止 AI 访问生产环境凭证、签名密钥

---

## 3. 代码生成与隔离

### 3.1 分支策略

```
main（主干）
  ↑ 仅通过 Pull Request 合入，禁止直接 push
  │
  ├── ai/draft/<feature-name>     ← AI 生成代码在此分支
  │     （DSH 工作区绑定到此分支）
  │
  ├── ai/refactor/<module-name>   ← AI 辅助重构
  │
  └── manual/<description>        ← 人工编写
```

**规则**：
- AI 生成代码**必须在** `ai/` 前缀分支上生成
- 一个 AI 分支对应一个明确任务，禁止"大杂烩"分支
- 分支命名格式：`ai/<type>/<short-description>-<date>`
  - 例：`ai/draft/lexer-scanner-20260830`

### 3.2 任务描述规范

向 AI 下达任务时，必须包含以下信息（写入分支描述或 PR 描述）：

```markdown
## AI 任务描述

- 模型：DeepSeek-R1 / 其他
- 工具：DSH v0.x（PTC / Standard 模式）
- 目标文件：compiler/src/scanner.rs
- 任务：实现词法分析器，支持 MusLang 基础 token（fn/let/if/else/struct）
- 约束：
  - 纯 Rust std，零第三方 crate
  - 无 unsafe 块
  - 每个 token 携带 span（行号/列号）
- 参考：PRD 3.1 节 FR-001 ~ FR-010
- 生成时间：2026-08-30
```

---

## 4. 自动化关卡

AI 分支推送后，CI 流水线自动执行以下检查。**任何一项失败，PR 标记为"需修改"，AI 重新生成，不进入人工审查。**

### 4.1 代码规范

| 检查项 | 工具 | 阈值 |
|---|---|---|
| 代码格式化 | `cargo fmt --check` | 必须零 diff |
| Clippy 静态分析 | `cargo clippy -- -D warnings` | 零 warning |
| 未使用导入/变量 | `cargo clippy` | 零允许 |

### 4.2 安全扫描

| 检查项 | 工具 | 说明 |
|---|---|---|
| 静态应用安全测试 | `cargo audit` / Semgrep | 检测已知漏洞模式 |
| 依赖漏洞扫描 | `cargo audit` | 零高危/严重 |
| 硬编码凭证检测 | Gitleaks / TruffleHog | 零命中 |
| 不安全模式检测 | 自定义规则 | 检测 `unsafe` 块、裸指针滥用 |

### 4.3 测试

| 检查项 | 要求 |
|---|---|
| 单元测试 | `cargo test` 全部通过 |
| 覆盖率 | 新增代码行覆盖率 ≥ 80%（核心模块 ≥ 95%） |
| 回归测试 | 已有测试全部通过，零退化 |
| Fuzz 测试 | 解析器/词法分析器必须通过 fuzz（如适用） |

### 4.4 构建验证

- [ ] Debug 构建成功
- [ ] Release 构建成功
- [ ] 交叉编译验证（LoongArch64 / ARM64 / x86_64）
- [ ] 静态链接验证（musl，零动态依赖）

---

## 5. 人工审查

自动化关卡全部通过后，进入人工审查阶段。

### 5.1 审查人要求

| 模块 | 审查人 |
|---|---|
| 编译器前端（词法/语法/类型） | 墨染柒 + 至少一名协作者 |
| 所有权检查器 | 墨染柒（必须） |
| 标准库（net/fs/async） | 墨染柒 + 协作者 |
| muslink 链接器 | 墨染柒（必须） |
| 文档/PRD | 墨染柒或指定维护者 |
| 其他模块 | 至少一名有经验的开发者 |

### 5.2 审查清单

审查人逐条确认，在 PR 中勾选：

#### 功能正确性
- [ ] 实现与任务描述一致
- [ ] 边界条件处理正确（空输入、超长输入、非法字符）
- [ ] 错误处理完整（无 panic 泄漏到调用方）

#### 架构一致性
- [ ] 符合项目现有代码风格和模块组织
- [ ] 接口设计与 PRD 定义一致
- [ ] 无不必要的依赖引入

#### 安全与内存安全
- [ ] 无 `unsafe` 块（MusKitty Rust 代码）
- [ ] 无裸指针误用
- [ ] 无数据竞争风险
- [ ] FFI 边界正确（如涉及 `@cImport` / `extern`）

#### 性能
- [ ] 无明显的 O(n²) 退化
- [ ] 无不必要的内存分配
- [ ] 无热路径上的锁竞争

#### AI 特有检查
- [ ] 生成代码无"幻觉 API"（调用不存在的函数/类型）
- [ ] 无复制粘贴的 GPL 代码
- [ ] 无隐藏的后门逻辑（如网络回调、数据外传）
- [ ] 逻辑可读、可维护，不是"能跑但看不懂"

### 5.3 审查结论

| 结论 | 含义 | 后续动作 |
|---|---|---|
| **Approve** | 审查通过 | 进入审批门 |
| **Request Changes** | 需修改 | AI 重新生成或人工修改，重新跑 CI |
| **Reject** | 整体方向不对 | 关闭 PR，重新设计 |

---

## 6. 审批与合并

### 6.1 审批权限

| 分支 | 审批人 |
|---|---|
| `main` | 墨染柒（必须） |
| `dev` / `develop` | 墨染柒 或指定 Maintainer |

### 6.2 合并条件（全部满足才可合）

1. ✅ CI 流水线全绿（4.1 ~ 4.4 全部通过）
2. ✅ 至少一名审查人 Approve
3. ✅ 墨染柒本人 Approve（核心模块）
4. ✅ 无未解决的 review comment
5. ✅ Commit 消息符合标记规范（见 7.1）

### 6.3 合并方式

- **Squash Merge**：将 AI 分支的多个 commit 压缩为一个，保持主干历史干净
- 禁止 Fast-forward 合并（保留 PR 上下文）
- 合并后自动删除 AI 分支

---

## 7. 提交标记规范

### 7.1 Commit 消息格式

AI 参与生成的 commit 必须包含以下标记：

```
<type>(<scope>): <subject>

<body>（可选）

Co-Authored-By: <AI-Tool> <model-version>
Reviewed-By: <human-reviewer>
AI-Task: <link-to-task-description>
```

**示例**：

```
feat(compiler): implement lexer token scanner

Support MusLang basic tokens: fn, let, if, else, struct, impl, async.
Each token carries span (line/col) for error reporting.

Co-Authored-By: DSH deepseek-r1-2026-08
Reviewed-By: 墨染柒
AI-Task: ai/draft/lexer-scanner-20260830
```

### 7.2 PR 标签

所有 AI 相关 PR 必须打标签：

| 标签 | 含义 |
|---|---|
| `ai-generated` | AI 生成代码 |
| `ai-assisted` | AI 辅助（人工主导，AI 补全） |
| `needs-human-review` | 等待人工审查 |
| `security-sensitive` | 涉及安全相关模块 |

---

## 8. 红线模块（AI 禁止直接修改）

以下模块**禁止 AI 直接生成或修改代码**，必须由人工编写后经人工审查合入：

| 模块 | 原因 |
|---|---|
| `compiler/src/borrow_checker.rs` | 所有权检查器是语言正确性的命根子 |
| `compiler/src/type_checker.rs` | 类型系统是安全模型的基石 |
| `muslink/src/relocation.rs` | 重定位错误导致二进制不可执行 |
| `muslink/src/elf_writer.rs` | ELF 格式写入错误不可逆 |
| `std/src/net/tls/` | 国密实现，安全攸关 |
| `std/src/alloc/` | 分配器错误导致内存安全失效 |
| 任何 `unsafe` 块 | 信创内存安全红线 |

**例外**：红线模块允许 AI 生成**单元测试**和**文档注释**，但实现代码必须人工编写。

---

## 9. 审计与追溯

### 9.1 追溯记录

每次 AI 贡献代码合并后，自动记录到 `audit/ai-contributions.log`：

```json
{
  "date": "2026-08-30",
  "pr": "#42",
  "branch": "ai/draft/lexer-scanner-20260830",
  "model": "deepseek-r1",
  "tool": "DSH v0.3",
  "files_changed": ["compiler/src/scanner.rs"],
  "lines_added": 342,
  "lines_removed": 12,
  "reviewer": "墨染柒",
  "approval": "approved",
  "ci_status": "pass"
}
```

### 9.2 SBOM 关联

AI 生成的代码在 SBOM（软件物料清单）中标记为：

```
component: compiler/src/scanner.rs
  supplier: MusCat Community
  author: AI-assisted (DeepSeek-R1) + 墨染柒
  license: Apache-2.0
  ai-generated: true
  human-reviewed: true
  review-date: 2026-08-30
```

### 9.3 信创合规

本流程满足以下信创测评要求：

| 测评项 | 对应条款 |
|---|---|
| 供应链可追溯 | 第 9.2 节 SBOM 标记 |
| 代码自主可控 | 人工审查 + sign-off 记录 |
| 安全审计 | 第 9.1 节追溯日志 |
| 第三方组件管理 | 红线模块人工编写 |

---

## 10. 违规处理

| 违规行为 | 处理 |
|---|---|
| AI 代码未经 CI 直接合入 | 立即 revert，审查人书面说明 |
| AI 代码未标记 Co-Authored-By | PR 打回，补充标记 |
| 红线模块被 AI 修改 | 立即 revert，该模块冻结审查 7 天 |
| AI 工具配置泄露凭证 | 立即轮换密钥，审计所有历史生成代码 |

---

## 11. 附录

### A. 参考标准

- T/CIIPA 00012-2024《软件自主可控能力评估方法》
- ISO/IEC 5230:2020（开源许可证合规）
- NIST AI RMF 1.0（AI 风险管理框架）

### B. 相关文档

- ./prd.md
- ./kitty-arch.md
- ./dsh-setup.md

---

> **AI 是工具，不是作者。代码的正确性、安全性、可维护性——最终责任在人。**
> **MusCat 生态的每一行代码，都经得起审视。🐱**