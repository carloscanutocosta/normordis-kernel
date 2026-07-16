#!/usr/bin/env bash
# Build Release — normordis-kernel
# Pipeline: fmt -> clippy -> testes -> build release.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SKIP_TESTS=0
WITH_DOCS=0
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/normordis-kernel/target}"

usage() {
  cat <<'EOF'
Uso: bash scripts/bash/build-release.sh [--skip-tests] [--with-docs]
                                          [--cargo-target-dir PASTA]

O cache de compilação é externo ao repositório por omissão:
  ~/.cache/normordis-kernel/target
EOF
}

while (($#)); do
  case "$1" in
    --skip-tests) SKIP_TESTS=1 ;;
    --with-docs) WITH_DOCS=1 ;;
    --cargo-target-dir) CARGO_TARGET_DIR="${2:?Falta a pasta após --cargo-target-dir}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Argumento desconhecido: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

command -v cargo >/dev/null 2>&1 || { printf 'cargo não encontrado. Instala Rust em https://rustup.rs\n' >&2; exit 1; }

mkdir -p "$ROOT/.logs"
mkdir -p "$CARGO_TARGET_DIR"
LOG_FILE="$ROOT/.logs/build-release-$(date -u '+%Y%m%d-%H%M%S').log"
exec > >(tee -a "$LOG_FILE") 2>&1
cd "$ROOT"

printf '\nBUILD RELEASE — normordis-kernel\nRaiz: %s\nCache Cargo: %s\nLog: %s\n\n' "$ROOT" "$CARGO_TARGET_DIR" "$LOG_FILE"
printf '>>> 1. A verificar ambiente...\n'; rustc --version; cargo --version
printf '\n>>> 2. A verificar formatação (cargo fmt)...\n'; cargo fmt --all -- --check
printf '\n>>> 3. A analisar código (cargo clippy)...\n'; CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo clippy --workspace --all-targets -- -D warnings
if (( SKIP_TESTS )); then
  printf '\n>>> 4. Testes ignorados (--skip-tests)\n'
else
  printf '\n>>> 4. A correr testes (cargo test)...\n'; CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo test --workspace
fi
printf '\n>>> 5. A compilar release (cargo build --release)...\n'; CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo build --release --workspace
if (( WITH_DOCS )); then
  printf '\n>>> 6. A gerar documentação (cargo doc)...\n'; CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo doc --workspace --no-deps --document-private-items
fi
printf '\nBUILD RELEASE CONCLUÍDO COM SUCESSO\nArtefactos: %s/release\n' "$CARGO_TARGET_DIR"
