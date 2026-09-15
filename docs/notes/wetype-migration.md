# 微信输入法词频迁移（2026-09-15）

用户需要迁移个人词频与选词习惯，同时保持严格全拼，不以自定义短语为目标。需求入口见 [个人定制](../plan/personal-macos-customization.md)，操作与回退见 [工具说明](../../tools/wetype-migrate/README.md)。

## 数据归属与格式证据

本机源应用为 `/Library/Input Methods/WeType.app`，bundle ID 为 `com.tencent.inputmethod.wetype`，版本 1.4.3。原始数据属于 `~/Library/Application Support/WeType/`，不是微信聊天应用数据库。

把 `userDict/` 与 `engine/user_dict/` 共 61 个文件、122,308,299 字节复制到仓库外，逐个比对复制前后源文件与副本 SHA256，未发现漂移。最终迁移只读取 `userDict/v5/<用户目录>/` 的 LevelDB 活跃记录。

读取 `CURRENT` 指定的 MANIFEST、活跃 SST 和当前 WAL，按序号保留最新版本并处理删除标记，不混入废弃文件。使用固定版本 dfindexeddb 解析物理记录，自己遍历活跃文件：其 `GetRecords(use_manifest=True)` 路径在多个 L0 文件时覆盖中间列表，不能直接用于此次完整迁移。对 LevelDB 文件结构的核对参考 [Table format](https://fuchsia.googlesource.com/third_party/leveldb/+/HEAD/doc/table_format.md) 与 [Log format](https://fuchsia.googlesource.com/third_party/leveldb/+/HEAD/doc/log_format.md)。

字段解释结合副本解码及已安装二进制的静态符号、反汇编核验，没有附加到运行进程，也没有读取聊天记录。值封装为 payload 每字节加 6（模 256），尾随原 payload 长度 u32 LE 和 `5aa55aa5`；解码逐字节减 6，检查长度和魔数。

| 记录 | 核验出的字段 | 本次用途 |
|---|---|---|
| `!v2u!`，键为逗号分隔的完整拼音 | WordItemV2：版本 0、u16 词长、48 字节元数据、UTF-8 词；元数据首 u32 是使用计数，下一 u32 是时间 | 词频和读音 |
| `!u_d_v_p!`，键为拼音、0x01、词 | 版本 1 + 64 字节；DebugInfo 明确标注 exact_match、full_jianpin、other_completion、prefix_completion、typo、force_emit、time | 仅 exact_match 进入精确全拼偏好 |
| `!user_bigram2!`、`!user_prefix2!` 等 | 上下文语义与权重尚未完整核对 | 不导入 |

WordItemV2 的构造、Serialize/Deserialize 和 Reduce 操作共同确认首字段计数性质，未把另一浮点热度字段当计数。Patch 的 DebugInfo 明确列出上述字段顺序。静态调查输出留在忽略的 `target/verification/wetype-{deserialize,fields}.asm`，私人解码中间数据留在私有迁移目录。

## 映射边界

- 两张微信表可能覆盖同一历史，对同一（拼音、词）取较大计数，避免直接相加双计；不同读音再按词汇总为青简全局词频。这是保守的迁移映射，不代表还原微信输入法的完整评分公式或时间衰减。
- 精确选词计数映射为去掉音节分隔符的完整拼音 → 词 → 次数，不声称这是原始敲键日志。简拼、补全、纠错偏好不导入。
- 只接收青简完整音节表可识别、音节数与汉字数一致的 BMP 汉字词。零计数、混合英文与无法确认的拼音排除。
- 与青简已积累的计数相加；用户词读音保留既有值，新词按源计数最高读音建立。上下文、英文学习、纠错表及自定义短语不修改。
- 用户词用于让词条可被检索，真正的偏好来自词频与按输入串的学习表，因此不只是导入一份静态词表。严格全拼规则仍由 Core 约束。

## 验证与本机结果

- 解码 10,472 个基础词记录、25,901 个 Patch 记录；分别排除 8 和 1,651 条，最终迁移 **25,543 个词的个人词频、24,250 条精确拼音选词偏好**。
- 8 项 Python 测试通过，覆盖封装/格式拒绝、计数映射、完整音节边界、活跃记录与删除、既有数据合并、漂移与重复导入拦截、写入失败恢复。测试先暴露 Patch 键前缀偏一错误，另一次失败暴露音节读取误包含 INITIALS；修正后通过。日志：`target/verification/wetype-tests-before.log`、`wetype-syllables-before.log`、`wetype-tests-after.log`。
- 真实 Qingjian Engine 加载全部 24,250 条偏好并核对计数；取最高频的 128 组不同输入，源偏好词居首由 119 组增至 128 组，9 组排名改善。`liuchu` 无流传、留长、流畅。该离线检查未加载模型，不能等同于实体键盘验收或与微信全套排序逐项相同。日志：`target/verification/wetype-engine-final.log`。
- workspace 测试、全目标 Clippy、格式检查通过；详见本轮验证日志。Python 测试用合成词，没有把私人词条放入代码。
- 原位应用前保留青简现有数据备份；第一次应用因 macOS 自动拉起青简被工具正确拒绝，暂时停用输入源后成功。三份安装文件 SHA256 与暂存结果一致，写入一次性收据防止重导。
- 应用二进制未更换。青简恢复为当前输入源 `app.qingjian.inputmethod`，新进程 53356 的启动日志显示 `learned=25567`（含青简既有词频记录），产品词库、附加词库及本地模型加载完成。用户对迁移后日常候选体验的实体键盘验收尚待反馈。

本机私有目录：`~/Library/Application Support/Qingjian/migrations/wetype-20260915-134419/`。源副本在 `source/`，最终计划在 `prepared-final/plan.json`，导入前备份在 `prepared-final/backup/`。收据在青简数据目录的 `wetype-migration-receipt.json`。这些文件不提交 Git，回退按工具说明执行。
