#!/usr/bin/env bash
# T05: измерение релизных артефактов (размер WASM/JS, raw/gzip).
# Использование: tools/bench.sh            # сборка + замер
#                tools/bench.sh --no-build # замер существующего dist/
set -euo pipefail
cd "$(dirname "$0")/.."

# Тулчейн с wasm32-таргетом (см. 00-physics-spec §6 «Окружение»).
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"

if [[ "${1:-}" != "--no-build" ]]; then
  (cd web && trunk build --release)
fi

WASM=$(ls dist/*_bg.wasm | head -1)
JS=$(ls dist/*.js | head -1)

size() { stat -f %z "$1"; }
mb() { awk -v b="$1" 'BEGIN { printf "%.2f", b / 1048576 }'; }
kb() { awk -v b="$1" 'BEGIN { printf "%.0f", b / 1024 }'; }

W_RAW=$(size "$WASM"); W_GZ=$(gzip -9 -kc "$WASM" | wc -c)
J_RAW=$(size "$JS");   J_GZ=$(gzip -9 -kc "$JS" | wc -c)

echo "== $(date '+%Y-%m-%d %H:%M') =="
echo "wasm raw : $(mb "$W_RAW") МБ"
echo "wasm gzip: $(mb "$W_GZ") МБ   (бюджет ≤ 8 МБ)"
echo "js  raw  : $(kb "$J_RAW") КБ"
echo "js  gzip : $(kb "$J_GZ") КБ"

# Brotli — если установлен (реальный CDN обычно отдаёт brotli).
if command -v brotli >/dev/null 2>&1; then
  W_BR=$(brotli -c "$WASM" | wc -c)
  echo "wasm brotli: $(mb "$W_BR") МБ"
else
  echo "(brotli не установлен — оценка пропущена; apt/brew install brotli)"
fi

# Напоминание о профиле (см. Cargo.toml [profile.release]).
grep -A 2 'opt-level' Cargo.toml | head -4
