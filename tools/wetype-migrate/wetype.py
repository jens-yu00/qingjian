"""微信输入法 1.4.3 的本地个人词频解析；仅处理已核验的记录格式。"""
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
import hashlib
import re
import struct


@dataclass(frozen=True)
class Word:
    """规范拼音、词条及对应计数。"""
    pinyin: str
    text: str
    count: int


def unwrap(value: bytes) -> bytes:
    """尾部长度和魔数约束用于拒绝截断或不同格式的数据。"""
    if len(value) < 8 or value[-4:] != b'\x5a\xa5\x5a\xa5':
        raise ValueError('unsupported value envelope')
    if int.from_bytes(value[-8:-4], 'little') != len(value) - 8:
        raise ValueError('value length mismatch')
    return bytes((byte - 6) % 256 for byte in value[:-8])


def word_items(value: bytes):
    """WordItemV2：版本 + u16 词长 + 48 字节元数据 + UTF-8 词。"""
    data = unwrap(value)
    pos = 0
    while pos < len(data):
        if len(data) - pos < 51 or data[pos] != 0:
            raise ValueError('unsupported WordItemV2 version or truncated header')
        size = struct.unpack_from('<H', data, pos + 1)[0]
        end = pos + 51 + size
        if not size or end > len(data):
            raise ValueError('truncated WordItemV2 text')
        count = struct.unpack_from('<I', data, pos + 3)[0]
        text = data[pos + 51:end].decode('utf8')
        yield text, count
        pos = end


def exact_count(value: bytes) -> int:
    """Patch v1 的首个 u32 在原生 DebugInfo 中命名为 exact_match。"""
    data = unwrap(value)
    if len(data) != 65 or data[0] != 1:
        raise ValueError('unsupported UserDictV2Patch version or length')
    return struct.unpack_from('<I', data, 1)[0]


def valid_word(code: str, text: str, count: int, syllables: set[str]):
    if not count:
        return None
    parts = code.split(',')
    if (len(parts) != len(text) or not all(p in syllables for p in parts)
            or not all('\u3400' <= c <= '\u9fff' for c in text)):
        return None
    return Word(' '.join(parts), text, count)


def parse_records(records: dict[bytes, bytes], syllables: set[str]):
    base, choices = {}, {}
    stats = Counter()
    for key, value in records.items():
        if key.startswith(b'!v2u!'):
            code = key[5:].decode('utf8')
            for text, count in word_items(value):
                stats['base_items'] += 1
                item = valid_word(code, text, count, syllables)
                if item is None:
                    stats['base_excluded'] += 1
                    continue
                pair = (item.pinyin, item.text)
                if pair in base:
                    raise ValueError('duplicate normalized base record')
                base[pair] = item.count
        elif key.startswith(b'!u_d_v_p!'):
            code, text = key[len(b'!u_d_v_p!'):].decode('utf8').split('\x01')
            stats['patch_items'] += 1
            item = valid_word(code, text, exact_count(value), syllables)
            if item is None:
                stats['patch_excluded'] += 1
                continue
            pair = (item.pinyin, item.text)
            if pair in choices:
                raise ValueError('duplicate normalized choice record')
            choices[pair] = item.count
    # 两张微信表覆盖同一段历史，取较大计数，不相加重复计数。
    frequencies = Counter()
    readings = {}
    for pinyin, text in base.keys() | choices.keys():
        count = max(base.get((pinyin, text), 0), choices.get((pinyin, text), 0))
        frequencies[text] += count
        previous = readings.get(text)
        if previous is None or (count, pinyin) > previous:
            readings[text] = (count, pinyin)
    stats.update({'imported_words': len(frequencies), 'imported_choices': len(choices)})
    return dict(frequencies), choices, {text: pinyin for text, (_, pinyin) in readings.items()}, dict(stats)


def load_records(folder: Path):
    """只读 CURRENT 指定的活跃文件，以最新序号裁定覆盖和删除。"""
    from dfindexeddb.leveldb.record import FolderReader, LevelDBRecord
    from dfindexeddb.leveldb import descriptor
    reader = FolderReader(folder)
    version = reader.GetLatestVersion()
    files = {folder / name for level in version.active_files.values() for name in level}
    if version.current_log:
        files.add(folder / version.current_log)
    previous_log = 0
    for edit in descriptor.FileReader(str(reader.GetCurrentManifestPath())).GetVersionEdits():
        if edit.prev_log_number is not None:
            previous_log = edit.prev_log_number
    if previous_log:
        raise ValueError('previous WAL requires explicit recovery validation')
    files.update({folder / 'CURRENT', reader.GetCurrentManifestPath()})
    hashes = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in files}
    latest = {}
    total = 0
    for path in sorted(files):
        if path.suffix not in ('.ldb', '.log'):
            continue
        for wrapper in LevelDBRecord.FromFile(path):
            record = wrapper.record
            total += 1
            if not record.key.startswith((b'!v2u!', b'!u_d_v_p!')):
                continue
            old = latest.get(record.key)
            if old is None or record.sequence_number > old[0]:
                latest[record.key] = (record.sequence_number, int(record.record_type), record.value)
    if any(hashlib.sha256(path.read_bytes()).hexdigest() != hashes[path.name] for path in files):
        raise ValueError('source changed while reading')
    records = {key: value for key, (_, kind, value) in latest.items() if kind == 1}
    return records, {'active_records_scanned': total, 'files': hashes}


def syllables_from_repo(repo: Path) -> set[str]:
    source = (repo / 'crates/qingjian-core/src/parser/syllable.rs').read_text()
    table = re.search(r'pub const SYLLABLES: &\[&str\] = &\[(.*?)\];', source, re.S)
    if table is None:
        raise ValueError('Qingjian syllable table declaration changed')
    return set(re.findall(r'"([a-z]+)"', table.group(1)))
