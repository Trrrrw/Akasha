# Yae 批量导出

用户指定的 [HolographicHat/YaeAchievement](https://github.com/HolographicHat/YaeAchievement) 当前重定向至 [HolographicHat/Yae](https://github.com/HolographicHat/Yae)。以官方仓库的最新 README、[使用说明](https://github.com/HolographicHat/Yae/blob/master/docs/Tutorial.md) 和 [Releases](https://github.com/HolographicHat/Yae/releases) 为准，不固定版本、资源名称或终端选项编号

## 必须取得的许可

向用户说明：将从官方项目获取并运行 Yae，启动并读取原神成就，官方教程中的读取流程会退出游戏，再生成本地 UIAF 文件。询问是否允许执行本次操作，并确认输出目录。许可之前不得下载、安装或启动工具，不得读取游戏进程；用户提供现成导出文件不需要此权限

## 执行流程

1. 确認系统和当前游戏版本是否受该发行版支持，检查官方说明及已知问题。不能确认兼容性时先告知用户，不试运行猜测
2. 用户允许后从官方 Releases 获取适用于当前 Windows 环境的资源，保留来源、版本和现有工具，不安装到公开仓库
3. 按该版本官方流程运行。需要用户登录、选择安装路径、系统提权或操作游戏界面时交给用户，不索取密码或绕过系统权限
4. 优先选择 UIAF JSON 输出选项，选项编号以实际工具为准。现行教程描述工具启动游戏，进入游戏后退出并让用户选择导出格式
5. 使用本 skill 的 `import` 校验并导入生成的文件；展示导入记录数、完成数、未完成数及多阶段记录，不把生成文件当作导入成功的证据
6. 保留原始导出文件和修改前备份，报告实际输出位置。失败时保留现有进度，不尝试未获许可的替代游戏读取工具

无法操作游戏或当前工具不兼容时，可以请用户自行导出 UIAF，继续进行本地导入；截图管理仍可独立使用
