#!/bin/sh
# ============================================================
#  Verify Integrity Manifest — normordis-kernel
#  Verifica MANIFEST.sha256 gerado por generate-manifest.sh.
#  Exit code != 0 se qualquer hash falhar ou ficheiro em falta.
#
#  Uso:
#    ./verify-manifest.sh
#    ./verify-manifest.sh artifacts/trust/MANIFEST.sha256
#    TRUST_MANIFEST=artifacts/trust/MANIFEST.sha256 ./verify-manifest.sh
# ============================================================
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MANIFEST="${1:-${TRUST_MANIFEST:-artifacts/trust/MANIFEST.sha256}}"

case "$MANIFEST" in
  /*) : ;;
  *)  MANIFEST="$ROOT/$MANIFEST" ;;
esac

printf '\n'
printf '  Manifesto : %s\n' "$MANIFEST"
printf '  Data      : %s\n' "$(date -u '+%Y-%m-%d %H:%M:%S')"
printf '\n'

if [ ! -f "$MANIFEST" ]; then
  printf '  [ERRO] Manifesto nao encontrado: %s\n' "$MANIFEST" >&2
  printf '         Corre primeiro: ./generate-manifest.sh\n' >&2
  printf '\n'
  exit 2
fi

# Preferir sha256sum nativo (Linux) — verifica tudo de uma vez
if command -v sha256sum >/dev/null 2>&1; then
  cd "$ROOT"
  if sha256sum -c "$MANIFEST" --quiet 2>/dev/null; then
    count=$(grep -c '  ' "$MANIFEST" || true)
    printf '  OK  MANIFESTO VERIFICADO COM SUCESSO\n'
    printf '\n'
    printf '  Ficheiros verificados : %d\n' "$count"
    printf '  Manifesto             : %s\n' "$MANIFEST"
    printf '\n'
    exit 0
  else
    printf '  FALHA  VERIFICACAO FALHOU\n' >&2
    sha256sum -c "$MANIFEST" 2>&1 | grep -v ': OK$' >&2 || true
    printf '\n'
    exit 1
  fi
fi

# Fallback: shasum (macOS) ou openssl
failures=0
verified=0

while IFS= read -r line; do
  [ -z "$line" ] && continue

  expected="${line%%  *}"
  repopath="${line#*  }"
  filepath="$ROOT/$repopath"

  if [ ! -f "$filepath" ]; then
    printf '  FALTA  %s\n' "$repopath" >&2
    failures=$((failures + 1))
    continue
  fi

  if command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$filepath" | awk '{print $1}')
  else
    actual=$(openssl dgst -sha256 "$filepath" | awk '{print $NF}')
  fi

  if [ "$actual" = "$expected" ]; then
    verified=$((verified + 1))
  else
    printf '  ADULTERADO  %s\n' "$repopath" >&2
    failures=$((failures + 1))
  fi
done < "$MANIFEST"

if [ "$failures" -gt 0 ]; then
  printf '  FALHA  VERIFICACAO FALHOU (%d problema(s))\n' "$failures" >&2
  printf '\n'
  exit 1
fi

printf '  OK  MANIFESTO VERIFICADO COM SUCESSO\n'
printf '\n'
printf '  Ficheiros verificados : %d\n' "$verified"
printf '  Manifesto             : %s\n' "$MANIFEST"
printf '\n'
