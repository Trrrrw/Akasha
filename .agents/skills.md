# Akasha Skills 开发指南

## 适用范围

创建或修改根目录 `skills/` 下对外发布的 Agent Skills 前阅读本文件

`skills/` 是公开发布源目录；`.agents/skills/` 是本地 Agent 运行时使用且被 Git 忽略的目录。不要把发布内容放入 `.agents/skills/`，也不要假设根目录 `skills/` 会被本地运行时自动发现

## Bundle 结构

每个 Skill 使用与 frontmatter `name` 相同的 kebab-case 目录名，且必须包含 `SKILL.md`。其余资源仅在实际工作流需要时添加：

```text
skills/<skill-name>/
├─ SKILL.md
├─ agents/openai.yaml  # 可选 UI 元数据
├─ references/         # 可选按需文档
├─ scripts/            # 可选确定性辅助程序
└─ assets/             # 可选输出素材
```

根目录 `skills/README.md` 是本仓库发布入口，用于列出可发布 Skill 和安装边界。不要在每个 bundle 中重复添加 README、安装指南、变更日志或无实际用途的占位目录

新建 Skill 时可以使用当前 Skill 开发工具提供的 initializer，但只创建确有用途的资源目录并删除示例占位内容。不要对现有 bundle 重新初始化

## SKILL.md

Frontmatter 至少包含：

```yaml
---
name: akasha
description: "Describe the capability and when it should be selected."
---
```

- `name` 使用小写字母、数字和连字符，不超过 64 个字符
- `description` 简洁说明能力与触发场景，避免穷举所有功能或吸引无关请求；包含冒号等 YAML 特殊字符时加引号
- 正文只保留会改变 Agent 决策的任务流程、真实约束和参考入口，不复述通用能力或堆积推测性边缘情况
- 详细参数、schema 和较长示例放入 `references/`，并在 `SKILL.md` 中说明何时读取对应文件
- 不复制完整生成文档；Akasha 公共 HTTP 能力以目标部署的 `/openapi.json` 为权威来源
- 不要求 Skill 使用者访问当前仓库源码、私有开发设施或 worker

## UI 元数据

`agents/openai.yaml` 仅在需要 UI 展示信息或调用策略时添加：

- 所有字符串值使用引号
- `short_description` 保持简短且与 Skill 能力一致
- `default_prompt` 使用一句示例提示，并显式包含 `$<skill-name>`
- 自动选择默认开启；只有用户明确要求 explicit-only 时才设置 `allow_implicit_invocation: false`
- 修改现有文件时保留未要求变更的 `policy`、`dependencies` 和其他字段

## References 与 Scripts

References 按实际任务或模式拆分，避免复制手册、重复 `SKILL.md` 或创建没有路由价值的索引层。示例中的标识应来自公开接口或明确标为占位符，不把当前数据库中的偶然值写成永久协议

只有在逻辑会被重复编写，或确定性执行能显著提升可靠性时才添加脚本。公共 API 辅助脚本应：

- 默认只读，并拒绝超出公开 Skill 边界的路径
- 正确处理 URL 编码、重复 query 参数、超时、非 JSON 响应和 HTTP 错误
- 不接受或隐式加载与公开能力无关的凭据
- 使用 stdout 输出结果、stderr 输出诊断，并以非零退出码表示失败
- 在 Windows 和 POSIX 环境可靠处理 UTF-8

新增或修改脚本后必须运行有意义的行为验证，不能只检查源码文字

## Akasha 公开 Skill 边界

公开查询 Skill 只使用无需认证的读取接口，不包含管理写入、worker 协调、私有采集实现、服务器访问方式或凭据。默认公开地址可以写入 Skill，但应允许用户指定自托管或本地地址

若未来需要写入能力，应作为独立范围设计，并在实际变更前取得相应授权。不要把敏感操作自动等同于 explicit-only；调用策略由用户明确选择

## 与后端同步

路由、query、DTO、分页、枚举、时间语义、游戏差异化筛选、限流、媒体 URL 生命周期或默认公开地址变化时，检查相关 Skill 是否需要同步。后端 OpenAPI 测试通过不代表 Skill 文档已经同步

## 验证

根据实际改动选择验证，至少覆盖受影响的行为：

1. 使用 Skill 开发工具的 `quick_validate.py` 或等价检查验证 frontmatter、名称和未完成占位符；Windows 上运行外部验证器时确保启用 UTF-8
2. 检查 `SKILL.md` 能以低成本准确触发，按需 references 均可从入口发现且没有重复内容
3. 运行新增或修改脚本的语法检查、`--help` 和代表性行为测试
4. 公共 API 契约或客户端逻辑变化时，验证 `/openapi.json`、中文与重复参数、JSON 与非 JSON 响应，以及管理路径拒绝
5. 搜索发布目录中的凭据和私有实现信息，并排除临时响应、下载文件和 `__pycache__`
6. 使用 `git status --short --untracked-files=all -- skills` 确认发布范围

复杂或高风险 Skill 在条件允许时使用独立 Agent 进行真实请求的前向测试，只提供 Skill 和完成任务所需的最小材料，不暗示预期答案

只修改 Skill 时不要求运行完整 Rust workspace 门禁；若同时修改后端协议，按 Backend 指南运行对应格式化、检查和测试
