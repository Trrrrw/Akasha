# Akasha

Akasha 是一个米哈游游戏信息聚合服务，包含 HTTP API、MCP 工具接口和定时爬虫。后端负责对外提供查询接口，爬虫负责定时同步官网、米游社等来源的数据，数据库使用 PostgreSQL。

## 项目结构

- `backend`：Axum 后端服务，默认监听 `0.0.0.0:7040`，内置 Scalar API 文档和 MCP 路由。
- `crawler`：爬虫命令行和定时任务入口。
- `crates/db`：SeaORM 数据库实体和连接池。
- `crates/crawler-*`：具体爬虫实现 crate。
- `crates/axum-mcp*`：把后端接口暴露为 MCP tools 的辅助 crate。

## Docker 部署

先构建后端和爬虫镜像：

```bash
docker build --target akasha -t akasha:latest .
docker build --target crawler -t akasha-crawler:latest .
```

复制示例 Compose 文件，并填写 PostgreSQL 用户、密码和米游社 Cookie：

```bash
cp docker-compose.example.yml docker-compose.yml
```

需要修改 `docker-compose.yml` 中这些值：

```yaml
POSTGRES_USER: "fill-postgres-user"
POSTGRES_PASSWORD: "fill-postgres-password"
MIYOUSHE_COOKIE: "fill-miyoushe-cookie"
```

启动服务：

```bash
docker compose up -d
```

查看日志：

```bash
docker logs -f Akasha
docker logs -f Akasha-crawler
```

访问服务：

```text
http://localhost:7040
```

## 环境变量

后端和爬虫都需要 PostgreSQL 连接配置：

| 变量                | 默认值      | 说明                    |
| ------------------- | ----------- | ----------------------- |
| `POSTGRES_HOST`     | `127.0.0.1` | PostgreSQL 地址         |
| `POSTGRES_PORT`     | `5432`      | PostgreSQL 端口         |
| `POSTGRES_USER`     | 无          | PostgreSQL 用户名，必填 |
| `POSTGRES_PASSWORD` | 无          | PostgreSQL 密码，必填   |
| `POSTGRES_DB`       | `Akasha`    | PostgreSQL 数据库名     |

爬虫额外需要：

| 变量              | 说明                                  |
| ----------------- | ------------------------------------- |
| `MIYOUSHE_COOKIE` | 米游社请求详情接口使用的 Cookie，必填 |

## 本地开发

启动后端：

```bash
cargo run -p Akasha
```

运行单个爬虫任务：

```bash
cargo run -p crawler -- run miyoushe
cargo run -p crawler -- run official_site
```

启动定时爬虫：

```bash
cargo run -p crawler -- serve
```

指定 cron 表达式：

```bash
cargo run -p crawler -- serve "0 */30 * * * *"
```
