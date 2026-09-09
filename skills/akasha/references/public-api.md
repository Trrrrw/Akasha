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

游戏数据集合目前仅支持：

| 游戏 | ID |
| --- | --- |
| 原神 | `ys` |
| 崩坏：星穹铁道 | `sr` |
| 绝区零 | `zzz` |

`GET /api/v1/games/{game_id}` 返回指定游戏详情。游戏列表可能包含尚未提供游戏数据集合的其他游戏，因此不要把列表中的任意 ID 直接用于 `/data`

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
  "meta": {}
}
```

分页默认 `limit=20`，最大 `100`。使用稳定的相同筛选条件递增 `offset`

## 游戏数据

### 发现集合

```http
GET /api/v1/games/{game_id}/data
```

先读取返回的集合 ID 和条目数量，再查询集合。常用集合是 `character`，其他集合随游戏不同

### 查询列表

```http
GET /api/v1/games/{game_id}/data/{collection}
```

通用参数：

- `q`：名称和摘要文本查询，支持空格 AND、`|` OR、`-` 排除和引号短语
- `limit`：1 到 100
- `offset`：从 0 开始

角色集合参数按游戏区分：

| 游戏 | 可用角色筛选 |
| --- | --- |
| `ys` | `element`、`weapon_type`、`rarity`、`region`、`affiliation`、`cv`、`birthday_month`、`birthday_day`、`special` |
| `sr` | `path`、`combat_type`、`rarity`、`camp`、`cv`、`birthday_month`、`birthday_day` |
| `zzz` | `specialty_id`、`specialty`、`element_id`、`element`、`hit_type_id`、`hit_type`、`camp_id`、`camp`、`rarity`、`gender`、`special_element`、`birthday_month`、`birthday_day` |

角色专用参数只能用于 `character` 集合。不支持的游戏与字段组合返回 `400`

### 查询详情

```http
GET /api/v1/games/{game_id}/data/{collection}/{id}
```

返回完整摘要、详情和后端资源链接。条目 ID 来自列表响应，不要根据名称猜测

## 新闻

### 发现来源

```http
GET /api/v1/games/{game_id}/news/sources
```

当前 `ys`、`sr`、`zzz` 常见来源为：

- `web_cn`：游戏官网
- `mys`：米游社

仍应以来源发现接口为准

### 标签

```http
GET /api/v1/games/{game_id}/news/tags?source={source}
```

返回指定来源的标签分组、新闻数量和最近新闻预览

### 新闻列表

```http
GET /api/v1/games/{game_id}/news?source={source}
```

参数：

- `source`：必填
- `q`：标题查询，支持空格 AND、`|` OR、`-` 排除、引号短语和反斜杠转义
- `tag`：任一匹配标签，可重复，最多 32 个
- `untagged`：是否包含无标签新闻
- `character`：任一匹配角色 ID，可重复，最多 32 个
- `news_type`：`article` 或 `video`
- `published_from`：包含该日，`YYYY-MM-DD`
- `published_to`：包含该日，`YYYY-MM-DD`
- `limit`：默认 20，最大 100
- `offset`：默认 0
- `order`：`asc` 或 `desc`，默认 `desc`

重复参数表示“匹配任意一个值”，例如 `tag=角色&tag=活动`

### 新闻详情

```http
GET /api/v1/games/{game_id}/news/{news_id}?source={source}
```

`source` 必填。视频新闻详情还可能包含相关推荐

### RSS

```http
GET /api/v1/games/{game_id}/news/rss?source={source}
```

新闻筛选参数与列表相同，并使用 `limit` 控制条目数。响应是 RSS XML，不是 JSON。该接口有限流，遇到 `429` 时遵循 `Retry-After`

### NFO

```http
GET /api/v1/games/{game_id}/news/{news_id}/media/nfo?source={source}
GET /api/v1/games/{game_id}/news/series/{tag_name}/media/nfo?source={source}
GET /api/v1/games/{game_id}/news/series/{tag_name}/episodes/{news_id}/media/nfo?source={source}&season={season}&episode={episode}
```

单集 `season` 范围为 0 到 9999，`episode` 范围为 1 到 999999。响应是 XML NFO

### 视频地址

```http
GET /api/v1/games/{game_id}/news/{news_id}/media/video?source={source}
```

该接口返回当前有效的视频播放信息并有限流。米游社视频 URL 可能包含临时签名，只在任务需要时获取，不作为永久地址缓存

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

`from`、`to`、`include` 和 `exclude` 与 JSON 接口语义相同；ICS 默认从中国标准时间今天开始，忽略 JSON 的 `limit` 和 `offset`。角色生日会导出为每年重复的全天事件；2 月 29 日生日按每年第 60 天重复，即闰年为 2 月 29 日、平年为 3 月 1 日

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
