# diyige 首选“的一个”的诊断（2026-10-14）

状态：已复现，尚未修复。本轮不改学习数据、不安装输入法。该缺陷与外接决策模型分别处理。

## 确认到的行为

用户期望“第一个”，实际首选“的一个”。仅筛选这两个文本的本机输入日志，确认出现了 `keys=diyige`、`pinyin=di'yi'ge`、`source=sentence`、`top[0]=的一个`、`top[1]=第一个`。既有神经重排后的错误，也有未重排时的错误，不能归因于单一神经模型。

产品词库本身有 `第一个 / di yi ge`，不是缺词。干净 CLI 使用相同产品词库、bigram、随包小模型与严格全拼，`diyige`、`dyg`、`dyige`、`diyig` 首选均为“第一个”。

把当前学习文件复制到被忽略的 `target/verification/ordinal-personal/` 后，同一条 `diyige` 查询稳定输出“的一个”首选、“第一个”第二。CLI 的写回目标只设为副本，未对当前输入法数据进行删除或清空。

## 单因素消融

每组从同一份副本开始，仅省略一张表，其余输入与参数相同：

| 加载条件 | diyige 首选 |
|---|---|
| 完整个人数据 | 的一个 |
| 省略个人词频 `user.tsv` | 第一个 |
| 省略个人 n-gram `user-ngram.tsv` | 第一个 |
| 省略用户词 `user-words.tsv` | 的一个 |
| 省略输入串偏好 `user-choices.tsv` | 的一个 |
| 省略敲错表或个人英文词 | 的一个 |

本机复现命令（副本不可提交或上传）：

```sh
target/release/qingjian-cli \
  --config target/verification/strict-probe.toml \
  --dict data/generated/dict.qj --neural data/model/model.qjm \
  --user-dict target/verification/ordinal-personal/user.tsv \
  --limit 8 diyige
```

## 代码侧解释与修复边界

主词库中“的”同时有 `de` 和较低频的 `di`。`sentence::viterbi` 查询词图时保留了输入读音，但语言模型与个人 n-gram 按词文本计分；已认识的字词使用语言模型分，不再使用那个读音的词频作为兜底分。全局个人词频也按文本加分。`engine/query::plain_sentence` 产生整句后，`insert_sentence` 将其放到普通中文词之前；完整词“第一个”的输入串选择偏好不能直接约束这条整句的插入。

这解释了为什么“的 + 一个”能借常用文本与个人统计挤掉一个已存在的完整词。消融证明个人词频与个人 n-gram 的组合触发了这个样例；尚未用最小合成数据证明哪项计分修正能在完整回归集上安全解决它，不能把某个修复猜想写成已证实的唯一根因。

修复应保护读音与完整词的竞争关系，保留个人学习。不要直接禁掉“的一个”、删除“的”的 `di` 读音或清空用户记录：`deyige` 的“的一个”以及“的确／目的／的士”等都需继续可用。

修复验收需要：把当前失败缩成可公开的最小数据；覆盖无模型与神经重排、同步与异步、正确完整词的精确选择偏好；检查 `deyige` 合法组句、多音字正常用法、既有 `keneng/jineng/heneng` 回归。安装后仍由用户启用和实机确认。
