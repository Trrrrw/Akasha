# Akasha 项目协作说明

## 项目定位

Akasha 是使用 Rust、Axum、SeaORM 和 SQLite 实现的游戏信息聚合后端，提供公开查询 API、公开资源和受保护的数据写入接口。

项目不提供终端用户账号体系，也不维护成就完成状态、收藏、偏好等个人数据。个人状态由客户端本地保存；跨设备同步由客户端接入外部同步服务。

公开仓库只包含后端，不包含具体数据采集实现或前端源码。站点根路径重定向到 `/scalar`。

## 仓库边界

- 根目录是公开的 Akasha 后端 Git 仓库
- `worker/`（如果存在）是本地单独克隆的私有 Git 仓库；仅在用户明确要求修改或运行 worker 时进入该目录
- 根仓库与 `worker/` 分别检查 Git 状态、提交和验证，不能跨仓库暂存文件
- `justfile`、`docker-compose.dev.yml` 和 `worker/` 是本地私有开发设施，不属于公开仓库
- 审查记录、临时说明和诊断输出放在 `.temp/`
- 公开文档、Docker 文件和发布产物不得包含私有 worker、数据适配器、凭据或其内部开发方式

## 客户端联动

- 修改公开接口路径、参数、响应字段、枚举或数据语义时，检查受影响的客户端调用，协调必要的适配与验证
- 后端内部修改若不影响客户端契约，无需为了同步而修改客户端代码
- 跨仓库修改前先读取目标仓库的协作指令，各仓库分别检查状态、验证和提交
- 破坏性接口变更需协调后端与客户端的发布顺序，避免上线期间出现不兼容
- 完成时说明受影响客户端、适配与验证结果，以及无需修改或尚未验证的范围
- 开始任务时，如存在 `.agents/workspace.local.md`，先读取其中的本地仓库映射与联动约定；文件不存在时正常继续，不假定其他协作者具有相同目录布局
- 本地说明仅补充工作区安排，不覆盖本文件的架构、仓库边界和安全约束；不得将其内容复制到公开文档或发布产物

## 核心架构

Rust workspace 的职责划分：

- `crates/backend`：Axum HTTP 交付层、DTO、鉴权、限流、OpenAPI、静态资源和应用装配
- `crates/application`：独立于 Axum、SeaORM 的应用服务、业务模型和 repository 端口
- `crates/db`：SeaORM Entity、SQLite repository、schema 同步和必需种子数据
- `crates/mys`：米游社视频地址和临时签名客户端

依赖与职责约束：

- endpoint 处理 HTTP 语义、输入校验、鉴权和响应映射，并调用 application service
- application 使用普通 Rust 类型表达业务，不依赖 Axum、SeaORM 或数据库 Entity
- db 实现 application 定义的 repository，不向 application 或 backend 暴露 SeaORM Model
- 跨表写入、关系替换和对应审计日志在同一数据库事务中完成
- 非 HTTP 的上游协议逻辑放入对应客户端 crate
- 架构替换应完整移除被取代的入口、类型和目录；兼容层只用于已确认的外部兼容需求
- 新增账号、权限、session、刷新 token 或多租户能力需要明确的当前需求

## Cargo 依赖管理

- 工作区内部 crate 的路径统一声明在根 `[workspace.dependencies]`，成员通过 `{ workspace = true }` 引用
- 被两个及以上成员直接使用的第三方依赖在根目录统一版本；只服务于单个成员的依赖留在该成员的 `Cargo.toml`
- 根依赖只声明共享的最小 feature 集；成员在自身依赖项上追加专属 feature
- 仅用于测试、示例或构建的依赖放入对应成员的 `[dev-dependencies]` 或 `[build-dependencies]`
- 每个直接使用某个 crate 的成员都显式声明该依赖，不依赖传递依赖
- 增删依赖时同步更新 `Cargo.lock`，并检查 workspace 全目标和全 feature 构建

## 鉴权与 API

受保护的数据写入接口统一使用：

```http
Authorization: Bearer <DATA_WRITE_TOKEN>
```

`DATA_WRITE_TOKEN` 是可信客户端的数据写入凭据，不代表终端用户身份。`worker_id`、`run_id` 只用于任务协调和审计。

- token 必须是满足配置最小长度的高熵随机值，并使用 constant-time 比较
- 公共接口使用 `OpenApiRouter` 和 `utoipa`，并出现在 `/scalar`
- `/api/v1/admin/**` 使用普通 `Router`，不进入公开 OpenAPI；管理接口统一验证 `DATA_WRITE_TOKEN`
- 管理写入保留输入限制和审计上下文
- path 已提供资源标识时，query 和 body 不再重复该标识
- 不存在或不受支持的资源返回 `404`；参数格式、枚举、范围或请求体错误返回 `400`
- backend 与 worker 的破坏性协议变更作为同一发布单元验证

## 配置与敏感信息

默认配置文件是 `config/backend.toml`；`AKASHA_CONFIG_FILE` 仅覆盖配置文件路径，环境变量可覆盖配置值。

`config/backend.toml` 和根目录 `.env` 含敏感值并被 Git 忽略。命令输出、日志、测试快照和最终回复不得包含密码、Cookie、token、私钥或完整配置文件。

增加后端配置项时同步更新配置结构、环境变量回退、`config/backend.toml.example` 和 README。

## 修改与发布

- 开发、合并和发行遵循 `CONTRIBUTING.md`；项目发行以版本源码、发行产物和 Release 可获取为完成条件，服务器部署及用户数据操作属于独立使用流程
- 修改前后检查 Git 状态，保留用户的无关改动
- 只暂存用户确认范围内的文件；提交前检查工作区和暂存区差异
- `main` 始终保持可发布；每个开发任务从最新 `main` 创建一个短生命周期分支，通过 Pull Request 合入，包括流程文档修改
- 默认先完成修改和适用的本地验证，提供运行或复现入口、验收步骤、预期结果及未验证范围，然后停在本地等待用户验收；验收确认前不提交、不推送、不创建或更新 PR，不创建远端标签或触发远端工作流
- 用户反馈在本地继续修改和验证；确认结果并要求继续后，在授权范围内连续完成提交、PR、CI、必要 review、合并及发行，无需逐步确认；后续改变已验收行为或兼容性时重新提供本地验收
- 每个 Pull Request 都由实现者或代理完成基础核对；完整 CI 必须通过，高风险或存在疑问时增加有明确目标的专门 review，不要求常规任务重复人工审批
- 用户验收通过并授权继续合并后，在基础核对、CI 和必要 review 完成时允许代理合并，无需逐次确认；用户指定只修改、只提 PR、等待确认或暂停时遵循指定终点
- CI 等待必须遵循 `CONTRIBUTING.md` 的有限预算：未出现或排队 5 分钟、同一步骤无可确认进展 10 分钟、单个 run 等待或运行 30 分钟时检查并报告，停止轮询并请求用户决定；相同故障自动修复或重试一轮后仍失败时同样停止，不因“继续上线”无限等待
- PR 确认合并后立即清理本次任务的远端和本地功能分支，核对最终 head SHA、未保存修改及其他任务占用，验证删除结果；无法安全清理时回报具体原因和待办，不等发行或部署结束
- 继续已有 PR 时使用其 head branch
- 根仓库与 `worker/` 分别从各自 `main` 创建分支、提交、推送和创建 Pull Request，不跨仓库混用分支或提交
- backend 与 worker 有破坏性 API 联动时，两边完成 CI、本地验证和兼容性 review 后再合并，并通过兼容过渡或明确版本要求保证各自 `main` 可发布
- 合并不自动要求正式发行；发行时机和版本由用户指定或在任务目标中明确授权，标签发行规则及当前自动化差异见 `CONTRIBUTING.md`
- 不因开发、合并或发行请求自动连接服务器、部署容器、回填上游或重处理用户数据

## 故障修复原则

- 错误、告警或异常日志必须先定位并修复根因，不得仅通过过滤、降级日志级别或吞掉错误来制造正常表象
- 只有确认行为符合预期、底层机制实际有效且无需人工处置时，才可降低重复日志噪声，并保留可验证的状态或指标
- 容器权限、cgroup、文件系统和网络问题优先采用运行时支持的最小权限配置；不得为消除告警无边界开放宿主资源

## 详细规则

按任务范围阅读：

- 开发流程、PR、按需 Review、版本与发行：`CONTRIBUTING.md`
- Backend、HTTP、OpenAPI、Calendar、资源接口：`.agents/backend.md`
- SeaORM、SQLite、schema、索引、事务：`.agents/database.md`
- 本地运行、测试、Smoke Test、故障诊断：`.agents/local-development.md`
- 对外发布 Skills 的结构、边界、同步和验证：`.agents/skills.md`

根文件中的架构、仓库边界和安全约束优先。
