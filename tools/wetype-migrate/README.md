# 微信输入法个人学习数据迁移

离线解析 macOS 微信输入法 1.4.3 的 `userDict/v5/<用户目录>/` 副本，生成青简的个人词频、精确全拼选词偏好和用户词。字段证据、映射边界与本机交付结果见 [迁移记录](../../docs/notes/wetype-migration.md)。这不是通用词库导入，也不是微信聊天数据解析器。

## 环境与验证

在仓库根目录执行；Python 3.12，LevelDB 只读解析使用 [dfindexeddb](https://github.com/google/dfindexeddb)。macOS 编译其 Snappy 依赖需要本地头文件和库：

```sh
brew install snappy
uv venv --python 3.12 target/wetype-parser-venv
CPLUS_INCLUDE_PATH="$(brew --prefix snappy)/include" LIBRARY_PATH="$(brew --prefix snappy)/lib" \
  uv pip install --python target/wetype-parser-venv/bin/python -r tools/wetype-migrate/requirements.txt
target/wetype-parser-venv/bin/python -m unittest discover -s tools/wetype-migrate -v
```

## 一次性迁移

1. 把微信输入法数据复制到仓库外的私有目录，校验源与副本的文件哈希一致。`--source` 必须指向含 `CURRENT` 的副本目录；不要对正在写入的原数据库操作。
2. 切换到其他输入源，让青简完成 `deactivateServer` 落盘，再停止青简。macOS 若自动拉起它，可通过系统输入源 API 暂时停用青简，完成后恢复。工具会拒绝在青简运行时应用迁移。
3. 生成独立计划。下列占位路径需要换成本机实际路径，输出目录必须尚不存在且位于仓库外：

```sh
target/wetype-parser-venv/bin/python tools/wetype-migrate/migrate.py prepare \
  --source '/private/snapshot/v5/user' \
  --qingjian "$HOME/Library/Application Support/Qingjian" \
  --out '/private/migration/prepared'
cargo run -p qingjian-cli --example check_migration --locked -- \
  "$HOME/Library/Application Support/Qingjian" '/private/migration/prepared'
target/wetype-parser-venv/bin/python tools/wetype-migrate/migrate.py apply \
  --plan '/private/migration/prepared/plan.json'
```

4. 重新启动并选中青简，查看加载日志，再用实体键盘核验常用词。验证程序使用真实产品词库与学习器，但不加载本地模型；它检查所有导入偏好权重、最高频的 128 组输入排序及 `liuchu` 严格匹配。它要求样本至少一组排序改善；其他用户若原先排序已全部符合偏好，应检查这一条件的失败原因。

## 写入与回退契约

- 只修改 `user.tsv`、`user-choices.tsv`、`user-words.tsv`；与青简既有计数相加，已有用户词读音保留。新增多音词只选源计数最高的一种读音，符合青简当前一词一读音的学习文件格式。
- 计划记录导入前、暂存文件及源数据库文件的 SHA256；准备或应用时发现漂移就拒绝，重新生成计划。格式版本、长度或未验证的历史 WAL 状态不符时也拒绝。
- 应用前把原文件备份到计划旁的 `backup/`。每个文件用临时文件、fsync 和原子替换落盘；可捕获的中途异常恢复已写文件。进程被强杀或断电不能保证三个文件一起回退，需要用备份人工恢复。
- 成功后写 `wetype-migration-receipt.json`，阻止重复导入造成词频翻倍；不支持增量迁移。
- 回退时先让青简落盘并停止，再依 `plan.json` 的 `before` 恢复三个文件（原先不存在的文件删除），核对哈希，最后移除收据并重启。回退会放弃迁移之后积累的相关学习记录，应先另存当前文件。
- 私人数据库、词条、计数、计划和备份都留在仓库外；命令只输出统计数字。原微信输入法数据不写入。
