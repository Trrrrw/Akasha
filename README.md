# Akasha

Akasha 是一个使用 Rust 和 PostgreSQL 实现的游戏信息聚合后端。数据通过受保护的管理接口写入，公开仓库不包含具体数据采集实现

## 项目结构

- `crates/backend`：Axum HTTP 交付层、认证、OpenAPI 文档及应用装配
- `crates/application`：与 HTTP、SeaORM 无关的应用服务、数据模型和 repository 端口
- `crates/db`：SeaORM Entity、PostgreSQL repository 和 schema 同步
- `crates/mys`：米游社视频临时签名客户端

## 环境配置

复制示例配置后填写实际值：

```bash
cp .env.example .env
```

后端使用以下主要变量：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
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

构建并启动 PostgreSQL 与后端：

```bash
docker compose up --build -d
```

查看日志：

```bash
docker logs -f akasha-backend
```

## 后端发布镜像

发布 Dockerfile 只包含后端：

```bash
docker build --target akasha -t akasha-backend:latest .
```
