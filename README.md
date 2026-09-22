# Juno Pulsar Desktop

Windows 10/11 x64 的非官方桌面飞行物应用。Tauri 2 + React + TypeScript + Rust/Win32，原创 Canvas 占位皮肤，无音效。

**当前是 0.1.1 开发预览，不是已经完成真实桌面删除验收的稳定版。** 删除默认关闭，启用后每次必须确认，只请求移入回收站。桌面挂载与混合 DPI 兼容性请按手工清单验证。

## 运行

安装 Node.js 22、Rust stable（至少 1.89）、MSVC Build Tools 的“使用 C++ 的桌面开发”、Windows SDK、Microsoft Edge WebView2 Runtime。

```powershell
npm ci
npm run tauri dev
```

浏览器设置预览：`npm run dev`，打开 `http://127.0.0.1:5173`。浏览器预览不能执行桌面操作。

```powershell
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run tauri build
pwsh -File scripts/package-portable.ps1
```

NSIS 输出：`src-tauri/target/release/bundle/nsis`。便携包在 `release/`；便携版仍将配置写入 AppData，依赖已安装的 WebView2，不是完全离线自包含发行物。

## 使用

1. 设置窗口随启动打开，透明桌面层在后台运行；关闭设置会收至托盘。
2. 在飞行物上按住左键 250ms 锁定；Esc 或右键取消。
3. 开启回收站操作后，拖到桌面实际文件／快捷方式／普通文件夹，悬停至少 600ms 后松开。
4. 核对独立确认窗口中的完整路径，点击“移入回收站”。取消是默认焦点。令牌 30 秒过期。
5. 第一次验证只使用自行创建的可丢弃文件，并从回收站恢复核验；不应以真实重要文件测试。

仅桌面直接子项可作为目标；不使用图标文字推测路径，不解析 .lnk 目的地。拒绝虚拟项目、网络、重解析点（包括部分云盘占位符）、只读／系统属性和无删除权限目标。Shell 不确认可回收就取消。任何结果不明确都不显示“回收成功”。

## 皮肤、显示器和隐私

见 [皮肤规范](docs/skins.md)、[架构与威胁模型](docs/architecture.md)、[手工验收](docs/manual-tests.md)。支持 PNG 横向帧图集目录和 .jpskin，不包含音效或游戏提取素材。代码 MIT；`assets/sample-skin` 原创素材 CC0-1.0。

配置与日志保存在 `%APPDATA%/org.junopulsar.desktop`（以 Tauri app_data_dir 实际返回目录为准）。没有遥测、自动更新网络调用或外部素材请求；日志不记录文件名／完整路径。开机启动仅修改当前用户 Run 注册表项。

## 实现边界

- WorkerW 是未公开稳定接口；失败使用置底透明窗口，可能受 Win+D、Explorer 主题与第三方桌面工具影响。
- 只有每屏第一枚飞行物可捕获，额外数量目前作为视觉伴随物。
- 点击区域为飞行物核心圆，不是图像逐像素 alpha；皮肤建议保持核心尺寸。
- Shell 命中调用遇到超时会禁用本进程后续探测，需重启应用；极小远程缓冲区保留到 Explorer 退出，避免延迟消息使用已释放内存。
- COM 不支持强制安全中断所有 Explorer 调用；卡住时不允许继续删除，退出应用恢复。
- 文件身份在确认和 Shell 执行前重验，但 IFileOperation 是路径／Shell 项目操作，不能宣称消除恶意同用户进程在最后一瞬间替换路径的全部竞态。本版不适用于存在主动本机攻击者的环境。
- Windows 10/11、回收站真实恢复、两小时性能、混合 DPI／热插拔未由单元测试替代。见各阶段记录。

## 发布

版本号同时维护 package.json、Cargo.toml 和 tauri.conf.json。GitHub Actions 在 `vMAJOR.MINOR.PATCH` 标签上构建 NSIS、便携 ZIP、SHA-256，创建 **草稿预发布 Release**。仓库中未包含签名证书；本地构建未签名。完成手工验收后再公开。

本项目非官方，与 Blizzard Entertainment 无关联或授权关系。相关游戏商标与原始参考素材归各自权利人所有。根目录提供的游戏图像被 .gitignore 排除，不进入源码提交或发行包。

Read-only native check: `src-tauri/target/debug/juno-pulsar-desktop.exe --diagnose` (outputs counts, no file paths). Startup/exit smoke: append `--smoke`; exits after eight seconds. Actual local results are in docs/native-references.md.
