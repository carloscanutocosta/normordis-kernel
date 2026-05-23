#!/bin/sh
# ============================================================
#  Generate Integrity Manifest — normordis-kernel
#  Gera MANIFEST.sha256 e MANIFEST.json com hashes SHA-256.
#
#  Uso:
#    ./generate-manifest.sh
#    ./generate-manifest.sh artifacts/trust
#
#  Variável de ambiente alternativa: TRUST_OUT_DIR
# ============================================================
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="${1:-${TRUST_OUT_DIR:-artifacts/trust}}"

# Resolver caminho relativo à raiz do repositório
case "$OUT_DIR" in
  /*) : ;;
  *)  OUT_DIR="$ROOT/$OUT_DIR" ;;
esac

SHA_FILE="$OUT_DIR/MANIFEST.sha256"
JSON_FILE="$OUT_DIR/MANIFEST.json"
TMP_FILE="$OUT_DIR/.manifest-files.tmp"

mkdir -p "$OUT_DIR"

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    openssl dgst -sha256 "$1" | awk '{print $NF}'
  fi
}

json_escape() {
  sed 's/\\/\\\\/g; s/"/\\"/g'
}

printf '\n'
printf '  Raiz  : %s\n' "$ROOT"
printf '  Saída : %s\n' "$OUT_DIR"
printf '  Data  : %s UTC\n' "$(date -u '+%Y-%m-%d %H:%M:%S')"
printf '\n'
printf '  [1/2] A calcular hashes SHA-256...\n'

cd "$ROOT"

# Excluir directórios de artefactos e dados locais
find . \
  \( -path './.git'       -o -path './.git/*' \
    -o -path './.vs'       -o -path './.vs/*' \
    -o -path './target'    -o -path './target/*' \
    -o -path './.logs'     -o -path './.logs/*' \
    -o -path './artifacts' -o -path './artifacts/*' \
    -o -path './tmp'       -o -path './tmp/*' \
    -o -path './_backups'  -o -path './_backups/*' \
    -o -path "./$SHA_FILE" \
    -o -path "./$JSON_FILE" \
    -o -path "./$TMP_FILE" \) -prune \
  -o -type f -print | LC_ALL=C sort > "$TMP_FILE"

: > "$SHA_FILE"
{
  printf '{\n'
  printf '  "schema_version": "1.0.0",\n'
  printf '  "algorithm": "SHA-256",\n'
  printf '  "generated_at": "%s",\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  printf '  "root": "%s",\n' "$ROOT"
  printf '  "files": [\n'
} > "$JSON_FILE"

first=1
count=0
while IFS= read -r file; do
  path="${file#./}"
  hash=$(hash_file "$file")
  printf '%s  %s\n' "$hash" "$path" >> "$SHA_FILE"

  escaped_path=$(printf '%s' "$path" | json_escape)
  if [ "$first" -eq 1 ]; then
    first=0
  else
    printf ',\n' >> "$JSON_FILE"
  fi
  printf '    { "path": "%s", "sha256": "%s" }' "$escaped_path" "$hash" >> "$JSON_FILE"
  count=$((count + 1))
done < "$TMP_FILE"

{
  printf '\n'
  printf '  ],\n'
  printf '  "file_count": %d\n' "$count"
  printf '}\n'
} >> "$JSON_FILE"

rm -f "$TMP_FILE"

printf '  [2/2] Manifesto gravado\n'
printf '\n'
printf '  OK  MANIFESTO GERADO COM SUCESSO\n'
printf '\n'
printf '  SHA-256 : %s\n' "$SHA_FILE"
printf '  JSON    : %s\n' "$JSON_FILE"
printf '  Total   : %d ficheiros\n' "$count"
printf '\n'
