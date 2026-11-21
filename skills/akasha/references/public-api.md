# Akasha 公开 API

默认根地址：`https://akasha.trrw.cn`

- Scalar：`/scalar`
- OpenAPI：`/openapi.json`
- 健康检查：`/healthz`
- API 前缀：`/api/v1`

`/openapi.json` 是当前部署的权威接口规范。本文按 Agent 任务整理稳定用法，不替代运行时 schema

## 游戏 ID

先调用：

```http
GET /api/v1/games
```

角色、成就和 Calendar 当前支持：

| 游戏 | ID |
| --- | --- |
| 原神 | `ys` |
| 崩坏：星穹铁道 | `sr` |
| 绝区零 | `zzz` |

`GET /api/v1/games/{game_id}` 返回指定游戏详情。游戏列表还包含其他新闻支持的游戏，不能据此假定它们具备角色、成就或 Calendar 资料

## 通用响应

列表通常使用：

```json
{
  "total": 2,
  "items": []
}
```

分页通常额外包含：

```json
{
  "total": 120,
  "limit": 20,
  "offset": 0,
  "items": [],
  "meta": null
}
```

分页默认 `limit=20`，最大 `100`。使用稳定的相同筛选条件递增 `offset`

## 游戏本体资料

角色、成就与版本是游戏本体资料。角色接口和响应 schema 按游戏独立，当前已提供原神、星铁、绝区零的目录：

```http
GET /api/v1/games/ys/characters
GET /api/v1/games/sr/characters
GET /api/v1/games/zzz/characters
GET /api/v1/games/{game_id}/achievements
GET /api/v1/games/{game_id}/achievement-groups
GET /api/v1/games/{game_id}/versions
```

角色目录当前返回 `id`、`name`、`icon`、`birthday_month`、`birthday_day`，支持 `limit`（默认 100，1 到 100）与 `offset`，暂无资料返回空列表。头像和生日可为 `null`。各游戏后续字段沿各自角色资源扩展，当前不提供战斗属性、配音等筛选，也不提供通用 `/data/{collection}` 公开读取接口

成就列表支持 `q`、`group_id`、`hidden`、`limit`、`offset`；版本接口一次按开始时间返回完整已保存时间线，区间为 `[start_time, end_time)`，最后一项的结束时间可以为空。接口不维护玩家状态

## 新闻

新闻是统一内容资源，游戏与来源是可组合的筛选条件：

```http
GET /api/v1/news
GET /api/v1/news?game=ys&game=sr&source=mys
GET /api/v1/news/sources?game=ys
GET /api/v1/news/tags?game=ys&source=mys
```

来源目录和标签目录仍按指定游戏发现配置，来源 ID 以实际返回为准。新闻列表支持：

- `game`、`source`：可重复；省略某项即不限制该维度
- `q`：标题查询，支持空格 AND、`|` OR、`-` 排除、引号短语和反斜杠转义
- `search_body=true`：将 `q` 的匹配范围扩展到标题和正文
- `tag`、`character`：可重复，任一匹配，最多 32 个；角色 ID 在所属游戏上下文中解释
- `untagged`：包含无标签新闻
- `news_type`：`article` 或 `video`
- `published_from`、`published_to`：包含当日，格式 `YYYY-MM-DD`
- `limit`：默认 20，最大 100；`offset`：默认 0
- `order`：`asc` 或 `desc`，默认 `desc`

跨游戏和来源在服务端统一排序与分页，每条新闻返回 `game_id`、`source` 和 `id`。分页 `meta` 为 `null`。三个标识共同定位新闻，不能仅凭新闻 ID 判断身份

启用正文搜索时，列表条目可以返回 `search_match.title` 和 `search_match.body`。前者包含完整标题，后者是命中位置附近的正文片段。两者均为有序的 `{text, highlight}` 纯文本片段，可按 `highlight` 渲染关键词。没有正向命中的字段会省略

```http
GET /api/v1/news/{game_id}/{source}/{news_id}
GET /api/v1/news/{game_id}/{source}/{news_id}/media/video
GET /api/v1/news/{game_id}/{source}/{news_id}/media/nfo
GET /api/v1/news/{game_id}/{source}/series/{tag_name}/media/nfo
GET /api/v1/news/{game_id}/{source}/series/{tag_name}/episodes/{news_id}/media/nfo?season={season}&episode={episode}
GET /api/v1/news/rss?game=ys&source=mys
```

RSS 支持列表相同筛选，返回 RSS XML；NFO 返回 XML。单集 `season` 为 0 到 9999，`episode` 为 1 到 999999。视频接口返回当前播放地址，米游社临时签名不能作为永久链接缓存。RSS 和视频接口遇到 `429` 时遵循 `Retry-After`

## 日历

### 发现日程能力

```http
GET /api/v1/games/{game_id}/calendar/capabilities
```

在构造 JSON 查询或 ICS 订阅地址前读取该接口。响应中的：

- `json`：该游戏当前的 JSON 日程路径
- `ics`：该游戏当前的 ICS 日程路径
- `selectors`：可用于 `include` 和 `exclude` 的筛选值

每个 selector 的 `value` 是一种日程类型，`children[].value` 是带细分类的完整筛选值。客户端可以显示 `label`，但发送查询时必须原样使用 `value`。不同游戏支持的 selector 和细分类可能不同，不要在 Agent 或客户端中硬编码列表

角色生日也是统一日程中的一种 selector。需要生日数据或生日 ICS 时，从 capabilities 找到对应 `value`，再通过 `include` 选择它

### 查询 JSON 日程

```http
GET /api/v1/games/{game_id}/calendar
```

参数：

- `from`：查询开始日期，`YYYY-MM-DD`；JSON 默认是中国标准时间今天之前 30 天
- `to`：不包含在查询范围内的结束日期，`YYYY-MM-DD`；默认是 `from` 后 366 天
- `include`：可重复；未提供时包含 capabilities 中的全部日程，提供多个值时匹配其中任意一个
- `exclude`：可重复；在 `include` 之后生效，匹配任意一个值的条目都会被排除
- `limit`：默认 100，范围 1 到 500
- `offset`：默认 0

类型 selector 匹配该类型的全部条目，子 selector 只匹配该类型下具有相应 label 的条目。例如先 `include=游戏内活动`，再 `exclude=游戏内活动:七圣召唤`，表示保留游戏内活动但排除其中的七圣召唤日程。具体值仍须来自该游戏本次返回的 capabilities；未知 selector 返回 `400`

日期范围必须为正且不超过 1100 天。普通日程只要与 `[from, to)` 相交就会返回；角色生日会按年份实例化到该范围内

响应使用分页外壳，`meta` 当前为 `null`：

```json
{
  "total": 1,
  "limit": 100,
  "offset": 0,
  "items": [
    {
      "id": "ENTRY_ID",
      "kind": "SELECTOR_VALUE",
      "title": "日程标题",
      "start": "2026-01-01T03:00:00Z",
      "end": "2026-01-10T19:59:00Z",
      "all_day": false,
      "version": "VERSION_ID",
      "cover": "https://example.invalid/cover.png",
      "labels": [],
      "url": "https://example.invalid/source"
    }
  ],
  "meta": null
}
```

定时日程的 `start`、`end` 是 UTC RFC 3339。生日等全天日程使用 `YYYY-MM-DD`，且 `end` 是不包含在事件内的结束日期；此时 `all_day` 为 `true`。`version` 和 `cover` 可以为 `null`

### 导出 ICS 日程

```http
GET /api/v1/games/{game_id}/calendar.ics
```

`from`、`to`、`include` 和 `exclude` 与 JSON 接口语义相同；ICS 默认从中国标准时间今天开始，忽略 JSON 的 `limit` 和 `offset`。角色生日会导出为每年重复的全天事件

2 月 29 日生日在 JSON 与 ICS 中有意采用不同的平年日期。JSON 会直接物化每年的日期，因此平年显示为二月最后一天（2 月 28 日）；ICS 为兼容无法正确展开负数 `BYMONTHDAY` 的日历软件，改用每年第 60 天，即闰年为 2 月 29 日、平年为 3 月 1 日

ICS 还支持：

- `event_mode`：`span` 表示一个连续事件，`milestones` 表示开始和结束两个节点；默认 `span`
- `start_reminder_minutes`、`end_reminder_minutes`：普通日程开始或结束前的提醒分钟数，最大 43200
- `birthday_reminder_time`：生日当天提醒时间，格式为 `HH:MM`
- `birthday_reminder_minutes_before`：生日当天 00:00 前的提醒分钟数，最大 43200

响应是 `text/calendar`，不是 JSON

## 错误与限流

- `400`：参数格式或字段组合无效
- `404`：游戏、集合、来源或条目不存在或不受支持
- `429`：读取 `Retry-After` 后等待，最多重试一次
- `500`：服务内部错误

公开查询无需认证。任何要求 Bearer token 的路径都不属于此 Skill 的能力范围
