# 后端审查记录 2026-08-10

## 审查范围

- `crates/backend` 的 HTTP 交付层、认证、配置和外部视频签名服务
- `crates/application` 的应用服务、普通 Rust 模型和 Repository 端口
- `crates/db` 的 SeaORM Entity、查询、写入和审计日志
- `crates/mys` 的米游社 API 封装
- `worker/src/news` 的同步、手动重处理、视频时长和标签解析
- Docker、Compose、Windows 启动与恢复脚本

## 当前架构结论

当前依赖方向合理：`backend -> application <- db`，`backend -> mys`。HTTP endpoint 不再直接访问数据库，应用层返回普通 Rust 结构体，数据库细节集中在 `db` crate。现阶段没有必要继续增加同类业务 crate

`ApplicationRepository` 目前较大但仍可维护。只有当某个领域继续明显增长时，再按 `AuthRepository`、`NewsRepository` 等能力拆分端口，暂时不需要为了文件数量提前拆分

## 本轮已直接修复

- 认证密钥和 worker token 至少要求 32 字节
- JWT 显式限定 HS256，并移除未使用且存在安全公告的 RSA 实现依赖
- OAuth state 与 worker token 使用固定时间比较
- HTTPS 部署的认证 Cookie 添加 `Secure`，并保留本地 HTTP 开发能力
- refresh token 轮换时锁定旧记录，避免同一 token 并发换取多个新 token
- 请求日志不再记录可能包含 OAuth code、state 等敏感值的查询字符串
- 审计日志记录连接来源 IP，数据库密码配置不再实现 `Debug`
- 共享 HTTP 客户端增加连接和总请求超时
- 公开新闻类型、角色性别和生日月份在进入数据库前校验
- 新闻、角色、游戏、来源和标签查询增加稳定的次级排序
- RSS 媒体地址进行 HTML 属性转义
- 视频详情和 RSS 按路由及客户端 IP 使用独立令牌桶限流，可信代理链经过显式配置后才会参与客户端地址解析，超限返回 `429` 和 `Retry-After`
- 单次 RSS 请求限制米游社签名刷新预算，缓存命中不消耗预算
- 前端移除后将 `/` 和 OAuth 成功回跳指向 `/scalar`
- 原神官网标签规则补充常见视频类型，并修复泛化 PV 与具体 PV 重复命中
- 米游社与官网手动重处理任务使用实际解析器版本
- Rust 函数均补有中文文档注释，长流程保留分阶段中文注释

## 后续高优先级

### 1. 认证失败语义与 refresh token 重放检测

Repository 目前只有统一基础设施错误，refresh token 不存在、过期、已撤销或用户禁用都会最终映射为 HTTP 500。应给认证端口增加明确结果类型，将凭据无效映射为 401，并在已轮换 token 再次出现时撤销同一 token family

### 2. worker 凭据按身份和权限拆分

所有 worker 当前共享一个静态 `WORKER_TOKEN`，任一 worker 泄露后可调用全部内部写入接口，也可以自行填写审计中的 worker ID。应改为可轮换的独立凭据，并把来源、游戏和写入能力绑定到凭据

## 后续中优先级

### 3. 米游社视频签名 single-flight

相同新闻在缓存未命中时的并发请求会重复访问上游。应在 `(game_id, news_id)` 粒度增加 single-flight，同时为缓存增加容量和过期清理

### 4. 标签与游戏摘要查询批量化

标签列表当前会为每个标签分别读取最新文章和视频，游戏列表也会逐个读取最近新闻。标签数量增长后查询次数会线性增加，应使用数据库窗口查询或少量批量查询一次取回每组最新记录

### 5. 应用层写入校验

新闻和标签写入用例仍以透传为主。应在 `application` 中统一校验非空标识、URL、视频时长、重复标签、重复新闻更新和批量上限，避免可信调用方的错误输入退化为数据库 500

### 6. 分页方式和偏移上限

公开列表允许任意大 offset，数据库需要扫描并跳过大量记录。数据继续增长后应改为基于发布时间和 ID 的游标分页，过渡期至少限制最大 offset

### 7. 数据库运行配置

连接池固定为最少 5、最多 100，且每次后端启动都会执行 Entity schema sync 和补充索引。这要求运行账号持有 DDL 权限并可能产生启动锁竞争。应将连接池参数环境化，并将 schema sync 独立为部署阶段命令

### 8. SeaORM 稳定版迁移

当前使用 `2.0.0-rc.40`。升级稳定版需要适配多对多关系生成方式，不能只改版本号。该迁移同时可以消除 `proc-macro-error2` 的未来兼容警告，应作为独立重构处理

### 9. 会话和管理员权限生命周期

refresh token 表没有定期清理和单用户会话上限。`ADMIN_GITHUB_ID` 只会授予管理员组，配置改变后不会自动撤销旧管理员。需要明确会话保留策略和管理员权限的权威来源

## 依赖安全结果

- 已更新存在修复版本的 `anyhow` 和 `event-listener`
- 已通过 SeaORM feature 收敛移除实际构建中的 `rkyv`
- 已将 `jsonwebtoken` 切换到项目本来就在使用的 AWS-LC 后端，实际依赖树中不再包含 `rsa`
- `cargo audit` 仍会从 lockfile 报告未启用的 `rkyv`，但 `cargo tree --workspace --target all -i rkyv` 确认实际构建没有该依赖
- 实际构建仍包含 `paste` 和 `proc-macro-error2` 的停止维护警告，分别来自 OpenAPI 集成和 SeaORM RC，未发现可直接替换且不引入架构迁移的版本

## 验证结果

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`：24 项 Rust 测试通过
- `bun run check`
- `bun test src/news`：22 项 worker 测试通过
- `bunx prettier --check "src/**/*.ts"`
- Windows PowerShell 脚本语法检查通过
