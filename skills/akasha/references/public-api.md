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

### 角色生日

```http
GET /api/v1/games/{game_id}/calendar/character-birthdays
GET /api/v1/games/{game_id}/calendar/character-birthdays.ics
```

参数：

- `q`：角色名称和简介查询
- `birthday_month`：1 到 12
- `gender`：`male` 或 `female`，仅 `zzz` 支持

JSON 返回生日条目，ICS 返回每年重复的日历事件

### 游戏活动

```http
GET /api/v1/games/{game_id}/calendar/events
GET /api/v1/games/{game_id}/calendar/events.ics
```

参数：

- `from`：开始日期，`YYYY-MM-DD`，默认包含最近 30 天
- `to`：结束日期，`YYYY-MM-DD`，默认查询未来 366 天
- `kind`：可重复使用，值为 `game_activity`、`banner` 或 `web_activity`

日期范围必须为正且不超过 1100 天。`to` 是查询边界，使用当前 OpenAPI 和返回结果确认具体包含关系

## 错误与限流

- `400`：参数格式或字段组合无效
- `404`：游戏、集合、来源或条目不存在或不受支持
- `429`：读取 `Retry-After` 后等待，最多重试一次
- `500`：服务内部错误

公开查询无需认证。任何要求 Bearer token 的路径都不属于此 Skill 的能力范围
