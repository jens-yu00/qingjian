# 专用分类决策模型：中文同音词实测

使用公开自编语境、青简真实候选和专门训练的决策头。没有通用 LLM 生成或 API 调用，不读取个人输入日志、学习数据或服务密钥，不修改已安装输入法。

## 冻结输入与口径

`cases.json` 共 108 条：16 条既有开发样例、92 条新评测样例。开发/评测按拼音词族隔离；同一词族的反向语境在同一分组。包含日常用词、计算机词、同拼音反义语境、否定对照、弱语境可接受答案集合，以及 yange/diyige 回归。这是小规模候选模型筛选，不是设计中 200 条开发 / 1,000 条独立验收集，也不代表日常输入分布。

先导出实际前 12 个完整候选，再冻结标注与指令后运行模型。`expected_in_pool=false` 单列为召回失败，模型不补造答案。仅一个候选时跳过推理、沿用原首选。使用当前已修复的严格全拼引擎、产品词库、bigram 和随包小模型作为基线；基线不含私人词频。候选顺序分别为原序、反序、固定种子乱序；三次是同一语境的稳定性检查，不能算三倍独立样本。

原始 JSON 保留每次 logits、概率、输入 token 哈希、时延及首选。`label-review.json` 专门记运行后发现的标注问题；原始答案不覆盖，报告区分原始和语义复核计分。复核统计通过给 `analyze.py` 添加 `--label-review tools/decision-bench/label-review.json` 输出到独立的 `*-audited-summary.json`。

## 模型与运行

Python 3.12，依赖固定于 `results/20261014/requirements.txt`。模型由社区 MLX 转换提供，带专门训练的 FP32 决策头、4-bit 编码骨干；使用作者 `load_bundle/encode/answer` 的分段选项协议，不使用聊天模板或文本生成入口。源码已阅读，Sol/Nox 的运行代码相同，加载器验证清单与权重哈希。

```sh
uv venv --python 3.12 target/verification/decision-venv
uv pip install --python target/verification/decision-venv/bin/python \
  -r tools/decision-bench/results/20261014/requirements.txt
```

使用 `huggingface_hub.snapshot_download` 下载 `jbilcke/vllm-decision-2.0-mlx`，固定 revision `623e99bfb301402ac98ae11341510eda133ec901`，仅允许 `sol/q4/*` 或 `nox/q4/*`，保存到 `target/verification/decision-models`。不要下载整个合集。权重及虚拟环境不提交。

```sh
cargo run --release -p qingjian-cli --features qingjian-neural/metal \
  --example homophone_probe -- tools/decision-bench/results/20261014/baseline.json \
  tools/decision-bench/cases.json

target/verification/decision-venv/bin/python tools/decision-bench/benchmark.py \
  --model target/verification/decision-models/sol/q4 \
  --cases tools/decision-bench/results/20261014/baseline.json \
  --out tools/decision-bench/results/20261014/sol-q4.json

python3 tools/decision-bench/analyze.py \
  --cases tools/decision-bench/results/20261014/baseline.json \
  --report tools/decision-bench/results/20261014/sol-q4.json \
  --out tools/decision-bench/results/20261014/sol-q4-summary.json
python3 -m unittest discover -s tools/decision-bench -p 'test_*.py'
```

Nox 使用同样命令，只替换模型目录和报告名。报告只提交最终 JSON，运行中增量 JSONL 留在本地。

## 挑战门槛与统计

`analyze.py` 仅在开发集考察固定 logit 差网格 `[0, .5, 1, 2, 4, 8, 16]`，要求三种候选顺序均不误改、且原序有正净收益；优先净纠正多者，再选较小门槛。若没有合格门槛则停用挑战。冻结后的同一门槛才用于评测集，不因看到评测错误而回调；全局高频保护与个人平局学习还没接入，不能称作完整产品策略达标。

主准确率只按原候选顺序、每个语境一次计数；误改分母是原首选正确的语境，净收益为纠正数减误改数。另报候选召回、三种顺序稳定性和误改率的近似 95% Wilson 区间。三个统计测试以手算结果验证纠正/误改、合理变体、单候选、门槛边界和零观测误改的非零区间上界；它们是新工具的正确性检查，没有既有实现故障可作修复前对照。

单模型在一个进程中驻留，逐条运行，不使用跨请求 KV 缓存。热态时延包含序列化、分词、整段推理、同步读取和决策头，不含模型加载、输入法防抖和 UI；冷加载、首次请求另报。MLX 缓存上限 256 MiB；MLX 峰值和进程 RSS 各自记录，口径不同不可相加。时延没有推算成耗电百分比。
