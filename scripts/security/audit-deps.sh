#!/bin/sh
# ============================================================
#  Audit Dependencies — normordis-kernel
#  Verifica vulnerabilidades conhecidas (RustSec Advisory DB)
#  via `cargo audit`. Gera relatório JSON em artifacts/trust/.
#
#  Uso:
#    ./audit-deps.sh
#    ./audit-deps.sh artifacts/trust
#    UPDATE_DB=1 ./audit-deps.sh      # actualiza advisory-db
#    ALLOW_WARNINGS=1 ./audit-deps.sh # não falha em unmaintained
# ============================================================
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="${1:-${TRUST_OUT_DIR:-artifacts/trust}}"
UPDATE_DB="${UPDATE_DB:-0}"
ALLOW_WARNINGS="${ALLOW_WARNINGS:-0}"

case "$OUT_DIR" in
  /*) : ;;
  *)  OUT_DIR="$ROOT/$OUT_DIR" ;;
esac

mkdir -p "$OUT_DIR"
REPORT="$OUT_DIR/audit-report.json"

cd "$ROOT"

printf '\n'
printf '  Raiz      : %s\n' "$ROOT"
printf '  Relatorio : %s\n' "$REPORT"
printf '  Data      : %s\n' "$(date -u '+%Y-%m-%d %H:%M:%S')"
printf '\n'

# ─── Verificar cargo-audit ──────────────────────────────────
printf '  [1/3] A verificar cargo-audit...\n'

if ! command -v cargo-audit >/dev/null 2>&1 && ! cargo audit --version >/dev/null 2>&1; then
  printf '  [!] cargo-audit nao encontrado. A instalar...\n'
  cargo install cargo-audit --locked
fi

AUDIT_VER=$(cargo audit --version 2>&1 | head -1)
printf '  [+] %s\n' "$AUDIT_VER"

# ─── Actualizar advisory-db ─────────────────────────────────
if [ "$UPDATE_DB" = "1" ]; then
  printf '\n  [2/3] A actualizar advisory-db...\n'
  cargo audit fetch || printf '  [!] Falha ao actualizar advisory-db\n'
else
  printf '  [2/3] A usar advisory-db local (UPDATE_DB=1 para refrescar)\n'
fi

# ─── Executar auditoria ─────────────────────────────────────
printf '\n  [3/3] A auditar dependencias (RustSec)...\n\n'

cargo audit --json > "$REPORT" 2>&1 || true
cargo audit
AUDIT_EXIT=$?

printf '\n'

if [ "$AUDIT_EXIT" -ne 0 ]; then
  # Verificar se é apenas "unmaintained" (sem vulnerabilidades reais)
  VULN_COUNT=0
  if command -v python3 >/dev/null 2>&1; then
    VULN_COUNT=$(python3 -c "
import json, sys
try:
    d = json.load(open('$REPORT'))
    print(len(d.get('vulnerabilities', {}).get('list', [])))
except: print(0)
" 2>/dev/null || echo 0)
  fi

  if [ "$ALLOW_WARNINGS" = "1" ] && [ "$VULN_COUNT" = "0" ]; then
    printf '  OK  AUDITORIA CONCLUIDA (avisos ignorados por ALLOW_WARNINGS=1)\n'
    printf '\n'
    printf '  Relatorio JSON : %s\n' "$REPORT"
    printf '\n'
    exit 0
  fi

  printf '  FALHA  AUDITORIA FALHOU\n' >&2
  printf '\n'
  printf '  Relatorio JSON : %s\n' "$REPORT"
  printf '\n'
  exit 1
fi

printf '  OK  AUDITORIA CONCLUIDA SEM VULNERABILIDADES\n'
printf '\n'
printf '  Relatorio JSON : %s\n' "$REPORT"
printf '\n'
