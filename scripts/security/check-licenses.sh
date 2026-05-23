#!/bin/sh
# ============================================================
#  Check License Compliance — normordis-kernel
#  Verifica se todas as dependências têm licenças compatíveis
#  com EUPL-1.2, usando `cargo deny`.
#
#  Uso:
#    ./check-licenses.sh
#    ./check-licenses.sh artifacts/trust
#    CHECK_ALL=1 ./check-licenses.sh    # inclui advisories e bans
# ============================================================
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="${1:-${TRUST_OUT_DIR:-artifacts/trust}}"
CHECK_ALL="${CHECK_ALL:-0}"

case "$OUT_DIR" in
  /*) : ;;
  *)  OUT_DIR="$ROOT/$OUT_DIR" ;;
esac

mkdir -p "$OUT_DIR"
REPORT="$OUT_DIR/license-report.txt"

cd "$ROOT"

printf '\n'
printf '  Raiz      : %s\n' "$ROOT"
printf '  Config    : deny.toml\n'
printf '  Relatorio : %s\n' "$REPORT"
printf '  Data      : %s\n' "$(date -u '+%Y-%m-%d %H:%M:%S')"
printf '\n'

# ─── Verificar deny.toml ────────────────────────────────────
if [ ! -f "$ROOT/deny.toml" ]; then
  printf '  [ERRO] deny.toml nao encontrado em %s\n' "$ROOT" >&2
  exit 1
fi

# ─── Verificar cargo-deny ───────────────────────────────────
printf '  [1/2] A verificar cargo-deny...\n'

if ! cargo deny --version >/dev/null 2>&1; then
  printf '  [!] cargo-deny nao encontrado. A instalar...\n'
  cargo install cargo-deny --locked
fi

DENY_VER=$(cargo deny --version 2>&1 | head -1)
printf '  [+] %s\n' "$DENY_VER"

# ─── Executar verificação ───────────────────────────────────
printf '\n  [2/2] A verificar conformidade de licencas...\n\n'

if [ "$CHECK_ALL" = "1" ]; then
  CHECKS="all"
  printf '  [+] Modo completo: licencas + advisories + bans\n'
else
  CHECKS="licenses"
fi

cargo deny check "$CHECKS" 2>&1 | tee "$REPORT"
DENY_EXIT=${PIPESTATUS:-$?}

printf '\n'

if [ "$DENY_EXIT" -ne 0 ]; then
  printf '  FALHA  VERIFICACAO DE LICENCAS FALHOU\n' >&2
  printf '\n'
  printf '  Relatorio : %s\n' "$REPORT"
  printf '  Config    : %s/deny.toml\n' "$ROOT"
  printf '\n'
  exit 1
fi

printf '  OK  LICENCAS EM CONFORMIDADE COM EUPL-1.2\n'
printf '\n'
printf '  Relatorio : %s\n' "$REPORT"
printf '\n'
