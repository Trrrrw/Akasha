# Akasha 查询配方

以下示例假设当前目录是 Skill bundle 根目录。其他安装方式应根据 Skill base directory 定位脚本

## 发现游戏和能力

```bash
python scripts/akasha_api.py /api/v1/games
python scripts/akasha_api.py /api/v1/games/ys/data
python scripts/akasha_api.py /api/v1/games/ys/news/sources
python scripts/akasha_api.py /api/v1/games/ys/calendar/capabilities
```

先发现再查询可以避免猜错游戏 ID、集合、来源或日程 selector。构造 Calendar 查询时使用 capabilities 返回的 `json`、`ics` 和 selector `value`，不要硬编码筛选值

## 搜索角色

按名称搜索原神角色：

```bash
python scripts/akasha_api.py /api/v1/games/ys/data/character \
  --query q=胡桃 \
  --query limit=10
```

查询五星火元素原神角色：

```bash
python scripts/akasha_api.py /api/v1/games/ys/data/character \
  --query element=火 \
  --query rarity=5 \
  --query limit=100
```

查询星铁命途角色：

```bash
python scripts/akasha_api.py /api/v1/games/sr/data/character \
  --query path=巡猎 \
  --query limit=100
```

筛选值应来自实际列表数据。若返回 `400`，先移除猜测字段并查看 `/openapi.json` 或集合中的实际值

## 查询新闻

最近 10 条官网新闻：

```bash
python scripts/akasha_api.py /api/v1/games/ys/news \
  --query source=web_cn \
  --query limit=10 \
  --query order=desc
```

查询标题同时包含两个词的新闻：

```bash
python scripts/akasha_api.py /api/v1/games/ys/news \
  --query source=web_cn \
  --query 'q=版本 活动'
```

查询多个标签中的任意一个：

```bash
python scripts/akasha_api.py /api/v1/games/ys/news \
  --query source=mys \
  --query tag=角色 \
  --query tag=活动
```

按角色过滤新闻时使用角色条目的 ID，而不是角色名称：

```bash
python scripts/akasha_api.py /api/v1/games/ys/news \
  --query source=mys \
  --query character=10000046 \
  --query news_type=video
```

获取详情前从列表条目的 `id` 和 `source` 字段读取新闻 ID 与来源：

```bash
python scripts/akasha_api.py /api/v1/games/ys/news/NEWS_ID \
  --query source=web_cn
```

## 查询统一日程

先发现该游戏当前支持的筛选规则：

```bash
python scripts/akasha_api.py /api/v1/games/ys/calendar/capabilities
```

不添加筛选时查询指定日期范围内的全部日程，并使用 `limit`、`offset` 分页：

```bash
python scripts/akasha_api.py /api/v1/games/ys/calendar \
  --query from=2026-01-01 \
  --query to=2026-02-01 \
  --query limit=100 \
  --query offset=0
```

只查询 capabilities 返回的两种类型时，重复传入 `include`。多个 `include` 是“匹配任意一个”：

```bash
python scripts/akasha_api.py /api/v1/games/ys/calendar \
  --query include=游戏内活动 \
  --query include=卡池
```

先选择一类日程、再排除其中一个细分类时，重复参数按顺序传给脚本即可。`exclude` 在 `include` 之后生效：

```bash
python scripts/akasha_api.py /api/v1/games/ys/calendar \
  --query include=游戏内活动 \
  --query exclude=游戏内活动:七圣召唤
```

示例中的中文值仅用于展示参数结构；实际查询前应从目标游戏的 capabilities 读取完整 `value`

回答“当前进行中”时，应查询覆盖当前日期的范围，再根据每条记录的 `start`、`end` 和 `all_day` 判断，不要仅依赖标题或返回顺序。响应包含 `total`、`limit`、`offset`、`items` 和 `meta`；若 `total > offset + limit`，保持全部筛选参数不变并增加 `offset`

## 查询生日

角色生日已整合进统一 Calendar。查询某月生日时，使用 capabilities 返回的角色生日 selector，并限定日期范围：

```bash
python scripts/akasha_api.py /api/v1/games/zzz/calendar \
  --query include=角色生日 \
  --query from=2026-07-01 \
  --query to=2026-08-01
```

只导出生日 ICS：

```bash
python scripts/akasha_api.py /api/v1/games/ys/calendar.ics \
  --query include=角色生日 \
  --output ys-birthdays.ics
```

## 导出 RSS 和活动 ICS

```bash
python scripts/akasha_api.py /api/v1/games/sr/news/rss \
  --query source=web_cn \
  --query limit=50 \
  --output sr-news.xml

python scripts/akasha_api.py /api/v1/games/sr/calendar.ics \
  --query include=卡池 \
  --output sr-banners.ics
```

使用 `--output` 时保存服务端原始字节，不做格式转换

## 获取视频地址

先用 `news_type=video` 查到新闻 ID，再请求视频接口：

```bash
python scripts/akasha_api.py /api/v1/games/ys/news/NEWS_ID/media/video \
  --query source=mys
```

米游社播放地址可能很快过期。除非用户明确要求，不主动请求视频地址；回答时将其标记为临时 URL

## 分页

先读取第一页及 `total`：

```bash
python scripts/akasha_api.py /api/v1/games/ys/data/character \
  --query limit=100 \
  --query offset=0
```

若 `total > offset + limit`，保持全部筛选参数不变并增加 `offset`：

```bash
python scripts/akasha_api.py /api/v1/games/ys/data/character \
  --query limit=100 \
  --query offset=100
```

只获取用户问题所需页数，不无条件抓取完整数据库

## 使用其他部署

PowerShell：

```powershell
$env:AKASHA_BASE_URL = 'http://localhost:7040'
python scripts/akasha_api.py /api/v1/games
```

POSIX shell：

```bash
AKASHA_BASE_URL=http://localhost:7040 \
  python scripts/akasha_api.py /api/v1/games
```

也可以单次覆盖：

```bash
python scripts/akasha_api.py --base-url http://localhost:7040 /api/v1/games
```
