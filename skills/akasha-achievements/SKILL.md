---
name: akasha-achievements
description: Manage HoYoverse achievement progress in local files, match achievement screenshots against the Akasha catalog, and import or export Genshin UIAF. Use for recording, checking, correcting, or initializing personal achievements; obtain explicit user permission before using Yae to read live Genshin data.
---

# 本地成就管理

将个人进度保存在用户指定的本地 JSON 文件，使用 Akasha 公开 API 作为成就目录。支持原神 `ys`、星穹铁道 `sr`、绝区零 `zzz`，与成就手册网站的“全部游戏备份”和原神 UIAF v1.0 / v1.1 互通

## 开始工作

1. 找到或询问用户希望管理的进度文件；不要另建文件后声称已修改网站进度
2. 读取文件并检查格式，修改前由脚本自动创建备份
3. 首次使用时创建空文件，下载相应游戏的完整目录，再匹配名称、描述和阶段 ID
4. 使用下面的脚本执行确定的修改，报告修改的游戏、名称、阶段、数量和文件位置
5. 用户需要网站使用结果时导出本站备份或原神 UIAF，在网站通过“导入”加载；本地文件不会自动更新浏览器进度

脚本仅依赖 Python 标准库。在使用 uv 的环境中以 `uv run python` 运行；其他环境使用可用的 Python 3.10+。命令中的脚本路径均相对于本 skill 根目录，不是工作仓库

```sh
uv run python scripts/achievements.py init --file progress.json
uv run python scripts/achievements.py catalog --game ys --output ys-catalog.json
uv run python scripts/achievements.py find --catalog ys-catalog.json --query "昨日重现"
uv run python scripts/achievements.py set --file progress.json --catalog ys-catalog.json --id VERIFIED_ID --completed
uv run python scripts/achievements.py import --file progress.json --input exported-uiaf.json
uv run python scripts/achievements.py export --file progress.json --format uiaf --catalog ys-catalog.json --output uiaf.json
uv run python scripts/achievements.py export --file progress.json --format backup --output website-backup.json
uv run python scripts/achievements.py status --file progress.json --catalog ys-catalog.json
```

`VERIFIED_ID` 必须替换为目录查到的实际 ID。`set` 支持重复 `--id`，取消完成使用 `--incomplete`，已知完成日期使用 `--date YYYY-MM-DD`。没有日期证据时保持未知，不以识别截图的当天冒充完成日期。已有文件无法通过校验时禁止覆盖

## 根据截图管理

- 先识别游戏、成就名称、描述、目标数值、完成或领奖标记，区分选中样式与完成状态
- 查询本地目录匹配原始 ID；同名、分阶段或 OCR 有歧义时结合描述及目标数值，仍有歧义则询问用户
- 多阶段成就的各阶段使用独立 ID。仅记录截图能证明的阶段，不能把单个阶段展开成全部完成
- 只标记截图明确显示的完成项目；不因未出现在截图中而取消其他成就，不推断无法看清的状态或日期
- 用户明确请求“把截图里的已完成项记上”时可直接执行已确定的匹配；不确定项单独报告，不用猜测填补
- 保留导入文件中不在当前目录的原始记录，避免解包残留或目录版本差异导致数据丢失

## 初始化与批量游戏读取

优先导入用户已有的 UIAF 或本站备份。如果需要直接从运行中的原神读取大量成就，先阅读 [Yae 使用与授权](references/yae.md)，说明操作会启动并读取游戏、可能退出游戏，再取得本次操作的明确许可。仅安装本 skill、请求识别截图或一般初始化请求均不代表允许运行 Yae

## 数据与安全边界

- Akasha 仅作为只读目录来源，默认 `https://akasha.trrw.cn`；自定义地址通过 `catalog --base-url` 指定，只允许根 HTTPS 地址或本机 HTTP
- 不调用 `/admin`，不读取服务器配置，不上传个人成就，不需要 Akasha 写入 token
- 进度文件不包含账号、密码、WebDAV 凭据或 GitHub Token；不要索取游戏密码、Cookie 或登录凭据
- 导入以文件列出的记录为准，不清除缺失记录；本站备份按更新时间合并，取消完成也保留为记录
- [文件格式与兼容规则](references/formats.md) 描述 UIAF 状态、未知日期和同步语义
- GitHub Gist 或网盘同步需要用户明确指定渠道及文件；本 skill 默认只读写本地文件
