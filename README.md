# Akasha

Akasha 是一个使用 Rust 和 PostgreSQL 实现的游戏信息聚合后端。数据通过受保护的管理接口写入，公开仓库不包含具体数据采集实现

## 项目结构

- `crates/backend`：Axum HTTP 交付层、认证、OpenAPI 文档及应用装配
- `crates/application`：与 HTTP、SeaORM 无关的应用服务、数据模型和 repository 端口
- `crates/db`：SeaORM Entity、PostgreSQL repository 和 schema 同步
- `crates/mys`：米游社视频临时签名客户端

## 环境配置

后端配置文件位于 `config/backend.toml`，复制示例后填写实际值：

```bash
cp config/backend.toml.example config/backend.toml
```

`config/backend.toml` 已被 Git 忽略，不会提交数据库密码和认证密钥。Docker Compose 会将整个 `config/` 目录只读挂载到容器的 `/app/config`。后端默认读取 `config/backend.toml`，只有需要使用其他位置时才通过 `AKASHA_CONFIG_FILE` 覆盖默认路径

环境变量仍可临时覆盖后端配置文件，但 `.env.example` 只保留 PostgreSQL 容器初始化所需的变量，不再重复列出后端配置。配置文件中的字段按功能分组，名称与下表的环境变量一一对应

| 配置文件字段 | 对应环境变量 | 说明 |
| --- | --- | --- |
| `[server].log_level` | `LOG_LEVEL` | 日志级别过滤器 |
| `[server].bind_addr` | `BIND_ADDR` | HTTP 监听地址 |
| `[server].asset_base_url` | `ASSET_BASE_URL` | 对外公开的后端根地址 |
| `[database]` | `POSTGRES_*` | PostgreSQL 连接配置 |
| `[auth]` | `JWT_SECRET`、`TOKEN_HASH_SECRET` | 应用 token 密钥 |
| `[github]` | `GITHUB_*`、`ADMIN_GITHUB_ID` | GitHub OAuth 配置 |
| `[worker].token` | `WORKER_TOKEN` | 内部 worker 凭据 |
| `[mys].cookie` | `MIYOUSHE_COOKIE` | 米游社视频签名 Cookie |
| `[rate_limits]` | `RATE_LIMIT_*`、`NEWS_*` | 公开接口限流配置 |
| `[audit].retention_days` | `AUDIT_LOG_RETENTION_DAYS` | 审计日志保留天数 |

后端支持的环境变量覆盖项如下：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `LOG_LEVEL` | `info` | 日志级别过滤器 |
| `BIND_ADDR` | `0.0.0.0:7040` | HTTP 监听地址 |
| `ASSET_BASE_URL` | 无 | 对外可访问的后端根地址，例如 `https://example.com` |
| `POSTGRES_HOST` | `127.0.0.1` | PostgreSQL 地址 |
| `POSTGRES_PORT` | `5432` | PostgreSQL 端口 |
| `POSTGRES_USER` | 无 | PostgreSQL 用户名 |
| `POSTGRES_PASSWORD` | 无 | PostgreSQL 密码 |
| `POSTGRES_DB` | `Akasha` | PostgreSQL 数据库名 |
| `JWT_SECRET` | 无 | access token 签名密钥，至少 32 字节 |
| `TOKEN_HASH_SECRET` | 无 | 敏感 token 哈希密钥，至少 32 字节 |
| `GITHUB_CLIENT_ID` | 无 | GitHub OAuth 客户端 ID |
| `GITHUB_CLIENT_SECRET` | 无 | GitHub OAuth 客户端密钥 |
| `GITHUB_OAUTH_REDIRECT_URL` | 无 | GitHub OAuth 回调地址 |
| `ADMIN_GITHUB_ID` | 无 | 自动授予管理员权限的 GitHub 用户 ID |
| `WORKER_TOKEN` | 无 | 内部写入接口凭据，至少 32 字节 |
| `MIYOUSHE_COOKIE` | 无 | 获取米游社视频临时签名所需的 Cookie |
| `RATE_LIMIT_TRUSTED_PROXY_IPS` | 空 | 可提供 `X-Forwarded-For` 的可信反向代理 IP，多个值用逗号分隔 |
| `NEWS_VIDEO_RATE_LIMIT_PER_MINUTE` | `30` | 视频详情接口每分钟为每个客户端 IP 补充的请求令牌数 |
| `NEWS_VIDEO_RATE_LIMIT_BURST` | `10` | 视频详情接口允许每个客户端 IP 突发使用的令牌数 |
| `NEWS_RSS_RATE_LIMIT_PER_MINUTE` | `12` | RSS 接口每分钟为每个客户端 IP 补充的请求令牌数 |
| `NEWS_RSS_RATE_LIMIT_BURST` | `3` | RSS 接口允许每个客户端 IP 突发使用的令牌数 |
| `NEWS_RSS_MYS_REFRESH_LIMIT` | `10` | 单次 RSS 请求最多触发的米游社视频签名刷新数，缓存命中不计入，设为 `0` 可禁用刷新 |
| `AUDIT_LOG_RETENTION_DAYS` | `180` | 审计日志保留天数，后端每天清理超过期限的记录 |

反向代理部署时，应仅将实际代理的直连 IP 写入 `RATE_LIMIT_TRUSTED_PROXY_IPS`。未配置或请求并非来自可信代理时，后端会忽略 `X-Forwarded-For`，避免客户端伪造地址绕过限流

可以分别生成三个随机密钥：

```bash
openssl rand -base64 32
```

## 本地开发

Windows：

```powershell
just backend
```

Linux：

```bash
./scripts/linux/start-dev.sh
```

运行检查：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

后端启动后可访问：

- 健康检查：`http://localhost:7040/healthz`
- Scalar API 文档：`http://localhost:7040/scalar`

## Docker 运行

先创建 PostgreSQL 使用的 `.env`，以及后端使用的实际配置文件，并填写部署参数：

```bash
cp .env.example .env
cp config/backend.toml.example config/backend.toml
```

Compose 从 `.env` 读取 PostgreSQL 容器初始化参数，后端配置不再通过 Compose 环境变量传入。`backend.toml` 会通过只读卷挂载到后端容器，修改配置后重建或重启后端容器即可生效

拉取公开镜像并启动 PostgreSQL 与后端：

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
