"""Sol 的本机 JSONL 工作进程；只返回决策 logits，不生成文本、不落盘输入。"""
import json
import math
import os
import queue
import sys
import threading
from pathlib import Path

INSTRUCTION = '根据已经输入的中文上文，选择最适合紧接在上文之后的候选词。只判断接续是否自然且语义正确，不解释词义，不优先选择列表靠前的词。所有选项均来自当前拼音的候选。'


def main():
    # stdin 关闭意味着输入法退出；即使 Metal 正忙也及时回收常驻模型。
    inbox = queue.Queue(maxsize=1)

    def read_requests():
        while True:
            line = sys.stdin.buffer.readline(65537)
            if not line or len(line) > 65536:
                os._exit(0)
            inbox.put(line)

    threading.Thread(target=read_requests, daemon=True).start()
    model_path = Path(__file__).resolve().parent / 'model'
    sys.path.insert(0, str(model_path))
    import model as runtime
    mx = runtime.mx
    mx.set_cache_limit(256 * 1024 * 1024)
    model, config, tokenizer = runtime.load_bundle(str(model_path), verify=True)
    if config['source_repo'] != 'vllm-sr/Decision-2.0-Sol-2B':
        raise ValueError('wrong model')

    def decide(request):
        context, pinyin, texts = request['context'], request['pinyin'], request['texts']
        if (not isinstance(context, str) or not 1 <= len(context) <= 64
                or not isinstance(pinyin, str) or not 1 <= len(pinyin) <= 256
                or not isinstance(texts, list) or not 2 <= len(texts) <= 12
                or not all(isinstance(t, str) and 1 <= len(t) <= 64 for t in texts)
                or len(set(texts)) != len(texts)):
            raise ValueError('invalid request')
        payload = {
            'task_type': 'choice',
            'state': {'上文': context, '当前拼音': pinyin},
            'instructions': INSTRUCTION,
            'options': [{'key': str(i), 'description': text} for i, text in enumerate(texts)],
        }
        encoded = runtime.encode(payload, tokenizer, config['max_input_tokens'])
        logits = model(mx.array([encoded['ids']], mx.int32),
                       encoded['candidate_positions'], encoded['query_position'])
        mx.eval(logits)
        scores = logits[0].tolist()
        if len(scores) != len(texts) or not all(math.isfinite(s) for s in scores):
            raise ValueError('invalid scores')
        return scores

    decide({'context': '炒菜时加入少量', 'pinyin': 'shiyan', 'texts': ['实验', '食盐']})
    print(json.dumps({'ready': True}), flush=True)
    while True:
        try:
            scores = decide(json.loads(inbox.get()))
        except Exception:
            scores = []
        print(json.dumps({'scores': scores}, allow_nan=False), flush=True)


if __name__ == '__main__':
    main()
