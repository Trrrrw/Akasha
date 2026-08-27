# Akasha Skills 安装说明

你需要安装本文件列出的 Akasha Agent Skills

## 待安装 Skills

### akasha

- 来源：https://github.com/Trrrrw/Akasha/tree/main/skills/akasha
- 用途：通过 Akasha 公开 API 查询游戏资料、角色、新闻、活动日历、生日和媒体信息

## 安装要求

1. 识别你当前使用的 Agent、CLI 或运行时及其 Skills 机制
2. 查询该产品当前的官方 Skills 安装文档，确认正确的安装目录、安装方式和生效条件
3. 如果环境提供内置 Skill 安装器，优先使用内置安装器；否则使用官方文档支持的方式从上述来源安装
4. 直接完成安装，不要只返回安装命令或操作说明
5. 保留每个 Skill 的完整 bundle，包括 `SKILL.md` 以及实际存在的 `agents`、`references`、`scripts` 和 `assets`
6. 只安装“待安装 Skills”中列出的公开 Skill，不要安装仓库中的其他目录、私有开发设施或凭据
7. 如果目标目录已存在，不要静默覆盖；先比较当前安装与来源，再使用运行时支持的安全更新方式
8. 安装后验证 Skill 能被运行时发现，并报告安装位置、验证结果以及是否需要重启工具或开启新会话
