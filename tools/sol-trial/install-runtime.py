"""把已验证的 Sol bundle 与固定 MLX 环境安装到用户数据目录，不改输入源。"""
import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--destination', type=Path,
                        default=Path.home() / 'Library/Application Support/Qingjian/sol')
    args = parser.parse_args()
    source = args.bundle.resolve()
    destination = args.destination.resolve()
    # load_bundle 再核对内部每个文件的清单；此处固定清单本身以钉住实测版本。
    manifest = source / 'MLX_MANIFEST.json'
    expected = 'aa8d9f269f69dfa8b58335ea698ff2601d205f40e23390f11adc7dd2a09304ba'
    if hashlib.sha256(manifest.read_bytes()).hexdigest() != expected:
        raise ValueError('unexpected Sol bundle revision')
    config = json.loads((source / 'config.json').read_text())
    if config['source_repo'] != 'vllm-sr/Decision-2.0-Sol-2B':
        raise ValueError('expected Sol 2B')
    destination.mkdir(parents=True, exist_ok=True)
    subprocess.run(['uv', 'venv', '--python', '3.12', str(destination / 'venv')], check=True)
    subprocess.run(['uv', 'pip', 'sync', '--python', str(destination / 'venv/bin/python'),
                    str(Path(__file__).with_name('requirements.txt'))], check=True)
    shutil.copytree(source, destination / 'model', dirs_exist_ok=True)
    shutil.copy2(Path(__file__).with_name('worker.py'), destination / 'worker.py')
    print(destination)


if __name__ == '__main__':
    main()
