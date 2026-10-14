# Sol 2B 本机试用工具

这是专用分类决策头，不调用生成接口，不启动 HTTP 服务。模型版权声明随 bundle 保留，输入仅通过输入法与本机 Python 的私有管道传递，不落盘。

先准备已验证的 `jbilcke/vllm-decision-2.0-mlx` Sol q4 bundle（集合 revision `623e99bfb301402ac98ae11341510eda133ec901`）。在仓库根目录运行：

```sh
python3 tools/sol-trial/install-runtime.py target/verification/decision-models/sol/q4
cargo run --release -p qingjian-cli --example sol_trial -- \
  "$HOME/Library/Application Support/Qingjian/sol" \
  tools/decision-bench/cases.json target/verification/sol-trial-replay.json
```

安装器只安装本机运行目录，不改配置、不安装应用或切换输入源。需本机 `uv` 和 Python 3.12，依赖版本固定在 `requirements.txt`。已存在的同名运行目录会更新；运行中的模型应先从偏好设置关闭。运行目录独立于仓库与 `target/`，模型不会在打字时下载。

应用配置 `[model] enabled = true, sol = true`（TOML 中分别写两行）使用此后端，`enabled = false` 关闭；`sol = false` 恢复旧字级后端。异常后通过本地模型总开关重新加载。

回放只使用公开语境，不读取个人学习库；评分目标仍是整个产品首选。设计、模型契约、结果与试用范围见 [设计](../../docs/design/sol-trial.md)、[记录](../../docs/notes/sol-trial-20261014.md)。
