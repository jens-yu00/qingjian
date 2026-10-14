# 中文决策模型筛选（2026-10-14）

结论：**中文质量的云端优先对照为 Microsoft-Decision-1 与 TypeSafe Jev；本地首个 MLX 试验选 Decision-2.0-Sol-2B。** 本地选择基于现成决策头、MLX 路径和资源规模，尚无足够中文同音词证据把它称为胜出模型。Laya 降为轻量对照，不再因支持 MLX 就默认优先。行为方案见 [语境挑战者与个人学习](../design/llm-first-homophones.md)。

本轮核对作者模型卡、源码仓库和原始评测；未下载新权重、未执行新模型、未调用收费服务。外部成绩只是入围理由，不能替代青简的同音词回放。

## 入围与排除依据

| 模型 | 中文证据与部署事实 | 本轮决定 |
|---|---|---|
| Microsoft-Decision-1（云端） | 社区原始评测的 200 道 Belebele 中文阅读为 194/200；官方确认是专用决策模型，通过 Foundry/OpenRouter 提供 | 与 Jev 做相同输入法样例的准确率对照；不下载基座 |
| TypeSafe Jev 1.13.0（云端） | 同一评测的同一中文切片为 188/200；已有 Rime 候选重排源码参考 | 云端共同入围，尤其比较实际等待时间与候选顺序稳定性 |
| Decision-2.0-Sol-2B（本地） | 1.88B 参数、专门训练的决策头；社区有完整 MLX q4 转换，包约 1.12 GB；没有已核实的中文同音词成绩 | 本地首测；先测中文质量，再测 M1 Pro 速度；不以 Qwen 基座替代中文证据 |
| Decision-1.0-Kai-0.6B（本地） | 作者 54 任务矩阵中 MASSIVE 中文 79.17%，Belebele 中文仅 31.88%；所读默认推理为 FP32 Transformers | 路由强不能推定接续选词强，暂不优先；不要和微软 Decision-1 混同 |
| Laya-multilingual 322M（本地） | 有专门中文评测及 MLX 实现，但作者收录的中文试验暴露高置信错误和候选顺序敏感 | 保留轻量/训练基座用途，排在 Sol 之后 |
| Chinese-Jev（中文专项） | 论文与训练代码已发布，仓库明确说明模型权重和真实数据尚未提供 | 等待可用权重；不能列为现在可下载的成品 |
| GLiNER2.5-multi-Decide 287M | 多语言开放权重，但模型卡明确公开主榜是英文测试 | 中文质量证据不足，不因英文榜好看而入选首轮 |
| 百炼 decision-model-preview / Bocha Jev | 官方有结构化决策服务；本轮未找到足够可核验的中文选词对比 | 保留候选，不按供应商所在地推定中文最强 |

## 哪些成绩支持上述决定

### 云端中文对照

[haxudev/jev-benchmark](https://github.com/haxudev/jev-benchmark) 公开固定样本清单和逐题结果。2026-10-12 的同场对照，Microsoft 在 200 道中文阅读中比 Jev 多对 6 题，但这不是统计显著的全面胜出证明；自然语言推断一项反而是 Jev 更好。100 道婚恋题上的高分区分度不足，不拿它决定输入法模型。

同一作者的短外部任务中位时延约为 Jev 254 ms、Microsoft 1,309 ms。它们说明需要核对质量与网络等待的交易，不是本机网络预报。两者均未在我们的同音词任务上调用，不能按该数字宣判谁一定够快。

[Microsoft 官方发布](https://commandline.microsoft.com/microsoft-decision-1-model-foundry/) 确认模型以 Qwen3.5-9B 后训练得到；本轮仅研究云端服务，不恢复用户叫停的 9B 下载与本机测试。[TypeSafe 模型说明](https://docs.typesafe.ai/models) 仍提醒英文训练优势，不能据此忽略实际中文测量。

### 本地 MLX 候选

[Sol 官方模型卡](https://huggingface.co/vllm-sr/Decision-2.0-Sol-2B) 给出候选决策任务与开放权重；[jbilcke 的 MLX 转换](https://huggingface.co/jbilcke/vllm-decision-2.0-mlx) 包含训练过的决策头，而不只是把通用模型限制成输出一个字母。其作者在 M4 Air 的 22 个自编通用决策例子上报告 Sol q4 22/22、热态中位约 215 ms、MLX 峰值约 1.54 GB。这是小样本社区测量，既不是中文专门结果，也不是 M1 Pro 实测。

转换作者还只用 5 个短例子核对主要数值一致性，开发环境是 macOS 27。青简本机仍需检查兼容性、量化后的选词变化和端到端延迟；下载必须限定 `sol/q4`，不把整个模型合集拉下来。此处只形成方案，没有执行下载。

更近的 [Decision 3.0 d3-nano](https://huggingface.co/vllm-sr/d3-nano) 官方通用榜更高，但包含视觉能力，所读交付路径为 Transformers/服务器推理，未确认可直接用的 MLX 转换。首轮不为追新增加移植工作；若 Sol 质量不过关，再按中文失败集决定是否值得移植。

### 为什么修正此前的 Laya 优先建议

[Laya 作者评测页](https://github.com/NandhaKishorM/laya/blob/main/BENCHMARKS.md) 收录中文多任务结果、重跑修正以及顺序影响，不能只挑其中一个好看的分类分数。具体 [中文短指令原始试验](https://github.com/NandhaKishorM/laya/blob/main/research/benchmarks/zh_short_commands/README.zh-CN.md) 中，模型会高置信地误解“太慢了”的纠正方向。输入法首选挑战必须能处理细微语义，原始置信度不能直接成为门槛。

[Kai 的完整任务矩阵](https://huggingface.co/vllm-sr/Decision-1.0-Kai-0.6B/blob/main/evaluation/BENCHMARK.md) 进一步说明任务差异：表头依次为 Kev、Qwen、Kai、Laya English、Laya Multilingual；Kai 的中文阅读是第三列 31.88%，不能误读成第一列 58.75% 或第二列 81.25%。它的综合分超过 Laya，不代表阅读与上下文选择足够强。Decision 1.0 Kai 编码器和 Decision 2.0 Kai Qwen 路线也不能混用分数。

### 中文专项与多语言标签的区别

[Chinese-Jev 官方仓库](https://github.com/gulucaptain/Chinese-Jev/blob/aed6193be5697488c8d0c6a77cc0c7e297329e61/README_zh.md) 写明权重待开源；[论文](https://arxiv.org/abs/2609.36965) 表明中文领域适配值得做，但未提供可供我们立即复测的成品。优先借鉴中文数据构造、校准与留出验证，暂不自行投入训练。

[GLiNER2.5-multi-Decide 模型卡](https://huggingface.co/fastino/GLiNER2.5-multi-Decide) 的分类榜明确是英文，不用它替代中文质量证据。[百炼官方说明](https://help.aliyun.com/zh/model-studio/decision-model-api) 与 [Bocha 官方页面](https://jev.bocha.cn/) 能确认服务存在，本轮没有它们在相同中文任务上的足够质量数据。

## 可交付的验证结论应是什么

最终应选出一条主路径，并给出：修复基础排序后的原首选表现、模型及平局学习加入后的净纠正/误改、候选置换稳定性、实际能赶上上屏的比例、端到端时延与本机资源开销。

若 Sol 不能满足质量门槛，停止把它作为产品首选；若云端候选达标且等待可用，选云端。若均未达到 ≤0.5% 的目标或几乎从不干预，保留规则修复与旧排序，不宣称智能升级完成。云端费用、账户和允许发送的上文范围在实际启用前确认。
