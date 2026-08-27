# Akasha 本地开发与验证指南

## 适用范围

本地启动、Rust 验证、backend 与 worker 联调、Docker build 或 SQLite 故障排查前阅读本文件。

## 本地启动

Windows 和 Linux 均可直接运行：

```bash
cargo run
```

也可使用 `just dev` 和 `just check`。

启动 backend 前检查 `7040` 端口和现有 `akasha-backend` 进程。Windows 上运行中的 backend 会锁定 `target/debug/akasha-backend.exe`；停止或重启用户正在使用的 backend、数据库或容器前先取得明确同意。

## Backend 验证

Rust 改动完成后按风险运行相关检查；提交前应完成适用于改动范围的全部检查：

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

修改 `Cargo.toml` 时同步更新 `Cargo.lock`，避免夹带无关依赖升级，并确保所有使用 `--locked` 的命令可运行。

## Worker 验证

仅在用户明确要求修改或运行私有 worker 时进入 `worker/`，并先阅读该仓库自己的 `AGENTS.md`。修改后按其 package scripts 运行依赖安装、静态检查、测试和构建。

运行 worker 前确认 backend 健康、所需环境变量已加载，并使用正确的 `DATA_WRITE_TOKEN`。上游请求统一经过共享的限速、超时和有限重试策略。

## Smoke Test

backend 启动后检查：

- `http://127.0.0.1:7040/healthz`
- `http://127.0.0.1:7040/scalar`
- 至少一个无需 token 的公共 API
- 管理接口缺少 token 和 token 错误时返回 `401`
- 正确 `DATA_WRITE_TOKEN` 可访问目标管理接口
- 未知资源返回 `404`
- 非法输入返回 `400`

测试输出不得回显真实 token 或完整配置。

## Backend 与 Worker 联调

发生协议联动修改时，对照验证 HTTP method、URL、path 参数、query、JSON body、响应状态和响应 JSON 类型。

涉及任务生命周期时覆盖 acquire、heartbeat、checkpoint、真实写入、complete 和 fail；涉及业务 DTO 时覆盖本次改动实际影响的 news、game-data、calendar 或 lease 请求与响应。

## SQLite locked / busy

遇到 SQLite `locked` 或 `busy`：

1. 停止持续产生请求的测试 worker
2. 检查重复 backend 实例
3. 检查长事务和连接池耗尽
4. 检查 WAL / SHM 状态与文件权限
5. 检查 busy timeout

## 故障日志处理

- 先验证报错对应的功能是否真实生效，再决定是否调整日志
- 不得用 `2>/dev/null`、无条件 `|| true`、日志过滤或降级级别替代根因修复
- 对确认可忽略的重复信息，记录判断依据，并提供健康检查、状态字段或指标证明底层机制正常
- 容器内权限问题先检查 namespace、挂载模式、运行用户和运行时支持范围，不以 `privileged` 或整棵宿主文件系统读写挂载作为默认修复

删除 WAL / SHM、替换数据库或恢复备份属于数据库状态变更，须遵循 `.agents/database.md` 的授权和备份要求。

## 工作区检查

修改和测试前后检查根仓库状态；操作 worker 时另行检查其独立仓库状态。保留用户已有改动，并确认生成文件没有进入错误的仓库。
