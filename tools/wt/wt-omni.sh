#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"
: "${WT_OMNI_TIMEOUT:=120}"

wt_omni_nonempty() {
  local text="$1"
  [[ -n "${text//[$' \t\r\n']/}" ]]
}

if [[ -n "${WT_OMNI_CURL:-}" ]]; then
  out="$("$WT_OMNI_CURL")" || {
    echo "wt-omni: WT_OMNI_CURL failed" >&2
    exit 1
  }
  wt_omni_nonempty "$out" || { echo "wt-omni: empty fake output" >&2; exit 1; }
  printf '%s\n' "$out"
  exit 0
fi

export WT_OMNI_URL WT_OMNI_MODEL WT_OMNI_TIMEOUT
python3 - <<'PY'
import json, os, sys, urllib.error, urllib.request

url = os.environ["WT_OMNI_URL"]
model = os.environ["WT_OMNI_MODEL"]
timeout = float(os.environ.get("WT_OMNI_TIMEOUT", "120"))
prompt = sys.stdin.read()
body = json.dumps({
    "model": model,
    "stream": False,
    "messages": [{"role": "user", "content": prompt}],
}).encode()
req = urllib.request.Request(
    url,
    data=body,
    headers={"Content-Type": "application/json"},
    method="POST",
)
try:
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        raw = resp.read().decode("utf-8")
except urllib.error.HTTPError as exc:
    err = exc.read().decode("utf-8", errors="replace")
    print(f"wt-omni: HTTP {exc.code}: {err}", file=sys.stderr)
    sys.exit(1)
except Exception as exc:
    print(f"wt-omni: request failed: {type(exc).__name__}: {exc}", file=sys.stderr)
    sys.exit(1)

try:
    data = json.loads(raw)
    text = data["choices"][0]["message"]["content"]
except (KeyError, IndexError, json.JSONDecodeError, TypeError) as exc:
    print(f"wt-omni: bad JSON: {exc}", file=sys.stderr)
    sys.exit(1)

if not isinstance(text, str) or not text.strip():
    print("wt-omni: empty content", file=sys.stderr)
    sys.exit(1)
sys.stdout.write(text if text.endswith("\n") else text + "\n")
PY
