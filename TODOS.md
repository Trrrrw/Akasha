# 后端接口设计待办

记录于 2026-11-13。以下来自当前实现、OpenAPI、客户端调用检查及隔离内存数据库验证，尚未实施；建议方案需在实现前确定具体契约。

## 优先修正

- [ ] **[P2] 统一错误响应与 OpenAPI 错误声明**
  - 已验证：新闻缺少 `source` 返回 `400 + text/plain`；Worker acquire 请求缺字段返回 `422 + text/plain`；游戏数据写入请求缺字段返回 `400 + JSON`；未知 API 路由返回空体 `404`
  - 统一参数提取、JSON 解析和 API 路由错误的状态码与 `{ "message": "..." }` 响应；同步补全公开 OpenAPI 中实际可能返回的错误
  - 验收：缺失参数、类型错误、非法 JSON、缺失字段和未知 API 路由均符合约定，客户端可统一读取错误
  - 位置：[错误映射](crates/backend/src/http/error.rs)、[新闻请求](crates/backend/src/features/news/admin.rs)、[游戏数据请求](crates/backend/src/features/game_data/admin.rs)、[公开接口声明](crates/backend/src/features/news/endpoints.rs)

- [ ] **[P2] 将请求自身的冲突映射为客户端错误**
  - 已验证：游戏数据 PATCH 请求将同一 ID 同时放入 `items` 和 `removed_ids` 时返回 `500`，会触发 Worker 的服务器错误重试
  - 在应用层验证此类跨字段约束，返回 `400`；一并检查重复 ID 等输入错误，保留真实数据库故障的 `500` 语义
  - 验收：非法请求不改变数据，返回明确的客户端错误；正常写入仍保持事务一致性
  - 位置：[游戏数据应用服务](crates/application/src/game_data.rs)、[数据库写入](crates/db/src/repositories/game_data.rs)、[错误映射](crates/backend/src/http/error.rs)

- [ ] **[P2] 统一游标分页的 `total` 与结束条件**
  - 已验证：同样有三条 ID 为 `1`、`2`、`3` 的记录，传入 `after_id=1&limit=1` 后，新闻原始数据接口返回 `total: 2`，游戏数据同步状态接口返回 `total: 3`
  - 确定共同契约：保留 `total` 时表示不受游标影响的筛选总数；若同步任务不需要总数，可改为只返回 `items` 和可靠的 `next_cursor`
  - 验收：空集合、最后一页和带筛选的多页读取具有一致含义；同步适配 Worker 的分页完整性检查
  - 位置：[新闻分页](crates/db/src/repositories/news/queries.rs)、[游戏数据分页](crates/db/src/repositories/game_data.rs)、[新闻分页响应](crates/backend/src/features/news/admin.rs)、[同步状态响应](crates/backend/src/features/game_data/admin.rs)

- [ ] **[P2] 统一非法分页参数的处理**
  - 已验证：新闻 `limit=0` 被静默改为 `1` 并返回 `200`；日历同样的参数返回 `400`；成就接口也显式拒绝越界值
  - 缺省参数使用默认值，显式传入的非法值返回 `400`；检查新闻列表、RSS、游戏数据及管理查询中的 `clamp` 行为
  - 验收：`0`、超过上限、负数及非数字输入均按约定处理；更新 OpenAPI 中的默认值和范围说明
  - 位置：[新闻参数](crates/backend/src/features/news/query.rs)、[游戏数据参数](crates/backend/src/features/game_data/query.rs)、[成就参数](crates/backend/src/features/game_data/achievements.rs)、[日历参数](crates/backend/src/features/calendar/events.rs)

## 设计调整

- [ ] **[P3] 统一管理写入的 PUT / PATCH 语义**
  - 当前日历和版本接口使用 `PUT`，再通过 `replace` 字段切换合并与整体替换；游戏数据已经使用 `PUT` / `PATCH` 区分
  - 建议沿用游戏数据的方式：`PUT` 提交完整集合，`PATCH` 提交增量，移除 `replace`；Worker acquire、heartbeat、complete 等命令继续使用 `POST`
  - 验收：覆盖新增、更新、删除、重复请求和空集合语义；后端与 Worker 联合验证并协调部署期间的兼容性
  - 位置：[日历路由](crates/backend/src/features/calendar.rs)、[日历写入](crates/backend/src/features/calendar/admin.rs)、[版本路由](crates/backend/src/features/games.rs)、[版本写入](crates/backend/src/features/games/version_admin.rs)、[游戏数据路由](crates/backend/src/features/game_data.rs)
  - 参考：[RFC 9110 PUT 语义](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.3.4)

- [ ] **[P3] 明确公开游戏数据的业务模型**
  - 当前 `/data/{collection}` 公开 `summary`、`detail`、`assets` 等任意 JSON，查询参数混合三款游戏的角色字段，OpenAPI 无法完整表达实际数据结构
  - 参考已有成就接口，为角色查询设计明确的公共模型；是否保留通用数据浏览入口，根据现有调用需求决定
  - 验收：明确各游戏字段、可空性和有效筛选条件；检查受影响前端与 Worker，避免重复维护两套含义不清的公共契约
  - 位置：[通用响应](crates/backend/src/features/game_data/dto.rs)、[通用查询](crates/backend/src/features/game_data/query.rs)、[成就接口](crates/backend/src/features/game_data/achievements.rs)

优先完成前四项契约修正，再推进管理写入协议及公共模型调整。保留公开查询与管理写入的分离。
