# macOS 个人定制验证（2026-09-15）

## 已实现

- 候选窗口整体缩放 100%–250%，25% 步进，缺省 150%，原生设置入口及配置保存。
- 中文模式下 Option + Shift + 句号切换标点偏好；沿用 `general.full_width_punctuation`，跨应用共享保存状态，组句确认与英文直输规则保持上游现状。
- 配置方案见 [个人定制计划](../plan/personal-macos-customization.md)，用户操作见 [偏好设置](../user/settings/preferences.md)。

## 已取得的证据

- 新增配置契约测试：原始实现 4 项失败，新增功能后 4 项通过。分别覆盖旧配置缺项默认值、配置往返保真、非法倍率拒绝、句号与双修饰键解析。日志在 `target/verification/preferences-before.log` 与 `preferences-after.log`。
- `cargo test --workspace --locked`：395 项通过，0 失败，1 项上游标记忽略的模型延迟测试未运行；日志在 `target/verification/workspace-tests.log`。全目标 Clippy（警告视为错误）与格式检查通过。
- 原生 AppKit 验证程序 `cargo run -p qingjian-macos --example candidate_preview --locked -- target/verification`：横排／竖排分别验证 7 个缩放档位，窗口尺寸、逻辑坐标系和屏幕右下边缘定位均通过；150% 竖排与 250% 横排 PNG 已人工检查，文字、译文、高亮与云图标无裁切。该程序使用真实候选绘制代码、固定示例内容，不需要启用输入法。
- 原生 NSEvent 验证：Option + Shift + 句号能读回基础字符 `.`，避免把 Shift 后的 `>` 当快捷键键名。
- 上游词库数据包和模型通过上游 SHA256SUMS 校验：词库 `e045d0f28236d0193087cf11cf35584327ebf85388310a5e2ceb885c0f7a6438`；模型 `eed5bd0bda0c7bd8b43d1acb2dc4678d4bbe295bd47b2b0d4eeace0af9daff4d`。
- 应用打包及 ad-hoc 签名验证通过，已复制到用户级 `~/Library/Input Methods/Qingjian.app`。安装前两个系统认可目录均不存在青简，也不存在青简用户配置，因此没有覆盖旧应用或配置。

## 未完成的系统验收

`qingjian-macos --register` 返回“输入源启用没有生效（等了 30 秒）：app.qingjian.inputmethod”。独立新进程查询 TIS：该输入源存在，支持启用和选择，但 `IsEnabled = false`。重开系统设置后，添加输入源页面仍搜索不到青简。原有微信输入法仍是当前输入源。

因此尚不能宣称快捷键在真实输入会话中的来回切换、跨应用持久化、重启后恢复、组句高亮保持和全屏显示已经实机通过。原生窗口预览不能代替这些检查。

上游开发安装文档给出的下一步是注销并重新登录，再到“系统设置 → 键盘 → 文本输入 → 编辑 → +”添加青简。本次没有自动注销会话；重新登录后需继续完成上述验收。若仍无法添加，继续诊断注册流程，不改系统安全设置或直接篡改系统输入源偏好。

## 回退与继续验证

- Git：`main` 跟踪 `upstream/main`，个人改动在 `codex/macos-personal-customization`，`origin` 指向个人 fork。用独立分支同步上游与回退，不改写 main 历史。
- 安装：切回原输入法即可停止使用定制版；如要卸载，先在系统输入源中移除青简，再将用户级 Qingjian.app 移至废纸篓。保留 `~/Library/Application Support/Qingjian/` 可保留配置和学习数据。
- 实机测试只使用新建的测试文档，不向聊天、邮件或其他外部系统发送内容。
