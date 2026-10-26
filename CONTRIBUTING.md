# 贡献指南

感谢你对 Audio Switch 的兴趣。这是一个体量不大但很在意「行为正确」的桌面工具：几乎所有改动都会碰真实的 Windows 音频设备，所以下面对提交质量的要求写得比较具体。

## 目录

- [行为准则](#行为准则)
- [报告问题](#报告问题)
- [开发环境](#开发环境)
- [提交代码](#提交代码)
- [代码约定](#代码约定)
- [提交前检查清单](#提交前检查清单)
- [Pull Request](#pull-request)
- [许可](#许可)

## 行为准则

参与本项目的讨论、Issue 与 PR 即表示你同意保持尊重与就事论事：针对代码和设计提出批评，不针对人；不接受人身攻击、歧视性言论或骚扰行为。维护者有权删除不当内容并拒绝相关贡献。

## 报告问题

提交 Issue 前请先搜索是否已有同类问题。一个能被快速定位的报告通常包含：

1. **现象**：期望发生什么、实际发生了什么，是否稳定复现。
2. **复现步骤**：从哪一步开始（托盘左键 / 右键菜单 / 主界面按钮），涉及哪个设备。
3. **环境**：Windows 版本（`winver`）、Audio Switch 版本（主界面右下角的状态栏）、设备连接方式（USB / HDMI / 蓝牙 / 3.5mm）。
4. **日志**：`%APPDATA%\com.audioswitch.desktop\logs\app.log` 的相关片段（注意日志里含设备名称与端点 ID，可自行打码）。
5. **诊断输出**（切换类问题强烈建议附上）：

   ```bash
   audio-switch.exe --diagnose
   ```

   它会列出所有输出端点与三个标准角色的默认端点，多数「设备找不到 / 切换没生效」的问题看这一段就能定位。

请顺手说明是否使用了第三方音频工具（Equalizer APO、Voicemeeter、虚拟声卡等），这类软件会拦截或重定向端点，影响很大。

## 开发环境

见 [README 的环境依赖](./README.md#环境依赖) 与 [安装与使用](./README.md#安装与使用)。简单说需要：

- Node.js ≥ 20、npm
- Rust stable（`x86_64-pc-windows-msvc`）
- VS 2022 Build Tools（含「使用 C++ 的桌面开发」工作负载与 Windows SDK）
- WebView2 运行时（Windows 11 自带）

首次拉取后：

```bash
npm install
npm run tauri dev
```

## 提交代码

1. Fork 仓库并从 `main` 切出分支，分支名建议 `feat/xxx`、`fix/xxx`、`docs/xxx`。
2. 一个 PR 只做一件事。重构与功能改动请分开，方便审阅与回滚。
3. 提交信息采用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/) 格式：

   ```
   <type>(<scope>): <摘要>

   <可选正文：为什么这么改>
   ```

   常用 `type`：`feat`、`fix`、`refactor`、`perf`、`docs`、`test`、`build`、`chore`。
   `scope` 用模块名，例如 `tray`、`audio`、`i18n`、`config`、`ui`。
   摘要可用中文，一句话说清改了什么，不要用 `update`、`fix bug` 这类无信息量的描述。

4. 面向用户的行为变化，请同时在 `CHANGELOG.md` 的 `[Unreleased]` 下补一行。

## 代码约定

这些不是风格偏好，而是这个项目已经踩过的坑，请务必遵守：

- **设备只认 `endpointId`**。不要用设备名、列表下标或枚举顺序定位设备，也不要做「按名称自动重新绑定」。
- **成功状态必须来自回读**。切换默认设备的唯一入口是 `audio::default_endpoint::set_default_and_verify`（设置 → 回读验证），任何「已切换」的判断都不能只依赖接口返回值。
- **不改 Communications 角色**。切换只更新 Console 与 Multimedia，避免悄悄改变会议软件的通话输出。这是与用户确认过的产品决定。
- **`IPolicyConfig` 的 vtable 必须显式声明**，不要换成不透明的第三方封装；`ComGuard` 必须活到对象释放。
- **前端不做业务判断**。界面状态一律来自 Rust 快照与事件推送（`audio-state-changed` / `audio-switch-result` / `ui-navigate`）。
- **配置只存用户意图**。运行态（图标状态、切换锁、索引缓存）不落盘；开机自启以系统注册表回读为准。
- **双语同步**。前端词表在 `src/i18n.ts`（`zh` 是基准表，`en` 必须写满同一批 key，漏翻会在 `vue-tsc` 阶段报错）；Rust 侧文案在 `src-tauri/src/i18n.rs`（托盘、提示、错误都走它）。新增用户可见文案时两侧都要加。
- **不引入 UI 框架**。样式集中在 `src/styles.css`，主题靠 `<html data-theme>` 上的 CSS 变量切换，不要在组件里写死颜色。
- **托盘图标状态用代次（generation）机制复位**，不要用固定 `sleep` 后无条件复位。

## 提交前检查清单

提交前请在本地跑一遍，维护者审阅时也会看这些结果：

```bash
npm run typecheck                       # vue-tsc 类型检查
npm run build                           # 类型检查 + 前端生产构建

cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

- 改到音频相关逻辑时，请补充或更新 `audio/cycle.rs`、`config.rs` 中的单元测试（纯函数，不需要真实设备）。
- 涉及界面的改动，请附上改动前后的截图（深浅色各一张更佳）。
- 涉及设备切换的改动，请在真实设备上验证一次，并在 PR 描述里写明「切到哪个设备、回读结果如何」。

## Pull Request

PR 描述请包含：

- **动机**：解决什么问题，关联 Issue 编号；
- **改动点**：关键文件与设计取舍；
- **验证方式**：跑了哪些命令、做了哪些手动验证、结果如何；
- **影响面**：是否改变配置文件格式、是否影响已有用户行为。

维护者可能需要几天时间回复，请耐心等待；如果 PR 长期没有回应，可以在 Issue 里礼貌 ping 一次。

## 许可

本项目以 [MIT](./LICENSE) 许可发布。提交 PR 即表示你同意你的贡献以同一许可授权。
