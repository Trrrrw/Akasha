# Akasha

Akasha 是一个使用 Rust 和 SQLite 实现的游戏信息聚合后端。公开查询接口无需认证，数据通过受保护的管理接口写入，公开仓库不包含具体数据采集实现

## 项目结构

- `crates/backend`：Axum HTTP 交付层、数据写入鉴权、OpenAPI 文档及应用装配
- `crates/application`：与 HTTP、SeaORM 无关的应用服务、数据模型和 repository 端口
- `crates/db`：SeaORM Entity、SQLite repository 和 schema 同步
- `crates/mys`：米游社视频临时签名客户端

`assets/games/` 中的内置游戏封面和图标同时提供 WebP 与 AVIF。公开 API 默认返回 WebP，原有 AVIF 地址继续可用；新增或更新这些图片时需同步维护两种格式及图标尺寸变体。动态游戏数据资源按其实际格式返回，不替换扩展名

## Agent Skills

复制下面的提示词并发送给你的 Agent：

```text
请访问以下安装说明，并根据说明安装其中列出的 Agent Skills：
https://github.com/Trrrrw/Akasha/blob/main/skills/README.md
```

## 环境配置

后端配置文件位于 `config/backend.toml`，复制示例后填写实际值：

```bash
cp config/backend.toml.example config/backend.toml
```

`config/backend.toml` 已被 Git 忽略，不会提交实际配置和认证密钥。Docker Compose 会将整个 `config/` 目录只读挂载到容器的 `/app/config`。后端默认读取 `config/backend.toml`，只有需要使用其他位置时才通过 `AKASHA_CONFIG_FILE` 覆盖默认路径

环境变量仍可临时覆盖后端配置文件。配置文件中的字段按功能分组，名称与下表的环境变量一一对应

| 配置文件字段 | 对应环境变量 | 说明 |
| --- | --- | --- |
| `[server].log_level` | `LOG_LEVEL` | 日志级别过滤器 |
| `[server].bind_addr` | `BIND_ADDR` | HTTP 监听地址 |
| `[server].asset_base_url` | `ASSET_BASE_URL` | 对外公开的后端根地址 |
| `[server].game_data_asset_dir` | `GAME_DATA_ASSET_DIR` | 游戏数据资源持久化目录 |
| `[database].path` | `SQLITE_PATH` | SQLite 数据库文件路径 |
| `[security].data_write_token` | `DATA_WRITE_TOKEN` | 受保护数据写入接口的 Bearer 凭据 |
| `[mys].cookie` | `MIYOUSHE_COOKIE` | 米游社视频签名 Cookie |
| `[rate_limits]` | `RATE_LIMIT_*`、`NEWS_*` | 公开接口限流配置 |
| `[audit].retention_days` | `AUDIT_LOG_RETENTION_DAYS` | 审计日志保留天数 |

后端支持的环境变量覆盖项如下：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `LOG_LEVEL` | `info` | 日志级别过滤器 |
| `BIND_ADDR` | `0.0.0.0:7040` | HTTP 监听地址 |
| `ASSET_BASE_URL` | 无 | 对外可访问的后端根地址，例如 `https://example.com` |
| `GAME_DATA_ASSET_DIR` | `data/game-assets` | 游戏数据资源持久化目录 |
| `SQLITE_PATH` | `data/akasha.sqlite` | SQLite 数据库文件路径，相对路径以进程工作目录为基准 |
| `DATA_WRITE_TOKEN` | 无 | 管理写入与 worker 协调接口共用的 Bearer 凭据，至少 32 字节 |
| `MIYOUSHE_COOKIE` | 无 | 获取米游社视频临时签名所需的 Cookie |
| `RATE_LIMIT_TRUSTED_PROXY_IPS` | 空 | 可提供 `X-Forwarded-For` 的可信反向代理 IP，多个值用逗号分隔 |
| `NEWS_VIDEO_RATE_LIMIT_PER_MINUTE` | `30` | 视频详情接口每分钟为每个客户端 IP 补充的请求令牌数 |
| `NEWS_VIDEO_RATE_LIMIT_BURST` | `10` | 视频详情接口允许每个客户端 IP 突发使用的令牌数 |
| `NEWS_RSS_RATE_LIMIT_PER_MINUTE` | `12` | RSS 接口每分钟为每个客户端 IP 补充的请求令牌数 |
| `NEWS_RSS_RATE_LIMIT_BURST` | `3` | RSS 接口允许每个客户端 IP 突发使用的令牌数 |
| `NEWS_RSS_MYS_REFRESH_LIMIT` | `10` | 单次 RSS 请求最多触发的米游社视频签名刷新数，缓存命中不计入，设为 `0` 可禁用刷新 |
| `AUDIT_LOG_RETENTION_DAYS` | `180` | 审计日志保留天数，后端每天清理超过期限的记录 |

公开读取接口不需要凭据。`/api/v1/admin/...` 下的管理写入和 worker 协调接口统一要求：

```http
Authorization: Bearer <DATA_WRITE_TOKEN>
```

反向代理部署时，应仅将实际代理的直连 IP 写入 `RATE_LIMIT_TRUSTED_PROXY_IPS`。未配置或请求并非来自可信代理时，后端会忽略 `X-Forwarded-For`，避免客户端伪造地址绕过限流

可以生成一个随机数据写入凭据：

```bash
openssl rand -base64 32
```

## 本地开发

开发、PR、CI、版本与发行约定见 [开发与发行流程](CONTRIBUTING.md)

启动后端：

```bash
cargo run
```

运行检查：

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

后端启动后可访问：

- 健康检查：`http://localhost:7040/healthz`
- Scalar API 文档：`http://localhost:7040/scalar`

API 中表示确定时间点的 JSON 字段统一使用精确到秒的 UTC RFC 3339 格式，例如 `2026-07-01T03:00:00Z`。RSS 使用等价的 UTC RFC 2822 时间，ICS 使用 UTC `Z` 时间。生日和发布日期等不含时区语义的纯日期仍使用 `YYYY-MM-DD`

## Docker 运行

先创建后端使用的实际配置文件，并填写部署参数：

```bash
cp config/backend.toml.example config/backend.toml
```

SQLite 文件会保存到宿主机的 `data` 目录，后端配置通过只读配置卷和可写数据卷挂载到容器。修改配置后重建或重启后端容器即可生效

该部署方式假定同一个 SQLite 文件只由一个后端实例使用，worker 通过后端 API 写入数据。备份时应先停止后端，再备份整个 `data` 目录

拉取公开镜像并启动后端：

```bash
docker compose up -d
```

Compose 默认使用 GitHub Container Registry。国内网络需要改用阿里云镜像时，交换 `akasha-backend.image` 相邻两行的注释即可

查看日志：

```bash
docker logs -f akasha-backend
```

## 后端发布镜像

需要从源码自行构建后端镜像时：

```bash
docker build --target akasha -t akasha-backend:latest .
```

## 成就目录

三款游戏使用 `ys`、`sr`、`zzz`，公开接口包括：

- `GET /api/v1/games/{game_id}/achievements`：支持 `q`、`group_id`、`hidden`、`limit` 和 `offset`，每页最多 100 条
- `GET /api/v1/games/{game_id}/achievement-groups`：分类、排序与每类成就数量

成就定义包含名称、描述、分类、排序和已确认的奖励等字段。分类直接由同一集合聚合，集合增量更新与审计日志在单个事务内完成。未知的隐藏状态、奖励或前置关系不推测填充

Akasha 只保存网站需要的规范化游戏定义和内容指纹，不保存完整游戏来源文件，也不维护玩家完成状态、备注或同步账号。个人状态由客户端保管

管理端游戏数据同步状态接口为 `/api/v1/admin/games/{game_id}/data/{collection}/sync-state`，返回 ID 和 `source_hash`。游戏数据写入不再接受 `raw_data`，新闻原始数据接口不受影响。客户端应与后端同步适配这一管理协议变更

既有数据库的游戏数据 `raw_data` 列可在停写、备份后一次性移除；服务启动不会执行这一清理。旧的成就分类嵌套记录需改为规范化的逐成就记录后再启用成就查询
