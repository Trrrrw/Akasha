# Akasha Backend 开发指南

## 适用范围

修改 `crates/backend`、HTTP 路由、DTO、OpenAPI、静态资源、Calendar、新闻或 game-data HTTP 层前阅读本文件。

## HTTP 交付层

endpoint 只负责：

- Path、Query、Header、Body 解析与 HTTP 输入校验
- 鉴权和限流
- application service 调用
- 应用错误到 HTTP 状态码的映射
- DTO 转换

数据库查询、跨表业务逻辑和上游协议解析分别放在 repository、application service 和对应客户端 crate 中。endpoint 不直接操作 SeaORM Entity。

## 路由与 OpenAPI

- 公共接口使用 `OpenApiRouter` 和 `utoipa`
- 每个公共操作提供简洁清晰的中文 `summary` 和 `description`
- 公共响应字段变化时同步更新 DTO、schema、示例和 OpenAPI 测试
- `/api/v1/admin/**` 使用普通 `Router`，不进入公开 OpenAPI 或 Scalar
- path 中的资源标识是唯一权威来源，query 和 body 不重复携带该标识
- 资源式写入优先使用对应 HTTP method；只有具有明确命令语义的操作才使用动作式 `POST`

统一状态语义：

- 资源不存在或当前能力不支持该资源：`404`
- path、query、body、枚举或范围非法：`400`
- Bearer Token 缺失或错误：`401`
- 并发或业务冲突：`409`
- 限流：`429`
- 未预期后端错误：`500`

## 游戏数据

公开游戏数据路由为：

```text
/api/v1/games/{game_id}/data/{collection}
```

角色使用 `character` collection。公开响应表示当前数据，不暴露采集来源、来源版本、内部状态或 `raw_data`。

外部静态资源先由可信写入客户端镜像到 backend。公开接口只返回 `ASSET_BASE_URL` 下的自有 URL，不透传外部资源地址。

## 新闻与 Calendar

- 新闻详情、来源、标签、系列和媒体能力作为新闻资源或其子资源组织
- 新闻管理写入保留审计上下文；标签和角色关系批量替换保持事务一致性
- Calendar 的内容类型使用明确且独立的路由，并同时提供项目支持的 JSON 与 ICS 表示
- 未知游戏或不受支持的 Calendar 资源返回 `404`，查询参数非法返回 `400`

## RSS、ICS、NFO 与视频

RSS、ICS 和 NFO 是正式内容输出格式。生成绝对地址时以 `ASSET_BASE_URL` 为部署基准，不根据请求 Host 推断公网地址。

导出内容使用数据库中已清洗的字段并保留有意义的换行，避免再次执行会改变语义的文本清洗。

米游社视频播放地址通过视频服务刷新或复用临时签名；协议逻辑位于 `crates/mys` 或对应服务层。

## 限流与客户端 IP

高成本公共接口保留限流并在 `429` 响应中提供合理的 `Retry-After`。真实客户端 IP 仅从配置的 trusted proxy 链路解析，不能直接信任任意 `X-Forwarded-For`。

## Rust 文件与注释

- 函数和变量使用 `snake_case`，类型和枚举使用 `PascalCase`，常量使用 `SCREAMING_SNAKE_CASE`
- 非平凡函数、公共接口、配置对象和状态类型使用简洁中文 doc comment
- 注释解释职责、约束或原因，不复述代码字面行为
- 模块定义在同名 `.rs` 文件，子模块放在同名目录，不使用 `mod.rs`
