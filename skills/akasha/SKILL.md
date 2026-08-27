---
name: akasha
description: "Query Akasha's public API for current Genshin Impact, Honkai: Star Rail, and Zenless Zone Zero game data, news, and calendars. Use when retrieving, filtering, comparing, or exporting information from Akasha."
---

# 使用 Akasha

通过 Akasha 的公开只读 API 获取游戏资料、新闻和日历信息。默认根地址为 `https://akasha.trrw.cn`；用户指定自托管或本地服务时使用该地址

## 能力边界

- 只使用公开 GET 接口；`/openapi.json` 是当前部署接口与 schema 的权威来源
- 不调用认证接口、`/api/v1/admin/**` 或 worker 协调接口，不处理任何凭据
- 视频播放地址可能带临时签名，不将其视为永久链接

## 工作流

1. 使用用户指定的服务地址，否则使用默认根地址。辅助脚本也支持 `AKASHA_BASE_URL` 和 `--base-url`
2. 仅在标识未知时发现能力：游戏 ID 查询 `/api/v1/games`，数据集合查询 `/api/v1/games/{game_id}/data`，新闻来源查询 `/api/v1/games/{game_id}/news/sources`
3. 优先使用服务端筛选和分页，只获取完成任务所需的页数
4. 从 Skill base directory 运行通用脚本；若 Python 不可用，使用能保留重复 query 参数的等价 HTTP 客户端

```bash
python scripts/akasha_api.py /api/v1/games/ys/data/character --query q=胡桃 --query limit=10
```

5. 在回答中区分 API 返回事实与推断，并保留相关条目的 `source_url` 或 API 路径

## 按需读取参考

- 选择接口、筛选字段、响应格式或解释状态码时，读取 [references/public-api.md](references/public-api.md)
- 需要 CLI 示例、重复参数、分页或 RSS/ICS 导出方式时，读取 [references/query-recipes.md](references/query-recipes.md)

## 失败与输出

- `400`：修正参数或不受支持的游戏与字段组合
- `404`：重新发现游戏、集合或新闻来源；不要把不支持误报为服务故障
- `429`：遵循 `Retry-After`，最多重试一次
- 网络错误或 `5xx`：报告服务暂时不可用，不凭空补全当前数据
- 用户要求 RSS、ICS 或 NFO 时保存原始响应；只在用户需要时获取临时视频地址
