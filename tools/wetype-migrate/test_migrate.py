"""用合成字词验证解码、去重合并和失败回退，不包含私人词库。"""
from pathlib import Path
import json
import struct
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

import migrate
from wetype import exact_count, load_records, parse_records, syllables_from_repo, word_items


def envelope(data):
    return bytes((byte + 6) % 256 for byte in data) + struct.pack('<I', len(data)) + b'\x5a\xa5\x5a\xa5'


def item(word, count):
    text = word.encode()
    return b'\0' + struct.pack('<HII', len(text), count, 1780000000) + bytes(40) + text


def choice(exact, typo=0):
    return envelope(b'\1' + struct.pack('<7I', exact, 0, 0, 0, typo, 0, 1780000000) + bytes(36))


class DecoderTests(unittest.TestCase):
    def test_multiple_items_and_reject_corruption(self):
        value = envelope(item('绿洲', 11) + item('滤轴', 2))
        self.assertEqual(list(word_items(value)), [('绿洲', 11), ('滤轴', 2)])
        for bad in [value[:-1], value[:-8] + b'\0' * 8, envelope(item('绿洲', 1)[:-1]), envelope(b'\2' + item('绿洲', 1)[1:])]:
            with self.assertRaises(ValueError):
                list(word_items(bad))

    def test_exact_choices_exclude_typo_and_dont_double_count_base(self):
        records = {b'!v2u!lv,zhou': envelope(item('绿洲', 11)),
                   b'!u_d_v_p!lv,zhou\x01' + '绿洲'.encode(): choice(8, typo=200),
                   b'!u_d_v_p!lv,zhou\x01' + '滤轴'.encode(): choice(3),
                   b'!u_d_v_p!lv,zhou\x01' + '律州'.encode(): choice(0, typo=10)}
        frequencies, choices, _, stats = parse_records(records, {'lv', 'zhou'})
        self.assertEqual(frequencies, {'绿洲': 11, '滤轴': 3})
        self.assertEqual(choices, {('lv zhou', '绿洲'): 8, ('lv zhou', '滤轴'): 3})
        self.assertEqual(stats['patch_excluded'], 1)
        self.assertEqual(exact_count(choice(5, 999)), 5)

    def test_uses_only_complete_syllables_from_owning_table(self):
        syllables = syllables_from_repo(migrate.REPO)
        self.assertIn("zhou", syllables)
        self.assertFalse({"l", "zh", "ch", "sh"} & syllables)

    def test_excludes_incomplete_pinyin_and_non_chinese(self):
        data = {b'!v2u!l,zhou': envelope(item('绿洲', 10)), b'!v2u!lv,zhou': envelope(item('abc', 20))}
        self.assertFalse(parse_records(data, {'lv', 'zhou'})[0])


class ActiveRecordsTests(unittest.TestCase):
    def test_newest_record_and_tombstone_win_across_active_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ['CURRENT', 'MANIFEST-1', '1.ldb', '2.ldb', '3.log']:
                (root / name).write_bytes(b'fixture')
            key = b'!v2u!lv,zhou'
            deleted = b'!v2u!lv,se'
            def record(k, seq, kind, value):
                return SimpleNamespace(record=SimpleNamespace(key=k, sequence_number=seq, record_type=kind, value=value))
            source = {'1.ldb': [record(key, 1, 1, b'old'), record(deleted, 2, 1, b'deleted')],
                      '2.ldb': [record(key, 3, 1, b'new')], '3.log': [record(deleted, 4, 0, b'')]}
            with patch('dfindexeddb.leveldb.record.FolderReader') as folder, patch('dfindexeddb.leveldb.record.LevelDBRecord.FromFile', side_effect=lambda p: iter(source[p.name])), patch('dfindexeddb.leveldb.descriptor.FileReader') as descriptor:
                folder.return_value.GetLatestVersion.return_value = SimpleNamespace(active_files={0: {'1.ldb': None, '2.ldb': None}}, current_log='3.log')
                folder.return_value.GetCurrentManifestPath.return_value = root/'MANIFEST-1'
                descriptor.return_value.GetVersionEdits.return_value = []
                result, _ = load_records(root)
            self.assertEqual(result, {key: b'new'})


class MergeTests(unittest.TestCase):
    def test_keeps_current_reading_and_adds_existing_learning(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / 'user.tsv').write_text('绿洲\t4\n')
            (path / 'user-choices.tsv').write_text('lvzhou\t绿洲\t2\n')
            (path / 'user-words.tsv').write_text('绿洲\tlv zhou\t77\n')
            output = migrate.merge(path, {'绿洲': 11}, {('lv zhou', '绿洲'): 8}, {'绿洲': 'other reading'})
            self.assertIn('绿洲\t15\n', output['user.tsv'])
            self.assertIn('lvzhou\t绿洲\t10\n', output['user-choices.tsv'])
            self.assertIn('绿洲\tlv zhou\t77\n', output['user-words.tsv'])

    def test_apply_checks_drift_and_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'q'; target.mkdir(); out = root / 'out'; out.mkdir()
            for name in migrate.FILES:
                (target / name).write_text('old')
                (out / name).write_text('new')
            plan = {'format': 1, 'qingjian': str(target), 'before': {n: migrate.digest(target/n) for n in migrate.FILES},
                    'after': {n: migrate.digest(out/n) for n in migrate.FILES}}
            p = out / 'plan.json'; p.write_text(json.dumps(plan))
            with patch('migrate.subprocess.run') as run:
                run.return_value.returncode = 1
                (target / 'user.tsv').write_text('drift')
                with self.assertRaisesRegex(ValueError, 'changed'):
                    migrate.apply(p)
                self.assertEqual((target/'user-choices.tsv').read_text(), 'old')
                (target / 'user.tsv').write_text('old')
                migrate.apply(p)
                with self.assertRaisesRegex(ValueError, 'already applied'):
                    migrate.apply(p)
            self.assertTrue(all((target/n).read_text() == 'new' for n in migrate.FILES))
            self.assertTrue(all((out/'backup'/n).read_text() == 'old' for n in migrate.FILES))

    def test_write_failure_rolls_back_completed_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'q'; target.mkdir(); out = root / 'out'; out.mkdir()
            for name in migrate.FILES:
                (target / name).write_text('old'); (out / name).write_text('new')
            plan = {'format': 1, 'qingjian': str(target), 'before': {n: migrate.digest(target/n) for n in migrate.FILES},
                    'after': {n: migrate.digest(out/n) for n in migrate.FILES}}
            p = out/'plan.json'; p.write_text(json.dumps(plan))
            original_write = migrate.atomic_write
            def fail(path, data):
                if path.name == 'user-choices.tsv':
                    raise OSError('synthetic write failure')
                original_write(path, data)
            with patch('migrate.subprocess.run') as run, patch('migrate.atomic_write', side_effect=fail):
                run.return_value.returncode = 1
                with self.assertRaises(OSError):
                    migrate.apply(p)
            self.assertTrue(all((target/n).read_text() == 'old' for n in migrate.FILES))
            self.assertFalse((target/migrate.RECEIPT).exists())


if __name__ == '__main__':
    unittest.main()
