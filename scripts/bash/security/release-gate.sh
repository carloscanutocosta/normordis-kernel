#!/usr/bin/env bash
# Portão de segurança: manifesto, auditoria, licenças, build e relatório JSON.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OUT_DIR="${TRUST_OUT_DIR:-artifacts/trust}"
SKIP_AUDIT=0; SKIP_LICENSES=0; SKIP_BUILD=0; UPDATE_DB=0; ALLOW_UNMAINTAINED=0
usage() { printf '%s\n' 'Uso: bash scripts/bash/security/release-gate.sh [--output-dir DIR] [--skip-audit] [--skip-licenses] [--skip-build] [--update-advisory-db] [--allow-unmaintained]'; }
while (($#)); do
  case "$1" in
    --output-dir) OUT_DIR="${2:?Falta a pasta}"; shift ;;
    --skip-audit) SKIP_AUDIT=1 ;; --skip-licenses) SKIP_LICENSES=1 ;; --skip-build) SKIP_BUILD=1 ;;
    --update-advisory-db) UPDATE_DB=1 ;; --allow-unmaintained) ALLOW_UNMAINTAINED=1 ;;
    -h|--help) usage; exit 0 ;; *) printf 'Argumento desconhecido: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done
[[ "$OUT_DIR" = /* ]] || OUT_DIR="$ROOT/$OUT_DIR"
mkdir -p "$OUT_DIR"
cd "$ROOT"
start="$(date +%s)"
declare -a steps=() statuses=() details=()
add_step() { steps+=("$1"); statuses+=("$2"); details+=("$3"); }
run_step() { local name="$1" detail="$2"; shift 2; if "$@"; then add_step "$name" pass "$detail"; else add_step "$name" fail "$detail"; fi; }

printf 'RELEASE SECURITY GATE — normordis-kernel\nSaída: %s\n' "$OUT_DIR"
if ! bash "$SCRIPT_DIR/generate-manifest.sh" "$OUT_DIR"; then printf '%s\n' 'Falha crítica a gerar o manifesto.' >&2; exit 1; fi
add_step integrity-manifest pass "MANIFEST.sha256 gerado"
if (( SKIP_AUDIT )); then add_step dep-audit skipped 'Ignorado via --skip-audit'; else
  if UPDATE_DB="$UPDATE_DB" ALLOW_WARNINGS="$ALLOW_UNMAINTAINED" bash "$SCRIPT_DIR/audit-deps.sh" "$OUT_DIR"; then add_step dep-audit pass 'Sem vulnerabilidades conhecidas'; else add_step dep-audit fail 'cargo audit reportou problemas'; fi
fi
if (( SKIP_LICENSES )); then add_step license-check skipped 'Ignorado via --skip-licenses'; elif bash "$SCRIPT_DIR/check-licenses.sh" "$OUT_DIR"; then add_step license-check pass 'Licenças conformes'; else add_step license-check fail 'cargo deny reportou problemas'; fi
if (( SKIP_BUILD )); then add_step release-build skipped 'Ignorado via --skip-build'; elif cargo build --release --workspace; then add_step release-build pass 'Build concluído'; else add_step release-build fail 'cargo build falhou'; fi

status=pass; for value in "${statuses[@]}"; do [[ "$value" == fail ]] && status=fail; done
json_escape() { printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'; }
{
  printf '{\n  "schema_version": "1.0.0",\n  "tool": "release-gate",\n  "project": "normordis-kernel",\n  "status": "%s",\n' "$status"
  printf '  "generated_at": "%s",\n  "duration_s": %s,\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$(( $(date +%s) - start ))"
  printf '  "git": { "commit": "%s", "branch": "%s" },\n' "$(json_escape "$(git rev-parse HEAD 2>/dev/null || true)")" "$(json_escape "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)")"
  printf '  "steps": ['
  for i in "${!steps[@]}"; do (( i == 0 )) || printf ','; printf '\n    { "step": "%s", "status": "%s", "detail": "%s" }' "${steps[i]}" "${statuses[i]}" "$(json_escape "${details[i]}")"; done
  printf '\n  ],\n  "artifacts_dir": "%s"\n}\n' "$(json_escape "$OUT_DIR")"
} > "$OUT_DIR/release-report.json"
printf 'Relatório: %s\n' "$OUT_DIR/release-report.json"
[[ "$status" == pass ]]
