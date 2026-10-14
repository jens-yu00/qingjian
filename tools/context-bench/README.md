# 本地大模型同音词决策测速

本工具只使用公开合成语境，不读取个人学习数据、不修改青简配置。测量的是拟议的 MLX 首选决策路径；当前输入法尚未接入此路径。

## 环境与模型

```sh
uv venv --python 3.12 target/verification/mlx-benchmark-venv
uv pip install --python target/verification/mlx-benchmark-venv/bin/python 'mlx-lm==0.32.0'
```

模型：`mlx-community/Qwen3.5-4B-4bit`，固定 revision `0e7ffd5c629ef7719d4cbc04069232580bfa9d9c`。使用 `huggingface_hub.snapshot_download` 保存到 `target/verification/mlx-models/Qwen3.5-4B-4bit`；需要 JSON、Jinja 与 safetensors 文件。依赖和权重留在被 Git 忽略的 `target/`，不提交。

## 对照与测速

```sh
cargo run --release -p qingjian-cli --features qingjian-neural/metal \
  --example homophone_probe -- target/verification/homophone-baseline.json

target/verification/mlx-benchmark-venv/bin/python tools/context-bench/benchmark.py \
  --model target/verification/mlx-models/Qwen3.5-4B-4bit \
  --cases target/verification/homophone-baseline.json \
  --out target/verification/mlx-context-benchmark.json
```

先用产品词库、bigram 和已交付的小模型查询 16 个公开样例，导出每个拼音的前 12 个完整候选。随后 4B 模型只在同一集合中选首选；每次改变选项位置，避免把本地顺序照抄当成语境能力。

输出记录第一次加载、第一次请求，以及四种测量：

- `uncached`：热模型，但每次完整处理提示词和上文。
- `system_prefix`：只复用固定系统指令前缀，上文每次重新处理。
- `context_prefix`：当前上文已预处理；预处理耗时独立记录，不能当成免费。改变拼音和候选后只计算新后缀。
- `new_long_context`：256 字背景加当前上文，只有系统指令缓存。用于检查新上文的耗时，准确率不单独用于选型。

思考关闭。读取模型下一 token 对候选标签的 logits，约束在合法标签内选取并归一化。标签分布是用于相对排序的模型分数，不是经过校准的正确率；同时记录未约束输出和标签总概率质量。MLX 显式同步后计时，包含分词、缓存副本和读取分数，不包括 HTTP、壳的防抖及绘制。

固定前缀缓存逐 token 验证；缓存只保存上文和指令，不复用答案。MLX 分配器缓存限制为 256 MiB；峰值 MLX 分配和进程 RSS 分开记录。时延以毫秒计，内存用十进制 GB。

这些样例用于行为对照与速度测试，不代表用户日常准确率。新测量无需访问云 API，不消耗付费 token。设计与证据见 `docs/design/llm-first-homophones.md` 和 `docs/notes/mlx-context-benchmark-20261014.md`。

## 直接输出候选词的对照

```sh
target/verification/mlx-benchmark-venv/bin/python tools/context-bench/word_probe.py \
  --model target/verification/mlx-models/Qwen3.5-4B-4bit \
  --cases target/verification/homophone-baseline.json \
  --out target/verification/mlx-word-probe.json
```

同一候选集合与三次位置轮换，去掉字母编号，贪心输出候选词，最多 8 token；固定指令前缀预热。统计包含输出词与结束符的完整时间，并检查输出是否属于候选。它不能提供全部候选的概率，未验证个人偏好融合。与标签路径的差异同时包含提示词长度和推理入口，不能把速度差全部归因于输出形式。

两份报告的 P50 使用标准中位数，P95 使用 nearest-rank。`correct` 是对预设首选的精确匹配；“制定／制订”这种合理变体需在人工结果解释中分开说明。标签报告的 `habit_2x_*` 是给旧首选施加 2 倍相对偏好的合成检查，不读取个人词频。

固定原始样例、逐项分数、时延、环境和依赖位于 [results/20261014](results/20261014/)。这些结果全部来自公开合成语境，与私有输入日志诊断分离。该次测量发现明确误提升，4B 未作为默认首选部署。
