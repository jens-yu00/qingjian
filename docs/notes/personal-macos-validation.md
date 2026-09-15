# macOS 个人定制验证

## 当前状态（2026-09-15）

用户已提供青简实际候选截图并反馈 `liuchu` 过度扩展，说明已进入实际使用阶段。下文首次注册失败属于历史诊断，不再作为当前阻塞；截图不能证明所有快捷键与跨应用验收均已完成。

匹配修复与本轮交付结果见文末。

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

## 首次安装时的系统验收缺口（历史）

`qingjian-macos --register` 返回“输入源启用没有生效（等了 30 秒）：app.qingjian.inputmethod”。独立新进程查询 TIS：该输入源存在，支持启用和选择，但 `IsEnabled = false`。重开系统设置后，添加输入源页面仍搜索不到青简。原有微信输入法仍是当前输入源。

该轮尚不能宣称快捷键在真实输入会话中的来回切换、跨应用持久化、重启后恢复、组句高亮保持和全屏显示已经实机通过。原生窗口预览不能代替这些检查。

上游开发安装文档给出的下一步是注销并重新登录，再到“系统设置 → 键盘 → 文本输入 → 编辑 → +”添加青简。该轮没有自动注销会话；重新登录后需继续完成上述验收。若仍无法添加，继续诊断注册流程，不改系统安全设置或直接篡改系统输入源偏好。

## 回退与继续验证

- Git：`main` 跟踪 `upstream/main`，个人改动在 `codex/macos-personal-customization`，`origin` 指向个人 fork。用独立分支同步上游与回退，不改写 main 历史。
- 安装：切回原输入法即可停止使用定制版；如要卸载，先在系统输入源中移除青简，再将用户级 Qingjian.app 移至废纸篓。保留 `~/Library/Application Support/Qingjian/` 可保留配置和学习数据。
- 实机测试只使用新建的测试文档，不向聊天、邮件或其他外部系统发送内容。


## 严格全拼匹配（2026-09-15）

### 问题证据与改动

用户截图：[liuchu 修复前候选](../file-4dc0ce536551b745a6e1ff0f5caa6703.png)。产品 CLI 也复现了流传、留长、流畅等候选：完整 chu 被放宽成前缀，并额外采用 liu + c + hu 的简拼切法。此问题不仅是自动纠错。

新增“通用 → 严格全拼匹配”，Core 在候选生成阶段收紧匹配，保留同音词、完整拼音歧义与逐段选词；具体契约见 [需求入口](../plan/personal-macos-customization.md)。

### 自动化证据

- 第一轮 7 项真实引擎测试：接入空开关时 5 失败 / 2 通过，实现后 7 通过；日志 `target/verification/strict-before.log`、`strict-after.log`。失败分别覆盖完整音节补长与简拼另解、合法音节纠错、整段纠错缓存、模糊音、云端词。补全和歧义/分段确认是原本已通过的保护项。
- 再补充其他输入方案与英文模式保护、配置旧值与持久化契约；全仓库最终 404 通过 / 0 失败 / 1 上游模型延迟测试忽略。日志 `target/verification/strict-workspace-tests.log`。
- `cargo fmt --all --check`、`cargo clippy --all-targets --locked -- -D warnings`、`git diff --check` 通过。
- 产品词库 CLI（无个人学习数据）：liuchu → 流出、六畜和较短前缀词，无流传/留长/流畅；liuchuan → 流传；liuch 仍补全；meiganxi → 没敢洗，不再变成没关系。截图中的“留出”来自用户数据等差异，固定回归词库另行覆盖了它；不要求冷启动排序等同个人排序。
- 加载随包本地模型重复验证 liuchu、liuchuan、meiganxi，候选仍符合相同拼音范围。日志 `strict-product.log`、`strict-neural.log` 位于 `target/verification/`。此为同步 CLI 检查，不代表 IMK 延迟。
- 复现命令：`cargo run -p qingjian-cli --locked -- --config target/verification/strict-config.toml --limit 12 liuchu liuchuan liuch meiganxi`；配置仅含 `general.strict_pinyin = true` 与 `predict.enabled = false`。加 `--neural data/model/model.qjm` 验证本地模型。

### 系统交付

- 已从代码提交 `1c43beb` 构建 `0.1.3-dev` / build 67，沿用原来的 debug 开发构建；安装到 `~/Library/Input Methods/Qingjian.app`。
- 应用与配置的安装前备份：`~/Library/Application Support/Qingjian/backups/strict-pinyin-20260915-133556/`，分别为 `Qingjian.app` 和 `config.toml`。备份不进入仓库。
- 本机配置启用 `general.strict_pinyin = true`；解析比较确认其他配置值未变。缩放、标点、行内显示、模型与云服务配置保留。
- 打包产物与安装二进制 SHA256 均为 `6ccb07564723d6f29a779f065e331aa082f2477d218784495e1bed07f88ecafc`；ad-hoc 签名深度严格检查通过。
- 旧进程退出后新版已启动，日志确认产品词库、本地模型加载完成。原生 TIS 选择 ABC 再选择青简均返回成功，最终输入源为 `app.qingjian.inputmethod`。
- 自动化 TextEdit 按键只产生字母，未能可靠触发候选窗口，该自动化结果自身不构成实机通过证据。随后用户用实体键盘核验 liuchu，并回复“已经消失”，确认流传／流畅等近似候选已消失；这个实际反馈补足了主问题的实机验收。测试文档保存到 `target/verification/strict-input-automation.rtf` 并关闭，原有文档保留。
- 如需恢复匹配，取消“通用 → 严格全拼匹配”；如需恢复旧应用，先切换其他输入法，再用备份替换用户级应用并重新切回青简。仅回退匹配无须替换词库或学习数据。

用户的实机确认范围是本次 liuchu 候选修复；其他拼音边界由自动化测试覆盖，早期标点快捷键的跨应用、全屏等项目未在本轮重新验收。

## 微信输入法学习迁移（2026-09-15）

已完成个人词频与精确全拼选词偏好迁移，并重启加载；来源、字段证据、数量、引擎验证及回退路径统一维护在 [微信迁移记录](wetype-migration.md)。本轮仅更新学习数据，应用仍为严格全拼交付的 `1c43beb` 构建。
