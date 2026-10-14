# 用决策模型改善中文首选（2026-10-14）

结论：采用“上文 + 合法候选 → 候选概率”的任务形态值得推进。准确率先过关，再在合格方案中比较时延、内存和能耗。当前 Qwen3.5-4B 的 [本机测量](mlx-context-benchmark-20261014.md) 证明短请求速度可用，但存在明确误提升，因此不接管首选；用户已叫停 9B，下载已终止并清理未完成文件。

这里的 Jev 是 TypeSafe 的决策模型，不是 JEPA。下面只引用厂商、项目作者与原始论文，不以同名聚合站或营销目录作为性能证据。本轮未安装下列模型、未调用付费接口，也没有将个人日志发往云端。

## 已接入 Jev 的开源输入法

用户进一步要求查看输入法本身的实现。检索后已阅读下面两个项目的实际请求、重排与交互代码；**首要接入参考改为 `Ideny42/jev-keyboard`**，通用 Jev-Reranker 降为补充。以下是源码核对，不是本机安装或 Jev 准确率实测。

### Jev Keyboard：首要参考

[`Ideny42/jev-keyboard`](https://github.com/Ideny42/jev-keyboard/tree/20624fd50f095bd61cbfccdd3aa2ed5c08237c0b) 是 macOS 鼠须管插件，当前为开发者预览版。

- **怎么用 Jev：** Rime 给出真实候选，后台把上文、未确认拼音及候选 ID 发送给 Jev Choice。默认请求把每个选项写成“候选文本 + 选中后的完整片段”，让模型判断接续自然度，而不是孤立词的常用程度。回包必须包含同一候选集合的完整概率，排序相同分数时保留原序。见 [`lib/ime-completion.mjs`](https://github.com/Ideny42/jev-keyboard/blob/20624fd50f095bd61cbfccdd3aa2ed5c08237c0b/lib/ime-completion.mjs)、[`lib/jev.mjs`](https://github.com/Ideny42/jev-keyboard/blob/20624fd50f095bd61cbfccdd3aa2ed5c08237c0b/lib/jev.mjs)。
- **何时请求、何时显示：** 启用后输入期间预取，默认防抖 150 ms、上文 20 字、9 个候选；Ctrl 是应用排序的按键，不是首次联网的按键。可开启结果返回后自动刷新。不能把这套默认操作直接当成青简的交互需求。见 [`配置`](https://github.com/Ideny42/jev-keyboard/blob/20624fd50f095bd61cbfccdd3aa2ed5c08237c0b/macos/lib/config.mjs)。
- **如何防串词：** 请求带会话、序列和候选快照；新输入取消旧请求，返回后再次核对当前输入及光标；20 秒短缓存的键包含候选快照和上文。见 [`bridge.mjs`](https://github.com/Ideny42/jev-keyboard/blob/20624fd50f095bd61cbfccdd3aa2ed5c08237c0b/macos/bridge.mjs)。
- **边界：** 所读 Lua 路径的上文来自连续上屏记录，`effective_context` 标为 `fallback`；不能仅因配置名为 `host` 就认为它读取了应用光标前全文。输入变化时校验也不等于青简要求的“用户已移动高亮后冻结”。见 [`jev_keyboard.lua`](https://github.com/Ideny42/jev-keyboard/blob/20624fd50f095bd61cbfccdd3aa2ed5c08237c0b/macos/rime/lua/jev_keyboard.lua)。

取用候选身份、完整片段提示、异步取消和快照校验的思路；保留青简已有的光标前文获取、严格全拼以及交互冻结。所读决策路径没有显式的模型概率与个人词频融合，仍需青简自己完成。仅参考原创桥接代码与设计，不引入其配套雾凇词库。

### Rime-Capsule-AI：能说明接法，不能直接搬运行路径

[`Love-Neko/Rime-Capsule-AI`](https://github.com/Love-Neko/Rime-Capsule-AI/tree/fab9f12809ddbef89f27da12156d17e36be9bb42) 使用小狼毫与 Python 桥接，界面主题称为 macOS 风格；不要据此把它当成原生 macOS 输入法。

- [`rerank.py`](https://github.com/Love-Neko/Rime-Capsule-AI/blob/fab9f12809ddbef89f27da12156d17e36be9bb42/jev_bridge/rerank.py) 将前 6 个候选作为 Jev Choice 选项，成功后把选中项移到第一，其余保留原序。虽然读取了概率与置信度，但没有按低置信度门控，也没有与个人频率做概率融合。
- [`local_ranker.py`](https://github.com/Love-Neko/Rime-Capsule-AI/blob/fab9f12809ddbef89f27da12156d17e36be9bb42/jev_bridge/local_ranker.py) 使用 SQLite 记录拼音、候选、上文及次数，按 `log1p(count)` 加分；还有手写“汽车→油箱”等规则。Jev 成功时首选直接由 Jev 覆盖，学习主要服务本地排序/回退，不能当成已解决“模型结合历史习惯”的证据。
- [`jev_filter.lua`](https://github.com/Love-Neko/Rime-Capsule-AI/blob/fab9f12809ddbef89f27da12156d17e36be9bb42/lua/jev_filter.lua) 在普通输入时就写请求，Tab 时读结果；未就绪会用 `os.clock()` 忙等至 1.2 秒，所谓 8 ms 微休同样是忙循环。返回文件只比较规范化拼音，没有校验上文与候选快照；同拼音不同语境可能读到旧结果。这两个实现点都不适合青简。

其源码确实调用远端 Jev，因此 README 的“打字数据完全本地闭环”不能作为实际数据流描述。这里不采用它的忙等、固定临时文件通信及手写语义规则。

### 不使用 Jev 的对照：MSIME / chinese-ime-lm

[`metasequoiaime/chinese-ime-lm`](https://github.com/metasequoiaime/chinese-ime-lm/tree/ed030178ebf7f361a08b57c6b179d512ea47042f) 提供中文输入法专用字级模型；项目报告 MSIME 已接入 4M/25M 两档重排。它不使用 Jev，也不是通用指令模型。其 [测量记录](https://github.com/metasequoiaime/chinese-ime-lm/blob/ed030178ebf7f361a08b57c6b179d512ea47042f/docs/measurements.md) 的主要启发是：判断候选是否覆盖完整拼音、区分词典命中与拼装路径，并分别统计纠正和误提升。

这份记录不同段落包含不同候选池口径，且存在用金标准等长过滤却称为运行时判据的矛盾，因此不采纳它的模型胜负与准确率数字。发布体积 4.5/25.5 MB 也不是本机内存测量。它值得借鉴评测方法和专用训练方向，不据此替换青简现有模型，更不把“词典命中永不重排”的门控搬来阻止强语境纠正同音词。

## 通用决策模型与重排项目

| 项目 | 已有能力 | 青简应取用的部分 | 不直接照搬的部分 |
|---|---|---|---|
| [hotchpotch/jev-reranker](https://github.com/hotchpotch/jev-reranker)（MIT） | Jev 列表、逐项、两两重排；可改任务指令，保留原候选索引，异步与取消清理 | 固定候选身份、共享上文、一次批量决策、结果诊断 | RAG 的“文档相关性”不是中文下一词自然度；默认 180 秒超时和重试策略不适合输入法，两两比较也太多 |
| [uspraveen/jev-reranker](https://github.com/uspraveen/jev-reranker) | 将多个模型判断合并成显式排序策略，用于记忆检索 | 模型信号与最终排序策略分开，方便历史偏好参与 | 不需要它的相关性、冲突、过期等多维记忆策略，先保持一个同音选择任务 |
| [Laya](https://github.com/NandhaKishorM/laya) + [laya-apple](https://github.com/tc3oliver/laya-apple)（Apache-2.0） | 多语言 checkpoint 322M；单次前向输出候选分数；MLX GPU 实现，另有 ANE 路径 | 本地小模型首选评测对象，固定多语言 checkpoint、先走 MLX；轻量专用决策头与领域微调流程 | 英文 checkpoint 不适合中文；M4 Max 的 ANE 成绩不能算成本机 M1 Pro 成绩；暂不引入双设备自动路由 |
| [jaredpalmer/kev](https://github.com/jaredpalmer/kev)（Apache-2.0 基座） | Qwen 基座加训练过的选项指针头，直接出分布；有 MLX 服务与概率一致性测试 | 指针头、状态前缀复用、校准及“决策模型专门训练”思路 | 4B 仍是大骨干，改成决策头不会消除读取上文的成本；本轮不下载它的模型 |
| [Nokia Applied Research / AnyJev](https://github.com/nokia-applied-research/AnyJev)（Apache-2.0） | 直接读取 LLM 分数、处理标签偏好与选项顺序偏差；另有蒸馏 checkpoint | 评测必须置换候选，原始 softmax 不视为校准置信度；领域蒸馏的方向 | 全轮换需多次前向，不能当成免费提准；检索时 MLX 后端仍列在路线图，不能宣传为现成 MLX 方案 |

输入法接入优先参考上面的 **Jev Keyboard**；**Jev-Reranker** 补充通用重排经验。本地 MLX 模型试验优先考察 **Laya-multilingual + laya-apple**。这些是可复用方向，不是已经通过中文同音词验收的产品。

## 模型供应商与其他可选实现

- **TypeSafe Jev**：官方 [模型文档](https://docs.typesafe.ai/models) 当前列出 `jev-1.13.0`，提供托管推理。官方说明英文是主训练语言，CJK 能处理但表现不等同英文；本轮未找到官方开放权重。它的 Choice 直接返回候选分布，与当前任务匹配。[官方原语说明](https://docs.typesafe.ai/introduction)
- **Fastino**：既有托管 [GLiDE](https://fastino.ai/blog/introducing-glide-the-first-thinking-decision-model)，也有开放权重 [GLiNER2.5-multi-Decide](https://huggingface.co/fastino/GLiNER2.5-multi-Decide)。前者能对难题追加推理，后者适合受约束的分类；英文 Decide 的成绩不能转算中文能力。当前优先级低于已有 MLX 路线的 Laya，但保留为准确率对照。
- **Knowledgator GLiClass**：[多语言模型](https://docs.knowledgator.com/docs/models/text-classification/gliclass/) 的训练语言包含中文，mini 约 288M。它是动态标签分类模型，值得作为专用选词微调基座；官方吞吐来自 RTX PRO 6000 Blackwell、batch=8，不当成本机交互延迟。

Jev 的 [发布文](https://typesafe.ai/blog/introducing-system-one-models-and-jev?render=full) 报告端到端 70–500 ms，并说明主要从美国西海岸测试。对于本机选型，只据此决定“值得测”，不使用其对长篇思考 LLM 的数十倍、百倍加速来估算输入法收益。[独立原始评测](https://arxiv.org/abs/2609.37647) 公开了跨任务、跨语言与选项置换结果，但未提供中文输入法首选的结论。

## 已核对的关键取舍

**速度来自减少工作，不来自“决策”这个名字。** 编码器与专用决策头可以一次得到多个选项分数，省掉逐 token 写解释或 JSON。若仍用完整 4B 骨干，读取每次新上文依旧要付出计算成本；AnyJev 的多轮去偏又会增加计算。

**受限输出保障候选合法性，不能保障选择正确。** TypeSafe [官方说明](https://docs.typesafe.ai/concepts/system-one) 明确区分概率校准与单次正确性。我们本机的标签法本来就只允许合法选项，仍会把“炒菜加少量食盐”选成“实验”。因此必须测“原本正确却被改错”，不能拿输出格式成功率充当准确率。

**历史偏好留在 Core，语境判断交给模型。** 初始试验使用有上限的对数偏好，最多改变 2 倍相对优势；语境明显时模型主导，语境近似时历史定首选。这个系数必须在保留集上调整，且只用明确选择作为高权重纠正信号，避免每次按空格接受后让错误自我强化。它也不能替代 [当前整句学习缺陷](ordinal-candidate-20261014.md) 的修复。

## 青简验证顺序

1. 先处理 `diyige → 的一个` 的现有缺陷，建立不依赖新模型的失败与修复证据。
2. 固定公开的中文候选评测集：明确语境、弱语境、常用虚词、序数词、多音字、专有名词、历史偏好冲突；候选置换，允许合理近义变体。随后才加入用户明确纠正的本机样例。
3. 对同一集合比较强模型/云端决策模型与本地专用决策模型；优先考察误提升与强语境正确率，再比较本机/网络 P50、P95、冷启动与资源占用。本轮只完成 Qwen3.5-4B 的实测。
4. 胜出模型接到独立的首选重排路径。现有 `qingjian-predict` 是补充新候选，布局层会去掉重复本地词，不能只填写一个地址就让它接管本地首选。
5. 保持即时候选，结果匹配当前拼音和上文才可提升；用户移动高亮或确认后冻结，超时不阻塞。不上来叠加“本地模型→云端→第三模型”的串行链，先验证一条主路径。

云端验证前仍需确定可用账户、模型和预算；本轮不读取现有密钥、不购买额度。公开例子可用于后续授权测试，个人历史数据继续留在本机。
