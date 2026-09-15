"""生成和应用可回退的个人词频迁移；私人输出必须放在仓库外。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from wetype import load_records, parse_records, syllables_from_repo

FILES = ('user.tsv', 'user-choices.tsv', 'user-words.tsv')
RECEIPT = 'wetype-migration-receipt.json'
REPO = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def rows(path, columns):
    if not path.exists():
        return []
    result = []
    for line in path.read_text().splitlines():
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        parts = line.split('\t')
        if len(parts) != columns:
            raise ValueError('unexpected Qingjian data columns')
        result.append(parts)
    return result


def number(value):
    count = int(value)
    if not 0 <= count <= 0xffffffff:
        raise ValueError('count outside uint32')
    return count


def merge(qingjian, frequencies, choices, readings):
    counts = {text: number(n) for text, n in rows(qingjian / FILES[0], 2)}
    existing_choices = {(code, text): number(n) for code, text, n in rows(qingjian / FILES[1], 3)}
    words = {text: (pinyin, number(n)) for text, pinyin, n in rows(qingjian / FILES[2], 3)}
    for text, count in frequencies.items():
        counts[text] = number(counts.get(text, 0) + count)
    for (pinyin, text), count in choices.items():
        key = (pinyin.replace(' ', ''), text)
        existing_choices[key] = number(existing_choices.get(key, 0) + count)
    for text, pinyin in readings.items():
        words.setdefault(text, (pinyin, 100))
    return {
        FILES[0]: '# 青简用户词频：词\\t选择次数\n' + ''.join(f'{word}\t{count}\n' for word, count in sorted(counts.items())),
        FILES[1]: '# 青简按输入串记的选择：输入串\\t词\\t次数\n' + ''.join(f'{code}\t{word}\t{count}\n' for (code, word), count in sorted(existing_choices.items())),
        FILES[2]: '# 青简用户词：词\\t拼音\\t词频\n' + ''.join(f'{word}\t{pinyin}\t{count}\n' for word, (pinyin, count) in sorted(words.items())),
    }


def prepare(source, qingjian, out):
    if out.is_relative_to(REPO):
        raise ValueError('private output must be outside repository')
    if (qingjian / RECEIPT).exists():
        raise ValueError('WeType migration already applied; incremental merge is unsupported')
    out.mkdir(mode=0o700)
    before = {name: digest(qingjian / name) for name in FILES}
    records, provenance = load_records(source)
    frequencies, choices, readings, stats = parse_records(records, syllables_from_repo(REPO))
    if not frequencies or not choices:
        raise ValueError('no verified personal frequencies or exact choices')
    output = merge(qingjian, frequencies, choices, readings)
    for name, contents in output.items():
        (out / name).write_text(contents)
    # 只用于验证同一拼音下偏好是否进入引擎，不是原始敲键日志。
    with (out / 'expected-choices.tsv').open('w') as f:
        for (pinyin, word), count in sorted(choices.items()):
            f.write(f'{pinyin.replace(" ", "")}\t{word}\t{count}\n')
    if before != {name: digest(qingjian / name) for name in FILES}:
        raise ValueError('Qingjian data changed during preparation; prepare again')
    plan = {'format': 1, 'qingjian': str(qingjian), 'before': before,
            'after': {name: digest(out / name) for name in FILES}, 'source': provenance, 'stats': stats}
    (out / 'plan.json').write_text(json.dumps(plan, ensure_ascii=False, indent=2))
    print(json.dumps(stats, ensure_ascii=False))


def atomic_write(path, data):
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as f:
        temp = Path(f.name)
        try:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())
            os.replace(temp, path)
        finally:
            temp.unlink(missing_ok=True)


def apply(plan_path):
    plan = json.loads(plan_path.read_text())
    if plan['format'] != 1 or set(plan['before']) != set(FILES) or set(plan['after']) != set(FILES):
        raise ValueError('unsupported migration plan')
    folder = plan_path.parent
    target = Path(plan['qingjian'])
    if (target / RECEIPT).exists():
        raise ValueError('migration already applied')
    if subprocess.run(['pgrep', '-x', 'qingjian-macos'], stdout=subprocess.DEVNULL).returncode != 1:
        raise ValueError('stop Qingjian after switching input source and flushing learning data')
    if plan['before'] != {name: digest(target / name) for name in FILES}:
        raise ValueError('Qingjian data changed; prepare a new plan')
    if plan['after'] != {name: digest(folder / name) for name in FILES}:
        raise ValueError('staged migration data changed')
    backup = folder / 'backup'
    backup.mkdir(mode=0o700)
    original = {name: (target / name).read_bytes() if (target / name).exists() else None for name in FILES}
    for name, data in original.items():
        if data is not None:
            (backup / name).write_bytes(data)
    written = []
    try:
        for name in FILES:
            atomic_write(target / name, (folder / name).read_bytes())
            written.append(name)
        if plan['after'] != {name: digest(target / name) for name in FILES}:
            raise ValueError('installed data hash mismatch')
        atomic_write(target / RECEIPT, json.dumps({'plan': str(plan_path), 'before': plan['before'], 'after': plan['after']}).encode())
    except Exception:
        for name in reversed(written):
            if original[name] is None:
                (target / name).unlink(missing_ok=True)
            else:
                atomic_write(target / name, original[name])
        raise
    print('Migration applied; all installed hashes verified.')


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    prep = commands.add_parser('prepare')
    for flag in ('source', 'qingjian', 'out'):
        prep.add_argument('--' + flag, type=Path, required=True)
    install = commands.add_parser('apply')
    install.add_argument('--plan', type=Path, required=True)
    args = parser.parse_args()
    if args.command == 'prepare':
        prepare(args.source.resolve(), args.qingjian.resolve(), args.out.resolve())
    else:
        apply(args.plan.resolve())


if __name__ == '__main__':
    main()
