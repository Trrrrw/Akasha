# Akasha 数据库开发指南

## 适用范围

修改 `crates/db`、SeaORM Entity、repository、SQLite schema、索引、事务或数据库诊断逻辑前阅读本文件。

## Schema 管理

项目使用 Entity-first schema sync，不使用 SeaORM migration。

- Entity 是长期 schema 定义，业务模型删除后同步移除 Entity 注册
- schema sync 必须幂等
- Entity 无法表达的复合索引或有序索引使用 SeaQuery 显式、幂等同步
- 一次性数据或表清理不进入 backend 启动流程
- 源码优先使用 SeaORM 和 SeaQuery；仅在二者无法表达时使用原始 SQL，并记录原因

## Repository 边界

application 定义 repository 端口，db 负责实现。`sea_orm::Model`、Entity 和 ActiveModel 只存在于 db 层。

筛选、聚合、排序、计数、分页和每组限制尽量由数据库完成。查询需要有明确边界，避免无界加载后在 Rust 内存中处理本可由数据库完成的工作。

## 事务

以下多步骤写入使用事务：

- 跨表写入
- 新闻标签或角色关系替换
- game-data 批量写入
- 业务数据与对应审计日志

同一业务操作的数据修改和审计日志共同提交或回滚。

## 数据模型

各游戏的 game-data 使用独立表，例如 `ys_game_data`、`sr_game_data` 和 `zzz_game_data`，字段按实际来源分别建模。跨游戏结构一致的新闻继续使用共享表。

`worker_states` 是 lease、checkpoint、worker 状态和 run 状态的任务协调表，不是认证表。lease 操作保持幂等，使用条件更新处理并发状态，并允许客户端进行有限重试。

## SQLite 运行约束

- 配置连接池获取超时和 SQLite busy timeout
- 写事务应尽量短
- 部署默认只有一个 backend 实例写入同一 SQLite 数据库

遇到启动卡住、`locked` 或 `busy` 时依次检查：

1. 是否存在多个 backend 实例或持续重试的 worker
2. 连接池是否耗尽或存在长事务
3. SQLite 文件权限
4. WAL / SHM 状态
5. busy timeout 配置

可使用以下只读诊断：

```sql
PRAGMA integrity_check;
PRAGMA foreign_key_check;
```

单条 Entity 注册日志不足以定位具体 schema 卡点，应结合连接、锁和后续日志判断。

## 数据库状态变更

DROP table、删除或覆盖数据库、替换数据库、恢复备份和 `VACUUM` 都会改变数据库状态。执行前必须确认目标数据库、停写条件、备份和用户的明确授权；这类操作不得作为普通测试或启动步骤隐式执行。
