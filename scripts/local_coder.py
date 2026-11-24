import urllib.request
import json
import time
import sys
import os

PROXY_URL = "http://172.18.26.176:11888/api/generate"
DEFAULT_MODEL = "qwen2.5-coder:7b"

def ask_local_model(prompt: str, model: str = DEFAULT_MODEL, timeout: int = 180) -> str:
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "options": {
            "temperature": 0.2,
            "num_predict": 2048,
        }
    }
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(PROXY_URL, data=data, headers={"Content-Type": "application/json"})
    t0 = time.time()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with opener.open(req, timeout=timeout) as resp:
        res = json.loads(resp.read().decode("utf-8"))
        dt = time.time() - t0
        prompt_tokens = res.get("prompt_eval_count", 0)
        eval_tokens = res.get("eval_count", 0)
        speed = (eval_tokens / dt) if dt > 0 else 0
        print(f"[Local Coder - {model}] Tempo: {dt:.2f}s | Prompt tokens: {prompt_tokens} | Gen tokens: {eval_tokens} ({speed:.1f} tok/s)", file=sys.stderr)
        return res.get("response", "")

if __name__ == "__main__":
    test_prompt = sys.argv[1] if len(sys.argv) > 1 else "Responda apenas: 'Local Coder pronto!'"
    print(ask_local_model(test_prompt))
